//! The wait `Run::press` makes before it presses the key it just let go, and the check on a raw
//! press of that key.

use crate::checking_frames::proofs;
use crate::checkrig::*;
use crate::shellrig::*;
use mcemu::Machine;

fn run_after_a_tap() -> Run {
    let mut run = Run::boot(proofs(), None);
    run.tap(DOWN);
    run
}

/// ON goes down and up inside the OS's wait for it between the release and the press, which raises
/// interrupts that scan no keys. Both OSes write port 3 = 08h for those as for a timer interrupt
/// and 09h only for a timer one, so a wait that counted 08h would end early and the OS would
/// ignore the press.
#[test]
fn an_on_edge_between_a_release_and_a_press_is_not_counted_as_a_scan() {
    let mut run = run_after_a_tap();
    run.m().press_key(Machine::KEY_ON);
    run.run(60_000);
    run.m().release_key(Machine::KEY_ON);
    let keys = run.scr_keys();
    run.tap(DOWN);
    run.run(30_000);
    assert_eq!(run.scr_keys(), keys + 1, "the second Down reached scr_key");
}

#[test]
#[should_panic(expected = "use Run::press")]
fn a_raw_press_of_the_key_just_let_go_panics() {
    let mut run = run_after_a_tap();
    run.m().press_key(DOWN);
    run.run(1_000);
}

#[test]
fn a_test_that_allows_it_presses_the_key_just_let_go_raw() {
    let mut run = run_after_a_tap();
    run.allow_raw_press();
    run.m().press_key(DOWN);
    run.run(1_000);
}

#[test]
fn a_raw_press_of_the_key_after_the_gap_is_not_flagged() {
    let mut run = run_after_a_tap();
    run.run(600_000);
    run.m().press_key(DOWN);
    run.run(1_000);
}

#[test]
fn a_raw_press_of_another_key_is_not_flagged() {
    let mut run = run_after_a_tap();
    run.m().press_key(F2);
    run.run(1_000);
}
