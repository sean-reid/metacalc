use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{CHUNK, Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm;
use std::path::Path;
use std::process::Command;

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kerncheck_test.bin"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/kerncheck_test.lab"));

/// The page of library chunk 0, below the one the test code is entered on.
const LIB_BASE: u8 = FLASH_TEST_PAGE - 1;
const OUT_PTR: u16 = 0xA000;
const OUT_SPEED: u16 = 0xA002;
const OUT_AFTER: u16 = 0xA003;
const OUT_BUF: u16 = 0xA010;
const ENTRY: u16 = 0x4000;
const LIMIT: u32 = 2_000_000_000;

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// The status numbers kernel.z80 documents.
fn code(error: KernelError) -> u8 {
    match error {
        KernelError::Malformed => 1,
        KernelError::BadRecord => 2,
        KernelError::NotATheorem => 3,
        KernelError::ForwardReference => 4,
        KernelError::OutOfMemory => 5,
        KernelError::StackUnderflow => 6,
        KernelError::BadBackReference => 7,
        KernelError::TypecodeMismatch => 8,
        KernelError::HypothesisMismatch => 9,
        KernelError::DisjointViolation => 10,
        KernelError::StackNotSingleton => 11,
        KernelError::WrongConclusion => 12,
        KernelError::WorkLimit => 13,
    }
}

fn corpus(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    let db = mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses");
    let all: Vec<_> = (0..db.stmts.len() as mm::StmtId).collect();
    pack(&db, &all).expect("it packs")
}

/// A library of `count` axioms, long enough to span several chunks.
fn axioms(count: usize) -> Vec<u8> {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..count {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = mm::parse(src.into_bytes()).expect("it parses");
    let all: Vec<_> = (0..db.stmts.len() as mm::StmtId).collect();
    pack(&db, &all).expect("it packs")
}

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

struct Rig {
    m: Machine,
}

impl Rig {
    fn new(library: &[u8], speed: Speed) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(speed);
        m.load_flash(FLASH_TEST_PAGE, 0, CODE);
        for (i, chunk) in library.chunks(CHUNK).enumerate() {
            m.load_flash(LIB_BASE - i as u8, 0, chunk);
        }
        Rig { m }
    }

    /// Bytes of the driver's scratch and output in RAM page 81h, which bank B shows except while
    /// the kernel has a library page mapped there.
    fn ram(&mut self, addr: u16, len: usize) -> Vec<u8> {
        let at = usize::from(addr - 0x8000);
        self.m.ram_page(1)[at..at + len].to_vec()
    }

    fn word(&mut self, name: &str) -> u16 {
        self.word_at(label(name))
    }

    fn byte(&mut self, addr: u16) -> u8 {
        self.ram(addr, 1)[0]
    }

    /// Everything the driver sent to putchar so far.
    fn trace(&mut self) -> Vec<u8> {
        let end = self.word_at(OUT_PTR).max(OUT_BUF);
        self.ram(OUT_BUF, usize::from(end - OUT_BUF))
    }

    fn word_at(&mut self, addr: u16) -> u16 {
        let b = self.ram(addr, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    fn run_to_end(&mut self) -> u32 {
        let run = self.m.run(ENTRY, LIMIT);
        assert_eq!(run.stop, Stop::Halted);
        run.tstates
    }

    /// Runs in steps of `step` T-states, from the start, until the trace holds `marker`.
    fn run_until_trace_has(&mut self, marker: &str, step: u32) -> u16 {
        let mut pc = ENTRY;
        for _ in 0..100_000 {
            let run = self.m.run(pc, step);
            assert_eq!(run.stop, Stop::Limit, "the run ended before {marker:?}");
            pc = self.m.pc();
            if String::from_utf8_lossy(&self.trace()).contains(marker) {
                return pc;
            }
        }
        panic!("no {marker:?} in the trace");
    }
}

/// A 16 by 8 text screen driven by the characters the driver prints.
struct Screen {
    rows: Vec<Vec<u8>>,
    row: usize,
    col: usize,
}

impl Screen {
    fn replay(trace: &[u8]) -> Screen {
        let mut screen = Screen {
            rows: vec![vec![b' '; 16]; 8],
            row: 0,
            col: 0,
        };
        for &c in trace {
            match c {
                0x0C => {
                    screen.rows = vec![vec![b' '; 16]; 8];
                    screen.row = 0;
                    screen.col = 0;
                }
                0x0A => {
                    screen.row += 1;
                    screen.col = 0;
                    assert!(screen.row < 8, "text ran off the screen");
                }
                0x0D => screen.col = 0,
                c => {
                    assert!(screen.col < 16, "a line is longer than the screen");
                    screen.rows[screen.row][screen.col] = c;
                    screen.col += 1;
                }
            }
        }
        screen
    }

    fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .rows
            .iter()
            .map(|r| String::from_utf8_lossy(r).trim_end().to_string())
            .collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines
    }
}

/// The screen after the last clear and the screen the progress lines left before it.
fn screens(trace: &[u8]) -> (Vec<String>, Vec<String>) {
    let clear = trace.iter().rposition(|&c| c == 0x0C).expect("a clear");
    (
        Screen::replay(&trace[..clear]).lines(),
        Screen::replay(&trace[clear..]).lines(),
    )
}

struct Model {
    count: usize,
    first: Option<(usize, u8)>,
    refused: usize,
}

fn model(bytes: &[u8]) -> Model {
    let capped = Limits {
        stack_syms: usize::from(label("CAP_STACK_SYMS")),
        stack_entries: usize::from(label("CAP_STACK_ENTRIES")),
        saved_syms: usize::from(label("CAP_SAVED_SYMS")),
        saved_entries: usize::from(label("CAP_SAVED_ENTRIES")),
    };
    let mut kernel = Kernel::new(bytes, capped).expect("the model loads it");
    let count = kernel.assertions();
    let mut first = None;
    let mut refused = 0;
    for id in 0..count {
        if let Err(e) = kernel.verify(id) {
            refused += 1;
            first.get_or_insert((id, code(e)));
        }
    }
    Model {
        count,
        first,
        refused,
    }
}

fn report(bytes: &[u8]) -> Vec<String> {
    let model = model(bytes);
    let mut lines = vec![
        "kernchk v1".to_string(),
        format!("crc {:08X}", crc32(&bytes[8..])),
        format!("checked {:05}", model.count),
        format!("accepted {:05}", model.count - model.refused),
    ];
    if let Some((id, status)) = model.first {
        lines.push(format!("first {id:05}"));
        lines.push(format!("status {status:05}"));
    }
    lines
}

fn progress(bytes: &[u8], done: usize, count: usize) -> Vec<String> {
    let chunks = bytes.len().div_ceil(CHUNK);
    vec![
        "kernchk v1".to_string(),
        format!("crc {chunks:05}/{chunks:05}"),
        format!("chk {done:05}/{count:05}"),
    ]
}

fn count_of(trace: &[u8], needle: &str) -> usize {
    String::from_utf8_lossy(trace).matches(needle).count()
}

#[test]
fn a_library_that_passes_is_reported_whole_with_its_checksum() {
    let bytes = axioms(2000);
    assert!(bytes.len() > 4 * CHUNK);
    let mut rig = Rig::new(&bytes, Speed::Mhz6);
    rig.run_to_end();
    let trace = rig.trace();
    let (progress_screen, result) = screens(&trace);
    assert_eq!(result, report(&bytes));
    assert_eq!(result[1], format!("crc {:08X}", crc32(&bytes[8..])));
    assert_eq!(progress_screen, progress(&bytes, 2000, 2000));
    assert_eq!(
        count_of(&trace, "\rchk "),
        1 + 2000 / 16 + 1,
        "a line at the start, one per 16 assertions, one at the end"
    );
    assert!(count_of(&trace, "\rchk 00016/02000") == 1);
    assert!(count_of(&trace, "\rchk 00032/02000") == 1);
    assert_eq!(count_of(&trace, "\rcrc "), 1 + bytes.len().div_ceil(CHUNK));
    assert_eq!(rig.word("K_DONE"), 2000);
    assert_eq!(rig.word("K_ACCEPTED"), 2000);
}

#[test]
fn a_library_with_failing_proofs_reports_the_first_refusal_and_its_status() {
    for name in [
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier-bad1.mm",
        "demo0-bad1.mm",
    ] {
        let bytes = corpus(name);
        let want = model(&bytes);
        assert!(want.first.is_some(), "{name} has a proof the model refuses");
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        rig.run_to_end();
        let (_, result) = screens(&rig.trace());
        assert_eq!(result, report(&bytes), "{name}");
        let (id, status) = want.first.unwrap();
        assert_eq!(rig.word("K_FIRST"), id as u16, "{name}");
        assert_eq!(rig.byte(label("K_FIRST_ST")), status, "{name}");
        assert_eq!(rig.word("K_DONE") as usize, want.count, "{name}");
        assert_eq!(
            rig.word("K_ACCEPTED") as usize,
            want.count - want.refused,
            "{name}"
        );
    }
}

#[test]
fn a_refusal_is_printed_with_the_status_the_kernel_returned() {
    let mut seen = std::collections::BTreeSet::new();
    for name in [
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier-bad1.mm",
        "big-unifier-bad2.mm",
        "big-unifier-bad3.mm",
        "demo0-bad1.mm",
    ] {
        let bytes = corpus(name);
        if let Some((_, status)) = model(&bytes).first {
            let mut rig = Rig::new(&bytes, Speed::Mhz15);
            rig.run_to_end();
            let (_, result) = screens(&rig.trace());
            assert!(result.contains(&format!("status {status:05}")), "{name}");
            seen.insert(status);
        }
    }
    assert!(!seen.is_empty());
}

#[test]
fn a_wrong_checksum_is_reported_and_nothing_is_verified() {
    let mut bytes = axioms(300);
    bytes[4] ^= 0x10;
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (progress_screen, result) = screens(&rig.trace());
    assert_eq!(
        result,
        ["kernchk v1", "crc bad", "checked 00000", "accepted 00000"]
    );
    assert_eq!(progress_screen[..2], ["kernchk v1", "crc 00000/00001"][..]);

    let mut bytes = axioms(2000);
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(
        result,
        ["kernchk v1", "crc bad", "checked 00000", "accepted 00000"]
    );
}

#[test]
fn a_library_the_kernel_refuses_to_load_is_reported_by_status() {
    let mut bytes = axioms(100);
    bytes[0] = b'X';
    reseal(&mut bytes);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, ["kernchk v1", "init st 00001"]);
}

#[test]
fn the_cpu_runs_at_15_mhz_and_returns_to_the_speed_it_had() {
    let bytes = axioms(40);
    for (start, was) in [(Speed::Mhz6, 0u8), (Speed::Mhz15, 1)] {
        let mut rig = Rig::new(&bytes, start);
        rig.run_to_end();
        assert_eq!(rig.byte(OUT_SPEED), 1, "during the run, from {start:?}");
        assert_eq!(rig.byte(OUT_AFTER), was, "after the run, from {start:?}");
    }
}

#[test]
fn on_held_before_the_run_stops_it_in_the_checksum() {
    let bytes = axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.press_key(Machine::KEY_ON);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(
        result,
        [
            "kernchk v1",
            "crc stopped",
            "checked 00000",
            "accepted 00000",
            "stopped 00000"
        ]
    );
}

#[test]
fn on_pressed_during_the_checksum_stops_it_at_that_chunk() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rcrc 00002/", 100_000);
    rig.m.press_key(Machine::KEY_ON);
    let run = rig.m.run(pc, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    let (_, result) = screens(&rig.trace());
    let at = rig.word("K_STOP_AT");
    assert!((2..=3).contains(&at), "stopped at chunk {at}");
    assert_eq!(
        result,
        [
            "kernchk v1".to_string(),
            "crc stopped".to_string(),
            "checked 00000".to_string(),
            "accepted 00000".to_string(),
            format!("stopped {at:05}"),
        ]
    );
}

#[test]
fn on_pressed_during_verification_stops_there_and_reports_where() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
    rig.m.press_key(Machine::KEY_ON);
    let run = rig.m.run(pc, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    let (_, result) = screens(&rig.trace());
    let done = rig.word("K_DONE");
    assert!((64..2000).contains(&usize::from(done)), "stopped at {done}");
    assert_eq!(
        result,
        [
            "kernchk v1".to_string(),
            format!("crc {:08X}", crc32(&bytes[8..])),
            format!("checked {done:05}"),
            format!("accepted {:05}", rig.word("K_ACCEPTED")),
            format!("stopped {done:05}"),
        ]
    );
    assert_eq!(rig.word("K_STOP_AT"), done);
}

/// Presses ON the `nth` time the run is seen inside the kernel's proof check, runs to the end, and
/// says whether the driver saw the key before the kernel did.
fn press_in_proof(bytes: &[u8], nth: usize) -> (bool, Rig) {
    let proofs = label("VERIFY_BODY")..label("KERNEL_CRC_BEGIN");
    let on_down = label("KD_ON_DOWN");
    let mut rig = Rig::new(bytes, Speed::Mhz15);
    let mut pc = rig.run_until_trace_has("\rchk 00000/", 5_000);
    let mut seen = 0;
    while seen < nth {
        let run = rig.m.run(pc, 40);
        assert_eq!(run.stop, Stop::Limit, "the run ended before a proof step");
        pc = rig.m.pc();
        seen += usize::from(proofs.contains(&pc));
    }
    rig.m.press_key(Machine::KEY_ON);
    let mut driver_looked = false;
    loop {
        let run = rig.m.run(pc, 40);
        pc = rig.m.pc();
        driver_looked |= pc == on_down;
        if run.stop == Stop::Halted {
            return (driver_looked, rig);
        }
    }
}

#[test]
fn on_pressed_inside_a_proof_makes_the_kernel_return_14_and_the_driver_stop() {
    let bytes = corpus("big-unifier.mm");
    let mut by_kernel = 0;
    for nth in [1, 5, 20, 60, 150, 400, 900] {
        let (driver_looked, mut rig) = press_in_proof(&bytes, nth);
        let done = rig.word("K_DONE");
        let (_, result) = screens(&rig.trace());
        assert_eq!(result.last().unwrap(), &format!("stopped {done:05}"));
        assert_eq!(rig.word("K_STOP_AT"), done);
        assert_eq!(rig.byte(label("K_HAVE_FIRST")), 0, "14 is not a refusal");
        by_kernel += usize::from(!driver_looked);
    }
    assert!(by_kernel > 0, "the kernel never returned 14 to the driver");
}

#[test]
fn the_driver_leaves_no_trace_of_the_kernel_in_the_pages_it_does_not_own() {
    let bytes = corpus("demo0.mm");
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    for page in [0u8, 3] {
        rig.m.ram_page(page).fill(0x6D);
    }
    rig.run_to_end();
    let os = rig.m.ram_page(0).to_vec();
    let changed: Vec<usize> = os
        .iter()
        .enumerate()
        .filter(|&(i, &b)| b != 0x6D && i < 0x4000 - 64)
        .map(|(i, _)| i)
        .collect();
    assert!(changed.is_empty(), "page 80h changed at {changed:x?}");
}

#[test]
fn the_app_builds_from_a_library_and_passes_the_checker() {
    let probe = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../probe");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("kernchk");
    std::fs::create_dir_all(&dir).unwrap();
    let library = dir.join("library.mclib");
    std::fs::write(&library, corpus("demo0.mm")).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let make = Command::new("make")
        .args(["-s", "-C"])
        .arg(&probe)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", dir.display()))
        .arg(format!("LIBRARY={}", library.display()))
        .arg("kernchk")
        .output()
        .expect("make runs");
    assert!(
        make.status.success(),
        "make: {}",
        String::from_utf8_lossy(&make.stdout)
    );
    let check = Command::new("sh")
        .arg(probe.join("../app/scripts/check-8xk.sh"))
        .arg("-l")
        .arg(&library)
        .arg(dir.join("kernchk.8xk"))
        .arg("kernchk")
        .output()
        .expect("the checker runs");
    assert!(
        check.status.success(),
        "check-8xk: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}
