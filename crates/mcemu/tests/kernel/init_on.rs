use crate::raw;
use crate::support::*;
use crate::work::{in_code, press_on_during, status_code, verify_again};
use mcemu::{Machine, Stop};
use mcpack::library::KernelError;

/// Bytes of the ON poll at the top of the loop: in, and, jp.
const POLL_LEN: u16 = 7;
const PAIR_LOOP: &[(&str, &str)] = &[("INIT_NAME_OUTER", "INIT_NAMES_DONE")];

fn malformed() -> u8 {
    status(Err(KernelError::Malformed))
}

fn init_alone(bytes: &[u8]) -> u32 {
    let mut device = Device::with_library(bytes);
    device.params(ASKED_UNBOUNDED);
    device.run("ENTRY_IFF")
}

fn accepted_then_fresh(device: &mut Device, what: &str) {
    let got = device.verify(0, ASKED_UNBOUNDED);
    assert_eq!(
        (got.init, got.verify),
        (0, 0),
        "{what}: a fresh init with ON up"
    );
}

#[test]
fn the_longest_comparison_of_names_runs_to_the_end_with_on_up_and_is_accepted() {
    let bytes = raw::shared_prefix_library();
    let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    assert_eq!((want_init, &want[..]), (0, &[0u8][..]));
    let (init, got, _) = Device::with_library(&bytes).verify_all(1, ASKED_UNBOUNDED);
    assert_eq!((init, got), (0, want));
    let tstates = init_alone(&bytes);
    println!(
        "255 names sharing 60 bytes: init {tstates} T-states, {:.1} s at 6 MHz",
        f64::from(tstates) / 6e6
    );
    assert!(tstates < CEILING_LONGEST_INIT, "{tstates} T-states");
}

#[test]
fn the_on_key_held_from_the_start_stops_init_and_leaves_nothing_ready() {
    let bytes = raw::shared_prefix_library();
    let mut device = Device::with_library(&bytes);
    accepted_then_fresh(&mut device, "before");
    device.m.press_key(Machine::KEY_ON);
    let got = device.verify(0, ASKED_UNBOUNDED);
    println!("ON held through init: {} T-states", got.tstates);
    assert_eq!(got.init, status_code("ST_INTERRUPTED"));
    assert_eq!(
        got.verify,
        status_code("ST_INTERRUPTED"),
        "ON is still down at the verify's exit"
    );
    assert!(
        got.tstates < BUDGET_INIT_ON_AT_START,
        "{} T-states",
        got.tstates
    );
    assert_eq!(
        verify_again(&mut device),
        malformed(),
        "no state is ready after the stop"
    );
    accepted_then_fresh(&mut device, "after");
}

#[test]
fn the_on_key_pressed_midway_through_init_stops_it_and_a_fresh_init_succeeds() {
    let bytes = raw::shared_prefix_library();
    let whole = init_alone(&bytes);
    let mut device = Device::with_library(&bytes);
    let (pc, after) = press_on_during(&mut device, 0, whole / 2);
    println!("ON seen {after} T-states after it went down in the pair loop");
    assert!(in_code(pc, PAIR_LOOP), "pc {pc:#06x}");
    assert_eq!(
        device.byte("TEST_INIT_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        status_code("ST_INTERRUPTED"),
        "ON is still down at the verify's exit"
    );
    assert!(after < BUDGET_POLL_GAP, "{after} T-states");
    assert_eq!(
        verify_again(&mut device),
        malformed(),
        "no state is ready after the stop"
    );
    accepted_then_fresh(&mut device, "midway");
}

#[test]
fn init_reads_the_on_key_within_one_outer_iteration_of_the_pair_loop() {
    let bytes = raw::shared_prefix_library();
    let mut device = Device::with_library(&bytes);
    device.params(ASKED_UNBOUNDED);
    device.m.load_ram(device.label("TEST_ASSERTION"), &[0, 0]);
    let outer = label("INIT_NAME_OUTER");
    let mut run = device.m.run(device.label("ENTRY_ONE"), 1);
    while device.m.pc() != outer {
        assert_eq!(run.stop, Stop::Limit, "init ended before the pair loop");
        run = device.m.run(device.m.pc(), 1);
    }
    for _ in 0..POLL_LEN {
        if device.m.pc() == outer + POLL_LEN {
            break;
        }
        let step = device.m.run(device.m.pc(), 1);
        assert_eq!(step.stop, Stop::Limit, "init ended inside the poll");
    }
    assert_eq!(device.m.pc(), outer + POLL_LEN, "the poll is where it was");
    device.m.press_key(Machine::KEY_ON);
    let rest = device.m.run(device.m.pc(), RUN_LIMIT);
    assert_eq!(rest.stop, Stop::Halted);
    println!(
        "first outer iteration: ON seen {} T-states after it went down",
        rest.tstates
    );
    assert_eq!(
        device.byte("TEST_INIT_STATUS"),
        status_code("ST_INTERRUPTED")
    );
    assert!(rest.tstates < BUDGET_POLL_GAP, "{} T-states", rest.tstates);
}

const CEILING_LONGEST_INIT: u32 = 124_000_000;
const BUDGET_INIT_ON_AT_START: u32 = 1_970_000;
const BUDGET_POLL_GAP: u32 = 1_000_000;

/// A refusal is a status like any other: with interrupts off and the latch set by a tap that ended
/// before the call, the exit replaces it with the stop, and with the latch clear it stands.
#[test]
fn a_refused_call_returns_the_stop_when_the_latch_is_set_and_its_refusal_when_clear() {
    for (entry, what) in [
        ("ENTRY_VERIFY", "kernel_verify after kernel_reset"),
        ("ENTRY_CHUNK", "kernel_crc_chunk with no run begun"),
    ] {
        for tapped in [false, true] {
            let mut device = Device::with_library(&raw::shared_prefix_library());
            device.run("ENTRY_RESET_ALONE");
            device.m.out_port(3, 0x0B);
            if tapped {
                device.m.press_key(Machine::KEY_ON);
                device.m.release_key(Machine::KEY_ON);
            }
            assert_eq!(
                device.m.in_port(4) & 1,
                u8::from(tapped),
                "{what}: the latch"
            );
            device.run(entry);
            let want = if tapped {
                status_code("ST_INTERRUPTED")
            } else {
                malformed()
            };
            assert_eq!(
                device.byte("TEST_VERIFY_STATUS"),
                want,
                "{what}, tapped {tapped}"
            );
        }
    }
}
