//! Keys that arrive while a screen has work pending: which are acted on and when the work goes on.
//! Times are printed in milliseconds from the start of the rig.

use crate::builds::*;
use crate::idlerig::*;
use crate::shellrig::*;

pub const SLICE_MS: u32 = 150;

/// `n` slices of 150 ms, each with more work pending but the last, with interrupts on or off.
pub fn slices(n: usize, off: bool) -> Script {
    let all: Vec<Slice> = (0..n)
        .map(|i| {
            let s = Slice::ms(SLICE_MS);
            let s = if off { s.off() } else { s };
            if i + 1 < n { s.more() } else { s }
        })
        .collect();
    script(&[KIND_LIBRARIES], &all)
}

pub fn ms(us: u32) -> String {
    format!("{:.1}", f64::from(us) / 1000.0)
}

/// When the slices began after `from`, in the order they did, watching for `us`.
pub fn begun_over(rig: &mut Rig, us: u32) -> Vec<u32> {
    let mut seen = usize::from(rig.state().logged);
    let mut at = Vec::new();
    rig.watch(us, |r| {
        let logged = usize::from(r.state().logged);
        while seen < logged {
            at.push(r.t);
            seen += 1;
        }
    });
    at
}

/// The rig with slice `k` running and `into` microseconds of it gone.
pub fn in_slice(real: bool, script: &Script, k: u8, into: u32) -> Rig {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, script);
    rig.until("the slice", 3_000_000, |r| {
        let s = r.state();
        s.in_slice && s.logged == k + 1
    });
    rig.step(into);
    rig
}

fn a_late_tap_is_handled_and_the_work_goes_on(real: bool) {
    let script = slices(8, false);
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &script);
    let mut begin = Vec::new();
    rig.until("slices 0 to 2", 3_000_000, |r| {
        let s = r.state();
        if usize::from(s.logged) == begin.len() + 1 {
            begin.push(r.t);
        }
        s.logged == 3
    });
    println!("slices began at {begin:?}");
    let d = begin[2] - begin[1];
    println!("a slice takes {} ms from one begin to the next", ms(d));
    let key_at = begin[2] + d - 45_000;
    rig.until("the tap", 400_000, |r| r.t >= key_at);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.step(30_000);
    rig.m().release_key(DOWN);
    let released = rig.t;
    rig.until("15 ms before the slice ends", 400_000, |r| {
        r.t + 15_000 >= begin[2] + d
    });
    assert_ne!(
        rig.scan_code(),
        0,
        "the OS's scan saw the key before the slice ended"
    );
    assert!(rig.state().in_slice, "the slice was still running");
    assert_eq!(
        rig.shell.scr_keys(),
        keys,
        "and the shell has not taken it yet"
    );
    let after = begun_over(&mut rig, 1_800_000);
    println!(
        "tap released at {}, slice 3 and on began at {:?}",
        ms(released),
        after.iter().map(|&t| ms(t)).collect::<Vec<_>>()
    );
    let ran = rig.ran();
    assert_eq!(ran.len(), 8, "the remaining slices ran");
    assert_eq!(ran[2].keys, 0);
    assert_eq!(ran[3].keys, 1, "the key was acted on before the next slice");
    assert_eq!(rig.shell.scr_keys(), 1, "once");
}

twin!(
    a_tap_released_before_a_slice_ends_with_interrupts_on_is_acted_on_before_the_next_slice,
    on_the_reference_os_a_tap_released_before_a_slice_ends_is_acted_on_before_the_next_slice,
    a_late_tap_is_handled_and_the_work_goes_on
);

fn an_early_tap_is_lost(real: bool) {
    let mut rig = in_slice(real, &slices(4, false), 1, 10_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.step(30_000);
    rig.m().release_key(DOWN);
    let mut saw = false;
    rig.watch(90_000, |r| saw |= r.scan_code() != 0);
    assert!(saw, "the OS's scan saw the key");
    assert_eq!(rig.scan_code(), 0, "and dropped the code unread");
    assert!(rig.state().in_slice, "while the slice was still running");
    rig.until("slice 3", 3_000_000, |r| r.state().logged == 4);
    rig.step(400_000);
    assert_eq!(rig.shell.scr_keys(), keys, "so the key was never acted on");
    assert_eq!(rig.ran().iter().map(|r| r.keys).collect::<Vec<_>>(), [0; 4]);
}

twin!(
    a_tap_that_ends_long_before_a_slice_does_is_lost_with_interrupts_on,
    on_the_reference_os_a_tap_that_ends_long_before_a_slice_does_is_lost,
    an_early_tap_is_lost
);

fn a_tap_inside_an_interrupts_off_slice_is_lost(real: bool) {
    let mut rig = in_slice(real, &slices(4, true), 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    let mut saw = false;
    rig.watch(30_000, |r| saw |= r.scan_code() != 0);
    rig.m().release_key(DOWN);
    rig.watch(60_000, |r| saw |= r.scan_code() != 0);
    assert!(rig.state().in_slice, "the whole tap was inside the slice");
    assert!(!saw, "the OS never scanned: interrupts were off");
    rig.until("slice 3", 3_000_000, |r| r.state().logged == 4);
    rig.step(600_000);
    assert_eq!(rig.shell.scr_keys(), keys, "the tap is lost");
}

twin!(
    a_tap_that_starts_and_ends_inside_an_interrupts_off_slice_is_lost,
    on_the_reference_os_a_tap_that_starts_and_ends_inside_an_interrupts_off_slice_is_lost,
    a_tap_inside_an_interrupts_off_slice_is_lost
);

fn a_key_down_at_the_end_of_an_interrupts_off_slice_is_acted_on(real: bool) {
    let mut rig = in_slice(real, &slices(4, true), 1, 30_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.step(30_000);
    assert!(rig.state().in_slice);
    assert_eq!(rig.scan_code(), 0, "the OS has not scanned it");
    rig.until("the key is taken", 400_000, |r| {
        r.shell.scr_keys() == keys + 1
    });
    let handled = rig.t;
    rig.m().release_key(DOWN);
    rig.step(2_000_000);
    println!(
        "down at the end of an off slice, handled at {} ms, slices begun {}",
        ms(handled),
        rig.state().logged
    );
    assert_eq!(rig.shell.scr_keys(), keys + 1, "once");
}

twin!(
    a_key_still_down_at_the_end_of_an_interrupts_off_slice_is_acted_on_once,
    on_the_reference_os_a_key_still_down_at_the_end_of_an_interrupts_off_slice_is_acted_on_once,
    a_key_down_at_the_end_of_an_interrupts_off_slice_is_acted_on
);

fn a_held_key_is_acted_on_before_the_next_slice(real: bool) {
    let mut rig = in_slice(real, &slices(5, false), 1, 30_000);
    let keys = rig.shell.scr_keys();
    assert!(rig.state().in_slice, "the slice was running at the press");
    rig.tap_when_taken(DOWN);
    let after = begun_over(&mut rig, 1_500_000);
    println!(
        "slices begun after the key: {:?}",
        after.iter().map(|&t| ms(t)).collect::<Vec<_>>()
    );
    let ran = rig.ran();
    assert_eq!(ran.len(), 5, "the remaining slices ran");
    assert_eq!(ran[1].keys, 0);
    assert_eq!(
        ran[2].keys, 1,
        "the key was acted on before the next slice began"
    );
    assert!(
        ran[2].blits > ran[1].blits,
        "and its frame was drawn before it"
    );
    assert_eq!(rig.shell.scr_keys(), keys + 1, "exactly once");
}

twin!(
    a_key_pressed_during_a_slice_is_acted_on_before_the_next_slice_and_once,
    on_the_reference_os_a_key_pressed_during_a_slice_is_acted_on_before_the_next_slice_and_once,
    a_held_key_is_acted_on_before_the_next_slice
);

fn second_quit_during_idle_work_leaves(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 20_000);
    rig.m().press_key(SECOND);
    rig.until("the slice ends", 600_000, |r| !r.state().in_slice);
    rig.step(150_000);
    assert_eq!(
        rig.state().logged,
        2,
        "a pending 2nd stops the work: app_key waits for the next key"
    );
    rig.m().release_key(SECOND);
    rig.step(100_000);
    rig.m().press_key(MODE);
    rig.run_until_left(1_000_000);
    rig.assert_put_away();
    assert_eq!(rig.state().calls, 2, "no idle call after 2nd+QUIT");
}

twin!(
    second_quit_during_idle_work_leaves_through_app_exit_with_the_regions_put_away,
    on_the_reference_os_second_quit_during_idle_work_leaves_with_the_regions_put_away,
    second_quit_during_idle_work_leaves
);
