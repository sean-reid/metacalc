//! The OS's memory of the last key: the same key pressed again soon after a release is not taken,
//! and a different key is. The memory lasts until the scan code clears, on the fifth scan in a row
//! with no key down. A release 6.5 ms after an interrupt puts that scan 39.9 ms after the release.

use crate::idlerig::*;
use crate::shellrig::*;
use crate::timer_spacing::{entries, rig_in, tick_loop};

/// Microseconds from an interrupt's entry to the release.
const RELEASE_AT: u32 = 6_500;

/// Releases `LETTER`, which the OS took, `RELEASE_AT` after an interrupt, waits `after_us` more,
/// zeroes the scan code and presses `key`. Returns the scan code after the first interrupt that
/// follows the press.
fn press_after(real: bool, after_us: u32, key: u8) -> u8 {
    let mut rig = rig_in(real, &tick_loop());
    entries(&mut rig, 3);
    rig.m().press_key(LETTER);
    entries(&mut rig, 2);
    assert_eq!(rig.scan_code(), LETTER, "the OS took the first press");
    let entry = entries(&mut rig, 1)[0].clock;
    let since = rig.m().clock().wrapping_sub(entry) / 6;
    rig.step(RELEASE_AT - since);
    rig.m().release_key(LETTER);
    rig.step(after_us);
    rig.m().load_ram(KBD_SCAN, &[0]);
    rig.m().press_key(key);
    entries(&mut rig, 1);
    rig.scan_code()
}

fn os(real: bool) -> &'static str {
    if real { "reference" } else { "stub" }
}

fn the_same_key_after_a_release(real: bool) {
    for (after, want) in [(2_000, 0), (38_000, 0), (42_000, LETTER)] {
        let got = press_after(real, after, LETTER);
        assert_eq!(
            got,
            want,
            "{} OS: the same key {} ms after a release",
            os(real),
            after / 1_000
        );
    }
}

fn a_different_key_after_a_release(real: bool) {
    for after in [100, 2_000, 20_000] {
        let got = press_after(real, after, ONE);
        assert_eq!(
            got,
            ONE,
            "{} OS: a different key {after} us after a release",
            os(real)
        );
    }
}

twin!(
    the_stub_ignores_the_same_key_until_the_scan_code_clears_and_takes_it_after,
    the_reference_os_ignores_the_same_key_until_the_scan_code_clears_and_takes_it_after,
    the_same_key_after_a_release
);

twin!(
    the_stub_takes_a_different_key_at_once_after_a_release,
    the_reference_os_takes_a_different_key_at_once_after_a_release,
    a_different_key_after_a_release
);
