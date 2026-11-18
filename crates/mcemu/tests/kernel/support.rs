use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm::{self, Database, Kind, StmtId};
use std::path::Path;

pub use crate::sources::*;

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

/// The same kernel with its two RAM regions moved to $C100 and $E000.
pub const MOVED: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/kernel_test_moved.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/kernel_test_moved.lab")),
};

/// The same kernel with its low region ending at $10000 and its high region at the bottom of bank C.
pub const TOP: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/kernel_test_top.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/kernel_test_top.lab")),
};

/// Every layout build.rs assembles as a kernel image.
pub const LAYOUTS: [(&str, Image); 3] = [("default", DEFAULT), ("moved", MOVED), ("top", TOP)];

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
        Err(KernelError::LooseReference) => "ST_LOOSE_REF",
        Err(KernelError::BadSymbol) => "ST_BAD_SYMBOL",
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

/// The two regions of the kernel's page that the image writes, as (start, length).
pub fn regions(image: Image) -> [(u16, usize); 2] {
    let at = |name: &str| label_in(image, name);
    let size = |name: &str| usize::from(at(name));
    [
        (at("KERNEL_RAM_LOW"), size("LOW_SIZE")),
        (at("KERNEL_RAM_HIGH"), size("HIGH_SIZE")),
    ]
}

/// The two regions of a built image as its build.rs entry places them, written out here and not
/// read from the kernel; an image this table does not know falls back to the labels.
pub fn layout_regions(layout: &str, image: Image) -> [(u16, usize); 2] {
    match layout {
        "default" => [(0xC500, 0x1200), (0xDF00, 0x2000)],
        "moved" => [(0xC100, 0x1200), (0xE000, 0x2000)],
        "top" => [(0xEE00, 0x1200), (0xC100, 0x2000)],
        _ => regions(image),
    }
}

pub fn in_regions(image: Image, addr: u16) -> bool {
    regions(image).iter().any(|&(start, len)| {
        (usize::from(start)..usize::from(start) + len).contains(&usize::from(addr))
    })
}

/// Fills every byte of the kernel's page that the image does not write.
pub fn fill_outside(page: &mut [u8], image: Image, value: u8) {
    let [(low, low_len), (high, high_len)] = regions(image);
    for (at, byte) in page.iter_mut().enumerate() {
        let addr = 0xC000 + at;
        let inside =
            |start: u16, len: usize| (usize::from(start)..usize::from(start) + len).contains(&addr);
        if !inside(low, low_len) && !inside(high, high_len) {
            *byte = value;
        }
    }
}

/// The address of the first byte outside the image's regions that is not `value`.
pub fn first_outside_not(page: &[u8], image: Image, value: u8) -> Option<u16> {
    let mut expected = page.to_vec();
    fill_outside(&mut expected, image, value);
    page.iter()
        .zip(&expected)
        .position(|(a, b)| a != b)
        .map(|at| 0xC000 + at as u16)
}

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
        Device::with_image_library(DEFAULT, library)
    }

    pub fn with_image_library(image: Image, library: &[u8]) -> Device {
        let mut device = Device::with_image(image);
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

    /// The nine-byte parameter block the kernel's init takes.
    pub fn param_block(&self, limits: Limits) -> Vec<u8> {
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
        block
    }

    /// Writes the block where the test code reads it by default, in the test's flash page, so
    /// that it survives whatever the test does to bank B.
    pub fn params(&mut self, limits: Limits) {
        let block = self.param_block(limits);
        self.write_params(&block);
    }

    pub fn write_params(&mut self, block: &[u8]) {
        let at = usize::from(self.label("TEST_PARAMS") - 0x4000);
        self.m.load_flash(FLASH_TEST_PAGE, at, block);
        let ptr = self.label("TEST_PARAMS").to_le_bytes();
        self.m.load_ram(self.label("TEST_PARAM_PTR"), &ptr);
    }

    /// Writes the block into bank B, where an app keeps its own data, and points init at it.
    pub fn params_in_bank_b(&mut self, limits: Limits, at: u16) {
        let block = self.param_block(limits);
        self.m.load_ram(at, &block);
        self.m
            .load_ram(self.label("TEST_PARAM_PTR"), &at.to_le_bytes());
    }

    /// Bytes of the kernel's page, which only the kernel sees while it runs.
    pub fn kram(&mut self, addr: u16, len: usize) -> Vec<u8> {
        let at = usize::from(addr - 0xC000);
        self.m.ram_page(SPARE_PAGE)[at..at + len].to_vec()
    }

    pub fn kram_write(&mut self, addr: u16, bytes: &[u8]) {
        let at = usize::from(addr - 0xC000);
        self.m.ram_page(SPARE_PAGE)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    pub fn kram_label(&mut self, name: &str, len: usize) -> Vec<u8> {
        self.kram(self.label(name), len)
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
        let room = usize::from(self.label("TEST_STATUSES_END") - self.label("TEST_STATUSES"));
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
    agree_in(DEFAULT, bytes, id, limits, expected, what);
}

pub fn agree_in(
    image: Image,
    bytes: &[u8],
    id: usize,
    limits: Limits,
    expected: Result<(), KernelError>,
    what: &str,
) {
    let model = Kernel::new(bytes, clamped(limits)).and_then(|mut kernel| kernel.verify(id));
    assert_eq!(model, expected, "{what}: the model");
    let got = Device::with_image_library(image, bytes).verify(id, limits);
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

pub const PORT_BANK_B: u8 = 7;
pub const SPARE_PAGE: u8 = 3;
pub const RAM_PAGE: u8 = 0x81;

/// Every assertion of the small library, which reaches every error path the model has.
pub fn small_library() -> (Database, Vec<u8>) {
    let db = parse(&format!(
        "{DB} ${{ $d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $. $}}"
    ));
    let bytes = pack_all(&db);
    (db, bytes)
}
