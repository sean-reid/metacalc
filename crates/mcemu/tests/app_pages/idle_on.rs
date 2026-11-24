//! ON and a pending 2nd at the idle point of a screen with work pending. Times are milliseconds
//! from the start of the rig.

use crate::builds::*;
use crate::idle_keys::{begun_over, in_slice, ms, slices};
use crate::idlerig::*;
use crate::shellrig::*;
use mcemu::Machine;

fn os_name(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

/// ON held 600 ms from 20 ms into slice 1 of eight: the work goes on while it is down and ON
/// reaches no screen.
fn on_held_through_slices(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 20_000);
    let keys = rig.shell.scr_keys();
    assert!(rig.state().in_slice, "the slice was running at the press");
    rig.m().press_key(Machine::KEY_ON);
    let pressed = rig.t;
    let mut flagged: Vec<u32> = Vec::new();
    let mut was_set = false;
    let begun = {
        let mut seen = usize::from(rig.state().logged);
        let mut at = Vec::new();
        rig.watch(600_000, |r| {
            let set = r.on_flag();
            if set && !was_set {
                flagged.push(r.t);
            }
            was_set = set;
            let logged = usize::from(r.state().logged);
            while seen < logged {
                at.push(r.t);
                seen += 1;
            }
        });
        at
    };
    println!(
        "{} OS: ON pressed {}, break flag seen set at {:?}, slices begun while it was down {:?}",
        os_name(real),
        ms(pressed),
        flagged.iter().map(|&t| ms(t)).collect::<Vec<_>>(),
        begun.iter().map(|&t| ms(t)).collect::<Vec<_>>()
    );
    assert_eq!(
        flagged.len(),
        1,
        "the OS flags the press once and the shell clears the flag at the next idle point"
    );
    assert!(
        begun.len() >= 3,
        "{}: the work went on while ON was down",
        begun.len()
    );
    assert_eq!(rig.shell.scr_keys(), keys, "ON reached no screen");
    rig.m().release_key(Machine::KEY_ON);
    let after = begun_over(&mut rig, 2_000_000);
    println!("slices begun after the release: {}", after.len());
    assert_eq!(rig.state().logged, 8, "every slice ran");
    assert_eq!(rig.shell.scr_keys(), keys, "ON reached no screen");
}

twin!(
    on_held_through_several_slices_neither_parks_the_work_nor_reaches_a_screen,
    on_the_reference_os_on_held_through_several_slices_neither_parks_the_work_nor_reaches_a_screen,
    on_held_through_slices
);

/// ON pressed for `hold` ms 20 ms into slice 1 of eight.
fn on_tap(real: bool, hold: u32) {
    let mut rig = in_slice(real, &slices(8, false), 1, 20_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(Machine::KEY_ON);
    let mut flagged = None;
    rig.watch(hold * 1000, |r| {
        if flagged.is_none() && r.on_flag() {
            flagged = Some(r.t);
        }
    });
    rig.m().release_key(Machine::KEY_ON);
    let after = begun_over(&mut rig, 2_000_000);
    println!(
        "{} OS: ON tapped {hold} ms, flag set at {:?}, slices begun after the tap {}",
        os_name(real),
        flagged.map(ms),
        after.len()
    );
    assert_eq!(
        flagged.is_some(),
        !real || hold >= 100,
        "the stub flags every press, the reference OS only one held about 47 ms"
    );
    assert_eq!(rig.state().logged, 8, "every slice ran");
    assert_eq!(rig.shell.scr_keys(), keys, "ON reached no screen");
}

fn on_tapped_during_work(real: bool) {
    on_tap(real, 30);
    on_tap(real, 100);
}

twin!(
    on_tapped_during_work_neither_parks_it_nor_reaches_a_screen,
    on_the_reference_os_on_tapped_during_work_neither_parks_it_nor_reaches_a_screen,
    on_tapped_during_work
);

/// The OS's state with ON still down and with it up, for ON held while `work` is pending and
/// released after the work ends, or pressed on a screen with nothing pending. The times are from the
/// moment the shell began to wait in app_key for good, which is when the OS's power down timer
/// starts to count: ON down 300 ms after it, ON up 700 ms after it.
fn on_state(real: bool, work: bool) -> (OsState, OsState) {
    let script = if work {
        slices(4, false)
    } else {
        script(&[], &[])
    };
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &script);
    let waiting_since = if work {
        rig.until("the work", 3_000_000, |r| r.state().logged == 1);
        rig.step(30_000);
        rig.m().press_key(Machine::KEY_ON);
        rig.until("the work ends", 3_000_000, |r| {
            let s = r.state();
            s.logged == 4 && !s.in_slice
        });
        rig.t
    } else {
        rig.step(100_000);
        rig.m().press_key(Machine::KEY_ON);
        0
    };
    let to = |rig: &mut Rig, since: u32| {
        let now = rig.t;
        rig.step((waiting_since + since).saturating_sub(now));
    };
    to(&mut rig, 300_000);
    let down = rig.os_state();
    rig.m().release_key(Machine::KEY_ON);
    to(&mut rig, 700_000);
    let up = rig.os_state();
    (down, up)
}

/// What differs from the state a plain wait is in, and nothing else does: while ON is still down
/// after the work, the break flag is clear (the shell cleared it at an idle point and the OS flags
/// a press once), where the plain wait holds it set until ON goes up; on the reference OS the
/// interrupt mask port also reads 15 where the plain wait reads 11. Once ON is up the two agree,
/// but for the power down timer, which counts from when each began to wait.
fn on_after_the_work_ends_leaves_the_os_state_of_a_plain_wait(real: bool) {
    let (work_down, work_up) = on_state(real, true);
    let (plain_down, plain_up) = on_state(real, false);
    println!("{} OS, ON down, work: {work_down:?}", os_name(real));
    println!("{} OS, ON down, none: {plain_down:?}", os_name(real));
    println!("{} OS, ON up, work: {work_up:?}", os_name(real));
    println!("{} OS, ON up, none: {plain_up:?}", os_name(real));
    let flag = crate::realos::ON_INTERRUPT;
    assert_eq!(
        plain_down.flags & flag,
        flag,
        "the plain wait holds the flag"
    );
    assert_eq!(work_down.flags & flag, 0, "the work case cleared it");
    assert_eq!(
        work_down.flags,
        plain_down.flags & !flag,
        "no other flag differs"
    );
    let (work_mask, plain_mask) = if real { (15, 11) } else { (11, 11) };
    assert_eq!(
        (work_down.mask, plain_down.mask),
        (work_mask, plain_mask),
        "the interrupt mask while ON is down"
    );
    for (work, plain, what) in [(work_down, plain_down, "down"), (work_up, plain_up, "up")] {
        assert_eq!(
            (work.scan, work.key),
            (plain.scan, plain.key),
            "{what}: key bytes"
        );
        assert_eq!(work.timers.1, plain.timers.1, "{what}: timer byte 2");
    }
    assert_eq!(
        work_up.flags, plain_up.flags,
        "ON up: the OS's flags are the same"
    );
    assert_eq!(work_up.mask, plain_up.mask, "ON up: the interrupt mask");
    let ran = |s: OsState, t: OsState| i32::from(s.timers.0) - i32::from(t.timers.0);
    assert_eq!(
        ran(work_down, work_up),
        ran(plain_down, plain_up),
        "the power down timers count down alike"
    );
    if real {
        assert!(
            ran(work_down, work_up) > 0,
            "the timer counted down between the two snapshots"
        );
    }
}

twin!(
    on_after_the_work_ends_leaves_the_os_state_an_on_press_leaves_on_a_screen_with_no_work,
    on_the_reference_os_on_after_the_work_ends_leaves_the_os_state_of_a_plain_wait,
    on_after_the_work_ends_leaves_the_os_state_of_a_plain_wait
);

/// 2nd tapped so that its code is still held when slice 1 ends.
fn a_pending_2nd_parks_the_work_until_the_next_key(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 0);
    let begin = rig.t;
    let keys = rig.shell.scr_keys();
    rig.until("the tap", 400_000, |r| r.t >= begin + SLICE_US - 45_000);
    rig.m().press_key(SECOND);
    rig.step(30_000);
    rig.m().release_key(SECOND);
    assert!(
        rig.state().in_slice,
        "the slice was still running at the release"
    );
    assert_ne!(rig.scan_code(), 0, "the OS holds the 2nd's code");
    rig.until("the slice ends", 400_000, |r| !r.state().in_slice);
    let logged = rig.state().logged;
    rig.step(900_000);
    println!(
        "{} OS: 2nd pending, slices begun in the next 0.9 s: {}",
        os_name(real),
        rig.state().logged - logged
    );
    assert_eq!(rig.state().logged, logged, "no slice runs");
    assert_eq!(rig.shell.scr_keys(), keys, "2nd is no key");
    rig.tap_when_taken(F4);
    rig.step(1_000);
    assert_eq!(
        rig.shell.scr_keys(),
        keys + 1,
        "the key after it reached the screen once"
    );
    let after = begun_over(&mut rig, 2_000_000);
    println!("after the key, slices begun {}", after.len());
    assert_eq!(rig.state().logged, 8, "and the work went on");
    assert_eq!(rig.shell.scr_keys(), keys + 1);
}

/// One slice, in microseconds, with the 8 ms the stub and the reference OS add to a 150 ms one.
const SLICE_US: u32 = 158_000;

twin!(
    a_pending_2nd_holds_the_work_until_the_key_after_it,
    on_the_reference_os_a_pending_2nd_holds_the_work_until_the_key_after_it,
    a_pending_2nd_parks_the_work_until_the_next_key
);
