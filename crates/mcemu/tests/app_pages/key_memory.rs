//! The OS's memory of the last key: the same key pressed again soon after a release is not taken,
//! and a different key is. The memory lasts until the scan code clears, on the fifth scan in a row
//! with no key down. A release 6.5 ms after an interrupt puts that scan 39.9 ms after the release.

use crate::idlerig::*;
use crate::shellrig::*;
use crate::timer_spacing::{entries, rig_in, tick_loop};

/// Writes to port 1 in a scan with a key down: the keypad check and each of the eight groups, with
/// FFh after each.
const GROUP_WRITES: usize = 18;

/// Microseconds from an interrupt's entry to the release.
const RELEASE_AT: u32 = 6_500;

/// Releases `LETTER`, which the OS took, `RELEASE_AT` after an interrupt, waits `after_us` more,
/// zeroes the scan code and presses `key`. Returns the scan code after the first interrupt that
/// follows the press, which has scanned the key.
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
    let scan = &entries(&mut rig, 1)[0];
    let groups = scan.ports.iter().filter(|w| w.port == 1).count();
    assert_eq!(groups, GROUP_WRITES, "the interrupt scanned the key");
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

/// What the OS does with a scan that finds two keys down between scans that find none: the
/// interrupts after the release of the first key, `empty` scans with no key, then `two` scans with
/// LETTER and ONE down, then both released. Returns the T-states each interrupt after the two keys
/// went up kept interrupts off, and the scan code after each, from the release on.
fn two_keys_between_empty_scans(real: bool, empty: usize, two: usize) -> Vec<(u32, u8)> {
    let mut rig = rig_in(real, &tick_loop());
    entries(&mut rig, 3);
    rig.m().press_key(LETTER);
    entries(&mut rig, 2);
    assert_eq!(rig.scan_code(), LETTER, "the OS took the first press");
    rig.m().release_key(LETTER);
    let mut seen: Vec<(u32, u8)> = Vec::new();
    for e in entries(&mut rig, empty) {
        seen.push((e.off, rig.scan_code()));
    }
    rig.m().press_key(LETTER);
    rig.m().press_key(ONE);
    for e in entries(&mut rig, two) {
        seen.push((e.off, rig.scan_code()));
    }
    rig.m().release_key(LETTER);
    rig.m().release_key(ONE);
    for e in entries(&mut rig, 8) {
        seen.push((e.off, rig.scan_code()));
    }
    println!(
        "{} OS, {empty} empty scans, {two} with two keys down, then empty: (T-states off, scan code) {seen:?}",
        os(real)
    );
    seen
}

/// Two keys down are neither taken nor counted as no key: whatever came before, the fifth scan
/// with no key down after the last scan with two keys down clears the scan code, and the scans
/// before it are the four of a release. The scan code is the first key's through the two-key scans.
fn two_keys_down_restart_the_count(real: bool) {
    for (empty, two) in [(2, 1), (1, 2), (3, 2)] {
        let seen = two_keys_between_empty_scans(real, empty, two);
        let at = |i: usize| (seen[i].0, seen[i].1);
        let kinds: Vec<char> = seen
            .iter()
            .map(|&(off, _)| match off {
                4_000.. => 'T',
                1_200..=2_000 => 'C',
                _ => 'o',
            })
            .collect();
        let after: String = kinds[empty + two..].iter().collect();
        assert_eq!(
            kinds[empty..empty + two].iter().collect::<String>(),
            "T".repeat(two),
            "{} OS, {empty} empty then {two} with two keys: the two-key scans",
            os(real)
        );
        assert!(
            after.starts_with("ooooCo"),
            "{} OS, {empty} empty then {two} with two keys: scans after them (o: counting or idle, \
             C: clearing) {after}",
            os(real)
        );
        for i in empty..empty + two {
            assert_eq!(
                at(i).1,
                LETTER,
                "{} OS: two keys down leave the scan code",
                os(real)
            );
        }
    }
}

twin!(
    the_stub_restarts_the_count_of_empty_scans_at_a_scan_with_two_keys_down,
    the_reference_os_restarts_the_count_of_empty_scans_at_a_scan_with_two_keys_down,
    two_keys_down_restart_the_count
);
