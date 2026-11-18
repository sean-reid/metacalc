use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};
use mcpack::library::{self, Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm::{self, Database, Kind, StmtId, Verifier};
use std::path::Path;

const KERNEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kernel_test.bin"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/kernel_test.lab"));

const LIB_PAGE: u8 = 0x40;
const PAGE: usize = 1 << 14;
const RUN_LIMIT: u32 = 200_000_000;

fn label(name: &str) -> u16 {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// What the model says, as the code the kernel reports for it.
fn status(result: Result<(), KernelError>) -> u8 {
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
    };
    label(name) as u8
}

/// The limits the model must use to match a device whose limits are `asked`.
fn clamped(asked: Limits) -> Limits {
    let cap = |asked: usize, name: &str| asked.min(label(name) as usize);
    Limits {
        stack_syms: cap(asked.stack_syms, "CAP_STACK_SYMS"),
        stack_entries: cap(asked.stack_entries, "CAP_STACK_ENTRIES"),
        saved_syms: cap(asked.saved_syms, "CAP_SAVED_SYMS"),
        saved_entries: cap(asked.saved_entries, "CAP_SAVED_ENTRIES"),
    }
}

/// Everything the device may need, as a model would see an unbounded budget.
const ASKED_UNBOUNDED: Limits = Limits {
    stack_syms: 0xFFFF,
    stack_entries: 0xFFFF,
    saved_syms: 0xFFFF,
    saved_entries: 0xFFFF,
};

struct Device {
    m: Machine,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Outcome {
    init: u8,
    verify: u8,
    tstates: u32,
}

impl Device {
    fn new() -> Device {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(Speed::Mhz15);
        m.load_flash(FLASH_TEST_PAGE, 0, KERNEL);
        Device { m }
    }

    fn with_library(library: &[u8]) -> Device {
        let mut device = Device::new();
        device.load(library);
        device
    }

    fn load(&mut self, library: &[u8]) {
        for (i, chunk) in library.chunks(PAGE).enumerate() {
            self.m.load_flash(LIB_PAGE + i as u8, 0, chunk);
        }
    }

    fn params(&mut self, limits: Limits) {
        let word = |value: usize| (value as u16).to_le_bytes();
        let mut block = vec![LIB_PAGE];
        for value in [
            limits.stack_syms,
            limits.stack_entries,
            limits.saved_syms,
            limits.saved_entries,
        ] {
            block.extend(word(value));
        }
        self.m.load_ram(label("KERNEL_LIB_PAGE"), &block);
    }

    fn run(&mut self, entry: &str) -> u32 {
        let run = self.m.run(label(entry), RUN_LIMIT);
        assert_eq!(run.stop, Stop::Halted, "{entry} did not halt");
        run.tstates
    }

    fn byte(&mut self, name: &str) -> u8 {
        self.m.read_ram(label(name), 1)[0]
    }

    fn verify(&mut self, id: usize, limits: Limits) -> Outcome {
        self.params(limits);
        self.m
            .load_ram(label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
        let tstates = self.run("ENTRY_ONE");
        Outcome {
            init: self.byte("TEST_INIT_STATUS"),
            verify: self.byte("TEST_VERIFY_STATUS"),
            tstates,
        }
    }

    /// Verifies `ids` in order on one initialisation.
    fn verify_two(&mut self, first: usize, second: usize, limits: Limits) -> (u8, u8, u8) {
        self.params(limits);
        self.m
            .load_ram(label("TEST_ASSERTION"), &(first as u16).to_le_bytes());
        self.m
            .load_ram(label("TEST_ASSERTION2"), &(second as u16).to_le_bytes());
        self.run("ENTRY_TWO");
        (
            self.byte("TEST_INIT_STATUS"),
            self.byte("TEST_VERIFY_STATUS"),
            self.byte("TEST_VERIFY2_STATUS"),
        )
    }

    /// Initialises once, then verifies assertions 0..count in order.
    fn verify_all(&mut self, count: usize, limits: Limits) -> (u8, Vec<u8>, u32) {
        assert!(count <= 0xF00, "the status array holds 3840 entries");
        self.params(limits);
        self.m
            .load_ram(label("TEST_COUNT"), &(count as u16).to_le_bytes());
        let tstates = self.run("ENTRY_ALL");
        let statuses = self.m.read_ram(label("TEST_STATUSES"), count);
        (self.byte("TEST_INIT_STATUS"), statuses, tstates)
    }
}

/// What the model says about the same bytes: the status of init, then of each assertion in order.
fn model_statuses(bytes: &[u8], limits: Limits) -> (u8, Vec<u8>) {
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

fn parse(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

fn everything(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

fn assertions(db: &Database) -> Vec<StmtId> {
    let is_assertion = |&id: &StmtId| matches!(db.stmt(id).kind, Kind::Axiom | Kind::Theorem);
    everything(db).into_iter().filter(is_assertion).collect()
}

fn number(db: &Database, label: &str) -> usize {
    let id = db.find(label).expect("label exists");
    assertions(db).iter().position(|&a| a == id).unwrap()
}

fn corpus(name: &str) -> Database {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses")
}

#[test]
fn the_kernel_agrees_with_the_model_on_every_assertion_of_the_corpus() {
    for name in [
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
    ] {
        let db = corpus(name);
        let bytes = pack(&db, &everything(&db)).expect("it packs");
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        let mut device = Device::with_library(&bytes);
        let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, &got), (want_init, &want), "{name}");
        let mut reference = Verifier::new(&db);
        for (local, &id) in assertions(&db).iter().enumerate() {
            let stmt = db.stmt(id);
            let expected = stmt.kind == Kind::Axiom || reference.verify(id).is_ok();
            let usage = reference.usage();
            let over_capacity = usage.stack_syms > label("CAP_STACK_SYMS") as usize
                || usage.stack_depth > label("CAP_STACK_ENTRIES") as usize
                || usage.saved_syms > label("CAP_SAVED_SYMS") as usize;
            let accepted = got[local] == 0;
            let out_of_memory = got[local] == status(Err(KernelError::OutOfMemory));
            assert!(
                accepted == expected || (expected && out_of_memory && over_capacity),
                "{name} {}: device {}, reference {expected}",
                stmt.label,
                got[local]
            );
        }
    }
}

const DB: &str = "
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

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

struct Fields {
    hyps: usize,
    dv: usize,
    extra_dv: usize,
    refs: usize,
}

/// Offsets inside a record, read from the documented layout; the last two only exist for a theorem.
fn fields(bytes: &[u8], id: usize) -> Fields {
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
fn agree(bytes: &[u8], id: usize, expected: Result<(), KernelError>, what: &str) {
    agree_with(bytes, id, ASKED_UNBOUNDED, expected, what);
}

fn agree_with(
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

fn pack_all(db: &Database) -> Vec<u8> {
    pack(db, &everything(db)).expect("it packs")
}

#[test]
fn the_kernel_reports_what_is_wrong_with_a_proof() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    for (label, expected) in [
        ("ax-d", Ok(())),
        ("good", Ok(())),
        ("dummy", Ok(())),
        ("both", Ok(())),
        ("bad", Err(KernelError::DisjointViolation)),
        ("same", Err(KernelError::DisjointViolation)),
        ("nodummy", Err(KernelError::DisjointViolation)),
    ] {
        agree(&bytes, number(&db, label), expected, label);
    }
}

#[test]
fn the_kernel_names_each_way_a_proof_can_fail() {
    for (theorem, expected) in [
        (
            "$d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $.",
            KernelError::HypothesisMismatch,
        ),
        (
            "$d x y $. t $p |- R x y $= vx ax-d $.",
            KernelError::StackUnderflow,
        ),
        (
            "$d x y $. t $p |- R x y $= vx vy ax-d vx $.",
            KernelError::StackNotSingleton,
        ),
        (
            "$d x y $. t $p |- R y x $= vx vy ax-d $.",
            KernelError::WrongConclusion,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ABCD $.",
            KernelError::BadBackReference,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ZABC $.",
            KernelError::StackUnderflow,
        ),
    ] {
        let db = parse(&format!("{DB} ${{ {theorem} $}}"));
        let id = db.find("t").unwrap();
        assert!(Verifier::new(&db).verify(id).is_err(), "{theorem}");
        agree(&pack_all(&db), number(&db, "t"), Err(expected), theorem);
    }
}

#[test]
fn a_typecode_mismatch_is_named() {
    let src = "
        $c wff set |- $. $v x ph $.
        vx $f set x $.
        wph $f wff ph $.
        ax $a |- ph $.
        ${ $d x ph $. ok $p |- ph $= wph ax $. $}
        wrong $p |- ph $= vx ax $.
    ";
    let db = parse(src);
    let bytes = pack_all(&db);
    agree(
        &bytes,
        number(&db, "wrong"),
        Err(KernelError::TypecodeMismatch),
        "wrong",
    );
}

#[test]
fn a_reference_to_a_dummy_that_is_not_a_variable_is_refused() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let refs = fields(&bytes, dummy).refs;
    assert_eq!(bytes[refs + 1], 0x80);
    for not_a_variable in [0, 0xFE] {
        bytes[refs] = not_a_variable;
        reseal(&mut bytes);
        agree(&bytes, dummy, Err(KernelError::Malformed), "dummy");
    }
}

#[test]
fn a_dummy_reference_past_one_byte_is_refused() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let refs = fields(&bytes, dummy).refs;
    bytes[refs + 1] = 0x81;
    reseal(&mut bytes);
    agree(&bytes, dummy, Err(KernelError::Malformed), "dummy");
}

#[test]
fn a_header_that_breaks_the_layout_does_not_load() {
    let db = parse(DB);
    let original = pack_all(&db);
    let variable_typecode = {
        let mut at = library::HEADER_LEN;
        for _ in 0..original[12] {
            at += 2 + original[at + 1] as usize;
        }
        at
    };
    for (at, value) in [(0, b'X'), (16, original[16] + 1), (variable_typecode, 0xFF)] {
        let mut bytes = original.clone();
        bytes[at] = value;
        reseal(&mut bytes);
        agree(
            &bytes,
            0,
            Err(KernelError::Malformed),
            &format!("byte {at}"),
        );
    }
}

#[test]
fn an_assertion_number_past_the_end_is_refused() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let count = assertions(&db).len();
    for id in [count, count + 1, 0xFFFF] {
        agree(&bytes, id, Err(KernelError::Malformed), "past the end");
    }
}

#[test]
fn a_proof_cannot_use_itself_or_a_later_assertion() {
    let db = parse(DB);
    let good = number(&db, "good");
    for target in [good, good + 1] {
        let mut bytes = pack_all(&db);
        let refs = fields(&bytes, good).refs;
        bytes[refs..refs + 2].copy_from_slice(&(target as u16).to_le_bytes());
        reseal(&mut bytes);
        agree(&bytes, good, Err(KernelError::ForwardReference), "forward");
    }
}

#[test]
fn a_dummy_variable_needs_its_own_disjoint_pair() {
    let db = parse(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack_all(&db);
    let extra = fields(&bytes, dummy).extra_dv;
    let (x, y, z) = (bytes[extra], bytes[extra] + 1, bytes[extra + 1]);
    assert_eq!(z, x + 2, "the pair is x z");
    bytes[extra] = y;
    reseal(&mut bytes);
    agree(&bytes, dummy, Err(KernelError::DisjointViolation), "dummy");
}

#[test]
fn a_pair_of_mandatory_variables_cannot_hide_among_the_dummy_pairs() {
    let db = parse(DB);
    let both = number(&db, "both");
    let mut bytes = pack_all(&db);
    let extra = fields(&bytes, both).extra_dv;
    let x = bytes[extra];
    bytes[extra + 1] = x + 1;
    reseal(&mut bytes);
    agree(&bytes, both, Err(KernelError::BadRecord), "both");
}

#[test]
fn an_assertion_with_a_variable_it_has_no_hypothesis_for_is_refused_wherever_it_is_used() {
    let db = parse(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack_all(&db);
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let next = u32_at(u32_at(16) + 4 * (axiom + 1));
    let z = bytes[next - 1] + 1;
    bytes[next - 1] = z;
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, good, Err(KernelError::BadRecord), "user");
}

#[test]
fn a_mandatory_pair_needs_hypotheses_for_both_variables() {
    let db = parse(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack_all(&db);
    let dv = fields(&bytes, axiom).dv;
    bytes[dv + 1] += 1;
    reseal(&mut bytes);
    agree(&bytes, axiom, Err(KernelError::BadRecord), "axiom");
    agree(&bytes, good, Err(KernelError::BadRecord), "user");
}

#[test]
fn an_essential_hypothesis_listed_before_its_variables_reads_this_use_only() {
    let db = parse(DB);
    let (axiom, dummy) = (number(&db, "ax-m"), number(&db, "dummy"));
    let mut bytes = pack_all(&db);
    let hyps = fields(&bytes, axiom).hyps;
    let floats = bytes[hyps..hyps + 4].to_vec();
    let essential = bytes[hyps + 4..hyps + 10].to_vec();
    assert_eq!(
        (floats[0], floats[2], essential[0], essential[1]),
        (0, 0, 1, 4)
    );
    bytes[hyps..hyps + 6].copy_from_slice(&essential);
    bytes[hyps + 6..hyps + 10].copy_from_slice(&floats);
    reseal(&mut bytes);
    agree(&bytes, number(&db, "good"), Ok(()), "good");
    agree(&bytes, axiom, Ok(()), "ax-m");
    agree(&bytes, dummy, Err(KernelError::TypecodeMismatch), "dummy");
}

#[test]
fn a_failed_check_leaves_nothing_behind_for_the_next_one() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let limits = Limits {
        stack_syms: 8,
        ..ASKED_UNBOUNDED
    };
    let (nodummy, good) = (number(&db, "nodummy"), number(&db, "good"));
    let mut kernel = Kernel::new(&bytes, clamped(limits)).unwrap();
    let want = (status(kernel.verify(nodummy)), status(kernel.verify(good)));
    assert_eq!(
        want,
        (status(Err(KernelError::DisjointViolation)), status(Ok(())))
    );
    let mut device = Device::with_library(&bytes);
    assert_eq!(
        device.verify_two(nodummy, good, limits),
        (0, want.0, want.1)
    );
    let dummy = number(&db, "dummy");
    let bad = number(&db, "bad");
    for (first, second) in [(bad, dummy), (dummy, bad), (good, good), (dummy, dummy)] {
        let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
        let want = (status(kernel.verify(first)), status(kernel.verify(second)));
        let got = device.verify_two(first, second, ASKED_UNBOUNDED);
        assert_eq!(got, (0, want.0, want.1), "{first} then {second}");
    }
}

#[test]
fn saved_steps_count_against_their_own_limit() {
    let src = "
        $c ( ) -> wff $.
        $v p q $.
        wp $f wff p $.
        wq $f wff q $.
        wi $a wff ( p -> q ) $.
        twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
    ";
    let db = parse(src);
    let bytes = pack_all(&db);
    let limits = |saved_syms, saved_entries| Limits {
        saved_syms,
        saved_entries,
        ..ASKED_UNBOUNDED
    };
    for (saved_syms, saved_entries, expected) in [
        (6, 1, Ok(())),
        (5, 1, Err(KernelError::OutOfMemory)),
        (6, 0, Err(KernelError::OutOfMemory)),
    ] {
        agree_with(
            &bytes,
            1,
            limits(saved_syms, saved_entries),
            expected,
            "twice",
        );
    }
}

fn step_numbers_src(compressed: bool) -> String {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let (mut labels, mut proof) = (
        String::new(),
        String::from(if compressed { "A" } else { "n0" }),
    );
    for i in 0..250 {
        src += &format!("n{i} $a wff T $.\n");
        labels += &format!("n{i} ");
        if i > 0 {
            if compressed {
                proof += &letters(i);
                proof += &letters(250);
            } else {
                proof += &format!(" n{i} drop");
            }
        }
    }
    if compressed {
        src += &format!("big $p wff T $= ( {labels}drop ) {proof} $.\n");
    } else {
        src += &format!("big $p wff T $= {proof} $.\n");
    }
    src
}

fn letters(mut number: usize) -> String {
    let mut out = vec![b'A' + (number % 20) as u8];
    number /= 20;
    while number > 0 {
        number -= 1;
        out.push(b'U' + (number % 5) as u8);
        number /= 5;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[test]
fn step_numbers_past_one_byte_are_followed() {
    for compressed in [false, true] {
        let db = parse(&step_numbers_src(compressed));
        Verifier::new(&db)
            .verify(db.find("big").unwrap())
            .expect("the proof is valid");
        agree(&pack_all(&db), number(&db, "big"), Ok(()), "big");
    }
}

#[test]
fn a_record_that_crosses_a_page_is_refused() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..700 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = parse(&src);
    let mut bytes = pack_all(&db);
    let u32_at =
        |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let directory = u32_at(&bytes, 16) as usize;
    let offset = |bytes: &[u8], id: usize| u32_at(bytes, directory + 4 * id) as usize;
    let first = (0..700)
        .find(|&id| offset(&bytes, id) >= library::CHUNK)
        .unwrap();
    let (start, end) = (offset(&bytes, first), offset(&bytes, first + 1));
    assert_eq!(start, library::CHUNK, "the packer pads to the page");
    bytes.copy_within(start..end, start - 3);
    let entry = directory + 4 * first;
    bytes[entry..entry + 4].copy_from_slice(&(start as u32 - 3).to_le_bytes());
    reseal(&mut bytes);
    agree(&bytes, first - 1, Ok(()), "before the page");
    agree(
        &bytes,
        first,
        Err(KernelError::Malformed),
        "across the page",
    );
}

#[test]
fn a_library_over_several_pages_is_read_across_them() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..2500 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = parse(&src);
    let bytes = pack_all(&db);
    assert!(bytes.len() > 4 * library::CHUNK);
    let (init, got, _) = Device::with_library(&bytes).verify_all(2500, ASKED_UNBOUNDED);
    assert_eq!((init, got), (0, vec![0; 2500]));
}

#[test]
fn the_checksum_is_not_checked_here() {
    let db = parse(DB);
    let mut bytes = pack_all(&db);
    let count = assertions(&db).len();
    bytes[4] ^= 0xFF;
    assert!(Kernel::new(&bytes, Limits::UNBOUNDED).is_err());
    let mut resealed = bytes.clone();
    reseal(&mut resealed);
    let (_, want) = model_statuses(&resealed, ASKED_UNBOUNDED);
    let (init, got, _) = Device::with_library(&bytes).verify_all(count, ASKED_UNBOUNDED);
    assert_eq!((init, got), (0, want));
}

fn model_one(bytes: &[u8], id: usize, limits: Limits) -> u8 {
    status(Kernel::new(bytes, clamped(limits)).and_then(|mut kernel| kernel.verify(id)))
}

/// The device's status for `id` under `limits`, which must be the model's.
fn same_as_model(device: &mut Device, bytes: &[u8], id: usize, limits: Limits, what: &str) -> u8 {
    let got = device.verify(id, limits);
    assert_eq!(got.init, 0, "{what}");
    assert_eq!(got.verify, model_one(bytes, id, limits), "{what}");
    got.verify
}

#[test]
fn the_kernel_needs_the_memory_the_reference_measures() {
    let ok = status(Ok(()));
    let oom = status(Err(KernelError::OutOfMemory));
    let mut fitting = 0;
    for name in ["big-unifier.mm", "demo0.mm", "miu.mm", "anatomy.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let mut device = Device::with_library(&bytes);
        let mut reference = Verifier::new(&db);
        for (local, &id) in assertions(&db).iter().enumerate() {
            if db.stmt(id).kind != Kind::Theorem {
                continue;
            }
            reference.verify(id).expect("the proof is valid");
            let usage = reference.usage();
            let what = format!("{name} {}", db.stmt(id).label);
            let exact = Limits {
                stack_syms: usage.stack_syms,
                stack_entries: usage.stack_depth,
                ..ASKED_UNBOUNDED
            };
            let short = |limits: Limits| {
                [
                    Limits {
                        stack_syms: limits.stack_syms - 1,
                        ..limits
                    },
                    Limits {
                        stack_entries: limits.stack_entries - 1,
                        ..limits
                    },
                ]
            };
            let fits = usage.stack_syms <= label("CAP_STACK_SYMS") as usize
                && usage.stack_depth <= label("CAP_STACK_ENTRIES") as usize
                && usage.saved_syms <= label("CAP_SAVED_SYMS") as usize;
            let want = if fits { ok } else { oom };
            fitting += usize::from(fits);
            assert_eq!(
                same_as_model(&mut device, &bytes, local, exact, &what),
                want
            );
            for limits in short(exact) {
                assert_eq!(
                    same_as_model(&mut device, &bytes, local, limits, &what),
                    oom
                );
            }
            if usage.saved_syms > 0 && fits {
                let saved = Limits {
                    saved_syms: usage.saved_syms,
                    ..ASKED_UNBOUNDED
                };
                assert_eq!(same_as_model(&mut device, &bytes, local, saved, &what), ok);
                let less = Limits {
                    saved_syms: usage.saved_syms - 1,
                    ..saved
                };
                assert_eq!(same_as_model(&mut device, &bytes, local, less, &what), oom);
            }
        }
    }
    assert!(fitting >= 3, "{fitting} theorems fit");
}

/// A theorem that pushes `copies` copies of an axiom's statement of `size` symbols and drops
/// them pairwise, saving each copy first when `saving` is set.
fn copies_src(size: usize, copies: usize, saving: bool) -> String {
    let body = "T ".repeat(size - 1);
    let proof = if saving {
        format!("( n0 drop ) A{}", "AZB".repeat(copies))
    } else {
        format!(
            "n0{}",
            " n0".repeat(copies - 1) + &" drop".repeat(copies - 1)
        )
    };
    format!(
        "$c T wff $.\n${{ d.1 $e wff {body}$. d.2 $e wff {body}$. drop $a wff {body}$. $}}\n\
         n0 $a wff {body}$.\nbig $p wff {body}$= {proof} $.\n"
    )
}

/// The device's status for the copies theorem under the largest limits, checked against the model
/// and the reference verifier.
fn copies(size: usize, copies: usize, saving: bool) -> (u8, mcpack::mm::Usage) {
    let db = parse(&copies_src(size, copies, saving));
    let mut reference = Verifier::new(&db);
    reference.verify(db.find("big").unwrap()).expect("valid");
    let bytes = pack_all(&db);
    let id = number(&db, "big");
    let mut device = Device::with_library(&bytes);
    let got = same_as_model(&mut device, &bytes, id, ASKED_UNBOUNDED, "copies");
    (got, reference.usage())
}

#[test]
fn a_limit_above_a_buffer_is_clamped_to_it() {
    let ok = status(Ok(()));
    let oom = status(Err(KernelError::OutOfMemory));
    let cap = |name: &str| label(name) as usize;

    let size = 96;
    let fit = cap("CAP_STACK_SYMS") / size - 1;
    let (got, usage) = copies(size, fit, false);
    assert_eq!(usage.stack_syms, cap("CAP_STACK_SYMS"));
    assert_eq!(got, ok, "stack symbols, at the capacity");
    assert_eq!(
        copies(size, fit + 1, false).0,
        oom,
        "stack symbols, past it"
    );

    let entries = cap("CAP_STACK_ENTRIES");
    let (got, usage) = copies(2, entries, false);
    assert_eq!(usage.stack_depth, entries);
    assert_eq!(got, ok, "stack entries, at the capacity");
    assert_eq!(
        copies(2, entries + 1, false).0,
        oom,
        "stack entries, past it"
    );

    let fit = cap("CAP_SAVED_SYMS") / size;
    let (got, usage) = copies(size, fit, true);
    assert_eq!(usage.saved_syms, cap("CAP_SAVED_SYMS"));
    assert_eq!(got, ok, "saved symbols, at the capacity");
    assert_eq!(copies(size, fit + 1, true).0, oom, "saved symbols, past it");

    let saved = cap("CAP_SAVED_ENTRIES");
    assert_eq!(
        copies(2, saved, true).0,
        ok,
        "saved entries, at the capacity"
    );
    assert_eq!(copies(2, saved + 1, true).0, oom, "saved entries, past it");
}

const REPLACEMENTS: [u8; 8] = [0x00, 0x01, 0x7F, 0x80, 0xEF, 0xF0, 0xFE, 0xFF];

/// The model's view of a library the device reads from flash: the device trusts the length field,
/// so the bytes the model sees are the ones that length covers, with erased flash past the end.
/// None when the length is more than the flash pages left can hold.
fn flash_view(bytes: &[u8]) -> Option<Vec<u8>> {
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let room = (0x80 - LIB_PAGE as usize) * PAGE;
    if length > room {
        return None;
    }
    let mut view = bytes.to_vec();
    view.resize(length, 0xFF);
    if view.len() >= 8 {
        reseal(&mut view);
    }
    Some(view)
}

/// Runs the device and the model over one byte string and reports how they differ; each status
/// the model gives is counted in `seen`.
fn disagreement(
    device: &mut Device,
    bytes: &[u8],
    limits: Limits,
    seen: &mut [usize; 16],
) -> Option<String> {
    device.load(bytes);
    let (want_init, want) = match flash_view(bytes) {
        Some(view) => model_statuses(&view, limits),
        None => (status(Err(KernelError::Malformed)), Vec::new()),
    };
    let malformed = vec![status(Err(KernelError::Malformed))];
    let want = if want.is_empty() { malformed } else { want };
    seen[want_init as usize] += 1;
    want.iter().for_each(|&s| seen[s as usize] += 1);
    let (init, got, _) = device.verify_all(want.len(), limits);
    ((init, &got) != (want_init, &want))
        .then(|| format!("device {init} {got:?}, model {want_init} {want:?}"))
}

fn sweep(original: &[u8], limits: Limits) -> [usize; 16] {
    let mut device = Device::new();
    let mut seen = [0; 16];
    let mut runs = 0;
    for at in 8..original.len() {
        for value in REPLACEMENTS {
            let mut bytes = original.to_vec();
            bytes[at] = value;
            reseal(&mut bytes);
            if let Some(difference) = disagreement(&mut device, &bytes, limits, &mut seen) {
                panic!("byte {at} set to {value:#04x}: {difference}");
            }
            runs += 1;
        }
    }
    println!("{runs} mutations, statuses seen by code: {seen:?}");
    seen
}

#[test]
fn no_single_byte_change_makes_the_kernel_disagree_with_the_model() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let seen = sweep(&bytes, ASKED_UNBOUNDED);
    for code in (0..=12).filter(|&code| code != 3 && code != 5) {
        assert!(seen[code] > 0, "no mutation gave status {code}");
    }
    let tight = Limits {
        stack_syms: 10,
        stack_entries: 4,
        saved_syms: 4,
        saved_entries: 1,
    };
    let seen = sweep(&bytes, tight);
    assert!(seen[5] > 0, "no mutation ran out of memory");
}

#[test]
fn no_single_byte_change_to_a_proof_library_makes_the_kernel_disagree() {
    for name in ["demo0.mm", "miu.mm"] {
        let bytes = pack_all(&corpus(name));
        sweep(&bytes, ASKED_UNBOUNDED);
    }
}

#[test]
fn a_length_at_the_edge_of_flash_is_judged_as_the_model_would() {
    let db = parse(DB);
    let original = pack_all(&db);
    let room = (0x80 - LIB_PAGE as u32) << 14;
    let mut device = Device::new();
    for length in [
        0,
        19,
        20,
        original.len() as u32 - 1,
        original.len() as u32,
        original.len() as u32 + 1,
        room - 1,
        room,
        room + 1,
    ] {
        let mut bytes = original.clone();
        bytes[8..12].copy_from_slice(&length.to_le_bytes());
        reseal(&mut bytes);
        if let Some(difference) = disagreement(&mut device, &bytes, ASKED_UNBOUNDED, &mut [0; 16]) {
            panic!("length {length}: {difference}");
        }
    }
}

#[test]
fn a_library_page_beyond_flash_is_refused() {
    let db = parse(DB);
    let bytes = pack_all(&db);
    let mut device = Device::with_library(&bytes);
    for page in [0x80u8, 0x81, 0xFF] {
        device.params(ASKED_UNBOUNDED);
        device.m.load_ram(label("KERNEL_LIB_PAGE"), &[page]);
        device.run("ENTRY_ONE");
        assert_eq!(
            device.byte("TEST_INIT_STATUS"),
            status(Err(KernelError::Malformed))
        );
        assert_eq!(
            device.byte("TEST_VERIFY_STATUS"),
            status(Err(KernelError::Malformed))
        );
    }
}

const PORT_BANK_B: u8 = 7;
const RAM_PAGE: u8 = 0x81;

/// Every assertion of the small library, which reaches every error path the model has.
fn small_library() -> (Database, Vec<u8>) {
    let db = parse(&format!(
        "{DB} ${{ $d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $. $}}"
    ));
    let bytes = pack_all(&db);
    (db, bytes)
}

#[test]
fn the_kernel_restores_bank_b_and_writes_only_its_own_memory() {
    let (db, bytes) = small_library();
    let count = assertions(&db).len();
    let marker = *b"flash page marker";
    let free_start = label("KERNEL_RAM_END");
    let free_len = usize::from(label("TEST_BLOCK") - free_start);
    for entry_page in [RAM_PAGE, 0x50] {
        let mut device = Device::with_library(&bytes);
        device.m.load_flash(0x50, 0, &marker);
        let ram_pattern = vec![0xA5; PAGE];
        let free_pattern = vec![0x5A; free_len];
        device.m.load_ram(0x8000, &ram_pattern);
        device.m.load_ram(free_start, &free_pattern);
        let stack = usize::from(label("KSTACK_TOP") - label("KSTACK"));
        device.m.load_ram(label("KSTACK"), &vec![0xCC; stack]);
        let mut deepest = stack;
        for id in 0..count {
            device.m.out_port(PORT_BANK_B, entry_page);
            device.verify(id, ASKED_UNBOUNDED);
            let bank = device.m.read_ram(0x8000, marker.len());
            if entry_page == RAM_PAGE {
                assert_eq!(bank, vec![0xA5; marker.len()], "bank B after {id}");
            } else {
                assert_eq!(bank, marker, "bank B after {id}");
            }
            assert_eq!(
                device.m.read_ram(free_start, free_len),
                free_pattern,
                "bank C after {id}"
            );
            let used = device.m.read_ram(label("KSTACK"), stack);
            deepest = deepest.min(used.iter().position(|&b| b != 0xCC).unwrap_or(stack));
        }
        device.m.out_port(PORT_BANK_B, RAM_PAGE);
        assert_eq!(
            device.m.read_ram(0x8000, PAGE),
            ram_pattern,
            "RAM page of bank B"
        );
        println!("kernel stack: {} of {stack} bytes used", stack - deepest);
        assert!(
            deepest >= 128,
            "the kernel stack comes within 128 bytes of its end"
        );
    }
}

#[test]
fn the_kernel_restores_the_interrupt_state() {
    let (_, bytes) = small_library();
    let mut device = Device::with_library(&bytes);
    device.params(ASKED_UNBOUNDED);
    device.run("ENTRY_IFF");
    assert_eq!(device.byte("TEST_INIT_STATUS"), 0);
    assert_eq!(device.byte("TEST_IFF"), 1, "interrupts were on at the call");
    device.verify(0, ASKED_UNBOUNDED);
    assert_eq!(
        device.byte("TEST_IFF"),
        0,
        "interrupts were off at the call"
    );
}

#[test]
fn the_kernel_halts_within_budget_on_every_corpus_proof() {
    let mut longest = 0;
    for name in ["big-unifier.mm", "demo0.mm", "miu.mm", "anatomy.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let (_, _, tstates) =
            Device::with_library(&bytes).verify_all(assertions(&db).len(), ASKED_UNBOUNDED);
        longest = longest.max(tstates);
    }
    assert!(longest < RUN_LIMIT / 4, "{longest} T-states");
}

#[test]
fn measurements_at_15_mhz_from_flash() {
    for name in ["big-unifier.mm", "demo0.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        let mut device = Device::with_library(&bytes);
        let init = device.verify_all(0, ASKED_UNBOUNDED).2;
        println!("{name}: init {init} T-states");
        let mut reference = Verifier::new(&db);
        let (mut total, mut steps) = (0, 0);
        for (local, &id) in assertions(&db).iter().enumerate() {
            let got = device.verify(local, ASKED_UNBOUNDED);
            let tstates = got.tstates - init;
            let stmt = db.stmt(id);
            if stmt.kind != Kind::Theorem {
                println!("{name} {}: axiom, {tstates} T-states", stmt.label);
                continue;
            }
            reference.verify(id).expect("the proof is valid");
            let proof_steps = reference.usage().steps;
            let outcome = if got.verify == 0 {
                total += tstates;
                steps += proof_steps;
                "accepted"
            } else {
                "stopped by the memory limit"
            };
            println!(
                "{name} {}: {tstates} T-states, {proof_steps} proof steps, {outcome}",
                stmt.label
            );
        }
        if steps > 0 {
            println!(
                "{name}: {} T-states per proof step over the accepted theorems",
                total / steps as u32
            );
        }
    }
}
