use crate::realos::ON_INTERRUPT;
use crate::rig::*;

struct Tap {
    out: Out,
    done: u8,
    key: (u8, u8),
    then: Option<u32>,
}

/// A round with one key pressed at `press` ms from the end of the busy loop and released at
/// `release`, if it is; then the round's outcome, and when app_key is still waiting a tap of Left
/// and what it returned.
fn tap_outcome(mk: fn() -> Rig, ints: bool, press: i32, release: Option<i32>, calls: u8) -> Tap {
    let mut rig = mk();
    let mut events = vec![(press * MS, DOWN, true)];
    events.extend(release.map(|r| (r * MS, DOWN, false)));
    let out = round(&mut rig, ints, calls, &events, 100_000);
    let mut then = None;
    if out.done == 1 {
        then = rig
            .play(&[(0, LEFT, true), (30_000, LEFT, false)], 200_000)
            .returned;
    }
    let (done, key) = (rig.done(), rig.key());
    Tap {
        out,
        done,
        key,
        then,
    }
}

/// The call answers `waiting` for the key pressed and released as given, takes nothing, and the
/// next app_key returns Down at once when it answered NZ and waits for the next key when it
/// answered Z, as it does with no call. `seen` is whether a scan of the keypad saw the key before
/// the busy loop ended, so a tap the OS never scanned cannot pass for one it scanned and cleared.
fn assert_tap(
    mk: fn() -> Rig,
    ints: bool,
    press: i32,
    release: Option<i32>,
    waiting: bool,
    seen: bool,
) {
    let with = tap_outcome(mk, ints, press, release, 1);
    let without = tap_outcome(mk, ints, press, release, 0);
    println!(
        "ints {ints} press {press} release {release:?}: {:?}",
        with.out
    );
    assert_eq!(with.out.wres, [u8::from(waiting)]);
    assert_eq!(with.out.saw_code, seen, "the scan of the keypad");
    assert_eq!(without.out.saw_code, seen, "the scan with no call");
    assert_unchanged(&with.out, !ints);
    assert_eq!(
        (with.done, with.key),
        (without.done, without.key),
        "app_key differs with the call"
    );
    assert_same_time(with.out.returned, without.out.returned);
    assert_same_time(with.then, without.then);
    let want = if waiting {
        (2, (CODE_DOWN, 0))
    } else {
        (2, (CODE_LEFT, 0))
    };
    assert_eq!((with.done, with.key), want, "{:?}", without.out);
}

macro_rules! tap_tests {
    ($($name:ident, $real:ident: $ints:expr, $press:expr, $release:expr, $waiting:expr, $seen:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_tap(Rig::stub, $ints, $press, $release, $waiting, $seen);
        }

        #[test]
        #[ignore = "needs the reference ROM image"]
        fn $real() {
            assert_tap(Rig::real, $ints, $press, $release, $waiting, $seen);
        }
    )*};
}

tap_tests! {
    a_30_ms_tap_early_in_the_busy_loop_is_gone_by_its_end,
    on_the_reference_os_a_30_ms_tap_early_in_the_busy_loop_is_gone_by_its_end:
        true, -230, Some(-200), false, true;
    a_30_ms_tap_released_75_ms_before_the_end_is_gone,
    on_the_reference_os_a_30_ms_tap_released_75_ms_before_the_end_is_gone:
        true, -105, Some(-75), false, true;
    a_5_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end,
    on_the_reference_os_a_5_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end:
        true, -30, Some(-25), true, true;
    a_30_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end,
    on_the_reference_os_a_30_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end:
        true, -60, Some(-30), true, true;
    taps_with_interrupts_off_are_never_seen_early,
    on_the_reference_os_taps_with_interrupts_off_are_never_seen_early:
        false, -230, Some(-200), false, false;
    a_5_ms_tap_late_with_interrupts_off_is_never_seen,
    on_the_reference_os_a_5_ms_tap_late_with_interrupts_off_is_never_seen:
        false, -30, Some(-25), false, false;
    a_key_pressed_with_interrupts_off_and_still_down_is_waiting,
    on_the_reference_os_a_key_pressed_with_interrupts_off_and_still_down_is_waiting:
        false, -20, None, true, false;
}

/// A key held between auto repeats has no scan code, so the call answers from port 1 with
/// interrupts on, and returns with them on, as it does on the other paths.
fn assert_returns_with_interrupts_on(mk: fn() -> Rig) {
    let mut rig = mk();
    rig.m().press_key(DOWN);
    rig.start_round(0, true, 0, 0);
    rig.step(100_000);
    assert_eq!(rig.done(), 1);
    assert_eq!(
        rig.ram(KBD_SCAN, 1),
        [0],
        "the key is held and its code taken"
    );
    for (name, setup) in [
        ("port", None),
        ("scan code", Some((KBD_SCAN, DOWN))),
        ("break flag", Some((ON_FLAGS_AT, ON_INTERRUPT))),
    ] {
        rig.poke(KBD_SCAN, &[0]);
        rig.poke(ON_FLAGS_AT, &[0]);
        if let Some((at, v)) = setup {
            rig.poke(at, &[v]);
        }
        rig.start_round(0, true, SKIP_KEY, 1);
        rig.step(30_000);
        let out = rig.out(1);
        assert_eq!(out.wres, [1], "{name}: waiting");
        assert_eq!(out.iff, 1, "{name}: interrupts are on after the call");
    }
}

#[test]
fn the_call_returns_with_interrupts_on_on_every_path() {
    assert_returns_with_interrupts_on(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_call_returns_with_interrupts_on_on_every_path() {
    assert_returns_with_interrupts_on(Rig::real);
}
