use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm::{self, Database, Kind, StmtId};
use std::path::Path;

/// A kernel assembled with the test entry stub, and the label file the assembler wrote for it.
#[derive(Clone, Copy)]
pub struct Image {
    pub code: &'static [u8],
    pub labels: &'static str,
}

pub const DEFAULT: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/kernel_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/kernel_test.lab")),
};

/// The same kernel with its RAM block moved to $C400.
pub const MOVED: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/kernel_test_moved.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/kernel_test_moved.lab")),
};

/// The flash page of chunk 0; chunk c is at this page minus c.
pub const LIB_PAGE: u8 = 0x6F;
pub const PAGE: usize = 1 << 14;
pub const RUN_LIMIT: u32 = 1_000_000_000;

pub fn label(name: &str) -> u16 {
    label_in(DEFAULT, name)
}

pub fn label_in(image: Image, name: &str) -> u16 {
    label_of(image.labels, name)
}

/// A label from an assembler label file.
pub fn label_of(labels: &str, name: &str) -> u16 {
    u16::try_from(constant_of(labels, name)).unwrap_or_else(|_| panic!("{name} is not an address"))
}

/// A constant of the assembled kernel, such as `WORK_LIMIT`, which may not fit 16 bits.
pub fn constant(name: &str) -> u32 {
    constant_of(DEFAULT.labels, name)
}

fn constant_of(labels: &str, name: &str) -> u32 {
    labels
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u32::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// What the model says, as the code the kernel reports for it.
pub fn status(result: Result<(), KernelError>) -> u8 {
    let name = match result {
        Ok(()) => "ST_OK",
        Err(KernelError::Malformed) => "ST_MALFORMED",
        Err(KernelError::BadRecord) => "ST_BAD_RECORD",
        Err(KernelError::NotATheorem) => "ST_NOT_A_THEOREM",
        Err(KernelError::ForwardReference) => "ST_FORWARD_REF",
        Err(KernelError::OutOfMemory) => "ST_OUT_OF_MEMORY",
        Err(KernelError::StackUnderflow) => "ST_STACK_UNDERFLOW",
        Err(KernelError::BadBackReference) => "ST_BAD_BACK_REF",
        Err(KernelError::TypecodeMismatch) => "ST_TYPECODE",
        Err(KernelError::HypothesisMismatch) => "ST_HYP_MISMATCH",
        Err(KernelError::DisjointViolation) => "ST_DISJOINT",
        Err(KernelError::StackNotSingleton) => "ST_NOT_SINGLETON",
        Err(KernelError::WrongConclusion) => "ST_WRONG_CONCLUSION",
        Err(KernelError::WorkLimit) => "ST_WORK_LIMIT",
    };
    label(name) as u8
}

/// The limits the model must use to match a device whose limits are `asked`.
pub fn clamped(asked: Limits) -> Limits {
    let cap = |asked: usize, name: &str| asked.min(label(name) as usize);
    Limits {
        stack_syms: cap(asked.stack_syms, "CAP_STACK_SYMS"),
        stack_entries: cap(asked.stack_entries, "CAP_STACK_ENTRIES"),
        saved_syms: cap(asked.saved_syms, "CAP_SAVED_SYMS"),
        saved_entries: cap(asked.saved_entries, "CAP_SAVED_ENTRIES"),
    }
}

/// Everything the device may need, as a model would see an unbounded budget.
pub const ASKED_UNBOUNDED: Limits = Limits {
    stack_syms: 0xFFFF,
    stack_entries: 0xFFFF,
    saved_syms: 0xFFFF,
    saved_entries: 0xFFFF,
};

pub struct Device {
    pub m: Machine,
    pub image: Image,
    pub lib_page: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Outcome {
    pub init: u8,
    pub verify: u8,
    pub tstates: u32,
}

impl Device {
    pub fn new() -> Device {
        Device::with_image(DEFAULT)
    }

    pub fn with_image(image: Image) -> Device {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(Speed::Mhz15);
        m.load_flash(FLASH_TEST_PAGE, 0, image.code);
        Device {
            m,
            image,
            lib_page: LIB_PAGE,
        }
    }

    pub fn label(&self, name: &str) -> u16 {
        label_in(self.image, name)
    }

    pub fn with_library(library: &[u8]) -> Device {
        let mut device = Device::new();
        device.load(library);
        device
    }

    pub fn with_library_at(page: u8, library: &[u8]) -> Device {
        let mut device = Device::new();
        device.lib_page = page;
        device.load(library);
        device
    }

    pub fn load(&mut self, library: &[u8]) {
        for (i, chunk) in library.chunks(PAGE).enumerate() {
            self.m.load_flash(self.lib_page - i as u8, 0, chunk);
        }
    }

    pub fn params(&mut self, limits: Limits) {
        let word = |value: usize| (value as u16).to_le_bytes();
        let mut block = vec![self.lib_page];
        for value in [
            limits.stack_syms,
            limits.stack_entries,
            limits.saved_syms,
            limits.saved_entries,
        ] {
            block.extend(word(value));
        }
        self.m.load_ram(self.label("KERNEL_LIB_PAGE"), &block);
    }

    pub fn run(&mut self, entry: &str) -> u32 {
        let run = self.m.run(self.label(entry), RUN_LIMIT);
        assert_eq!(run.stop, Stop::Halted, "{entry} did not halt");
        run.tstates
    }

    pub fn byte(&mut self, name: &str) -> u8 {
        self.m.read_ram(self.label(name), 1)[0]
    }

    pub fn verify(&mut self, id: usize, limits: Limits) -> Outcome {
        self.params(limits);
        self.m
            .load_ram(self.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
        let tstates = self.run("ENTRY_ONE");
        Outcome {
            init: self.byte("TEST_INIT_STATUS"),
            verify: self.byte("TEST_VERIFY_STATUS"),
            tstates,
        }
    }

    /// Verifies `ids` in order on one initialisation.
    pub fn verify_two(&mut self, first: usize, second: usize, limits: Limits) -> (u8, u8, u8) {
        self.params(limits);
        self.m
            .load_ram(self.label("TEST_ASSERTION"), &(first as u16).to_le_bytes());
        self.m.load_ram(
            self.label("TEST_ASSERTION2"),
            &(second as u16).to_le_bytes(),
        );
        self.run("ENTRY_TWO");
        (
            self.byte("TEST_INIT_STATUS"),
            self.byte("TEST_VERIFY_STATUS"),
            self.byte("TEST_VERIFY2_STATUS"),
        )
    }

    /// Initialises once, then verifies assertions 0..count in order.
    pub fn verify_all(&mut self, count: usize, limits: Limits) -> (u8, Vec<u8>, u32) {
        let room = usize::from(self.label("TEST_BLOCK") - self.label("TEST_STATUSES"));
        assert!(count <= room, "the status array holds {room} entries");
        self.params(limits);
        self.m
            .load_ram(self.label("TEST_COUNT"), &(count as u16).to_le_bytes());
        let tstates = self.run("ENTRY_ALL");
        let statuses = self.m.read_ram(self.label("TEST_STATUSES"), count);
        (self.byte("TEST_INIT_STATUS"), statuses, tstates)
    }
}

/// What the model says about the same bytes: the status of init, then of each assertion in order.
pub fn model_statuses(bytes: &[u8], limits: Limits) -> (u8, Vec<u8>) {
    match Kernel::new(bytes, clamped(limits)) {
        Err(e) => (status(Err(e)), Vec::new()),
        Ok(mut kernel) => {
            let all = (0..kernel.assertions())
                .map(|id| status(kernel.verify(id)))
                .collect();
            (status(Ok(())), all)
        }
    }
}

pub fn parse(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

pub fn everything(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

pub fn assertions(db: &Database) -> Vec<StmtId> {
    let is_assertion = |&id: &StmtId| matches!(db.stmt(id).kind, Kind::Axiom | Kind::Theorem);
    everything(db).into_iter().filter(is_assertion).collect()
}

pub fn number(db: &Database, label: &str) -> usize {
    let id = db.find(label).expect("label exists");
    assertions(db).iter().position(|&a| a == id).unwrap()
}

pub fn corpus(name: &str) -> Database {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses")
}

pub const DB: &str = "
$c R wff |- $.
$v x y z $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
${
  $d x y $.
  ax-d $a |- R x y $.
$}
${
  m.1 $e |- R x y $.
  ax-m $a |- R x x $.
$}
${
  $d x y $.
  good $p |- R x y $= vx vy ax-d $.
$}
bad $p |- R x y $= vx vy ax-d $.
same $p |- R x x $= vx vx ax-d $.
${
  $d x z $.
  dummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
$}
nodummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
${
  t.1 $e |- R x z $.
  ax-t $a |- R x y $.
$}
${
  $d x y z $.
  both $p |- R x y $= vx vy vz vx vz ax-d ax-t $.
$}
";

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

pub struct Fields {
    pub hyps: usize,
    pub dv: usize,
    pub extra_dv: usize,
    pub refs: usize,
}

/// Offsets inside a record, read from the documented layout; the last two only exist for a theorem.
pub fn fields(bytes: &[u8], id: usize) -> Fields {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = u32_at(u32_at(16) + 4 * id);
    at += 2 + bytes[at + 1] as usize;
    let hyps = at + 1;
    let count = bytes[at];
    at += 1;
    for _ in 0..count {
        at += if bytes[at] == 0 {
            2
        } else {
            2 + bytes[at + 1] as usize
        };
    }
    let dv = at + 2;
    at += 2 + 2 * u16_at(at);
    at += 1 + bytes[at] as usize;
    let extra_dv = at + 2;
    let refs = extra_dv + 2 * u16_at(at) + 2;
    Fields {
        hyps,
        dv,
        extra_dv,
        refs,
    }
}

/// The model and the device on one assertion of one library, each on a fresh start.
pub fn agree(bytes: &[u8], id: usize, expected: Result<(), KernelError>, what: &str) {
    agree_with(bytes, id, ASKED_UNBOUNDED, expected, what);
}

pub fn agree_with(
    bytes: &[u8],
    id: usize,
    limits: Limits,
    expected: Result<(), KernelError>,
    what: &str,
) {
    let model = Kernel::new(bytes, clamped(limits)).and_then(|mut kernel| kernel.verify(id));
    assert_eq!(model, expected, "{what}: the model");
    let got = Device::with_library(bytes).verify(id, limits);
    assert_eq!(got.verify, status(expected), "{what}: the device");
    if expected == Err(KernelError::Malformed) {
        let loads = Kernel::new(bytes, clamped(limits)).is_ok();
        assert_eq!(got.init == 0, loads, "{what}: init");
    }
}

pub fn pack_all(db: &Database) -> Vec<u8> {
    pack(db, &everything(db)).expect("it packs")
}

pub fn model_one(bytes: &[u8], id: usize, limits: Limits) -> u8 {
    status(Kernel::new(bytes, clamped(limits)).and_then(|mut kernel| kernel.verify(id)))
}

/// The device's status for `id` under `limits`, which must be the model's.
pub fn same_as_model(
    device: &mut Device,
    bytes: &[u8],
    id: usize,
    limits: Limits,
    what: &str,
) -> u8 {
    let got = device.verify(id, limits);
    assert_eq!(got.init, 0, "{what}");
    assert_eq!(got.verify, model_one(bytes, id, limits), "{what}");
    got.verify
}

#[derive(Clone, Copy)]
pub enum Tok {
    Var(usize),
    Const,
}

/// A library whose theorem `t` applies an axiom of `terms.len()` variables, all pairwise disjoint,
/// to one term per variable; `nv` variables w0.. are free to use in the terms and the theorem
/// declares all of them disjoint. Without `checked` neither declares a pair, and the proof does
/// the same steps with no disjoint check to make.
pub fn disjoint_source(terms: &[Vec<Tok>], nv: usize, checked: bool) -> String {
    let zs: Vec<String> = (0..terms.len()).map(|i| format!("z{i}")).collect();
    let ws: Vec<String> = (0..nv).map(|i| format!("w{i}")).collect();
    let mut src = format!(
        "$c wff |- R k $. $v p q {} {} $.\n\
         fp $f wff p $. fq $f wff q $. wk $a wff k $. wc $a wff p q $.\n",
        zs.join(" "),
        ws.join(" ")
    );
    for var in zs.iter().chain(&ws) {
        src += &format!("f{var} $f wff {var} $.\n");
    }
    let dv = |vars: &[String]| {
        if checked && vars.len() > 1 {
            format!("$d {} $.", vars.join(" "))
        } else {
            String::new()
        }
    };
    src += &format!("${{ {} ax-d $a |- R {} $. $}}\n", dv(&zs), zs.join(" "));
    let (mut expr, mut proof) = (Vec::new(), Vec::new());
    for term in terms {
        for (i, tok) in term.iter().enumerate() {
            let (sym, label) = match tok {
                Tok::Var(v) => (format!("w{v}"), format!("fw{v}")),
                Tok::Const => ("k".to_string(), "wk".to_string()),
            };
            expr.push(sym);
            proof.push(label);
            if i > 0 {
                proof.push("wc".into());
            }
        }
    }
    proof.push("ax-d".into());
    src += &format!(
        "${{ {} t $p |- R {} $= {} $. $}}\n",
        dv(&ws),
        expr.join(" "),
        proof.join(" ")
    );
    src
}

/// `x` variables then `xk` constants, and `y` other variables then `yk` constants.
pub fn two_terms(x: usize, y: usize, xk: usize, yk: usize) -> Vec<Vec<Tok>> {
    let mut left: Vec<Tok> = (0..x).map(Tok::Var).collect();
    left.extend(vec![Tok::Const; xk]);
    let mut right: Vec<Tok> = (x..x + y).map(Tok::Var).collect();
    right.extend(vec![Tok::Const; yk]);
    vec![left, right]
}

pub fn disjoint_library(terms: &[Vec<Tok>], nv: usize, checked: bool) -> (Database, Vec<u8>) {
    let db = parse(&disjoint_source(terms, nv, checked));
    let bytes = pack_all(&db);
    (db, bytes)
}

pub const PORT_BANK_B: u8 = 7;
pub const RAM_PAGE: u8 = 0x81;

/// Every assertion of the small library, which reaches every error path the model has.
pub fn small_library() -> (Database, Vec<u8>) {
    let db = parse(&format!(
        "{DB} ${{ $d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $. $}}"
    ));
    let bytes = pack_all(&db);
    (db, bytes)
}
