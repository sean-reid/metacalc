//! The shell's idle point: what the loop does between frames and keys, with the app's four kinds
//! and with a kind whose idle routine is a script (idlerig.rs). Each test runs on the stub OS, and
//! again on the reference OS where it is ignored.

use crate::builds::*;
use crate::idlerig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use std::collections::BTreeSet;

fn counts(rig: &mut Rig) -> (u16, u16, u16, u16) {
    let s = &mut rig.shell;
    (
        s.blits(),
        s.key_waiting_calls(),
        s.idle_calls(),
        s.scr_keys(),
    )
}

fn shipped_kinds_cost_one_waiting_and_one_idle(real: bool) {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &script(&[], &[]));
    rig.step(600_000);
    assert_eq!(counts(&mut rig), (1, 1, 1, 0), "the first frame");
    for (scan, name, blits) in [
        (F4, "F4, which no screen answers", 0),
        (LETTER, "a letter", 0),
        (DOWN, "Down, which redraws", 1),
        (F3, "F3, which opens a stage's About", 1),
        (CLEAR, "CLEAR, which closes it", 1),
        (DOWN, "Down to the third stage", 1),
        (DOWN, "Down to My proofs", 1),
        (F1, "F1, which opens a notice", 1),
    ] {
        let before = counts(&mut rig);
        rig.tap_when_taken(scan);
        rig.step(300_000);
        let after = counts(&mut rig);
        let want = (before.0 + blits, before.1 + 1, before.2 + 1, before.3 + 1);
        assert_eq!(
            after, want,
            "{name}: blits, app_key_waiting, scr_idle, scr_key"
        );
    }
    assert_eq!(rig.shell.depth(), 2);
}

twin!(
    each_frame_and_key_of_the_shipped_kinds_costs_one_app_key_waiting_and_one_scr_idle,
    on_the_reference_os_each_frame_and_key_of_the_shipped_kinds_costs_one_waiting_and_one_idle,
    shipped_kinds_cost_one_waiting_and_one_idle
);

fn slices_run_in_order_and_then_the_shell_waits(real: bool) {
    let slices = [
        Slice::ms(100).more(),
        Slice::ms(100).more(),
        Slice::ms(100).more(),
        Slice::ms(100),
    ];
    let built = three_stages();
    let mut rig = Rig::new(
        real,
        built,
        Enter::Real,
        &script(&[KIND_LIBRARIES], &slices),
    );
    rig.until("the four slices", 3_000_000, |r| {
        let s = r.state();
        s.logged == 4 && !s.in_slice
    });
    rig.step(200_000);
    let want: Vec<Ran> = (0..4)
        .map(|index| Ran {
            index,
            blits: 1,
            keys: 0,
            ints_on: true,
        })
        .collect();
    assert_eq!(
        rig.ran(),
        want,
        "four slices, no blit and no key between them"
    );
    let s = rig.state();
    assert_eq!(
        (s.calls, s.over, s.index),
        (4, 0, 4),
        "calls, calls past the end, next slice"
    );
    let (_, waiting, idles, _) = counts(&mut rig);
    assert_eq!(
        (waiting, idles),
        (4, 4),
        "one app_key_waiting before each scr_idle"
    );
    let mut seen = BTreeSet::new();
    let mut moved = 0;
    rig.watch(2_000_000, |r| {
        let (page, pc) = r.at();
        seen.insert((page, pc));
        if r.real {
            assert_ne!(page, BASE, "the app's own code runs at {pc:04X}");
        } else {
            assert!(
                Rig::in_stub_key_wait(pc),
                "the app is at {pc:04X}, not in the key wait"
            );
        }
        let s = r.state();
        moved += usize::from(s.calls != 4 || s.logged != 4);
    });
    println!(
        "blocked in the OS's key wait, {} distinct (page, pc): {seen:04X?}",
        seen.len()
    );
    assert_eq!(moved, 0, "no slice runs while the shell waits");
    assert_eq!(rig.shell.blits(), 1);
    same_frame(
        "Libraries",
        &rig.shell.app.frame(),
        &model::libraries(built),
    );
}

twin!(
    slices_with_carry_set_run_back_to_back_and_then_the_shell_waits_in_the_key_wait,
    on_the_reference_os_slices_with_carry_set_run_back_to_back_and_then_the_shell_waits,
    slices_run_in_order_and_then_the_shell_waits
);

fn a_redraw_from_idle_is_one_draw_one_blit(real: bool) {
    let slices = [
        Slice::ms(20).more(),
        Slice::ms(20).answers(REDRAW).more(),
        Slice::ms(20).more(),
        Slice::ms(20),
    ];
    let built = three_stages();
    let mut rig = Rig::new(
        real,
        built,
        Enter::Real,
        &script(&[KIND_LIBRARIES], &slices),
    );
    rig.until("the slices", 3_000_000, |r| {
        let s = r.state();
        s.logged == 4 && !s.in_slice
    });
    rig.step(100_000);
    let blits: Vec<u16> = rig.ran().iter().map(|r| r.blits).collect();
    assert_eq!(
        blits,
        [1, 1, 2, 2],
        "one blit between the slice that redraws and the next"
    );
    assert_eq!(
        rig.shell.enters(KIND_LIBRARIES),
        1,
        "no enter: a redraw is not a refresh"
    );
    same_frame(
        "Libraries",
        &rig.shell.app.frame(),
        &model::libraries(built),
    );
}

twin!(
    a_redraw_from_idle_is_followed_by_one_draw_and_one_blit_before_the_next_slice,
    on_the_reference_os_a_redraw_from_idle_is_followed_by_one_draw_and_one_blit,
    a_redraw_from_idle_is_one_draw_one_blit
);

fn the_sentinel_is_armed_after_an_idle_redraw(real: bool) {
    let slices = [
        Slice::ms(50).more(),
        Slice::ms(20).pushes(KIND_NOTICE, NOTICE_LIST).more(),
    ];
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::Real,
        &script(&[KIND_LIBRARIES], &slices),
    );
    let (sentinel, frame) = rig.shell.sentinel();
    rig.until("the first slice", 2_000_000, |r| r.state().in_slice);
    let byte = rig.ram(frame, 1)[0];
    assert_eq!(
        rig.ram(sentinel, 1)[0],
        !byte,
        "armed after the first frame"
    );
    let stale = !byte ^ 0x5A;
    rig.m().load_ram(sentinel, &[stale]);
    rig.until("the push", 3_000_000, |r| r.shell.depth() == 2);
    rig.step(200_000);
    assert_eq!(rig.shell.blits(), 2, "the push was drawn once");
    let byte = rig.ram(frame, 1)[0];
    assert_ne!(
        stale, !byte,
        "the stale value is not the complement the redraw leaves"
    );
    assert_eq!(
        rig.ram(sentinel, 1)[0],
        !byte,
        "armed after the redraw the idle's push caused"
    );
    let enters = rig.shell.enters(KIND_NOTICE);
    rig.tap_when_taken(F4);
    rig.step(300_000);
    assert_eq!(
        rig.shell.enters(KIND_NOTICE),
        enters,
        "the next key refreshes nothing"
    );
    assert_eq!(rig.shell.blits(), 2);
}

twin!(
    the_sentinel_is_armed_after_a_redraw_that_idle_caused,
    on_the_reference_os_the_sentinel_is_armed_after_a_redraw_that_idle_caused,
    the_sentinel_is_armed_after_an_idle_redraw
);

fn interrupts_are_on_at_every_way_into_the_idle_point(real: bool) {
    let slices = [
        Slice::ms(20).off().more(),
        Slice::ms(20).off().answers(REDRAW).more(),
        Slice::ms(20).off().more(),
        Slice::ms(5),
    ];
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::Real,
        &script(&[KIND_LIBRARIES], &slices),
    );
    let mut ways = Vec::new();
    let mut arrive = |rig: &mut Rig, way: &str| {
        rig.run_to_label("app_idle", 6_000_000);
        let on = rig.m().regs().iff1;
        ways.push((way.to_string(), on));
        assert_eq!(on, 1, "interrupts are off on the way in: {way}");
        rig.step(100);
    };
    arrive(&mut rig, "after the first frame");
    arrive(&mut rig, "after an idle slice that ran with interrupts off");
    arrive(&mut rig, "after an idle redraw");
    arrive(&mut rig, "after the last idle slice but one");
    rig.until("the script", 3_000_000, |r| {
        let s = r.state();
        s.logged == 4 && !s.in_slice
    });
    rig.step(100_000);
    rig.tap_when_taken(F4);
    arrive(&mut rig, "after a key no screen answers");
    rig.step(200_000);
    rig.tap_when_taken(DOWN);
    arrive(&mut rig, "after a key that redraws");
    rig.step(200_000);
    rig.shell.power_down();
    rig.tap_when_taken(F4);
    arrive(&mut rig, "after the refresh a power down makes");
    println!("interrupt state on every way into the idle point: {ways:?}");
    let on: Vec<bool> = rig.ran().iter().map(|r| r.ints_on).collect();
    assert_eq!(on, [true; 4], "every slice began with interrupts on");
}

twin!(
    interrupts_are_on_on_every_way_into_the_idle_point,
    on_the_reference_os_interrupts_are_on_on_every_way_into_the_idle_point,
    interrupts_are_on_at_every_way_into_the_idle_point
);

/// The most T-states one trip round the idle point may take with an empty slice: app_key_waiting,
/// app_error_open, scr_idle and the routine's own bookkeeping.
const ITERATION: u32 = 2_246;
/// The least: a trip with an empty slice takes 2,224 T-states.
const ITERATION_FLOOR: u32 = 2_180;

#[test]
fn a_trip_round_the_idle_point_with_an_empty_slice_costs_little() {
    let slices: Vec<Slice> = (0..8)
        .map(|i| {
            if i < 7 {
                Slice::ms(0).more()
            } else {
                Slice::ms(0)
            }
        })
        .collect();
    let mut rig = Rig::stub(
        three_stages(),
        Enter::Real,
        &script(&[KIND_LIBRARIES], &slices),
    );
    rig.run_to_label("app_idle", 6_000_000);
    let before = (rig.shell.key_waiting_calls(), rig.shell.idle_calls());
    let mut last = rig.m().clock();
    let mut each = Vec::new();
    for _ in 0..6 {
        rig.step(10);
        rig.run_to_label("app_idle", 1_000_000);
        let now = rig.m().clock();
        each.push(now.wrapping_sub(last));
        last = now;
    }
    println!("a trip round the idle point with an empty slice: {each:?} T-states");
    let after = (rig.shell.key_waiting_calls(), rig.shell.idle_calls());
    assert_eq!(
        (after.0 - before.0, after.1 - before.1),
        (6, 6),
        "each trip is one app_key_waiting and one scr_idle"
    );
    let (best, worst) = (
        *each[1..].iter().min().unwrap(),
        *each[1..].iter().max().unwrap(),
    );
    assert!(worst <= ITERATION, "{worst} over {ITERATION}");
    assert!(
        best >= ITERATION_FLOOR,
        "{best} under {ITERATION_FLOOR}: the trip includes app_key_waiting and scr_idle"
    );
}
