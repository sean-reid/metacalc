//! The check job in the app of routines/check_test.z80, called one routine at a time. The library
//! is in the flash pages after the app's three, as it is on the calculator.

use crate::layout::{self, Table};
use mcemu::{Machine, STUB_ROM, Stop};
use mcpack::library::{Kernel, Limits, crc32, decode, pack};
use mcpack::mm::{self, StmtId};
use mcui::data::{Installed, Lib};
use mcui::{Mark, Status};
use std::path::Path;

pub const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/check_test.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/check_test.lab"));
pub const BASE: u8 = 0x6F;
/// The page of library chunk 0: the app has three code pages.
pub const LIB_PAGE: u8 = BASE - 3;
pub const VAR: &str = "MCJOBT";
const LIMIT: u32 = 2_000_000_000;
const PAGE: usize = 1 << 14;

pub const READY: u8 = 0;
pub const MORE: u8 = 1;
pub const NEED_CRC: u8 = 2;
pub const DONE: u8 = 3;
pub const PAUSED: u8 = 4;
pub const REFUSED: u8 = 5;
pub const STORE: u8 = 6;
pub const RANGE: u8 = 7;
pub const BAD_STATE: u8 = 9;

pub const ST_ABSENT: u8 = 7;
pub const ST_ARCHIVED: u8 = 2;

pub fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// What a routine of the job returned: A, and BC, DE and HL, with the T-states the call took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Out {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub tstates: u32,
}

impl Out {
    pub fn b(&self) -> u8 {
        (self.bc >> 8) as u8
    }
}

pub struct Job {
    pub m: Machine,
}

impl Job {
    /// The app on the stub OS with `library` after it, launched and halted at its start with the
    /// reader open.
    pub fn new(library: &[u8]) -> Job {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(APP, BASE).expect("the app installs");
        for (i, chunk) in library.chunks(PAGE).enumerate() {
            m.load_flash(LIB_PAGE - i as u8, 0, chunk);
        }
        m.launch(BASE).expect("the app launches");
        let run = m.run_app(LIMIT);
        assert_eq!(run.stop, Stop::Halted, "the app halts at its start");
        let mut job = Job { m };
        assert_eq!(job.byte("T_OPEN"), 0, "the reader opens the library");
        job
    }

    pub fn byte(&mut self, name: &str) -> u8 {
        self.m.read_ram(label(name), 1)[0]
    }

    pub fn word(&mut self, name: &str) -> u16 {
        let b = self.m.read_ram(label(name), 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    pub fn bytes(&mut self, name: &str, len: usize) -> Vec<u8> {
        self.m.read_ram(label(name), len)
    }

    pub fn poke(&mut self, name: &str, bytes: &[u8]) {
        self.m.load_ram(label(name), bytes);
    }

    /// The four counts n0 to n3 the job keeps for the first N assertions.
    pub fn counts(&mut self) -> [u16; 4] {
        let b = self.bytes("ck_counts", 8);
        [0, 2, 4, 6].map(|i| u16::from_le_bytes([b[i], b[i + 1]]))
    }

    pub fn first_failure(&mut self) -> Option<(u16, u8)> {
        (self.byte("ck_have_first") != 0).then(|| (self.word("ck_first"), self.byte("ck_first_st")))
    }

    /// How many real calls the job has made to each kernel routine, in the order reset, init,
    /// crc_begin, crc_chunk, verify, refs.
    pub fn kernel_calls(&mut self) -> [u8; 6] {
        self.bytes("T_COUNTERS", 6).try_into().unwrap()
    }

    pub fn clear_kernel_calls(&mut self) {
        self.poke("T_COUNTERS", &[0; 6]);
    }

    fn out(&mut self, tstates: u32) -> Out {
        let b = self.bytes("T_OUT_A", 9);
        let word = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
        Out {
            a: b[0],
            bc: word(1),
            de: word(3),
            hl: word(5),
            tstates,
        }
    }

    /// Runs an entry to its halt.
    pub fn call(&mut self, entry: &str) -> Out {
        let run = self.m.run(label(entry), LIMIT);
        assert_eq!(run.stop, Stop::Halted, "{entry} did not halt");
        self.out(run.tstates)
    }

    /// Runs an entry with ON pressed `after` T-states into it, and held to its halt. When the
    /// entry halts first, ON is not pressed and the second value is false.
    pub fn call_with_on(&mut self, entry: &str, after: u32) -> (Out, bool) {
        if after == 0 {
            self.m.press_key(Machine::KEY_ON);
            let out = self.call(entry);
            return (out, true);
        }
        let first = self.m.run(label(entry), after);
        if first.stop == Stop::Halted {
            let out = self.out(first.tstates);
            return (out, false);
        }
        assert_eq!(first.stop, Stop::Limit);
        self.m.press_key(Machine::KEY_ON);
        let rest = self.m.resume(LIMIT);
        assert_eq!(rest.stop, Stop::Halted, "{entry} did not halt");
        (self.out(first.tstates + rest.tstates), true)
    }

    /// Lets go of ON and takes the interrupt it latched, as a caller does after a pause.
    pub fn release_on(&mut self) {
        self.m.release_key(Machine::KEY_ON);
        let run = self.m.run(label("t_ack"), LIMIT);
        assert_eq!(run.stop, Stop::Halted);
    }

    pub fn arg(&mut self, page: u8, n: u16, limit: u16) {
        self.poke("T_IN_A", &[page]);
        self.poke("T_IN_DE", &n.to_le_bytes());
        self.poke("T_IN_N", &limit.to_le_bytes());
        self.poke("T_IN_C", &[0xFF, 0xFF]);
    }

    pub fn begin(&mut self, n: u16) -> Out {
        self.arg(LIB_PAGE, n, 0);
        self.call("t_begin")
    }

    pub fn crc_step(&mut self) -> Out {
        self.call("t_crc_step")
    }

    pub fn step(&mut self) -> Out {
        self.call("t_step")
    }

    pub fn end(&mut self) -> Out {
        self.call("t_end")
    }

    /// Calls made by the last `t_run_crc`, `t_run_steps` or `t_job`.
    pub fn calls(&mut self) -> u16 {
        self.word("T_CALLS")
    }

    /// Begins and runs the CRC steps and the steps to the end, or to `limit` steps.
    pub fn job(&mut self, n: u16, limit: u16) -> Out {
        self.arg(LIB_PAGE, n, limit);
        self.call("t_job")
    }

    pub fn run_crc(&mut self, limit: u16) -> Out {
        self.poke("T_IN_C", &limit.to_le_bytes());
        self.call("t_run_crc")
    }

    pub fn table(&mut self) -> Option<Table> {
        let var = self.m.read_var(VAR)?;
        Some(layout::decode(&var.data).expect("the table decodes"))
    }

    pub fn put_table(&mut self, t: &Table, padding: u8) {
        self.m.delete_var(VAR);
        self.m
            .create_var(VAR, &layout::encode_padded(t, padding))
            .expect("room for the table");
    }
}

pub fn corpus(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    let db = mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("it packs")
}

pub fn pack_source(src: &str) -> Vec<u8> {
    let db = mm::parse(src.as_bytes().to_vec()).expect("it parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("it packs")
}

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

pub fn header_crc(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[4..8].try_into().unwrap())
}

pub fn count_of(bytes: &[u8]) -> u16 {
    decode(bytes).expect("the library decodes").assertions.len() as u16
}

/// The limits the device clamps to, as the model must take them.
fn device_limits() -> Limits {
    let cap = |name: &str| usize::from(label(name));
    Limits {
        stack_syms: cap("CAP_STACK_SYMS"),
        stack_entries: cap("CAP_STACK_ENTRIES"),
        saved_syms: cap("CAP_SAVED_SYMS"),
        saved_entries: cap("CAP_SAVED_ENTRIES"),
    }
}

/// Whether the model accepts each assertion of the library, one fresh verify after another on a
/// single kernel. None when the model refuses the library.
pub fn verdicts(bytes: &[u8]) -> Option<Vec<bool>> {
    let mut kernel = Kernel::new(bytes, device_limits()).ok()?;
    Some(
        (0..kernel.assertions())
            .map(|id| kernel.verify(id).is_ok())
            .collect(),
    )
}

/// What the status table holds for each assertion when the model's verdicts are all in: the host
/// model's mark of each, as the table writes it.
pub fn marks(bytes: &[u8], passes: &[bool]) -> Vec<u8> {
    let status = passes
        .iter()
        .map(|&p| if p { Status::Checked } else { Status::Failed })
        .collect();
    let installed = Installed {
        name: "library".into(),
        bytes: bytes.to_vec(),
    };
    let lib = Lib::new(installed, status).expect("the host model loads the library");
    (0..passes.len())
        .map(|i| match lib.mark(i) {
            Mark::Unchecked => 0,
            Mark::Checked => 1,
            Mark::Failed => 2,
            Mark::Resting => 3,
        })
        .collect()
}

/// The assertion numbers each assertion's proof names, dummies left out.
pub fn references(bytes: &[u8]) -> Vec<Vec<usize>> {
    use mcpack::library::Reference;
    decode(bytes)
        .expect("the library decodes")
        .assertions
        .iter()
        .map(|a| {
            a.references
                .iter()
                .filter_map(|r| match r {
                    Reference::Assertion(i) => Some(*i),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

pub fn xorshift(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

/// A table for the library with `values` and the library's CRC and count.
pub fn table_for(bytes: &[u8], values: Vec<u8>) -> Table {
    let mut t = Table::new(header_crc(bytes), count_of(bytes));
    t.values = values;
    t
}

impl Job {
    /// The bytes of the table variable after the OS's size word, if it is in RAM or the archive.
    pub fn raw(&mut self) -> Option<Vec<u8>> {
        self.m.read_var(VAR).map(|v| v.data)
    }
}

/// A library of `count` axioms with long labels, so that it spans several 16 KB chunks.
pub fn axioms(count: usize) -> Vec<u8> {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..count {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    pack_source(&src)
}
