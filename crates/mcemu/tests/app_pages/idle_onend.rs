//! ON still down when a screen's work ends, against the same press on a screen with nothing
//! pending: the OS's state and where the app is, after ON goes up, after a tap of Down, and after
//! ON has been held 2 s past the end of the work. Times are milliseconds from the start of the rig.

use crate::builds::*;
use crate::idle_keys::{ms, slices};
use crate::idlerig::*;
use crate::realos::ON_INTERRUPT;
use crate::shellrig::*;
use mcemu::Machine;

fn os_name(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

/// A rig that has ON down and the shell in `app_key` for good: with `work`, ON went down 30 ms
/// into slice 1 of four and the work ended with it still down; without, ON went down 100 ms after
/// the first frame. Returns the rig and the time the shell began to wait.
fn on_down(real: bool, work: bool) -> (Rig, u32) {
    let sc = if work {
        slices(4, false)
    } else {
        script(&[], &[])
    };
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &sc);
    if work {
        rig.until("slice 1", 3_000_000, |r| r.state().logged == 1);
        rig.step(30_000);
        assert!(rig.state().in_slice, "the slice was running at the press");
        rig.m().press_key(Machine::KEY_ON);
        rig.until("the work ends", 3_000_000, |r| {
            let s = r.state();
            s.logged == 4 && !s.in_slice
        });
        assert_eq!(rig.state().logged, 4, "every slice ran with ON down");
    } else {
        rig.step(100_000);
        rig.m().press_key(Machine::KEY_ON);
    }
    let since = rig.t;
    (rig, since)
}

struct Snapshot {
    os: OsState,
    at: Where,
}

fn snapshot(rig: &mut Rig) -> Snapshot {
    let os = rig.os_state();
    let at = rig.whereabouts();
    Snapshot { os, at }
}

/// The snapshots with ON down 300 ms after the shell began to wait, and 200 ms after ON went up,
/// then what a tap of Down did: scr_key calls and blits.
struct Case {
    down: Snapshot,
    up: Snapshot,
    tap: (u16, u16),
}

fn run_case(real: bool, work: bool) -> Case {
    let (mut rig, since) = on_down(real, work);
    rig.until("300 ms", 1_000_000, |r| r.t >= since + 300_000);
    let down = snapshot(&mut rig);
    rig.m().release_key(Machine::KEY_ON);
    rig.step(200_000);
    let up = snapshot(&mut rig);
    let (keys, blits) = (rig.shell.scr_keys(), rig.shell.blits());
    rig.tap_when_taken(DOWN);
    rig.step(300_000);
    let tap = (rig.shell.scr_keys() - keys, rig.shell.blits() - blits);
    println!(
        "{} OS, work {work}: ON down {:?} at {:?}; 200 ms after the release {:?} at {:?}; \
         a tap of Down made {} scr_key calls and {} blits",
        os_name(real),
        down.os,
        down.at,
        up.os,
        up.at,
        tap.0,
        tap.1
    );
    Case { down, up, tap }
}

/// After ON goes up the two cases are in one state but for the power down timer, which differs by a
/// count at most, and a tap of Down is acted on once in each.
/// The OS flags a press once, so the break flag the shell cleared during the work stays clear while
/// ON is down, where the plain wait holds it set until ON goes up; on the reference OS the interrupt
/// mask also reads 15 where the plain wait reads 11 while ON is down.
fn the_two_cases_agree_once_on_is_up(real: bool) {
    let work = run_case(real, true);
    let plain = run_case(real, false);
    assert_eq!(
        plain.down.os.flags & ON_INTERRUPT,
        ON_INTERRUPT,
        "the plain wait holds the flag while ON is down"
    );
    assert_eq!(
        work.down.os.flags & ON_INTERRUPT,
        0,
        "the work case cleared it"
    );
    assert_eq!(
        work.down.os.mask,
        if real { 15 } else { 11 },
        "the mask with ON down after work"
    );
    assert_eq!(plain.down.os.mask, 11, "the mask with ON down, plain");
    assert_eq!(
        work.down.at,
        Where::OsKeyWait,
        "after work the app waits in the OS's key wait with ON down"
    );
    assert_eq!(
        plain.down.at,
        Where::WaitingForOnUp,
        "the plain wait is in app_key's wait for ON to go up"
    );
    assert_eq!(work.up.at, Where::OsKeyWait, "after ON goes up, work");
    assert_eq!(plain.up.at, Where::OsKeyWait, "after ON goes up, plain");
    assert_eq!(work.up.os.flags, plain.up.os.flags, "flags after ON is up");
    assert_eq!(work.up.os.mask, plain.up.os.mask, "mask after ON is up");
    assert_eq!(
        (work.up.os.scan, work.up.os.key),
        (plain.up.os.scan, plain.up.os.key),
        "key bytes after ON is up"
    );
    assert_eq!(work.up.os.timers.1, plain.up.os.timers.1, "timer byte 2");
    let ran = |c: &Case| i32::from(c.down.os.timers.0) - i32::from(c.up.os.timers.0);
    println!(
        "{} OS: the power down timer ran down {} counts in the work case and {} in the plain one",
        os_name(real),
        ran(&work),
        ran(&plain)
    );
    assert!(
        (ran(&work) - ran(&plain)).abs() <= 1,
        "the timers ran down alike, to a count"
    );
    if real {
        assert!(ran(&work) > 0, "the timer counted down");
    }
    assert_eq!(work.tap, (1, 1), "Down acted on once after work");
    assert_eq!(plain.tap, (1, 1), "Down acted on once, plain");
}

twin!(
    after_on_goes_up_the_state_after_work_is_the_state_of_a_plain_wait_and_a_tap_acts_once,
    on_the_reference_os_after_on_goes_up_the_state_after_work_is_a_plain_waits,
    the_two_cases_agree_once_on_is_up
);

/// ON held 2 s past the end of the work: the times the break flag is set after the work ended (the
/// plain case has the press's own), the scr_key calls, and where the app is at the end.
fn holding_on_after_the_work(real: bool, work: bool) -> (u32, u16, Where) {
    let (mut rig, since) = on_down(real, work);
    let keys = rig.shell.scr_keys();
    let mut set_at = Vec::new();
    let mut was_set = rig.on_flag();
    let end = since + 2_000_000;
    let mut waits = 0;
    let mut samples = 0;
    rig.watch(end - rig.t, |r| {
        let set = r.on_flag();
        if set && !was_set {
            set_at.push(r.t);
        }
        was_set = set;
        samples += 1;
        waits += usize::from(matches!(r.whereabouts(), Where::OsKeyWait));
    });
    let at = rig.whereabouts();
    println!(
        "{} OS, work {work}: ON held 2 s past the end of the work: the break flag was set again at \
         {:?} ms, in the OS's key wait in {waits} of {samples} samples, at the end {at:?}, held \
         byte {}",
        os_name(real),
        set_at.iter().map(|&t| ms(t)).collect::<Vec<_>>(),
        rig.held()
    );
    assert_eq!(rig.shell.scr_keys(), keys, "ON reached no screen");
    if work {
        assert!(samples >= 3_000, "{samples} samples in 2 s");
        assert_eq!(waits, samples, "in the OS's key wait at every sample");
    }
    rig.m().release_key(Machine::KEY_ON);
    rig.step(200_000);
    assert_eq!(rig.whereabouts(), Where::OsKeyWait, "after the release");
    (set_at.len() as u32, rig.shell.scr_keys() - keys, at)
}

fn on_held_two_seconds_after_the_work(real: bool) {
    let (flagged_work, keys_work, at_work) = holding_on_after_the_work(real, true);
    let (flagged_plain, keys_plain, _) = holding_on_after_the_work(real, false);
    assert_eq!((keys_work, keys_plain), (0, 0), "ON is no key");
    assert_eq!(flagged_work, 0, "the OS does not flag the same press again");
    assert_eq!(
        flagged_plain, 1,
        "the plain wait's one flag is the press itself"
    );
    assert_eq!(at_work, Where::OsKeyWait, "app_key waits in the OS");
}

twin!(
    on_held_two_seconds_after_the_work_ends_is_not_flagged_again_and_reaches_no_screen,
    on_the_reference_os_on_held_two_seconds_after_the_work_ends_is_not_flagged_again,
    on_held_two_seconds_after_the_work
);
