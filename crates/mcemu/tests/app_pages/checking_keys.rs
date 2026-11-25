//! Keys the checking screen ignores, pressed in the middle of a check.

use crate::checking_exits::{mid_check, settled};
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::keys::PROBES;
use crate::shellrig::*;

/// A key the screen ignores in the middle of a check costs one trip through app_key and loses no
/// step: every idle call between the frames did one verify and every assertion was verified once.
#[test]
fn a_key_the_screen_ignores_loses_no_step() {
    let mut run = mid_check();
    let built = run.built;
    let (idles, waits, verifies, keys) = (
        run.shell.idle_calls(),
        run.shell.key_waiting_calls(),
        run.verifies(),
        run.scr_keys(),
    );
    run.tap(F4);
    let mut last = run.next_frame();
    assert_eq!(run.depth(), 2, "the screen is still open");
    while last.1.as_ref().unwrap().contains(&0) {
        last = run.next_frame();
    }
    let n = last.1.unwrap().len();
    assert_eq!(run.scr_keys(), keys + 1, "the key reached the screen once");
    let verified = run.verifies() - verifies;
    let idled = run.shell.idle_calls() - idles;
    let waited = run.shell.key_waiting_calls() - waits;
    println!("{verified} verifies, {idled} idle calls, {waited} key checks after the key");
    assert_eq!(idled, verified, "every idle call did one step");
    assert_eq!(
        waited,
        idled + 1,
        "the key cost one more trip through the idle point"
    );
    assert_eq!(
        usize::from(run.verifies()),
        n,
        "no assertion was verified twice or skipped"
    );
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    assert_eq!(run.values().unwrap(), expected_values(built, &verdicts, n));
}

/// Every key of the probe list but F1 and CLEAR, which stop a check, pressed in the middle of a
/// check of the padded library, F2 and the 2nd chord with no key among them: the key reaches the
/// screen once, the chord's as no key, the screen stays the checking screen with the check
/// going on, and the check finishes with each assertion verified once.
#[test]
fn every_key_the_screen_ignores_loses_no_step_and_the_check_finishes() {
    let built = padded();
    let n = usize::from(crate::table::header_of(built.library.as_ref().unwrap()).1);
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let mut tried = Vec::new();
    for probe in PROBES.iter().filter(|p| !["f1", "clear"].contains(&p.name)) {
        let name = probe.name;
        let mut run = Run::boot(built, Some(&empty_table(built)));
        run.m().press_key(F2);
        for _ in 0..2 {
            run.next_blit();
        }
        run.m().release_key(F2);
        run.run(1_000_000);
        run.next_blit();
        let (keys, before) = (run.scr_keys(), settled(&mut run));
        assert!(
            before >= EVERY && before < n / 2,
            "{name}: in the middle of the steps"
        );
        run.tap_scans(probe.scans);
        run.next_blit();
        let during = settled(&mut run);
        assert!(
            during > before && during < n,
            "{name}: the check went on, {during} of {n}"
        );
        assert_eq!(
            (run.depth(), run.top_kind()),
            (2, KIND_CHECKING),
            "{name}: the screen is still the checking screen"
        );
        assert_eq!(
            run.scr_keys(),
            keys + 1,
            "{name}: reached the screen once, the chord's second key as no key"
        );
        let (last, values) = loop {
            let (frame, values) = run.next_frame();
            assert_eq!(run.top_kind(), KIND_CHECKING, "{name}");
            let values = values.unwrap();
            if values.iter().all(|&v| v != 0) {
                break (frame, values);
            }
        };
        assert_eq!(values, expected_values(built, &verdicts, n), "{name}");
        assert_eq!(
            usize::from(run.verifies()),
            n,
            "{name}: none lost or repeated"
        );
        shows(name, &last, &checking_model(built, &values, 0));
        tried.push(name);
    }
    assert_eq!(tried.len(), 12, "{tried:?}");
    assert!(tried.contains(&"f2") && tried.contains(&"2nd and 1, no key"));
}
