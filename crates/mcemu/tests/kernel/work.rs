use crate::edges::REGISTERS;
use crate::support::*;
use mcemu::{Machine, Stop};
use mcpack::library::{Kernel, KernelError, WORK_LIMIT};

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
        (two_terms(27, 8, 13, 77), 35, Ok(())),
        (two_terms(27, 8, 14, 77), 35, Err(KernelError::WorkLimit)),
        (two_terms(4, 38, 0, 0), 42, Err(KernelError::WorkLimit)),
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

/// T-states the device spends on the disjoint check, and the work the model charges for it.
fn check_cost(terms: &[Vec<Tok>], nv: usize) -> (u32, u32) {
    let (db, bytes) = disjoint_library(terms, nv, true);
    let (_, plain) = disjoint_library(terms, nv, false);
    let id = number(&db, "t");
    let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
    let _ = kernel.verify(id);
    let with = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
    let without = Device::with_library(&plain).verify(id, ASKED_UNBOUNDED);
    (with.tstates - without.tstates, kernel.work())
}

#[test]
fn the_worst_disjoint_check_stays_under_ten_seconds_at_6_mhz() {
    let var = |i| Tok::Var(i);
    let many_pairs: Vec<Vec<Tok>> = (0..20).map(|i| constants_after(var(i), 10)).collect();
    let mut repeated = vec![var(0); 100];
    repeated.push(Tok::Const);
    let shapes: Vec<(&str, Vec<Vec<Tok>>, usize)> = vec![
        ("27 x 8 variables, 595 pairs", two_terms(27, 8, 13, 77), 35),
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
        let (tstates, work) = check_cost(&terms, nv);
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

const BUDGET_OVER_LIMIT: u32 = 12_000_000;

#[test]
fn a_check_over_the_limit_returns_within_budget() {
    let (db, bytes) = disjoint_library(&two_terms(27, 8, 14, 77), 35, true);
    let got = Device::with_library(&bytes).verify(number(&db, "t"), ASKED_UNBOUNDED);
    println!("over the limit: {} T-states", got.tstates);
    assert_eq!(got.verify, status(Err(KernelError::WorkLimit)));
    assert!(got.tstates < BUDGET_OVER_LIMIT, "{} T-states", got.tstates);
}

const BUDGET_MID_PROOF: u32 = 25_000;
const BUDGET_MID_CHECK: u32 = 100_000;
const BUDGET_ON_AT_START: u32 = 350_000;

/// Loads the assertion and the limits and runs the init-and-verify entry for `press_at` T-states;
/// then holds ON and finishes the run. Returns the T-states after the key went down.
fn press_on_during(device: &mut Device, id: usize, press_at: u32) -> u32 {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    let first = device.m.run(device.label("ENTRY_ONE"), press_at);
    assert_eq!(first.stop, Stop::Limit, "the run ended before ON went down");
    device.m.press_key(Machine::KEY_ON);
    let rest = device.m.run(device.m.pc(), RUN_LIMIT);
    assert_eq!(rest.stop, Stop::Halted);
    rest.tstates
}

fn verify_again(device: &mut Device) -> u8 {
    device.m.release_key(Machine::KEY_ON);
    device.run("ENTRY_VERIFY");
    device.byte("TEST_VERIFY_STATUS")
}

#[test]
fn the_on_key_interrupts_a_proof_and_a_long_check_and_the_call_can_be_run_again() {
    let terms = two_terms(27, 8, 13, 77);
    let (db, bytes) = disjoint_library(&terms, 35, true);
    let (_, plain) = disjoint_library(&terms, 35, false);
    let id = number(&db, "t");
    let steps = Device::with_library(&plain)
        .verify(id, ASKED_UNBOUNDED)
        .tstates;
    let whole = Device::with_library(&bytes)
        .verify(id, ASKED_UNBOUNDED)
        .tstates;
    let interrupted = status_code("ST_INTERRUPTED");
    for (name, press_at, budget) in [
        ("mid-proof", steps / 2, BUDGET_MID_PROOF),
        ("mid-check", steps + (whole - steps) / 2, BUDGET_MID_CHECK),
    ] {
        let mut device = Device::with_library(&bytes);
        let after = press_on_during(&mut device, id, press_at);
        println!("{name}: ON seen {after} T-states after it went down");
        assert_eq!(device.byte("TEST_VERIFY_STATUS"), interrupted, "{name}");
        assert!(after < budget, "{name}: {after} T-states");
        assert_eq!(verify_again(&mut device), status(Ok(())), "{name}");
    }
}

fn status_code(name: &str) -> u8 {
    label(name) as u8
}

#[test]
fn the_on_key_held_from_the_start_stops_the_first_proof_step() {
    let (db, bytes) = disjoint_library(&two_terms(27, 8, 13, 77), 35, true);
    let id = number(&db, "t");
    let mut device = Device::with_library(&bytes);
    device.m.press_key(Machine::KEY_ON);
    let got = device.verify(id, ASKED_UNBOUNDED);
    println!("ON held: {} T-states", got.tstates);
    assert_eq!(got.init, 0);
    assert_eq!(got.verify, status_code("ST_INTERRUPTED"));
    assert!(got.tstates < BUDGET_ON_AT_START, "{} T-states", got.tstates);
    assert_eq!(verify_again(&mut device), status(Ok(())));
}

#[test]
fn an_interrupted_call_restores_registers_the_bank_and_interrupts() {
    let marker = *b"flash page marker";
    let (db, bytes) = disjoint_library(&two_terms(27, 8, 13, 77), 35, true);
    let id = number(&db, "t");
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
        device.byte("TEST_VERIFY_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert_eq!(device.m.read_ram(device.label("TEST_REGS"), 10), REGISTERS);
    for page in [0x81u8, 0x50] {
        for interrupts in [0u8, 1] {
            device.m.out_port(7, page);
            device
                .m
                .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
            device.run("ENTRY_IFFV");
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
