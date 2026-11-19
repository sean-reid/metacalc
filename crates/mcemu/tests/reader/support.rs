use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{Library, decode, pack};
use mcpack::mm::{self, Database, StmtId};
use std::path::Path;

#[derive(Clone, Copy)]
pub struct Image {
    pub code: &'static [u8],
    pub labels: &'static str,
    /// The address of the first byte of `code`; below $4000 the image starts in page 0.
    pub org: u16,
}

macro_rules! image {
    ($name:literal) => {
        Image {
            code: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".bin")),
            labels: include_str!(concat!(env!("OUT_DIR"), "/", $name, ".lab")),
            org: 0x4000,
        }
    };
}

pub const DEFAULT: Image = image!("reader_test");
pub const WITH_KERNEL: Image = image!("reader_kernel_test");
pub const MOVED: Image = image!("reader_moved");
pub const STRADDLE: Image = image!("reader_straddle");

pub const LIB_PAGE: u8 = 0x6F;
pub const CHUNK: usize = 1 << 14;
pub const RUN_LIMIT: u32 = 1_000_000_000;
pub const OS_PAGE: u8 = 0;
pub const APP_PAGE: u8 = 1;
pub const SPARE_PAGE: u8 = 3;

/// Where the tests keep the caller's buffers, in bank B.
pub const BUF: u16 = 0xA100;
pub const SCRATCH: u16 = 0xA800;
pub const KERNEL_PARAMS: u16 = 0xA400;

pub const ST_OK: u8 = 0;
pub const ST_RANGE: u8 = 1;
pub const ST_ARG: u8 = 2;
pub const ST_BAD: u8 = 3;
pub const ST_CLOSED: u8 = 4;
pub const ST_INDEX: u8 = 5;

pub fn constant_of(labels: &str, name: &str) -> u32 {
    labels
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u32::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

pub fn label_in(image: Image, name: &str) -> u16 {
    u16::try_from(constant_of(image.labels, name)).unwrap_or_else(|_| panic!("{name} too wide"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub ix: u16,
    pub iy: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Out {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub ix: u16,
    pub iy: u16,
    pub iff: u8,
    pub port5: u8,
    pub port7: u8,
    pub tstates: u32,
}

impl Out {
    pub fn status(&self) -> u8 {
        (self.af >> 8) as u8
    }
}

/// The caller's side of the reader: a machine with the library loaded in flash and a driver that
/// makes one call per run, with the registers and interrupt state a test sets.
pub struct Rig {
    pub m: Machine,
    pub image: Image,
    pub lib_page: u8,
    pub ei: bool,
    pub sp: u16,
    /// T-states the last call took, the driver's own included.
    pub spent: u32,
}

impl Rig {
    pub fn new() -> Rig {
        Rig::with_image(DEFAULT)
    }

    pub fn with_image(image: Image) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(Speed::Mhz6);
        let (head, tail) = image
            .code
            .split_at(usize::from(0x4000 - image.org).min(image.code.len()));
        if !head.is_empty() {
            m.load_flash(0, usize::from(image.org), head);
        }
        m.load_flash(FLASH_TEST_PAGE, 0, tail);
        Rig {
            m,
            image,
            lib_page: LIB_PAGE,
            ei: false,
            sp: 0,
            spent: 0,
        }
    }

    pub fn with_library(library: &[u8]) -> Rig {
        let mut rig = Rig::new();
        rig.load(library);
        rig
    }

    pub fn load(&mut self, library: &[u8]) {
        for (i, chunk) in library.chunks(CHUNK).enumerate() {
            self.m.load_flash(self.lib_page - i as u8, 0, chunk);
        }
    }

    pub fn label(&self, name: &str) -> u16 {
        label_in(self.image, name)
    }

    pub fn poke(&mut self, addr: u16, bytes: &[u8]) {
        self.m.load_ram(addr, bytes);
    }

    pub fn peek(&mut self, addr: u16, len: usize) -> Vec<u8> {
        self.m.read_ram(addr, len)
    }

    pub fn named(&mut self, name: &str, len: usize) -> Vec<u8> {
        self.peek(self.label(name), len)
    }

    /// Runs the routine `target` once with `regs` and returns what it left behind.
    pub fn call(&mut self, target: &str, regs: Regs) -> Out {
        self.prepare(target, regs);
        let run = self.m.run(self.label("CALL_ENTRY"), RUN_LIMIT);
        assert_eq!(run.stop, Stop::Halted, "{target} did not halt");
        self.outcome(run.tstates)
    }

    /// Like `call`, with the ON key pressed after `after` T-states, so that an interrupt arrives
    /// in the middle of the routine if interrupts are on there. Returns the outcome and how many
    /// interrupts the handler took.
    pub fn call_interrupted(&mut self, target: &str, regs: Regs, after: u32) -> (Out, u8) {
        self.prepare(target, regs);
        self.poke(self.label("TEST_MASK"), &[1]);
        self.poke(self.label("IRQ_COUNT"), &[0]);
        let mut run = self.m.run(self.label("CALL_ENTRY"), after);
        if run.stop == Stop::Limit {
            self.m.press_key(Machine::KEY_ON);
            run = self.m.run_app(RUN_LIMIT);
            self.m.release_key(Machine::KEY_ON);
        }
        assert_eq!(run.stop, Stop::Halted, "{target} did not halt");
        let taken = self.peek(self.label("IRQ_COUNT"), 1)[0];
        self.poke(self.label("TEST_MASK"), &[0]);
        (self.outcome(run.tstates), taken)
    }

    /// Puts the interrupt handler in RAM and points the interrupt vector at it.
    pub fn install_handler(&mut self) {
        let org = self.image.org;
        let start = usize::from(self.label("IRQ_BODY_START") - org);
        let end = usize::from(self.label("IRQ_BODY_END") - org);
        let code = self.image.code[start..end].to_vec();
        self.poke(self.label("IRQ_HANDLER"), &code);
        let at = self.label("IRQ_HANDLER").to_le_bytes();
        self.m.load_flash(0, 0x38, &[0xC3, at[0], at[1]]);
    }

    fn prepare(&mut self, target: &str, regs: Regs) {
        let word = |value: u16| value.to_le_bytes();
        let image = self.image;
        let at = |name: &str| label_in(image, name);
        let writes = [
            ("TEST_TARGET", word(at(target)).to_vec()),
            ("TEST_SP", word(self.sp).to_vec()),
            ("TEST_EI", vec![u8::from(self.ei)]),
            ("TEST_MASK", vec![0]),
            ("IN_A", vec![regs.a]),
            ("IN_BC", word(regs.bc).to_vec()),
            ("IN_DE", word(regs.de).to_vec()),
            ("IN_HL", word(regs.hl).to_vec()),
            ("IN_IX", word(regs.ix).to_vec()),
            ("IN_IY", word(regs.iy).to_vec()),
        ];
        for (name, bytes) in writes {
            self.poke(at(name), &bytes);
        }
    }

    fn outcome(&mut self, tstates: u32) -> Out {
        self.spent = tstates;
        let at = |name: &str| label_in(self.image, name);
        let out = self.peek(at("OUT_AF"), 0x0F);
        let w = |i: usize| u16::from_le_bytes([out[i], out[i + 1]]);
        Out {
            af: w(0),
            bc: w(2),
            de: w(4),
            hl: w(6),
            ix: w(8),
            iy: w(10),
            iff: out[12],
            port5: out[13],
            port7: out[14],
            tstates,
        }
    }

    /// T-states a call takes beyond the driver's own.
    pub fn cost(&mut self, target: &str, regs: Regs) -> u32 {
        let base = self.call("CALL_NOP", regs).tstates;
        self.call(target, regs).tstates - base
    }

    pub fn open(&mut self) -> u8 {
        let regs = Regs {
            a: self.lib_page,
            de: SCRATCH,
            ..Regs::default()
        };
        self.call("LIB_OPEN", regs).status()
    }

    pub fn open_ok(&mut self) {
        assert_eq!(self.open(), ST_OK);
    }

    /// `kernel_init` on the loaded library with every limit at its largest.
    pub fn kernel_init(&mut self) -> u8 {
        let mut block = vec![self.lib_page];
        for value in [0xFFFFu16; 4] {
            block.extend(value.to_le_bytes());
        }
        self.poke(KERNEL_PARAMS, &block);
        let regs = Regs {
            hl: KERNEL_PARAMS,
            ..Regs::default()
        };
        self.call("KERNEL_INIT", regs).status()
    }

    pub fn kernel_verify(&mut self, id: usize) -> u8 {
        let regs = Regs {
            hl: id as u16,
            ..Regs::default()
        };
        self.call("KERNEL_VERIFY", regs).status()
    }

    pub fn read_call(&mut self, offset: u32, len: u16, dest: u16) -> Out {
        let regs = Regs {
            a: (offset >> 16) as u8,
            bc: len,
            de: dest,
            hl: offset as u16,
            ..Regs::default()
        };
        self.call("LIB_READ", regs)
    }

    /// Bytes `lib_read` copies for the offset and length, and its status.
    pub fn read(&mut self, offset: u32, len: u16) -> (u8, Vec<u8>) {
        let status = self.read_call(offset, len, BUF).status();
        (status, self.peek(BUF, usize::from(len.min(256))))
    }

    pub fn counts(&mut self) -> (u8, u16, u8, u8) {
        let a = self.call("LIB_ASSERTION_COUNT", Regs::default());
        let s = self.call("LIB_SYMBOL_COUNTS", Regs::default());
        (a.status(), a.hl, (s.bc >> 8) as u8, s.bc as u8)
    }

    /// (status, name, typecode id or FFh).
    pub fn symbol_name(&mut self, id: u8) -> (u8, Vec<u8>, u8) {
        let regs = Regs {
            a: id,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_SYMBOL_NAME", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), self.peek(BUF, len), out.bc as u8)
    }

    pub fn label_of(&mut self, n: u16) -> (u8, Vec<u8>) {
        let regs = Regs {
            hl: n,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_LABEL", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), self.peek(BUF, len))
    }

    pub fn record(&mut self, n: u16) -> u8 {
        let regs = Regs {
            hl: n,
            ..Regs::default()
        };
        self.call("LIB_RECORD", regs).status()
    }

    pub fn conclusion(&mut self, n: u16) -> (u8, Vec<u8>) {
        let regs = Regs {
            hl: n,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_CONCLUSION", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), self.peek(BUF, len))
    }

    /// (status, symbols, FFh for an essential hypothesis or the float's variable).
    pub fn hypothesis(&mut self, n: u16, k: u8) -> (u8, Vec<u8>, u8) {
        let regs = Regs {
            hl: n,
            bc: u16::from(k) << 8,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_HYPOTHESIS", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), self.peek(BUF, len), out.bc as u8)
    }

    /// A byte of the descriptor or state block, by label.
    pub fn state_byte(&mut self, name: &str) -> u8 {
        self.named(name, 1)[0]
    }
}

pub fn parse(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

pub fn corpus(name: &str) -> Database {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses")
}

pub fn pack_all(db: &Database) -> Vec<u8> {
    let everything: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(db, &everything).expect("it packs")
}

pub const CORPUS: [&str; 12] = [
    "anatomy.mm",
    "anatomy-bad1.mm",
    "anatomy-bad2.mm",
    "anatomy-bad3.mm",
    "big-unifier.mm",
    "big-unifier-bad1.mm",
    "big-unifier-bad2.mm",
    "big-unifier-bad3.mm",
    "demo0.mm",
    "demo0-bad1.mm",
    "emptyline.mm",
    "miu.mm",
];

/// Thousands of axioms of every small shape, so that the directory and the records cross several
/// 16 KB chunks and records start in every one of them.
pub fn big_library() -> Vec<u8> {
    let mut src = String::from("$c wff |- ( ) -> & $.\n$v p q r s $.\n");
    src += "wp $f wff p $.\nwq $f wff q $.\nwr $f wff r $.\nws $f wff s $.\n";
    for i in 0..6000 {
        let body = [
            "p",
            "( p -> q )",
            "( p & ( q -> r ) )",
            "( ( p -> q ) & ( r -> s ) )",
        ];
        let expr = |k: usize| body[(i + k) % 4];
        let mut hyps = String::new();
        for h in 0..i % 4 {
            hyps += &format!("h{h}-{i} $e |- {} $.\n", expr(h + 1));
        }
        let disjoint = if i % 5 == 0 { "$d p q $.\n" } else { "" };
        let tail = "( p -> q ) ".repeat(i % 11);
        src += &format!(
            "${{\n{disjoint}{hyps}ax-{i:05}{pad} $a |- ( {} {tail}) $.\n$}}\n",
            expr(0),
            pad = "x".repeat(i % 23),
        );
    }
    pack_all(&parse(&src))
}

pub fn names(library: &Library) -> Vec<String> {
    let variables = library.variables.iter().map(|v| v.name.clone());
    library.constants.iter().cloned().chain(variables).collect()
}

pub fn decoded(bytes: &[u8]) -> Library {
    decode(bytes).expect("the library decodes")
}
