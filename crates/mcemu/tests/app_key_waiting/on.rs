use crate::realos::ON_INTERRUPT;
use crate::rig::*;
use mcemu::Machine;

struct OnRun {
    out: Out,
    done: u8,
    key: (u8, u8),
    then: Option<u32>,
}

/// ON pressed at `press` ms from the end of the busy loop and released at `release` (never when
/// None), with `calls` calls: the outcome, then Left tapped once ON is up.
fn on_outcome(mk: fn() -> Rig, ints: bool, press: i32, release: Option<i32>, calls: u8) -> OnRun {
    let mut rig = mk();
    let mut events = vec![(press * MS, Machine::KEY_ON, true)];
    events.extend(release.map(|r| (r * MS, Machine::KEY_ON, false)));
    let out = round(&mut rig, ints, calls, &events, 100_000);
    rig.m().release_key(Machine::KEY_ON);
    rig.step(300_000);
    let then = rig
        .play(&[(0, LEFT, true), (30_000, LEFT, false)], 300_000)
        .returned;
    let (done, key) = (rig.done(), rig.key());
    OnRun {
        out,
        done,
        key,
        then,
    }
}

/// `waiting` is the answer expected, None for exactly when the OS had set the break flag before
/// the call; `flagged` is whether the flag is expected set before the call, None to leave it open.
fn assert_on(
    mk: fn() -> Rig,
    ints: bool,
    press: i32,
    release: Option<i32>,
    waiting: Option<bool>,
    flagged: Option<bool>,
) {
    let with = on_outcome(mk, ints, press, release, 1);
    let without = on_outcome(mk, ints, press, release, 0);
    let out = &with.out;
    println!("ON ints {ints} press {press} release {release:?}: {out:?}");
    let flag = out.pre[2] & ON_INTERRUPT != 0;
    if let Some(f) = flagged {
        assert_eq!(flag, f, "the OS's break flag before the call");
    }
    if release.is_some() && flagged == Some(true) {
        assert_eq!(
            out.pre[0], 0,
            "no scan code is held, so only the flag answers"
        );
    }
    match waiting {
        Some(w) => assert_eq!(out.wres, [u8::from(w)]),
        None => assert_eq!(out.wres[0] == 1, flag),
    }
    assert_unchanged(out, !ints);
    assert_eq!(
        (with.done, with.key),
        (without.done, without.key),
        "app_key differs with the call"
    );
    assert_same_time(with.then, without.then);
    assert_eq!(
        (with.done, with.key),
        (2, (CODE_LEFT, 0)),
        "ON is never returned"
    );
}

#[test]
fn on_held_through_the_end_of_the_busy_loop_is_waiting() {
    assert_on(Rig::stub, true, -100, None, Some(true), Some(true));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_held_through_the_end_of_the_busy_loop_is_waiting() {
    assert_on(Rig::real, true, -100, None, Some(true), Some(true));
}

#[test]
fn on_pressed_with_interrupts_off_and_still_down_is_waiting() {
    assert_on(Rig::stub, false, -20, None, Some(true), Some(false));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_pressed_with_interrupts_off_and_still_down_is_waiting() {
    assert_on(Rig::real, false, -20, None, Some(true), Some(false));
}

#[test]
fn an_on_tap_in_the_busy_loop_is_waiting_exactly_when_the_os_flagged_it() {
    assert_on(Rig::stub, true, -200, Some(-170), None, None);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_an_on_tap_in_the_busy_loop_is_waiting_exactly_when_the_os_flagged_it() {
    assert_on(Rig::real, true, -200, Some(-170), None, None);
}

#[test]
fn on_released_before_app_key_runs_is_waiting_by_the_break_flag_alone() {
    assert_on(Rig::stub, true, -150, Some(-50), Some(true), Some(true));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_released_before_app_key_runs_is_waiting_by_the_break_flag_alone() {
    assert_on(Rig::real, true, -150, Some(-50), Some(true), Some(true));
}
