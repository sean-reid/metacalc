use crate::edges::REGISTERS;
use crate::support::*;
use mcemu::{Machine, Stop};
use mcpack::library::{Kernel, KernelError, WORK_LIMIT, pack};
use mcpack::mm::Database;

#[test]
fn the_kernel_and_the_model_share_one_work_limit() {
    assert_eq!(constant("WORK_LIMIT"), WORK_LIMIT);
    assert_eq!(label("ST_WORK_LIMIT"), 13);
    assert_eq!(label("ST_INTERRUPTED"), 14);
}

#[test]
fn the_kernel_stops_where_the_model_stops() {
    for (terms, nv, expected) in [
        (two_terms(1, 1, 0, 0), 2, Ok(())),
        (two_terms(4, 4, 7, 9), 8, Ok(())),
        (two_terms(27, 8, 7, 77), 35, Ok(())),
        (two_terms(31, 9, 36, 51), 40, Ok(())),
        (two_terms(31, 9, 37, 51), 40, Err(KernelError::WorkLimit)),
        (two_terms(4, 60, 0, 0), 64, Err(KernelError::WorkLimit)),
    ] {
        let (db, bytes) = disjoint_library(&terms, nv, true);
        let id = number(&db, "t");
        agree(&bytes, id, expected, "work limit");
        let got = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
        println!("{} T-states, status {}", got.tstates, got.verify);
    }
}

fn constants_after(first: Tok, constants: usize) -> Vec<Tok> {
    let mut term = vec![first];
    term.extend(vec![Tok::Const; constants]);
    term
}

/// T-states one verify call spends after the library is loaded, and the work the model charges.
fn call_cost(terms: &[Vec<Tok>], nv: usize) -> (u32, u32) {
    let (db, bytes) = disjoint_library(terms, nv, true);
    let id = number(&db, "t");
    let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
    let units = match kernel.verify(id) {
        Ok(()) => kernel.work(),
        Err(KernelError::WorkLimit) => WORK_LIMIT,
        Err(error) => panic!("{error:?}"),
    };
    let call = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
    (call.tstates - init_tstates(&bytes), units)
}

#[test]
fn the_worst_call_per_unit_of_work_stays_under_ten_seconds_at_6_mhz() {
    let var = |i| Tok::Var(i);
    let many_pairs: Vec<Vec<Tok>> = (0..20).map(|i| constants_after(var(i), 10)).collect();
    let mut repeated = vec![var(0); 100];
    repeated.push(Tok::Const);
    let shapes: Vec<(&str, Vec<Vec<Tok>>, usize)> = vec![
        ("27 x 8 variables, 595 pairs", two_terms(27, 8, 7, 77), 35),
        (
            "60 x 60 variables, 7140 pairs",
            two_terms(60, 60, 0, 0),
            120,
        ),
        (
            "60 x 60 reversed",
            {
                let mut t = two_terms(60, 60, 0, 0);
                t.reverse();
                t
            },
            120,
        ),
        (
            "100 variables against 150 constants",
            vec![repeated, constants_after(var(1), 150)],
            2,
        ),
        ("190 pairs on the applied assertion", many_pairs, 20),
    ];
    let mut worst = 0.0f64;
    for (name, terms, nv) in shapes {
        let (tstates, work) = call_cost(&terms, nv);
        let per_unit = f64::from(tstates) / f64::from(work.max(1));
        println!("{name}: {work} units, {tstates} T-states, {per_unit:.1} T-states per unit");
        worst = worst.max(per_unit);
    }
    let total = worst * f64::from(WORK_LIMIT);
    println!(
        "worst: {worst:.1} T-states per unit, {total:.0} T-states at the limit, {:.2} s at 6 MHz",
        total / 6.0e6
    );
    assert!(total < 60.0e6, "{total} T-states at the limit");
}

const BUDGET_OVER_LIMIT: u32 = 17_800_000;

#[test]
fn a_check_over_the_limit_returns_within_budget() {
    let (db, bytes) = disjoint_library(&two_terms(31, 9, 37, 51), 40, true);
    let got = Device::with_library(&bytes).verify(number(&db, "t"), ASKED_UNBOUNDED);
    println!("over the limit: {} T-states", got.tstates);
    assert_eq!(got.verify, status(Err(KernelError::WorkLimit)));
    assert!(got.tstates < BUDGET_OVER_LIMIT, "{} T-states", got.tstates);
}

const BUDGET_MID_PROOF: u32 = 25_000;
const BUDGET_MID_CHECK: u32 = 60_000;
const BUDGET_ON_AT_START: u32 = 226_000;
const BUDGET_LONG_SCAN: u32 = 900_000;

/// Loads the assertion and the limits and runs the init-and-verify entry for `press_at` T-states;
/// then holds ON and finishes the run. Returns the program counter when the key went down and the
/// T-states after that.
pub fn press_on_during(device: &mut Device, id: usize, press_at: u32) -> (u16, u32) {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    let first = device.m.run(device.label("ENTRY_ONE"), press_at);
    assert_eq!(first.stop, Stop::Limit, "the run ended before ON went down");
    let pc = device.m.pc();
    device.m.press_key(Machine::KEY_ON);
    let rest = device.m.resume(RUN_LIMIT);
    assert_eq!(rest.stop, Stop::Halted);
    (pc, rest.tstates)
}

/// The T-states `kernel_init` takes on `bytes`, entry and exit included.
pub fn init_tstates(bytes: &[u8]) -> u32 {
    Device::with_library(bytes).verify_all(0, ASKED_UNBOUNDED).2
}

/// Runs `entry` until init, which takes `init` T-states, has just ended, then holds ON and
/// finishes the run. The last read of ON in init is far earlier, so only kernel_verify sees it.
pub fn run_with_on_after_init(device: &mut Device, entry: &str, init: u32) {
    let first = device.m.run(device.label(entry), init - 500);
    assert_eq!(first.stop, Stop::Limit, "{entry} ended before init did");
    device.m.press_key(Machine::KEY_ON);
    let rest = device.m.run(device.m.pc(), RUN_LIMIT);
    assert_eq!(rest.stop, Stop::Halted, "{entry}");
}

pub fn verify_again(device: &mut Device) -> u8 {
    device.m.release_key(Machine::KEY_ON);
    device.run("ENTRY_VERIFY");
    device.byte("TEST_VERIFY_STATUS")
}

/// Whether `pc` is in one of the code ranges `[from, to)` named by labels.
pub fn in_code(pc: u16, ranges: &[(&str, &str)]) -> bool {
    ranges
        .iter()
        .any(|&(from, to)| (label(from)..label(to)).contains(&pc))
}

/// The proof loop, the copy helpers, the record parser and the parts of an apply around its
/// disjoint check.
const PROOF_CODE: &[(&str, &str)] = &[
    ("COPY_BC", "CMP24"),
    ("PR_U8", "REQUIRE_SOUND"),
    ("MAP_CUR", "WORK_ONE"),
    ("APPLY", "AP_DV"),
    ("AP_INSTALL", "KERNEL_CRC_BEGIN"),
];
/// The disjoint check: its loops and what they call.
const CHECK_CODE: &[(&str, &str)] = &[("WORK_ONE", "APPLY"), ("AP_DV", "AP_INSTALL")];

#[test]
fn the_on_key_interrupts_a_proof_and_a_long_check_and_the_call_can_be_run_again() {
    let terms = two_terms(27, 8, 7, 77);
    let (db, bytes) = disjoint_library(&terms, 35, true);
    let (_, plain) = disjoint_library(&terms, 35, false);
    let id = number(&db, "t");
    let init = Device::with_library(&bytes)
        .verify_all(0, ASKED_UNBOUNDED)
        .2;
    let steps = Device::with_library(&plain)
        .verify(id, ASKED_UNBOUNDED)
        .tstates;
    let whole = Device::with_library(&bytes)
        .verify(id, ASKED_UNBOUNDED)
        .tstates;
    let interrupted = status_code("ST_INTERRUPTED");
    println!("init {init}, proof steps end at {steps}, whole call {whole} T-states");
    let mut worst_check = 0;
    let presses = [(init + (steps - init) / 2, PROOF_CODE, BUDGET_MID_PROOF)]
        .into_iter()
        .chain((1..16).map(|i| {
            (
                steps + (whole - steps) * i / 16,
                CHECK_CODE,
                BUDGET_MID_CHECK,
            )
        }));
    for (press_at, code, budget) in presses {
        let mut device = Device::with_library(&bytes);
        let (pc, after) = press_on_during(&mut device, id, press_at);
        let name = if code == PROOF_CODE { "proof" } else { "check" };
        assert!(
            in_code(pc, code),
            "{name}: pc {pc:#06x} at {press_at} T-states"
        );
        if code == CHECK_CODE {
            worst_check = worst_check.max(after);
        } else {
            println!("mid-proof: ON seen {after} T-states after it went down");
            assert!(after < budget, "{name}: {after} T-states");
        }
        assert_eq!(device.byte("TEST_VERIFY_STATUS"), interrupted, "{name}");
        assert_eq!(verify_again(&mut device), status(Ok(())), "{name}");
    }
    println!("mid-check: ON seen at most {worst_check} T-states after it went down");
    assert!(worst_check < BUDGET_MID_CHECK, "{worst_check} T-states");
}

#[test]
fn the_longest_stretch_without_a_poll_is_one_scan_of_the_longest_pair_list() {
    let (db, bytes, pairs) = longest_pair_list();
    let id = number(&db, "t");
    let mut device = Device::with_library(&bytes);
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.m.run(device.label("ENTRY_ONE"), 500_000);
    let scan = (label("PAIR_SCAN"), label("APPLY"));
    while !(scan.0..scan.1).contains(&device.m.pc()) {
        let run = device.m.resume(500);
        assert_eq!(run.stop, Stop::Limit, "the call ended before a scan began");
    }
    device.m.press_key(Machine::KEY_ON);
    let rest = device.m.resume(RUN_LIMIT);
    assert_eq!(rest.stop, Stop::Halted);
    println!(
        "{pairs} pairs: ON seen {} T-states after it went down, {:.3} s at 6 MHz",
        rest.tstates,
        f64::from(rest.tstates) / 6.0e6
    );
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert!(rest.tstates < BUDGET_LONG_SCAN, "{} T-states", rest.tstates);
}

pub fn status_code(name: &str) -> u8 {
    label(name) as u8
}

#[test]
fn the_on_key_down_as_init_ends_stops_the_first_proof_step() {
    let (db, bytes) = disjoint_library(&two_terms(27, 8, 7, 77), 35, true);
    let id = number(&db, "t");
    let init = Device::with_library(&bytes)
        .verify_all(0, ASKED_UNBOUNDED)
        .2;
    let mut device = Device::with_library(&bytes);
    let (_, after) = press_on_during(&mut device, id, init);
    println!("ON down as init ends: {after} T-states");
    assert_eq!(device.byte("TEST_INIT_STATUS"), 0);
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert!(after < BUDGET_ON_AT_START, "{after} T-states");
    assert_eq!(verify_again(&mut device), status(Ok(())));
}

#[test]
fn an_interrupted_call_restores_registers_the_bank_and_interrupts() {
    let marker = *b"flash page marker";
    let (db, bytes) = disjoint_library(&two_terms(27, 8, 7, 77), 35, true);
    let id = number(&db, "t");
    let init = init_tstates(&bytes);
    let mut device = Device::with_library(&bytes);
    device.m.load_flash(0x50, 0, &marker);
    device.m.load_ram(0x8000, &[0xA5; 64]);
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.m.press_key(Machine::KEY_ON);
    device.run("ENTRY_REGS");
    assert_eq!(
        device.byte("TEST_INIT_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert_eq!(device.m.read_ram(device.label("TEST_REGS"), 10), REGISTERS);
    device.m.release_key(Machine::KEY_ON);
    run_with_on_after_init(&mut device, "ENTRY_REGS", init);
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert_eq!(device.m.read_ram(device.label("TEST_REGS"), 10), REGISTERS);
    for page in [0x81u8, 0x50] {
        for interrupts in [0u8, 1] {
            device.m.release_key(Machine::KEY_ON);
            device.m.out_port(7, page);
            device
                .m
                .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
            run_with_on_after_init(&mut device, "ENTRY_IFFV", init);
            assert_eq!(
                device.byte("TEST_VERIFY_STATUS"),
                status_code("ST_INTERRUPTED")
            );
            assert_eq!(device.byte("TEST_IFF"), interrupts);
            let bank = device.m.read_ram(0x8000, marker.len());
            let expect = if page == 0x81 {
                vec![0xA5; marker.len()]
            } else {
                marker.to_vec()
            };
            assert_eq!(bank, expect, "bank B from page {page:#x}");
        }
    }
}

/// The theorem `t` with the most disjoint pairs a record allows: `vars` variables, all pairwise
/// disjoint, the first check being the last pair of the list. Returns the library and its pair count.
fn longest_pair_list() -> (Database, Vec<u8>, usize) {
    for vars in (3..=250).rev() {
        let terms = vec![
            vec![Tok::Var(vars - 2)],
            vec![Tok::Var(vars - 1)],
            (0..vars - 2).map(Tok::Var).collect(),
        ];
        let db = parse(&disjoint_source(&terms, vars, true));
        if let Ok(bytes) = pack(&db, &everything(&db)) {
            return (db, bytes, vars * (vars - 1) / 2);
        }
    }
    panic!("no pair list packs");
}

type Applies = Vec<(Vec<Tok>, Vec<Tok>)>;

fn pair(terms: Vec<Vec<Tok>>) -> (Vec<Tok>, Vec<Tok>) {
    let mut terms = terms.into_iter();
    (terms.next().unwrap(), terms.next().unwrap())
}

/// Proofs of `|- R k` that apply the disjoint axiom twice, the first with all but a little of the
/// work the limit allows; the second is refused for a variable outside the 35 listed.
fn joined_cases() -> Vec<(&'static str, Applies, usize, usize, KernelError)> {
    let var = |i| (vec![Tok::Var(0)], vec![Tok::Var(i)]);
    vec![
        (
            "each apply alone is under the limit",
            vec![pair(two_terms(27, 8, 0, 0)), pair(two_terms(27, 8, 0, 0))],
            35,
            35,
            KernelError::WorkLimit,
        ),
        (
            "a violation whose comparison charge is the last the limit allows",
            vec![pair(two_terms(20, 15, 61, 158)), var(35)],
            36,
            35,
            KernelError::DisjointViolation,
        ),
        (
            "a violation whose comparison charge crosses the limit",
            vec![pair(two_terms(12, 23, 114, 164)), var(35)],
            36,
            35,
            KernelError::WorkLimit,
        ),
    ]
}

#[test]
fn the_work_of_every_apply_in_a_call_counts_on_the_kernel_too() {
    let (db, bytes) = joined_library(&[pair(two_terms(27, 8, 0, 0))], 35, 35);
    agree(&bytes, number(&db, "t"), Ok(()), "one apply");
    let mut wrong = Vec::new();
    for (name, applies, nv, listed, expected) in joined_cases() {
        let (db, bytes) = joined_library(&applies, nv, listed);
        let id = number(&db, "t");
        let model = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).and_then(|mut k| k.verify(id));
        assert_eq!(model, Err(expected), "{name}: the model");
        let got = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
        println!("{name}: {} T-states, status {}", got.tstates, got.verify);
        if got.verify != status(Err(expected)) {
            wrong.push(name);
        }
    }
    assert!(wrong.is_empty(), "the device disagrees on: {wrong:?}");
}

const DOUBLINGS: usize = 9;
const COPY_STEPS: usize = 15_900;

/// A theorem that doubles a term `DOUBLINGS` times and saves it, then `cycles` times pushes two
/// copies, compares them with a rule and joins the result into the one entry kept. Every step
/// is one byte; the copies and substitutions are most of the work and none of it is a disjoint
/// check.
fn copying_library(cycles: usize) -> (Database, Vec<u8>) {
    let proof = format!(
        "( a0 wd del join ) A{}ZEC{}",
        "B".repeat(DOUBLINGS),
        "EECD".repeat(cycles - 1)
    );
    let db = parse(&format!(
        "$c T wff $. $v p $. wp $f wff p $. a0 $a wff T $. wd $a wff p p $.\n\
         ${{ del.1 $e wff p $. del $a wff T $. $}}\n\
         ${{ join.1 $e wff T $. join.2 $e wff T $. join $a wff T $. $}}\n\
         big $p wff T $= {proof} $.\n"
    ));
    let bytes = pack_all(&db);
    (db, bytes)
}

fn model_work(bytes: &[u8], id: usize) -> (Result<(), KernelError>, u32) {
    let mut kernel = Kernel::new(bytes, clamped(ASKED_UNBOUNDED)).unwrap();
    let result = kernel.verify(id);
    (result, kernel.work())
}

const BUDGET_COPYING: u32 = 18_200_000;

#[test]
fn thousands_of_steps_that_copy_a_term_stop_at_the_work_limit_within_budget() {
    let cycles = COPY_STEPS / 4;
    let (db, bytes) = copying_library(cycles);
    let id = number(&db, "big");
    println!("{} bytes, {} proof steps", bytes.len(), 4 * cycles);
    agree(&bytes, id, Err(KernelError::WorkLimit), "copying");
    let got = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
    println!("copying: {} T-states", got.tstates);
    assert!(got.tstates < BUDGET_COPYING, "{} T-states", got.tstates);

    let (mut fits, mut over) = (1, cycles);
    while over - fits > 1 {
        let mid = (fits + over) / 2;
        let (db, bytes) = copying_library(mid);
        match model_work(&bytes, number(&db, "big")).0 {
            Ok(()) => fits = mid,
            Err(KernelError::WorkLimit) => over = mid,
            Err(error) => panic!("{error:?}"),
        }
    }
    let (db, bytes) = copying_library(fits);
    let id = number(&db, "big");
    let (result, work) = model_work(&bytes, id);
    assert_eq!(result, Ok(()));
    assert!(work <= WORK_LIMIT && WORK_LIMIT - work < 2_000, "{work}");
    agree(&bytes, id, Ok(()), "the last cycle that fits");
    let mut device = Device::with_library(&bytes);
    assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, status(Ok(())));
    assert_eq!(device.work_charged(), work);
    let (db, bytes) = copying_library(over);
    agree(
        &bytes,
        number(&db, "big"),
        Err(KernelError::WorkLimit),
        "the first cycle that does not",
    );
}
