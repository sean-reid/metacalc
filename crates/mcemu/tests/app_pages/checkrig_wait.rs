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

/// ON goes down and up between the release and the press, which raises interrupts that scan no
/// keys: the stub acknowledges each with 08h, 0Ah and 0Bh, and the OS's wait for ON on the reference
/// OS with 08h alone. A wait that counted an 08h would end early and the OS would ignore the press.
/// The writes of the ON edge and of the timer interrupts around it are counted by their port 1
/// writes and by the rig's counter, which must agree.
#[test]
fn an_on_edge_between_a_release_and_a_press_is_not_counted_as_a_scan() {
    let mut run = run_after_a_tap();
    assert_eq!(run.scans_since_release(), 0, "no scan before ON goes down");
    let m = &mut run.shell.app.m;
    m.clear_port_writes();
    m.press_key(Machine::KEY_ON);
    let mut writes = Vec::new();
    for step in 0.. {
        assert!(step < 400, "the edge and two scans came in {step} steps");
        assert_eq!(m.run_app(20_000).stop, mcemu::Stop::Limit);
        writes.extend(m.take_port_writes());
        let edge = writes.iter().position(|w| (w.port, w.value) == (3, 0x0A));
        if edge.is_some_and(|at| {
            writes[at..]
                .iter()
                .filter(|w| (w.port, w.value) == (1, 0))
                .count()
                >= 2
        }) {
            break;
        }
    }
    m.release_key(Machine::KEY_ON);
    let edges = writes
        .iter()
        .filter(|w| (w.port, w.value) == (3, 0x0A))
        .count();
    let timer = writes
        .iter()
        .filter(|w| (w.port, w.value) == (1, 0))
        .count();
    let acks_08 = writes
        .iter()
        .filter(|w| (w.port, w.value) == (3, 8))
        .count();
    assert_eq!(edges, 1, "one ON edge");
    assert!(timer >= 2, "{timer} timer scans");
    assert_eq!(acks_08, edges + timer, "every interrupt acknowledges 08h");
    let mut counted = Scans::default();
    counted.release(DOWN);
    counted.feed(&writes, false);
    assert_eq!(
        counted.seen(),
        timer,
        "the scans counted are the timer's, not the edge's"
    );
    assert!(
        counted.seen() < 5,
        "{} scans before the second Down",
        counted.seen()
    );
    run.tap(DOWN);
    run.run(30_000);
    assert_eq!(run.scr_keys(), 2, "both Downs reached scr_key");
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
#[should_panic(expected = "use Run::press")]
fn an_opt_out_covers_one_gap_only() {
    let mut run = run_after_a_tap();
    run.allow_raw_press();
    run.m().press_key(DOWN);
    run.run(1_000);
    run.m().release_key(DOWN);
    run.release(DOWN);
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

#[test]
#[should_panic(expected = "use Run::press")]
fn a_press_through_the_shells_machine_inside_the_gap_panics() {
    let mut run = run_after_a_tap();
    run.shell.app.m.press_key(DOWN);
}

#[test]
#[should_panic(expected = "use Run::press")]
fn a_press_after_part_of_the_gap_has_run_panics() {
    let mut run = run_after_a_tap();
    run.run(120_000);
    assert!(run.scans_since_release() < 5, "the gap has not passed");
    run.shell.app.m.press_key(DOWN);
}

#[test]
#[should_panic(expected = "use Run::press")]
fn the_shells_tap_of_the_key_just_let_go_panics() {
    let mut run = run_after_a_tap();
    run.shell.tap(DOWN);
}

#[test]
#[should_panic(expected = "use Run::press")]
fn the_shells_lost_tap_of_the_key_just_let_go_panics() {
    let mut run = run_after_a_tap();
    run.shell.tap_lost(DOWN);
}

#[test]
#[should_panic(expected = "use Run::press")]
fn the_shells_leave_with_the_key_just_let_go_panics() {
    let mut run = run_after_a_tap();
    run.shell.leave(DOWN);
}

#[test]
fn a_press_of_another_key_through_the_shells_machine_is_not_flagged() {
    let mut run = run_after_a_tap();
    run.shell.app.m.press_key(F2);
}

#[test]
fn a_press_through_the_shells_machine_is_allowed_when_the_test_says_so() {
    let mut run = run_after_a_tap();
    run.allow_raw_press();
    run.shell.app.m.press_key(DOWN);
}
