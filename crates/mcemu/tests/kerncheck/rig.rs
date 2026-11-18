use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{CHUNK, Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm;
use std::path::Path;

pub const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kerncheck_test.bin"));
pub const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/kerncheck_test.lab"));

/// The page of library chunk 0, below the one the test code is entered on.
pub const LIB_BASE: u8 = FLASH_TEST_PAGE - 1;
pub const OUT_PTR: u16 = 0xA000;
pub const OUT_SPEED: u16 = 0xA002;
pub const OUT_AFTER: u16 = 0xA003;
pub const OUT_BUF: u16 = 0xA010;
pub const ENTRY: u16 = 0x4000;
pub const LIMIT: u32 = 2_000_000_000;
/// T-states, about 13 ms at 15 MHz: longer than the interval between timer interrupts.
pub const SLICE: u32 = 200_000;

#[derive(Debug, PartialEq)]
pub enum Step {
    Done,
    Waiting(u16),
    Running(u16),
}

pub fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// The status numbers kernel.z80 documents.
pub fn code(error: KernelError) -> u8 {
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
        KernelError::LooseReference => 15,
    }
}

pub fn corpus(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    let db = mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses");
    let all: Vec<_> = (0..db.stmts.len() as mm::StmtId).collect();
    pack(&db, &all).expect("it packs")
}

/// A library of `count` axioms, long enough to span several chunks.
pub fn axioms(count: usize) -> Vec<u8> {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..count {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = mm::parse(src.into_bytes()).expect("it parses");
    let all: Vec<_> = (0..db.stmts.len() as mm::StmtId).collect();
    pack(&db, &all).expect("it packs")
}

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

pub struct Rig {
    pub m: Machine,
}

impl Rig {
    pub fn new(library: &[u8], speed: Speed) -> Rig {
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
    pub fn ram(&mut self, addr: u16, len: usize) -> Vec<u8> {
        let at = usize::from(addr - 0x8000);
        self.m.ram_page(1)[at..at + len].to_vec()
    }

    pub fn word(&mut self, name: &str) -> u16 {
        self.word_at(label(name))
    }

    pub fn byte(&mut self, addr: u16) -> u8 {
        self.ram(addr, 1)[0]
    }

    /// Everything the driver sent to putchar so far.
    pub fn trace(&mut self) -> Vec<u8> {
        let end = self.word_at(OUT_PTR).max(OUT_BUF);
        self.ram(OUT_BUF, usize::from(end - OUT_BUF))
    }

    pub fn word_at(&mut self, addr: u16) -> u16 {
        let b = self.ram(addr, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    pub fn run_to_end(&mut self) -> usize {
        self.finish(ENTRY, 0)
    }

    /// True while the code is inside `on_release_and_clear`.
    pub fn in_wait(&self) -> bool {
        (label("ON_RELEASE_AND_CLEAR")..label("ON_RELEASE_END")).contains(&self.m.pc())
    }

    /// Runs one slice, long enough for a timer interrupt, from `pc` with interrupts off. A slice
    /// that ends inside the wait for ON comes back as `Waiting(restart)`: the wait is a loop that
    /// starts with `ei`, so starting it again is the same as going on, and the halt in it wakes on
    /// the timer interrupt as it does on the calculator.
    pub fn step(&mut self, pc: u16) -> Step {
        match self.m.run(pc, SLICE).stop {
            Stop::Halted => Step::Done,
            Stop::Limit if self.in_wait() => Step::Waiting(label("ON_RELEASE_AND_CLEAR")),
            Stop::Limit => Step::Running(self.m.pc()),
            other => panic!("the run stopped with {other:?}"),
        }
    }

    /// Runs slices from `pc` until one ends inside the wait; returns where to go on from.
    pub fn until_waiting(&mut self, mut pc: u16) -> u16 {
        loop {
            match self.step(pc) {
                Step::Waiting(restart) => return restart,
                Step::Running(next) => pc = next,
                Step::Done => panic!("the routine ended without waiting for ON"),
            }
        }
    }

    /// Runs from `pc` to the end of the test routine. Each slice that ends inside the wait for ON
    /// to come up counts one wait, and after `holds` of them the key is released. Returns the
    /// waits.
    pub fn finish(&mut self, mut pc: u16, holds: usize) -> usize {
        let mut waits = 0;
        loop {
            match self.step(pc) {
                Step::Done => return waits,
                Step::Running(next) => pc = next,
                Step::Waiting(restart) => {
                    waits += 1;
                    if waits == holds {
                        self.m.release_key(Machine::KEY_ON);
                    }
                    pc = restart;
                }
            }
        }
    }

    /// Runs in steps of `step` T-states, from the start, until the trace holds `marker`.
    pub fn run_until_trace_has(&mut self, marker: &str, step: u32) -> u16 {
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
pub struct Screen {
    rows: Vec<Vec<u8>>,
    row: usize,
    col: usize,
}

impl Screen {
    pub fn replay(trace: &[u8]) -> Screen {
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

    pub fn lines(&self) -> Vec<String> {
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
pub fn screens(trace: &[u8]) -> (Vec<String>, Vec<String>) {
    let clear = trace.iter().rposition(|&c| c == 0x0C).expect("a clear");
    (
        Screen::replay(&trace[..clear]).lines(),
        Screen::replay(&trace[clear..]).lines(),
    )
}

pub struct Model {
    pub count: usize,
    pub first: Option<(usize, u8)>,
    pub refused: usize,
}

pub fn model(bytes: &[u8]) -> Model {
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

pub fn report(bytes: &[u8]) -> Vec<String> {
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

pub fn progress(bytes: &[u8], done: usize, count: usize) -> Vec<String> {
    let chunks = bytes.len().div_ceil(CHUNK);
    vec![
        "kernchk v1".to_string(),
        format!("crc {chunks:05}/{chunks:05}"),
        format!("chk {done:05}/{count:05}"),
    ]
}

pub fn count_of(trace: &[u8], needle: &str) -> usize {
    String::from_utf8_lossy(trace).matches(needle).count()
}
