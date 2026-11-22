//! The probe on the reference OS 2.43, with its real _PutC, _NewLine, variable store and archive.
//! What it does is compared with the stub's run of the same scenario, the screen with the same
//! text drawn by another app in the OS's own font, and the job's variables in SCR_VARS with
//! themselves every millisecond: the OS calls the probe makes must not change them. A scenario
//! that needs a variable in place first runs a second app, jobsetup, that makes it.

use crate::layout::{self, Table};
use crate::probe::{Probe, build, expected, label_of, library, screens};
use crate::realos::{self, *};
use crate::rig::*;
use mcemu::Machine;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const P_TEMP: u16 = 0x982E;
const PROG_PTR: u16 = 0x9830;
const SYM_TABLE: u16 = 0xFE66;
const APPVAR: u8 = 0x15;

/// What the probe finds when it starts.
#[derive(Clone)]
struct Setup {
    /// MCJOBT's bytes, if the app makes it, and whether it archives it.
    table: Option<(Vec<u8>, bool)>,
    /// Leave 20 bytes of free RAM.
    fill: bool,
}

const NONE: Setup = Setup {
    table: None,
    fill: false,
};

/// Assembles `source` of tests/routines, or the probe itself, with the files written first into
/// a directory of its own that comes first on the include path.
fn assemble(name: &str, source: &Path, flags: &[&str], files: &[(&str, String)]) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    let out = dir.join("app.8xk");
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let run = Command::new("sh")
        .arg(root.join("../../scripts/spasm.sh"))
        .env("SPASM", spasm)
        .args(flags)
        .arg("-I")
        .arg(&dir)
        .args(["-I", "../../app/src", "-I", "../../kernel", "-I", "../.."])
        .args(["-I", "../../probe", "-I", "tests/routines"])
        .arg(source)
        .arg(&out)
        .current_dir(root)
        .output()
        .expect("the assembler runs");
    assert!(
        run.status.success(),
        "spasm: {}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::read(out).unwrap()
}

fn db(bytes: &[u8]) -> String {
    let mut text = String::new();
    for line in bytes.chunks(16) {
        let list: Vec<String> = line.iter().map(|b| format!("{b}")).collect();
        text += &format!("\t.db {}\n", list.join(", "));
    }
    text
}

/// The probe around `library` built with JOB_WITH_SETUP, which makes what `setup` asks for first.
fn setup_probe(name: &str, library: &[u8], setup: &Setup) -> (Vec<u8>, PathBuf) {
    let (data, archive) = setup.table.clone().unwrap_or_default();
    let defs = format!(
        "PREP_LEN = {}\nPREP_ARCHIVE = {}\nPREP_FILL = {}\n",
        data.len(),
        u8::from(archive),
        u8::from(setup.fill)
    );
    let table = if data.is_empty() {
        "\t.db 0".to_string()
    } else {
        db(&data)
    };
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let lib = dir.join("library.mclib");
    std::fs::write(&lib, library).unwrap();
    let include = Command::new("sh")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/scripts/library-include.sh"))
        .arg(&lib)
        .output()
        .expect("the library include is written");
    assert!(include.status.success());
    let files = [
        ("prep_defs.inc", defs),
        ("prep_data.inc", format!("prep_data:\n{table}\n")),
        ("library.inc", String::from_utf8(include.stdout).unwrap()),
    ];
    let probe = Path::new("../../probe/jobcheck.z80");
    let app = assemble(
        name,
        probe,
        &["-L", "-DLIBRARY", "-DJOB_WITH_SETUP"],
        &files,
    );
    (app, dir.join("app.lab"))
}

/// A screen of `lines` drawn by another app on the OS, as lit pixels.
fn drawn(name: &str, lines: &[String]) -> Vec<bool> {
    let mut bytes = lines.join("\n").into_bytes();
    bytes.push(0);
    let text = format!("text:\n{}\n", db(&bytes));
    let app = assemble(
        name,
        Path::new("tests/routines/show_text.z80"),
        &[],
        &[("text.inc", text)],
    );
    let mut os = RealOs::boot(&app);
    os.start_from_menu();
    os.run_ms(1500);
    os.screen()
}

fn read(os: &mut RealOs, addr: u16, len: usize) -> Vec<u8> {
    let (page, base) = if addr < 0xC000 {
        (1, 0x8000)
    } else {
        (0, 0xC000)
    };
    let at = usize::from(addr) - base;
    os.m.ram_page(page)[at..at + len].to_vec()
}

fn word(os: &mut RealOs, addr: u16) -> u16 {
    let b = read(os, addr, 2);
    u16::from_le_bytes([b[0], b[1]])
}

/// The variables of the OS: type, flash page (0 in RAM), data address and name.
fn entries(os: &mut RealOs) -> Vec<(u8, u8, u16, Vec<u8>)> {
    let (named, low) = (word(os, PROG_PTR), word(os, P_TEMP));
    let (mut at, mut out) = (SYM_TABLE, Vec::new());
    while at > low {
        let b = read(os, at - 6, 7);
        let len = if at > named { 3 } else { u16::from(b[0]) };
        let skip = if at > named { 5 } else { 6 };
        let mut name = read(os, at - skip - len, usize::from(len));
        name.reverse();
        out.push((b[6], b[1], u16::from_le_bytes([b[3], b[2]]), name));
        at -= skip + 1 + len;
    }
    out
}

fn names(os: &mut RealOs) -> BTreeSet<String> {
    entries(os)
        .into_iter()
        .map(|e| String::from_utf8_lossy(&e.3).into_owned())
        .collect()
}

/// MCJOBT's bytes after its size word, and whether it is archived.
fn table_of(os: &mut RealOs) -> Option<(Vec<u8>, bool)> {
    let e = entries(os)
        .into_iter()
        .find(|e| e.0 == APPVAR && e.3 == b"MCJOBT")?;
    if e.1 == 0 {
        let size = usize::from(word(os, e.2));
        return Some((read(os, e.2 + 2, size), false));
    }
    let at = usize::from(e.2) - 0x4000 + 10 + 6;
    let size = os.m.read_flash(e.1, at, 2);
    let size = usize::from(u16::from_le_bytes([size[0], size[1]]));
    Some((os.m.read_flash(e.1, at + 2, size), true))
}

const FLAGS: [&str; 4] = ["J_KEPT", "J_STOP", "J_STATUS", "J_DETAIL"];

/// The probe on the reference OS, with its labels.
struct Real {
    os: RealOs,
    lab: PathBuf,
    before: BTreeSet<String>,
    /// The first bytes of the job's variables, which hold the library, N and the key and never
    /// change once begin has set them.
    held: Option<Vec<u8>>,
}

/// What a run leaves, as read from the machine.
struct Ended {
    screen: Vec<bool>,
    block: Vec<u8>,
    flags: Vec<u8>,
    table: Option<(Vec<u8>, bool)>,
}

/// The bytes of the job's block that never change during a job: ck_params, ck_n, ck_count, ck_crc.
const HELD: usize = 0x11;

impl Real {
    /// Boots the reference OS with the probe installed. What `setup` asks for is made by the
    /// probe's own first instructions, so the variables a run starts with are those of a fresh
    /// OS and the ones `setup` names.
    fn new(name: &str, bytes: &[u8], setup: &Setup) -> Real {
        let (plain, _) = build(&format!("{name}-plain"), bytes);
        let mut before = names(&mut RealOs::boot(&plain));
        before.extend(setup.table.iter().map(|_| "MCJOBT".to_string()));
        before.extend(setup.fill.then(|| "FILLER".to_string()));
        let (app, lab) = if setup.table.is_some() || setup.fill {
            setup_probe(name, bytes, setup)
        } else {
            build(name, bytes)
        };
        let os = RealOs::boot(&app);
        Real {
            os,
            lab,
            before,
            held: None,
        }
    }

    fn launch_probe(&mut self) {
        self.os.start_from_menu();
    }

    fn at(&self, name: &str) -> usize {
        usize::from(label_of(&self.lab, name)) - 0x8000
    }

    fn block(&mut self) -> Vec<u8> {
        let (start, len) = (
            self.at("ck_params"),
            usize::from(label_of(&self.lab, "CK_USED")),
        );
        self.os.ram(start, len)
    }

    fn flag(&mut self, name: &str) -> u8 {
        let at = self.at(name);
        self.os.ram(at, 1)[0]
    }

    /// Asserts that the job's constants stand, once begin has set them, and returns the block.
    fn watch(&mut self) -> Vec<u8> {
        let block = self.block();
        let n = usize::from(label_of(&self.lab, "ck_n") - label_of(&self.lab, "ck_params"));
        match &self.held {
            Some(held) => assert_eq!(&block[..HELD], &held[..], "SCR_VARS changed"),
            None if block[n] != 0 => self.held = Some(block[..HELD].to_vec()),
            None => {}
        }
        block
    }

    /// Runs a millisecond at a time, watching SCR_VARS, until `until` holds of the block.
    fn run_until(&mut self, what: &str, until: impl Fn(&[u8]) -> bool) {
        for _ in 0..120_000 {
            self.os.run_ms(1);
            if until(&self.watch()) {
                return;
            }
        }
        panic!("never reached {what}");
    }

    /// The probe has stopped when it has written its status, which is never 0 once it has; the
    /// screen has settled 800 ms later.
    fn run_to_the_end(&mut self) {
        let status = self.at("J_STATUS");
        for _ in 0..120_000 {
            self.os.run_ms(1);
            self.watch();
            if self.os.ram(status, 1)[0] != 0 {
                self.os.run_ms(800);
                return;
            }
        }
        panic!("the probe never stopped");
    }

    fn ended(&mut self) -> Ended {
        let flags = FLAGS.iter().map(|f| self.flag(f)).collect();
        Ended {
            screen: self.os.screen(),
            block: self.block(),
            flags,
            table: table_of(&mut self.os),
        }
    }

    fn created(&mut self) -> (usize, usize) {
        let after = names(&mut self.os);
        (
            after.difference(&self.before).count(),
            self.before.difference(&after).count(),
        )
    }
}

/// The same scenario on the stub OS: the block, the flags, the table and the result screen.
fn stub_run(
    name: &str,
    bytes: &[u8],
    setup: &Setup,
) -> (Vec<u8>, Vec<u8>, Option<Vec<u8>>, Vec<String>) {
    let mut p = Probe::new(name, bytes);
    if let Some((data, archive)) = &setup.table {
        p.m.create_var(VAR, data).unwrap();
        if *archive {
            p.m.archive_var(VAR);
        }
    }
    if setup.fill {
        p.m.set_free_ram(20);
    }
    p.launch();
    let text = p.run(None);
    let (_, lines) = screens(&text);
    let block =
        p.m.read_ram(p.label("ck_params"), usize::from(p.label("CK_USED")));
    let flags = FLAGS.iter().map(|f| p.byte(f)).collect();
    (block, flags, p.table(), lines)
}

/// What the stub and the reference OS must both give for one scenario.
fn same_on_both(
    name: &str,
    bytes: &[u8],
    setup: &Setup,
    lines: &[String],
    table: Option<&[u8]>,
    made: (usize, usize),
) {
    let (block, flags, stub_table, text) = stub_run(&format!("{name}-stub"), bytes, setup);
    assert_eq!(text, lines, "the stub's screen");
    assert_eq!(stub_table.as_deref(), table, "the stub's table");
    let shown = drawn(&format!("{name}-text"), lines);
    let mut real = Real::new(name, bytes, setup);
    real.launch_probe();
    real.run_to_the_end();
    let ended = real.ended();
    write_pgm(&format!("{name}.pgm"), &ended.screen);
    assert!(
        ended.screen == shown,
        "the screen is not the text drawn by the OS: {lines:?}"
    );
    assert_eq!(ended.block[0], realos::BASE - 3, "the library's first page");
    assert_eq!(
        ended.block[1..],
        block[1..],
        "the job's variables, as the stub left them"
    );
    assert_eq!(ended.flags, flags);
    let kept = ended.table.map(|(data, archived)| {
        assert!(!archived, "the table is in RAM");
        layout::decode(&data).unwrap().values
    });
    assert_eq!(kept.as_deref(), table, "MCJOBT");
    assert_eq!(
        real.created(),
        made,
        "variables made and deleted by the probe"
    );
}

fn stop_line(status: u8, detail: u8) -> String {
    format!("stop {status:05} {detail:05}")
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_full_run_on_the_reference_os_gives_the_stub_s_block_table_and_screen() {
    let (bytes, want) = library();
    let first = failures(&bytes).first().copied();
    let lines = expected("made", &want, first, "done");
    same_on_both("jobreal-done", &bytes, &NONE, &lines, Some(&want), (1, 0));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_archived_table_of_this_library_is_brought_back_and_finished() {
    let (bytes, want) = library();
    let prior: Vec<u8> = want
        .iter()
        .enumerate()
        .map(|(i, &v)| if i < 12 { v } else { 0 })
        .collect();
    let table = layout::encode(&table_for(&bytes, prior.clone()));
    let setup = Setup {
        table: Some((table, true)),
        fill: false,
    };
    let first = failures(&bytes)
        .into_iter()
        .find(|&(id, _)| prior[usize::from(id)] == 0);
    let lines = expected("made", &want, first, "done");
    same_on_both(
        "jobreal-archived",
        &bytes,
        &setup,
        &lines,
        Some(&want),
        (0, 0),
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn another_librarys_table_is_replaced_once_the_crc_run_has_passed() {
    let (bytes, want) = library();
    let mut other = Table::new(header_crc(&bytes) ^ 1, count_of(&bytes));
    other.values.fill(2);
    let setup = Setup {
        table: Some((layout::encode(&other), false)),
        fill: false,
    };
    let first = failures(&bytes).first().copied();
    let lines = expected("made", &want, first, "done");
    same_on_both("jobreal-other", &bytes, &setup, &lines, Some(&want), (0, 0));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn too_little_ram_for_the_table_stops_the_run_and_creates_nothing_but_the_filler() {
    let (bytes, _) = library();
    let setup = Setup {
        table: None,
        fill: true,
    };
    let mut lines = expected("none", &vec![0; usize::from(count_of(&bytes))], None, "");
    lines.pop();
    lines.push(stop_line(6, 1));
    same_on_both("jobreal-noram", &bytes, &setup, &lines, None, (0, 0));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_pauses_the_probe_on_the_reference_os_and_a_second_run_resumes() {
    let (bytes, want) = library();
    let count = usize::from(count_of(&bytes));
    let fails = failures(&bytes);
    let mut real = Real::new("jobreal-pause", &bytes, &NONE);
    real.launch_probe();
    let (state, n0, n) = (
        usize::from(label_of(&real.lab, "ck_state") - label_of(&real.lab, "ck_params")),
        usize::from(label_of(&real.lab, "ck_counts") - label_of(&real.lab, "ck_params")),
        usize::from(label_of(&real.lab, "ck_n") - label_of(&real.lab, "ck_params")),
    );
    real.run_until("three assertions checked", |b| {
        b[state] == 3
            && u16::from_le_bytes([b[n0], b[n0 + 1]]) + 3 <= u16::from_le_bytes([b[n], b[n + 1]])
    });
    real.os.m.press_key(Machine::KEY_ON);
    real.os.run_ms(300);
    real.os.m.release_key(Machine::KEY_ON);
    real.run_to_the_end();
    let paused = real.ended();
    assert_eq!(
        paused.flags,
        [2, 1, 4, 14],
        "made, paused, CK_PAUSED, ST_INTERRUPTED"
    );
    let partial = layout::decode(&paused.table.clone().expect("MCJOBT").0)
        .unwrap()
        .values;
    assert!(partial.contains(&0) && partial.iter().any(|&v| v != 0));
    let first = fails
        .iter()
        .copied()
        .find(|&(id, _)| partial[usize::from(id)] == 2);
    let lines = expected("made", &partial, first, "paused");
    assert!(
        paused.screen == drawn("jobreal-pause-text", &lines),
        "the paused screen: {lines:?}"
    );

    real.os.tap(KEY_CLEAR, 150, 600);
    real.launch_probe();
    real.held = None;
    real.run_to_the_end();
    let done = real.ended();
    assert_eq!(done.flags[..3], [1, 0, 3], "kept, done, CK_DONE");
    let values = layout::decode(&done.table.clone().unwrap().0)
        .unwrap()
        .values;
    assert_eq!(values, want);
    let new_first = fails
        .iter()
        .copied()
        .find(|&(id, _)| partial[usize::from(id)] == 0);
    let lines = expected("kept", &want, new_first, "done");
    assert!(
        done.screen == drawn("jobreal-resume-text", &lines),
        "the second run's screen: {lines:?}"
    );
    let counts = [0, 1, 2, 3].map(|v| want.iter().filter(|&&w| w == v).count());
    let at = real.at("ck_counts");
    let kept: Vec<usize> = real
        .os
        .ram(at, 8)
        .chunks(2)
        .map(|c| usize::from(u16::from_le_bytes([c[0], c[1]])))
        .collect();
    assert_eq!(kept, counts);
    assert_eq!(real.created(), (1, 0));
    assert_eq!(count, want.len());
}
