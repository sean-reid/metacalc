use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{CHUNK, Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm;
use std::ops::Range;
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
/// T-states in two timer periods at 15 MHz, five at 6 MHz: every slice spans an interrupt, so the
/// halt in the wait wakes at least once in it. A wait is a slice that ended in the loop, not a poll.
pub const SLICE: u32 = 2 * crate::stubos::TIMER_PERIOD_15MHZ;
/// A slice that ends below the app's page, where the stub OS and its handler are, is carried on in
/// steps of `OS_NUDGE` T-states, so that it is judged where the code has come back to; slices are a whole number of
/// timer periods, and would otherwise end at the same place in the handler every time.
const OS_NUDGE: u32 = 37;
const OS_NUDGES: usize = 200;
/// The most slices any rig loop runs before it gives up: about 20 times the longest legitimate run.
pub const MAX_SLICES: usize = 40_000;

#[derive(Debug, PartialEq)]
pub enum Step {
    Done,
    /// Ended inside the wait; the pc to start the wait again from.
    Waiting(u16),
    Running,
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
        KernelError::BadSymbol => 16,
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
    library_of_axioms(count, "|- T")
}

/// The same axioms over one symbol, so that kernel_init has no names to compare and never reads
/// the ON key.
pub fn single_symbol_axioms(count: usize) -> Vec<u8> {
    library_of_axioms(count, "T")
}

/// The axioms of `axioms`, with `names` more constants whose names share a prefix of 30 bytes, so
/// that kernel_init spends about 100,000 T-states between two reads of the ON key.
pub fn long_name_axioms(count: usize, names: usize) -> Vec<u8> {
    library_with("|- T", &format!("$c {} $.\n", long_names(names)), count)
}

fn long_names(names: usize) -> String {
    (0..names)
        .map(|i| format!("{}{i:03}", "n".repeat(30)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn library_of_axioms(count: usize, conclusion: &str) -> Vec<u8> {
    library_with(conclusion, "", count)
}

fn library_with(conclusion: &str, extra: &str, count: usize) -> Vec<u8> {
    let mut src = format!("$c T wff |- $.\n{extra}");
    for i in 0..count {
        src += &format!("a-long-label-to-fill-the-page-{i} $a {conclusion} $.\n");
    }
    let db = mm::parse(src.into_bytes()).expect("it parses");
    let all: Vec<_> = (0..db.stmts.len() as mm::StmtId).collect();
    pack(&db, &all).expect("it packs")
}

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

/// Where the kernel reads the ON key and gates on it, `in a,(4)` then `and 8`, in the image
/// `bytes` loaded at `base`. The exit check reads it with `xor 8` and does not match.
pub fn poll_sites(bytes: &[u8], base: u16) -> Vec<u16> {
    let at = |i: usize| base + u16::try_from(i).expect("the image fits a bank");
    (0..bytes.len().saturating_sub(3))
        .filter(|&i| bytes[i..i + 4] == [0xDB, 0x04, 0xE6, 0x08])
        .map(at)
        .collect()
}

/// Steps one instruction at a time until the pc is `target` in the app's page. Returns the T-states
/// that took.
pub fn step_to(m: &mut Machine, target: u16, page: Option<u8>) -> u32 {
    let mut total = 0;
    for _ in 0..50_000_000 {
        if m.pc() == target && page.is_none_or(|p| m.in_port(6) == p) {
            return total;
        }
        let run = m.resume(1);
        assert_eq!(run.stop, Stop::Limit, "the run ended before {target:04x}");
        total += run.tstates;
    }
    panic!("never reached {target:04x}");
}

/// Runs one instruction at a time for at least `tstates` T-states, with ON down if `down`, and
/// proves meanwhile that the code stays inside the kernel with interrupts off and reaches none of
/// the `polls`, so that the level poll cannot have seen the key. Returns the T-states run.
pub fn run_in_kernel(m: &mut Machine, kernel: &Range<u16>, polls: &[u16], tstates: u32) -> u32 {
    let mut total = 0;
    while total < tstates {
        let (pc, iff) = (m.pc(), m.regs().iff1);
        assert!(kernel.contains(&pc) && iff == 0, "pc {pc:04x}, iff {iff}");
        assert!(
            !polls.contains(&pc),
            "the code reached the poll at {pc:04x}"
        );
        total += m.resume(1).tstates;
    }
    let (pc, iff) = (m.pc(), m.regs().iff1);
    assert!(kernel.contains(&pc) && iff == 0, "pc {pc:04x}, iff {iff}");
    total
}

/// Presses ON, holds it for `tstates` T-states inside the kernel, and releases it. Returns the
/// T-states it was down.
pub fn tap_in_kernel(m: &mut Machine, kernel: &Range<u16>, polls: &[u16], tstates: u32) -> u32 {
    m.press_key(Machine::KEY_ON);
    let down = run_in_kernel(m, kernel, polls, tstates);
    m.release_key(Machine::KEY_ON);
    down
}

/// A call of a kernel entry point that is about to start: where it returns to and the stack
/// pointer then.
pub struct Call {
    ret: u16,
    sp: u16,
}

impl Call {
    /// Reads the return address of the call whose first instruction is next.
    pub fn at_entry(m: &mut Machine) -> Call {
        let sp = m.regs().sp;
        let ret = m.read_ram(sp, 2);
        Call {
            ret: u16::from_le_bytes([ret[0], ret[1]]),
            sp: sp + 2,
        }
    }

    /// Runs on until the call has returned, and gives the registers it returned with.
    pub fn returns(&self, m: &mut Machine) -> mcemu::Regs {
        for _ in 0..50_000_000 {
            if m.pc() == self.ret && m.regs().sp == self.sp {
                return m.regs();
            }
            assert_eq!(m.resume(1).stop, Stop::Limit);
        }
        panic!("the call never returned");
    }
}

/// Bit 0 of port 4, the ON key's interrupt latch.
pub fn latched(m: &mut Machine) -> bool {
    m.in_port(4) & 1 != 0
}

pub struct Rig {
    pub m: Machine,
}

impl Rig {
    pub fn new(library: &[u8], speed: Speed) -> Rig {
        Rig::with_code(library, speed, CODE)
    }

    /// A rig that runs `code` in place of the test image as built.
    pub fn with_code(library: &[u8], speed: Speed, code: &[u8]) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(speed);
        m.load_flash(FLASH_TEST_PAGE, 0, code);
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

    /// Runs the first instruction, so that `resume` and the stepping helpers can go on from it.
    pub fn start(&mut self) {
        assert_eq!(self.m.run(ENTRY, 1).stop, Stop::Limit);
    }

    /// True while the code is in the loop of `on_release_and_clear`, which ends where it clears the
    /// flag.
    pub fn in_wait(&self) -> bool {
        (label("ON_RELEASE_AND_CLEAR")..label("ON_WAIT_END")).contains(&self.m.pc())
    }

    /// Runs one slice from `pc` with interrupts off, or on from where the last slice stopped with
    /// registers, interrupts and halt as it left them. A slice that ends inside the wait for ON
    /// comes back as `Waiting(restart)`: the wait is a loop that starts with `ei`, so starting it
    /// again is the same as going on, and the halt in it wakes on the timer interrupt.
    pub fn step(&mut self, from: Option<u16>) -> Step {
        let mut run = match from {
            Some(pc) => self.m.run(pc, SLICE),
            None => self.m.resume(SLICE),
        };
        for _ in 0..OS_NUDGES {
            if run.stop != Stop::Limit || self.m.pc() >= ENTRY {
                break;
            }
            run = self.m.resume(OS_NUDGE);
        }
        match run.stop {
            Stop::Halted => Step::Done,
            Stop::Limit if self.in_wait() => Step::Waiting(label("ON_RELEASE_AND_CLEAR")),
            Stop::Limit => Step::Running,
            other => panic!("the run stopped with {other:?}"),
        }
    }

    fn gave_up(&self, what: &str, slices: usize) -> ! {
        panic!(
            "{what}: no end after {slices} slices, at pc {:04x}",
            self.m.pc()
        );
    }

    /// Runs slices from `pc` until one ends inside the wait; returns where to go on from.
    pub fn until_waiting(&mut self, pc: u16) -> u16 {
        let mut from = Some(pc);
        for _ in 0..MAX_SLICES {
            match self.step(from.take()) {
                Step::Waiting(restart) => return restart,
                Step::Running => {}
                Step::Done => panic!("the routine ended without waiting for ON"),
            }
        }
        self.gave_up("until_waiting", MAX_SLICES)
    }

    /// Runs from `pc` to the end of the test routine. Each slice that ends inside the wait for ON
    /// to come up counts one wait, and after `holds` of them the key is released. Returns the
    /// waits.
    pub fn finish(&mut self, pc: u16, holds: usize) -> usize {
        self.finish_from(Some(pc), holds)
    }

    /// `finish` from where the last run stopped, interrupts as it left them, when `from` is `None`.
    pub fn finish_from(&mut self, mut from: Option<u16>, holds: usize) -> usize {
        let mut waits = 0;
        for _ in 0..MAX_SLICES {
            match self.step(from.take()) {
                Step::Done => return waits,
                Step::Running => {}
                Step::Waiting(restart) => {
                    waits += 1;
                    if waits == holds {
                        self.m.release_key(Machine::KEY_ON);
                    }
                    from = Some(restart);
                }
            }
        }
        self.gave_up(&format!("finish, {waits} waits"), MAX_SLICES)
    }

    /// Runs in steps of `step` T-states, from the start, until the trace holds `marker`.
    pub fn run_until_trace_has(&mut self, marker: &str, step: u32) -> u16 {
        let mut run = self.m.run(ENTRY, step);
        for _ in 0..100_000 {
            assert_eq!(run.stop, Stop::Limit, "the run ended before {marker:?}");
            if String::from_utf8_lossy(&self.trace()).contains(marker) {
                return self.m.pc();
            }
            run = self.m.resume(step);
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
