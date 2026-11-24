use crate::rig::*;

/// A key down at the end of the busy loop is seen, taken by nothing, and returned by app_key.
fn held_key(mut rig: Rig, calls: u8) -> Out {
    round(&mut rig, true, calls, &[(-230 * MS, DOWN, true)], 60_000)
}

fn assert_held(mk: fn() -> Rig) {
    let out = held_key(mk(), 2);
    println!("held across the call: {out:?}");
    assert_eq!(out.wres, [1, 1], "a key held at the end is waiting");
    assert_eq!(out.pre[0], DOWN, "the OS holds the scan code");
    assert_unchanged(&out, false);
    assert_eq!(
        (out.done, out.key),
        (2, (CODE_DOWN, 0)),
        "app_key returns it"
    );
    let control = held_key(mk(), 0);
    assert_eq!((control.done, control.key), (out.done, out.key));
    assert_same_time(out.returned, control.returned);
}

#[test]
fn a_key_held_at_the_end_of_the_busy_loop_is_waiting_and_comes_back_from_app_key() {
    assert_held(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_key_held_at_the_end_of_the_busy_loop_is_waiting_and_comes_back() {
    assert_held(Rig::real);
}

/// What the reference OS measured over ten press phases a tenth of a scan apart, to the quarter
/// millisecond: the code set 1.5 to 10.0 ms after the press and cleared 38.0 to 46.25 ms after the
/// release. The windows below add one scan period outside that range on each side.
const SET_BY_US: u32 = 10_000 + SCAN_US;
const KEPT_UNTIL_US: u32 = 38_000 - SCAN_US;
const GONE_BY_US: u32 = 46_500 + SCAN_US;

#[test]
fn the_stub_keeps_a_scan_code_as_the_reference_os_does() {
    scan_code_timeline(Rig::stub());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_keeps_a_scan_code_from_the_press_to_about_42_ms_after_the_release() {
    scan_code_timeline(Rig::real());
}

/// The scan code of a key nothing asks for: set by `SET_BY_US` after the press, kept while the key
/// is down and until `KEPT_UNTIL_US` after the release, gone by `GONE_BY_US`.
fn scan_code_timeline(mut rig: Rig) {
    let at = |rig: &mut Rig, us: u32| {
        rig.step(us);
        rig.ram(KBD_SCAN, 1)[0]
    };
    assert_eq!(at(&mut rig, 100_000), 0, "no key, no code");
    rig.m().press_key(DOWN);
    assert_eq!(at(&mut rig, SET_BY_US), DOWN, "set at the press");
    assert_eq!(at(&mut rig, 200_000), DOWN, "kept while the key is down");
    rig.m().release_key(DOWN);
    assert_eq!(at(&mut rig, KEPT_UNTIL_US), DOWN, "kept after the release");
    assert_eq!(
        at(&mut rig, GONE_BY_US - KEPT_UNTIL_US),
        0,
        "gone after the release"
    );
}
