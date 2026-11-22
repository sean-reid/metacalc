//! The probe app probe/jobcheck.z80, built around a library and run on the stub OS: it keeps its
//! table in MCJOBT, shows a result, pauses on ON and carries on at a second run.

use crate::damage::damaged;
use crate::damage::synthetic;
use crate::rig::*;
use mcemu::{Machine, STUB_ROM, Stop};
use std::path::{Path, PathBuf};
use std::process::Command;

const CHUNK: u32 = 20_000_000;
const NEW_LINE: u16 = 0x452E;

struct Probe {
    m: Machine,
    lab: PathBuf,
}

fn build(name: &str, library: &[u8]) -> (Vec<u8>, PathBuf) {
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

impl Probe {
    fn new(name: &str, library: &[u8]) -> Probe {
        let (app, lab) = build(name, library);
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&app, BASE).expect("the app installs");
        Probe { m, lab }
    }

    fn label(&self, name: &str) -> u16 {
        let text = std::fs::read_to_string(&self.lab).unwrap();
        text.lines()
            .find_map(|line| {
                let (key, value) = line.split_once(" = ")?;
                (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
            })
            .unwrap_or_else(|| panic!("no label {name}"))
    }

    fn byte(&mut self, name: &str) -> u8 {
        let at = self.label(name);
        self.m.read_ram(at, 1)[0]
    }

    fn launch(&mut self) {
        self.m.launch(BASE).expect("the app launches");
    }

    /// Runs `chunks` slices of the app. The stub has no _NewLine, so a call of it is stepped over
    /// with the registers the call saved.
    fn run(&mut self, chunks: u32) {
        for _ in 0..chunks {
            let mut left = CHUNK;
            loop {
                let run = self.m.resume(left);
                match run.stop {
                    Stop::Limit | Stop::Exited => break,
                    Stop::UnsupportedBcall(NEW_LINE) => {
                        left = left.saturating_sub(run.tstates).max(1000);
                        self.skip_bcall();
                    }
                    other => panic!("{other:?}"),
                }
            }
        }
    }

    fn run_for(&mut self, tstates: u32) {
        let mut left = tstates;
        while left > 0 {
            let run = self.m.resume(left);
            left = left.saturating_sub(run.tstates.max(1));
            match run.stop {
                Stop::Limit => {}
                Stop::Exited => break,
                Stop::UnsupportedBcall(NEW_LINE) => self.skip_bcall(),
                other => panic!("{other:?}"),
            }
        }
    }

    fn skip_bcall(&mut self) {
        let mut regs = self.m.regs();
        let b = self.m.read_ram(regs.sp, 10);
        let word = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        regs.af = word(0);
        regs.bc = word(2);
        regs.de = word(4);
        regs.hl = word(6);
        regs.pc = word(8);
        regs.sp += 10;
        self.m.set_regs(&regs);
    }

    fn table(&mut self) -> Option<Vec<u8>> {
        let var = self.m.read_var(VAR)?;
        Some(crate::layout::decode(&var.data).expect("it decodes").values)
    }

    fn shot(&mut self, name: &str) {
        if let Some(dir) = std::env::var_os("MC_SHOTS") {
            let lcd = self.m.lcd();
            let mut pgm = b"P5\n96 64\n255\n".to_vec();
            for y in 0..64 {
                for x in 0..96 {
                    pgm.push(if lcd.pixel(x, y) { 0 } else { 255 });
                }
            }
            std::fs::write(Path::new(&dir).join(format!("{name}.pgm")), pgm).unwrap();
        }
    }
}

fn library() -> (Vec<u8>, Vec<u8>) {
    let mut seed = 0x0B5E_55ED;
    let bytes = synthetic(10, 40, &mut seed);
    let (copy, passes) = damaged(&bytes, &mut seed).expect("a damaged copy");
    (copy.clone(), marks(&copy, &passes))
}

#[test]
fn the_probe_makes_its_table_checks_the_library_and_shows_the_result() {
    let (bytes, want) = library();
    let mut p = Probe::new("jobchk-run", &bytes);
    p.launch();
    p.run(3);
    let (st, de) = (p.byte("J_STATUS"), p.byte("J_DETAIL"));
    assert_eq!(p.byte("J_STOP"), 0, "stopped: status {st}, detail {de}");
    assert!(p.m.read_var("MCSTAT").is_none(), "MCSTAT is not touched");
    assert_eq!(p.table().expect("the table is made"), want);
    assert_eq!(p.byte("J_KEPT"), 2, "made");
    assert_eq!(p.byte("J_STOP"), 0, "done");
    p.shot("done");
    p.launch();
    p.run(3);
    assert_eq!(p.byte("J_KEPT"), 1, "kept at the second run");
    assert_eq!(p.table().unwrap(), want);
    p.shot("kept");
}

#[test]
fn on_pauses_the_probe_and_a_second_run_carries_on() {
    let (bytes, want) = library();
    let mut paused_with_a_table = 0;
    for press_at in [
        200_000, 1_000_000, 2_500_000, 3_000_000, 3_500_000, 4_500_000, 6_000_000,
    ] {
        let mut p = Probe::new("jobchk-pause", &bytes);
        p.launch();
        p.run_for(press_at);
        p.m.press_key(Machine::KEY_ON);
        p.run_for(3_000_000);
        p.m.release_key(Machine::KEY_ON);
        p.run(1);
        if p.byte("J_STOP") != 1 {
            continue;
        }
        p.shot(&format!("paused-{press_at}"));
        let partial = p.table();
        p.launch();
        p.run(2);
        assert_eq!(
            p.byte("J_STOP"),
            0,
            "second run after a press at {press_at}"
        );
        assert_eq!(p.table().unwrap(), want, "press at {press_at}");
        if let Some(partial) = partial {
            assert!(partial.contains(&0), "the pause left work to do");
            assert_eq!(p.byte("J_KEPT"), 1, "the second run found the table");
            paused_with_a_table += 1;
        }
    }
    assert!(
        paused_with_a_table > 0,
        "no press paused a run with a table"
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_probe_runs_and_draws_its_result() {
    let (bytes, _) = library();
    let (app, lab) = build("jobchk-real", &bytes);
    let probe = Probe {
        m: Machine::new(STUB_ROM).unwrap(),
        lab,
    };
    let mut os = crate::realos::RealOs::boot(&app);
    os.launch();
    let rows = |os: &mut crate::realos::RealOs, from: usize, to: usize| {
        os.screen()[from * 96..to * 96]
            .iter()
            .filter(|&&p| p)
            .count()
    };
    let mut settled = false;
    for _ in 0..600 {
        os.run_ms(200);
        if rows(&mut os, 56, 64) > 0 && rows(&mut os, 8, 16) > 0 {
            settled = true;
            break;
        }
    }
    assert!(settled, "the result screen came up");
    let at = usize::from(probe.label("J_STOP")) - 0x8000;
    assert_eq!(os.ram(at, 1)[0], 0, "the run is done");
    let screen = os.screen();
    crate::realos::write_pgm("jobchk-done.pgm", &screen);
}
