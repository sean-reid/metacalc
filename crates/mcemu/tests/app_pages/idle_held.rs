//! A key the shell has handled that is still down, ON, and a pending 2nd, at the idle point of a
//! screen with work pending. Times are milliseconds from the start of the rig.

use crate::builds::*;
use crate::idle_keys::{SLICE_MS, begun_over, in_slice, ms, slices};
use crate::idlerig::*;
use crate::shellrig::*;

fn the_launch_key_down_at_the_first_idle_point_does_not_park_the_work(real: bool) {
    let mut rig = Rig::launched_holding(real, three_stages(), Enter::Real, &slices(6, false));
    rig.until("slice 0", 2_000_000, |r| r.state().in_slice);
    let started = rig.t;
    assert_eq!(rig.state().logged, 1);
    assert_eq!(rig.held(), 2, "the count is 2 at the start");
    assert_eq!(
        rig.scan_code(),
        0,
        "the OS holds no code for the launch key"
    );
    assert_eq!(rig.shell.scr_keys(), 0);
    rig.until("slice 1", 600_000, |r| {
        let s = r.state();
        s.in_slice && s.logged == 2
    });
    let second = rig.t;
    rig.step(50_000);
    println!(
        "launch key down: slice 0 began at {}, slice 1 at {} ms",
        ms(started),
        ms(second)
    );
    assert_eq!(rig.held(), 2, "still 2 while the key is down");
    assert_eq!(rig.scan_code(), 0, "and no code came");
    let begun_at_release = rig.state().logged;
    rig.m().release_key(LAUNCH_KEY);
    let released = rig.t;
    let flips = rig.until("the byte clears", 1_000_000, |r| r.held() == 0);
    println!(
        "released at {}, byte clear {} ms later",
        ms(released),
        ms(flips)
    );
    let after = begun_over(&mut rig, 400_000);
    assert!(
        !after.is_empty() && rig.state().logged > begun_at_release,
        "slices began after the release: {} before it, {} now",
        begun_at_release,
        rig.state().logged
    );
    rig.step(1_500_000);
    assert_eq!(rig.state().logged, 6, "every slice ran");
    assert_eq!(rig.shell.scr_keys(), 0, "the launch key reached no screen");
    assert_eq!(rig.held(), 0);
}

twin!(
    the_key_that_launched_the_app_down_at_the_first_idle_point_does_not_park_pending_work,
    on_the_reference_os_the_key_that_launched_the_app_does_not_park_pending_work,
    the_launch_key_down_at_the_first_idle_point_does_not_park_the_work
);

/// Down pressed during slice 1 of eight and held 100 ms past the point where the shell handled it.
fn a_handled_key_still_down_does_not_park_the_work(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    let pressed = rig.t;
    assert!(rig.state().in_slice, "the slice was running at the press");
    rig.step(20_000);
    assert!(rig.state().in_slice, "and still is");
    assert_eq!(
        rig.scan_code(),
        DOWN,
        "the slice runs with interrupts on, so the OS has scanned the key and holds its code"
    );
    assert_eq!(rig.shell.scr_keys(), keys, "the shell has not taken it yet");
    let answer = rig.answer_at_the_idle_point();
    assert_eq!(
        answer,
        Answer {
            a: DOWN,
            zero: false,
            carry: false
        },
        "the shell takes the key through its scan code when the slice ends"
    );
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    let handled = rig.t;
    assert_eq!(rig.held(), 2, "the count is 2 once the key is handled");
    rig.step(100_000);
    assert!(rig.state().in_slice, "a slice began while the key is down");
    rig.m().release_key(DOWN);
    let released = rig.t;
    let begun_before = rig.state().logged;
    let calls = rig.shell.key_waiting_calls();
    assert_eq!(rig.held(), 2, "the count is still 2 when the key goes up");
    let cleared = rig.until("the byte clears", 1_000_000, |r| r.held() == 0);
    assert!(
        rig.shell.key_waiting_calls() > calls,
        "the byte was cleared at an idle point"
    );
    assert_eq!(rig.scan_code(), 0, "where the OS held no code");
    println!("the byte cleared {} ms after the release", ms(cleared));
    let after = begun_over(&mut rig, 2_500_000);
    println!(
        "{} OS: pressed {}, handled {}, released {}; slices begun by the release {}, after it {:?}",
        if real { "reference" } else { "stub" },
        ms(pressed),
        ms(handled),
        ms(released),
        begun_before,
        after.iter().map(|&t| ms(t)).collect::<Vec<_>>()
    );
    assert_eq!(begun_before, 3, "slice 2 began before the release");
    assert_eq!(rig.state().logged, 8, "the remaining slices ran");
    assert_eq!(
        rig.shell.scr_keys(),
        keys + 1,
        "and the key was acted on once"
    );
    assert_eq!(rig.held(), 0, "the byte clears once every key is up");
}

twin!(
    a_key_still_down_when_the_shell_has_handled_it_does_not_park_the_work,
    on_the_reference_os_a_key_still_down_when_the_shell_has_handled_it_does_not_park_the_work,
    a_handled_key_still_down_does_not_park_the_work
);

/// Slice `k` begun, `into` microseconds of it gone, with the byte APP_HELD clear: the first key is
/// pressed 30 ms into slice 1, handled at its end and released, and the rig runs on until the byte
/// is clear.
pub fn with_the_byte_clear(real: bool, script: &Script) -> Rig {
    let mut rig = in_slice(real, script, 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    assert_eq!(rig.held(), 2);
    rig.step(20_000);
    rig.m().release_key(DOWN);
    rig.until("the byte clears", 1_000_000, |r| r.held() == 0);
    rig
}

/// After the byte is clear, a key pressed inside an interrupts-off slice and still down at its end is
/// a new press: the OS has not scanned it during the slice, and the shell takes it when the slice
/// ends and acts on it once.
fn a_new_press_in_an_interrupts_off_slice_is_taken_when_it_ends(real: bool) {
    let mut all: Vec<Slice> = (0..3).map(|_| Slice::ms(150).more()).collect();
    all.extend((0..5).map(|i| {
        let s = Slice::ms(150).off();
        if i < 4 { s.more() } else { s }
    }));
    let mut rig = with_the_byte_clear(real, &script(&[KIND_LIBRARIES], &all));
    let keys = rig.shell.scr_keys();
    assert_eq!(rig.held(), 0);
    rig.until("an interrupts-off slice", 2_000_000, |r| {
        let s = r.state();
        s.in_slice && s.logged >= 4
    });
    rig.step(30_000);
    let logged = rig.state().logged;
    rig.m().press_key(UP);
    let pressed = rig.t;
    assert!(rig.state().in_slice, "the slice was running at the press");
    rig.step(20_000);
    assert!(rig.state().in_slice, "and still is");
    assert_eq!(
        rig.scan_code(),
        0,
        "the slice runs with interrupts off, so the OS has not scanned the key"
    );
    assert_eq!(rig.held(), 0);
    rig.until("the slice ends", 900_000, |r| !r.state().in_slice);
    let slice_ended = rig.t;
    let begun = rig.state().logged;
    rig.until("the key is acted on", 400_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    let taken = rig.t;
    assert_eq!(
        rig.state().logged,
        begun,
        "no slice began between the slice's end and the key"
    );
    assert!(
        taken - slice_ended <= 1_000,
        "acted on {} us after the slice",
        taken - slice_ended
    );
    println!(
        "{} OS: pressed {} in slice {}, the slice ended {}, acted on {} ({} ms after the slice)",
        if real { "reference" } else { "stub" },
        ms(pressed),
        logged - 1,
        ms(slice_ended),
        ms(taken),
        ms(taken - slice_ended)
    );
    assert_eq!(rig.held(), 2, "app_key returned, so the count is 2");
    rig.m().release_key(UP);
    rig.step(2_000_000);
    assert_eq!(rig.shell.scr_keys(), keys + 1, "once");
    assert_eq!(rig.state().logged, 8, "the work went on");
}

twin!(
    a_new_press_in_an_interrupts_off_slice_with_the_byte_clear_is_taken_when_the_slice_ends_and_acted_on_once,
    on_the_reference_os_a_new_press_in_an_interrupts_off_slice_is_taken_when_the_slice_ends,
    a_new_press_in_an_interrupts_off_slice_is_taken_when_it_ends
);

/// A key pressed while the shell loops through 3 ms slices that run with interrupts off, at twelve
/// different phases, each after 100 ms with every key up so that APP_HELD is clear. The OS scans the
/// keypad every 9.2 ms, so at some idle points the key is down with no code held, which only the
/// port shows; at others the code is held. Either way the shell takes the key before the next call
/// of scr_idle and acts on it once. The keys differ from one press to the next, since the stub
/// reports a key only when it differs from the last one it took.
fn a_press_the_os_has_not_scanned_is_taken_before_the_next_slice(real: bool) {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(3).off().more()]);
    sc.forever = true;
    let mut rig = in_slice(real, &sc, 0, 0);
    let (mut port_only, mut scanned) = (0, 0);
    let mut seen = Vec::new();
    for (i, key) in [LEFT, RIGHT, F4, LETTER]
        .into_iter()
        .cycle()
        .take(12)
        .enumerate()
    {
        rig.step(100_000 + 700 * i as u32);
        assert_eq!(rig.held(), 0, "press {i}: the byte is clear");
        let keys = rig.shell.scr_keys();
        rig.m().press_key(key);
        rig.run_to_label("app_idle", 3_000_000);
        let (code, calls) = (rig.scan_code(), rig.state().calls);
        rig.run_to_label("scr_key", 3_000_000);
        assert_eq!(
            rig.state().calls,
            calls,
            "press {i}: scr_idle was not called before the key was taken"
        );
        rig.m().release_key(key);
        rig.step(1_000);
        assert_eq!(rig.shell.scr_keys(), keys + 1, "press {i}: acted on once");
        seen.push(code);
        if code == 0 {
            port_only += 1;
        } else {
            scanned += 1;
        }
    }
    println!(
        "{} OS: idle points with the key down and no code {port_only}, with a code {scanned}: {seen:02X?}",
        if real { "reference" } else { "stub" }
    );
    assert_eq!(
        (port_only, scanned),
        if real { (11, 1) } else { (10, 2) },
        "presses seen only on the port, presses already scanned"
    );
}

twin!(
    a_press_the_os_has_not_scanned_by_the_end_of_an_interrupts_off_slice_is_taken_before_the_next_slice,
    on_the_reference_os_a_press_the_os_has_not_scanned_is_taken_before_the_next_slice,
    a_press_the_os_has_not_scanned_is_taken_before_the_next_slice
);

/// What a second key did while the handled one (Down) was held, then after Down was released.
struct Second {
    /// Scr_key calls in the 600 ms after the press with Down still held.
    while_held: u16,
    /// The scan code the OS held 50 ms after the press.
    code: u8,
    /// Slices begun when Down went up.
    logged_at_release: u8,
    /// Microseconds from the release of Down to the call of scr_key for the second key, and the
    /// number of slices begun by then.
    taken: Option<(u32, u8)>,
    /// Microseconds from the release to the end of the last slice.
    work_ends: Option<u32>,
    keys_after: u16,
}

/// The first key (Down) is handled and held; a second key (Up) goes down 40 ms into slice 2 of 12,
/// and Down goes up 600 ms after that. `off` runs the slices with interrupts off.
fn second_key(real: bool, off: bool) -> Second {
    let mut rig = in_slice(real, &slices(12, off), 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    assert_eq!(rig.held(), 2);
    rig.until("slice 2", 900_000, |r| {
        r.state().in_slice && r.state().logged == 3
    });
    rig.step(40_000);
    assert!(rig.state().in_slice, "the slice was running at the press");
    rig.m().press_key(UP);
    let pressed = rig.t;
    rig.step(50_000);
    let code = rig.scan_code();
    rig.step(550_000);
    let while_held = rig.shell.scr_keys() - (keys + 1);
    rig.m().release_key(DOWN);
    let released = rig.t;
    let logged_at_release = rig.state().logged;
    let mut taken = None;
    let mut ended = None;
    rig.watch(3_000_000, |r| {
        if taken.is_none() && r.shell.scr_keys() == keys + 2 {
            taken = Some((r.t - released, r.state().logged));
            r.m().release_key(UP);
        }
        let s = r.state();
        if ended.is_none() && s.logged == 12 && !s.in_slice {
            ended = Some(r.t - released);
        }
    });
    println!(
        "{} OS, slices {}: UP pressed at {}, scan code {code:02X} 50 ms later, scr_key calls with Down held {while_held}, \
         Down released at {} in slice {logged_at_release}; UP taken {:?} us after the release with that many slices begun, last slice ended {:?} us after it",
        if real { "reference" } else { "stub" },
        if off {
            "interrupts off"
        } else {
            "interrupts on"
        },
        ms(pressed),
        ms(released),
        taken,
        ended
    );
    Second {
        while_held,
        code,
        logged_at_release,
        taken,
        work_ends: ended,
        keys_after: rig.shell.scr_keys() - (keys + 1),
    }
}

/// With the first key held until 500 ms after the last slice, so that the shell waits in app_key
/// for the second, the OS reports nothing for it: scr_key is not called.
fn a_second_key_is_not_reported_while_the_first_is_held(real: bool, off: bool) {
    let mut rig = in_slice(real, &slices(8, off), 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.until("the shell takes the key", 900_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    rig.until("slice 2", 900_000, |r| {
        r.state().in_slice && r.state().logged == 3
    });
    rig.m().press_key(UP);
    rig.until("the work ends", 3_000_000, |r| {
        let s = r.state();
        s.logged == 8 && !s.in_slice
    });
    rig.step(500_000);
    println!(
        "{} OS, slices {}: scr_key calls with two keys down through the end of the work and 500 ms after: {}",
        if real { "reference" } else { "stub" },
        if off {
            "interrupts off"
        } else {
            "interrupts on"
        },
        rig.shell.scr_keys() - (keys + 1)
    );
    assert_eq!(rig.scan_code(), 0, "the OS holds no code");
    assert_eq!(
        rig.shell.scr_keys(),
        keys + 1,
        "the second key was not reported"
    );
}

/// A second key goes unseen while the first is held, and once the first is up it is taken when the
/// slice running at the release ends, which with interrupts off is the OS's one scan in the gap
/// between two slices.
fn a_second_key_is_taken_through_its_scan_code(real: bool) {
    for off in [false, true] {
        a_second_key_is_not_reported_while_the_first_is_held(real, off);
        let got = second_key(real, off);
        let way = if off {
            "interrupts off"
        } else {
            "interrupts on"
        };
        assert_eq!(
            got.code, 0,
            "{way}: the OS holds no code for the second key"
        );
        assert_eq!(got.while_held, 0, "{way}: not taken while Down is held");
        let (delay, begun) = got.taken.unwrap_or_else(|| panic!("{way}: never taken"));
        assert_eq!(begun, got.logged_at_release, "{way}: no slice began first");
        assert!(
            (1_000..=SLICE_MS * 1000 + 10_000).contains(&delay),
            "{way}: taken {delay} us after the release"
        );
        let ends = got.work_ends.expect("the last slice ended");
        assert!(
            delay + 500_000 < ends,
            "{way}: taken long before the work ended"
        );
        assert_eq!(got.keys_after, 1, "{way}: acted on once");
    }
}

twin!(
    a_new_key_pressed_while_the_handled_one_is_down_is_taken_through_its_scan_code,
    on_the_reference_os_a_new_key_pressed_while_the_handled_one_is_down_is_taken,
    a_second_key_is_taken_through_its_scan_code
);
