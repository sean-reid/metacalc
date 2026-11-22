//! The probe app probe/jobcheck.z80, built around a library and run on the stub OS: it keeps its
//! table in MCJOBT, shows a result, pauses on ON and carries on at a second run. The text it
//! prints is collected at its putchar and compared with what the model's counts give.

use crate::damage::{damaged, synthetic};
use crate::rig::*;
use mcemu::{Machine, Reached, STUB_ROM, Stop};
use std::path::{Path, PathBuf};
use std::process::Command;

const SLICE: u32 = 2_000_000;
/// T-states without a character after which the app is waiting for its key.
const QUIET: u64 = 40_000_000;
const TITLE: &str = "jobchk v1";

pub struct Probe {
    pub m: Machine,
    lab: PathBuf,
}

pub fn build(name: &str, library: &[u8]) -> (Vec<u8>, PathBuf) {
    let probe = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../probe");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let lib = dir.join("library.mclib");
    std::fs::write(&lib, library).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let make = Command::new("make")
        .args(["-s", "-C"])
        .arg(&probe)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", dir.display()))
        .arg(format!("LIBRARY={}", lib.display()))
        .arg("jobchk")
        .output()
        .expect("make runs");
    assert!(
        make.status.success(),
        "make: {}{}",
        String::from_utf8_lossy(&make.stdout),
        String::from_utf8_lossy(&make.stderr)
    );
    let check = Command::new("sh")
        .arg(probe.join("../app/scripts/check-8xk.sh"))
        .args(["-p", "3", "-l"])
        .arg(&lib)
        .arg(dir.join("jobchk.8xk"))
        .arg("jobchk")
        .output()
        .expect("the checker runs");
    assert!(
        check.status.success(),
        "check-8xk: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    (
        std::fs::read(dir.join("jobchk.8xk")).unwrap(),
        dir.join("jobchk.lab"),
    )
}

pub fn label_of(lab: &Path, name: &str) -> u16 {
    let text = std::fs::read_to_string(lab).unwrap();
    text.lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// When ON goes down, and for how long, as T-states into a run.
pub type Press = Option<(u32, u32)>;

impl Probe {
    pub fn new(name: &str, library: &[u8]) -> Probe {
        let (app, lab) = build(name, library);
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&app, BASE).expect("the app installs");
        Probe { m, lab }
    }

    pub fn label(&self, name: &str) -> u16 {
        label_of(&self.lab, name)
    }

    pub fn byte(&mut self, name: &str) -> u8 {
        let at = self.label(name);
        self.m.read_ram(at, 1)[0]
    }

    pub fn launch(&mut self) {
        self.m.launch(BASE).expect("the app launches");
    }

    /// Runs the app until it waits for its key and returns every character it passed to putchar.
    /// The stub has no _NewLine: a new line is taken at putchar's entry, as the OS call that
    /// destroys every register. ON goes down and up as `press` says.
    pub fn run(&mut self, press: Press) -> Vec<u8> {
        let putchar = self.label("putchar");
        let (mut text, mut quiet, mut elapsed) = (Vec::new(), 0u64, 0u64);
        let (mut down, mut up) = (false, false);
        while quiet < QUIET {
            if let Some((at, hold)) = press {
                if !down && elapsed >= u64::from(at) {
                    self.m.press_key(Machine::KEY_ON);
                    down = true;
                }
                if down && !up && elapsed >= u64::from(at) + u64::from(hold) {
                    self.m.release_key(Machine::KEY_ON);
                    up = true;
                }
            }
            let start = self.m.clock();
            match self.m.run_to(BASE, putchar, SLICE) {
                Reached::Target => {
                    let mut regs = self.m.regs();
                    text.push((regs.af >> 8) as u8);
                    quiet = 0;
                    if text.last() == Some(&0x0A) {
                        let at = usize::from(regs.sp);
                        let ret = self.m.read_ram(at as u16, 2);
                        regs.pc = u16::from_le_bytes([ret[0], ret[1]]);
                        regs.sp += 2;
                        (regs.bc, regs.de, regs.hl) = (0x1234, 0x5678, 0x9ABC);
                        self.m.set_regs(&regs);
                    } else {
                        self.m.resume(1);
                    }
                }
                Reached::Vector38 | Reached::Limit => {}
            }
            let spent = u64::from(self.m.clock().wrapping_sub(start));
            quiet += spent;
            elapsed += spent;
        }
        text
    }

    /// Any key leaves the app.
    pub fn leave(&mut self) {
        self.m.press_key(0x09);
        let run = self.m.resume(SLICE * 5);
        assert_eq!(run.stop, Stop::Exited, "a key leaves the probe");
        self.m.release_key(0x09);
    }

    pub fn table(&mut self) -> Option<Vec<u8>> {
        let var = self.m.read_var(VAR)?;
        Some(crate::layout::decode(&var.data).expect("it decodes").values)
    }
}

/// The characters as a 16 by 8 screen: the lines after the last clear, and the lines the app
/// had drawn before it.
pub fn screens(text: &[u8]) -> (Vec<String>, Vec<String>) {
    let clear = text.iter().rposition(|&c| c == 0x0C).expect("a clear");
    (replay(&text[..clear]), replay(&text[clear..]))
}

fn replay(text: &[u8]) -> Vec<String> {
    let mut rows = vec![vec![b' '; 16]; 8];
    let (mut row, mut col) = (0, 0);
    for &c in text {
        match c {
            0x0C => {
                rows = vec![vec![b' '; 16]; 8];
                (row, col) = (0, 0);
            }
            0x0A => (row, col) = (row + 1, 0),
            0x0D => col = 0,
            c => {
                assert!(col < 16 && row < 8, "text ran off the screen");
                rows[row][col] = c;
                col += 1;
            }
        }
    }
    let mut lines: Vec<String> = rows
        .iter()
        .map(|r| String::from_utf8_lossy(r).trim_end().to_string())
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// The result screen the counts of a table give: checked counts what store.inc calls checked,
/// n1 + n3, and resting the part of it that rests.
pub fn expected(kept: &str, table: &[u8], first: Option<(u16, u8)>, stop: &str) -> Vec<String> {
    let n = |v: u8| table.iter().filter(|&&w| w == v).count();
    let mut lines = vec![
        TITLE.to_string(),
        format!("table {kept}"),
        format!("checked {:05}", n(1) + n(3)),
        format!("failed {:05}", n(2)),
        format!("resting {:05}", n(3)),
    ];
    if let Some((id, status)) = first {
        lines.push(format!("first {id:05}"));
        lines.push(format!("status {status:05}"));
    }
    lines.push(stop.to_string());
    lines
}

pub fn library() -> (Vec<u8>, Vec<u8>) {
    let mut seed = 0x0B5E_55ED;
    let bytes = synthetic(10, 40, &mut seed);
    let (copy, passes) = damaged(&bytes, &mut seed).expect("a damaged copy");
    (copy.clone(), marks(&copy, &passes))
}

#[test]
fn the_probe_makes_its_table_checks_the_library_and_prints_what_the_counts_give() {
    let (bytes, want) = library();
    let count = want.len();
    let first = failures(&bytes).first().copied();
    assert!(first.is_some() && want.contains(&3), "a library with both");
    let mut p = Probe::new("jobchk-run", &bytes);
    p.launch();
    let text = p.run(None);
    assert!(p.m.read_var("MCSTAT").is_none(), "MCSTAT is not touched");
    assert_eq!(p.table().expect("the table is made"), want);
    let (progress, result) = screens(&text);
    assert_eq!(result, expected("made", &want, first, "done"));
    assert_eq!(
        progress,
        [TITLE.to_string(), format!("chk {count:05}/{count:05}")]
    );
    let shown = |needle: &str| String::from_utf8_lossy(&text).matches(needle).count();
    assert_eq!(shown("\rchk "), 1 + (count - 1) / 16 + 1, "a line a tick");
    assert_eq!(
        shown("\rcrc 00000/00001"),
        2,
        "before the run and after its begin"
    );
    assert_eq!(p.byte("J_STOP"), 0);
    p.leave();

    p.launch();
    let text = p.run(None);
    assert_eq!(p.table().unwrap(), want, "a second run changes nothing");
    let (_, result) = screens(&text);
    assert_eq!(result, expected("kept", &want, None, "done"));
    assert_eq!(String::from_utf8_lossy(&text).matches("\rcrc ").count(), 0);
    p.leave();
}

#[test]
fn on_pauses_the_probe_and_a_second_run_carries_on() {
    let (bytes, want) = library();
    let fails = failures(&bytes);
    let (mut paused_with_a_table, mut paused_in_the_crc) = (0, 0);
    for press_at in [
        200_000, 1_000_000, 2_500_000, 3_000_000, 3_500_000, 4_500_000, 6_000_000,
    ] {
        let mut p = Probe::new("jobchk-pause", &bytes);
        p.launch();
        let text = p.run(Some((press_at, 3_000_000)));
        let (_, result) = screens(&text);
        if result.last().map(String::as_str) != Some("paused") {
            continue;
        }
        let partial = p.table();
        assert_eq!(p.byte("J_STOP"), 1);
        let first = partial.as_ref().and_then(|t| {
            fails
                .iter()
                .copied()
                .find(|&(id, _)| t[usize::from(id)] == 2)
        });
        match &partial {
            Some(t) => {
                assert_eq!(
                    result,
                    expected("made", t, first, "paused"),
                    "at {press_at}"
                );
                assert!(t.contains(&0), "the pause left work to do");
                paused_with_a_table += 1;
            }
            None => {
                assert_eq!(
                    result,
                    expected("none", &vec![0; want.len()], None, "paused")
                );
                paused_in_the_crc += 1;
            }
        }
        p.leave();
        p.launch();
        let text = p.run(None);
        let table = p.table().expect("the second run made or kept the table");
        assert_eq!(table, want, "press at {press_at}");
        let (_, result) = screens(&text);
        let new_first = partial.as_ref().map_or(fails.first().copied(), |t| {
            fails
                .iter()
                .copied()
                .find(|&(id, _)| t[usize::from(id)] == 0)
        });
        let kept = if partial.is_some() { "kept" } else { "made" };
        assert_eq!(
            result,
            expected(kept, &want, new_first, "done"),
            "at {press_at}"
        );
        let crc_lines = String::from_utf8_lossy(&text).matches("\rcrc ").count();
        assert_eq!(
            crc_lines > 0,
            partial.is_none(),
            "the run is skipped with a table"
        );
    }
    assert!(
        paused_with_a_table > 0,
        "no press paused a run with a table"
    );
    assert!(paused_in_the_crc > 0, "no press paused the CRC run");
}

#[test]
fn a_run_that_stops_before_its_table_is_ready_prints_zero_counts_whatever_the_ram_held() {
    let (good, _) = library();
    let mut unopened = good.clone();
    unopened[0] ^= 0xFF;
    reseal(&mut unopened);
    let mut wrong_crc = good.clone();
    wrong_crc[4] ^= 1;
    for (what, bytes, line) in [
        ("the reader refuses the library", unopened, "stop 00008"),
        ("the CRC run refuses it", wrong_crc, "stop 00005 00001"),
    ] {
        let mut p = Probe::new("jobchk-refused", &bytes);
        let (used, at) = (usize::from(p.label("CK_USED")), p.label("ck_params"));
        p.m.load_ram(at, &vec![0xAA; used]);
        p.launch();
        let text = p.run(None);
        let (_, result) = screens(&text);
        assert!(
            result.last().unwrap().starts_with(line),
            "{what}: {result:?}"
        );
        assert_eq!(
            &result[2..5],
            ["checked 00000", "failed 00000", "resting 00000"],
            "{what}"
        );
        assert!(
            !result.iter().any(|l| l.starts_with("first")),
            "{what}: {result:?}"
        );
        assert!(p.table().is_none(), "{what}");
    }
}
