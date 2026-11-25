//! How a check ends other than by running out: ON as a tap between two steps and as a press held
//! through a kernel call, and a power down after the end.

use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use mcemu::Machine;

/// A run of the proofs library with its table, stopped at the blit of the second frame, EVERY steps in.
pub fn mid_check() -> Run {
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    for _ in 0..2 {
        run.next_blit();
    }
    run.m().release_key(F2);
    run
}

pub fn settled(run: &mut Run) -> usize {
    run.values().unwrap().iter().filter(|&&v| v != 0).count()
}

/// ON pressed at the idle point, a tap between two steps: the shell sets APP_ON, the idle call
/// reads and clears it and pops before the next step, so no kernel call is made. ON pressed just
/// as the kernel starts a verify, a press held through a kernel call: the call answers
/// CK_PAUSED with nothing written for its assertion and the idle call pops. Both leave "next" at
/// the first assertion not verified.
#[test]
fn on_between_two_steps_and_on_held_through_a_kernel_call_both_stop_the_check() {
    // Between two steps: the blit is followed by the idle point.
    let mut run = mid_check();
    let (done, verifies) = (settled(&mut run), run.verifies());
    assert_eq!(done, EVERY);
    run.m().press_key(Machine::KEY_ON);
    run.next_blit();
    assert_eq!(run.depth(), 1, "popped");
    assert_eq!(run.verifies(), verifies, "no kernel call was made");
    assert_eq!(settled(&mut run), done);
    assert_eq!(usize::from(run.next_number()), done);
    assert_eq!(run.byte("APP_ON"), 0, "the idle call cleared it");
    run.m().release_key(Machine::KEY_ON);
    // Held through a kernel call: the verify is entered and answers CK_PAUSED.
    let mut run = mid_check();
    run.run_to("far_kernel_verify", WAIT);
    run.m().press_key(Machine::KEY_ON);
    run.next_blit();
    assert_eq!(run.depth(), 1, "popped");
    assert_eq!(
        run.verifies(),
        verifies + 1,
        "the verify was called and stopped"
    );
    assert_eq!(
        settled(&mut run),
        done,
        "nothing was written for its assertion"
    );
    assert_eq!(
        usize::from(run.next_number()),
        done,
        "next is the assertion in hand"
    );
    assert_eq!(run.byte("APP_ON"), 0, "CK_PAUSED, not APP_ON, stopped it");
    run.m().release_key(Machine::KEY_ON);
    run.run(2_000_000);
    assert_eq!(run.depth(), 1, "the app did not leave");
}

/// ON tapped between two idle points, so that the OS has flagged the press and ON is up when the
/// shell looks: the flag is the only trace. The shell turns it into APP_ON, the idle call pops
/// before the next step, "next" is the first assertion not verified and no kernel call is made.
/// `run` is in the middle of the steps.
pub fn an_on_tap_between_two_idle_points_stops_the_check(run: &mut Run, name: &str) {
    let (done, verifies) = (settled(run), run.verifies());
    assert_eq!(run.depth(), 2, "{name}: the screen is open");
    assert!(!run.on_flag(), "{name}: no flag yet");
    run.run_to("app_idle", WAIT);
    run.set_on_flag();
    let after = run.shell.app.label("app_idle") + 3;
    run.run_to_addr(name, after, WAIT);
    assert!(
        run.on_flag(),
        "{name}: the flag is set when the shell looks"
    );
    assert_ne!(run.m().in_port(4) & 8, 0, "{name}: ON is up on the port");
    run.next_blit();
    assert_eq!(run.depth(), 1, "{name}: popped");
    assert_eq!(run.top_kind(), KIND_LIBRARIES, "{name}");
    assert_eq!(run.verifies(), verifies, "{name}: no kernel call was made");
    assert_eq!(settled(run), done, "{name}: nothing was written");
    assert_eq!(
        usize::from(run.next_number()),
        done,
        "{name}: check_end wrote next"
    );
    assert_eq!(run.byte("APP_ON"), 0, "{name}: the idle call cleared it");
    assert!(!run.on_flag(), "{name}: the shell cleared the flag");
    run.run(2_000_000);
    assert_eq!(run.depth(), 1, "{name}: the app did not leave");
}

#[test]
fn an_on_tap_between_two_idle_points_leaves_only_the_os_flag_and_stops_the_check() {
    an_on_tap_between_two_idle_points_stops_the_check(&mut mid_check(), "stub");
}

/// When the check has ended the shell blocks and the OS powers down as on any screen; the refresh on
/// wake runs the enter again, which finds the table, counts it and shows "All checked", with no
/// CRC run and no step.
#[test]
fn a_power_down_after_the_end_is_refreshed_from_the_table_with_no_crc_run() {
    let built = proofs();
    let mut run = Run::boot(built, None);
    run.m().press_key(F2);
    let (finished, values) = loop {
        let (frame, values) = run.next_frame();
        if let Some(values) = values
            && values.iter().all(|&v| v != 0)
        {
            break (frame, values);
        }
    };
    run.m().release_key(F2);
    let (crc, verifies) = (run.crc_calls(), run.verifies());
    assert_eq!(crc.0, 1, "the CRC run began once");
    assert!(crc.1 >= 1);
    let want = checking_model(built, &values, 0);
    shows("finished", &finished, &want);
    run.run(3_000_000);
    run.shell.power_down();
    let blits = run.blits();
    run.tap(F4);
    let frame = run.next_blit();
    shows("after the power down", &frame, &want);
    run.run(6_000_000);
    assert_eq!(
        run.blits(),
        blits + 1,
        "one frame, the refresh, and no step after it"
    );
    assert_eq!(run.crc_calls(), crc, "no CRC run");
    assert_eq!(run.verifies(), verifies, "no step");
    assert_eq!((run.depth(), run.top_kind()), (2, KIND_CHECKING));
    assert_eq!(run.values().unwrap(), values, "the table is as it was");
    run.run(2_000_000);
    run.tap(CLEAR);
    run.next_blit();
    assert_eq!(run.depth(), 1);
    assert_eq!(usize::from(run.next_number()), values.len());
}
