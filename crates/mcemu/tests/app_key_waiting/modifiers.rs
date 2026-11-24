use crate::rig::*;

/// 2nd or ALPHA tapped late in the busy loop, the call, and then `key` tapped once app_key waits:
/// what app_key returns.
fn modifier_key(mk: fn() -> Rig, modifier: u8, key: u8, calls: u8) -> (Out, (u8, u8)) {
    let mut rig = mk();
    let out = round(
        &mut rig,
        true,
        calls,
        &[(-30 * MS, modifier, true), (-25 * MS, modifier, false)],
        100_000,
    );
    assert_eq!(out.done, 1, "app_key waits for the key after the modifier");
    rig.play(&[(0, key, true), (30_000, key, false)], 200_000);
    assert_eq!(rig.done(), 2);
    (out, rig.key())
}

fn assert_modifier(mk: fn() -> Rig, modifier: u8, key: u8, want: (u8, u8)) {
    let (out, got) = modifier_key(mk, modifier, key, 1);
    let (_, control) = modifier_key(mk, modifier, key, 0);
    assert_eq!(out.wres, [1], "a pending modifier is waiting");
    assert_unchanged(&out, false);
    assert_eq!(got, control, "the call changed what the key gives");
    assert_eq!(got, want);
}

#[test]
fn a_pending_2nd_is_waiting_and_the_key_after_it_gives_the_same_with_the_call() {
    assert_modifier(Rig::stub, SECOND, MODE, (CODE_QUIT, 0));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pending_2nd_is_waiting_and_the_key_after_it_gives_the_same() {
    assert_modifier(Rig::real, SECOND, MODE, (CODE_QUIT, 0));
}

#[test]
fn a_pending_alpha_is_waiting_and_the_key_after_it_gives_the_same_with_the_call() {
    assert_modifier(Rig::stub, ALPHA, SEVEN, (CODE_LETTER, b'7'));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pending_alpha_is_waiting_and_the_key_after_it_gives_the_same() {
    assert_modifier(Rig::real, ALPHA, SEVEN, (CODE_LETTER, b'7'));
}

/// 2nd tapped, then Down within the busy loop: Down replaces the modifier's scan code, so the call
/// answers NZ and app_key returns Down with no modifier on it (after 2nd it would give nothing),
/// with or without the call.
fn assert_key_over_modifier(mk: fn() -> Rig) {
    let run = |calls| {
        let mut rig = mk();
        let events = [
            (-60 * MS, SECOND, true),
            (-55 * MS, SECOND, false),
            (-30 * MS, DOWN, true),
            (-25 * MS, DOWN, false),
        ];
        let out = round(&mut rig, true, calls, &events, 100_000);
        assert_unchanged(&out, false);
        out
    };
    let with = run(1);
    let without = run(0);
    println!("2nd then Down in one busy loop: {with:?}");
    assert_eq!(with.wres, [1]);
    assert_eq!((with.done, with.key), (2, (CODE_DOWN, 0)));
    assert_eq!((without.done, without.key), (2, (CODE_DOWN, 0)));
    assert_same_time(with.returned, without.returned);
}

#[test]
fn a_key_pressed_after_a_2nd_in_the_busy_loop_gives_the_same_with_the_call() {
    assert_key_over_modifier(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_key_pressed_after_a_2nd_in_the_busy_loop_gives_the_same() {
    assert_key_over_modifier(Rig::real);
}

/// Alpha lock set before the round: the key tapped late in it, the call and app_key's result. The
/// stub does not model alpha lock, so this runs on the reference OS only.
fn lock_key(calls: u8) -> (Out, (u8, u8)) {
    let mut rig = Rig::real();
    rig.start_round(1, true, 0, 0);
    rig.play(
        &[
            (20_000, SECOND, true),
            (50_000, SECOND, false),
            (100_000, ALPHA, true),
            (130_000, ALPHA, false),
            (300_000, SEVEN, true),
            (330_000, SEVEN, false),
        ],
        600_000,
    );
    assert_eq!(rig.done(), 1);
    assert_eq!(rig.key(), (CODE_LETTER, b'7'), "locked 7");
    let out = round(
        &mut rig,
        true,
        calls,
        &[(-40 * MS, SEVEN, true), (-28 * MS, SEVEN, false)],
        100_000,
    );
    let key = rig.key();
    (out, key)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_alpha_lock_gives_the_same_key_with_the_call() {
    let (out, key) = lock_key(1);
    let (control, control_key) = lock_key(0);
    assert_eq!(out.wres, [1]);
    assert_unchanged(&out, false);
    assert_eq!(key, control_key);
    assert_eq!(key, (CODE_LETTER, b'7'));
    assert_same_time(out.returned, control.returned);
}
