//! F2 from the theorem screen and from the too large screen, CLEAR back to each, the keys the Dist
//! screen ignores and the notices F3 and F4 still push.

use crate::app::*;
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_caps::{CASES, caps_built};
use crate::theorem_frames::find;
use crate::theorem_rig::*;
use mcui::Key;

/// Every key but CLEAR, UP and DOWN, which the Dist screen does nothing with.
const IGNORED: [(u8, Key); 9] = [
    (F1, Key::F1),
    (F2, Key::F2),
    (F3, Key::F3),
    (F4, Key::F4),
    (F5, Key::F5),
    (ENTER, Key::Enter),
    (LEFT, Key::Left),
    (RIGHT, Key::Right),
    (LETTER, Key::Letter('A')),
];

/// Taps every ignored key: none redraws and nothing the screen holds moves.
fn ignores(walker: &mut Walker) -> usize {
    let mut tried = 0;
    for (scan, key) in IGNORED {
        assert!(!press(walker, scan, key), "{scan:02X} stays");
        assert_eq!(walker.list.run.top_kind(), KIND_DIST, "{scan:02X}");
        tried += 1;
    }
    tried
}

#[test]
fn f2_on_a_theorem_opens_dist_and_clear_goes_back_to_the_models_refit() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = Walker::at(built, None, 0, n);
    for (scan, key) in [
        (LEFT, Key::Left),
        (DOWN, Key::Down),
        (DOWN, Key::Down),
        (ENTER, Key::Enter),
        (RIGHT, Key::Right),
    ] {
        walker.press(scan, key);
    }
    let before = walker.frame();
    assert_ne!(before[5], 0, "the cursor is inside a statement");
    walker.mirrored = false;
    open_dist(&mut walker);
    assert_eq!(ignores(&mut walker), 9);
    leave(&mut walker);
    walker.mirrored = true;
    let after = walker.frame();
    assert_eq!(after[..4], before[..4], "the statement and the cursor");
    for (scan, key) in [
        (DOWN, Key::Down),
        (UP, Key::Up),
        (LEFT, Key::Left),
        (ENTER, Key::Enter),
        (RIGHT, Key::Right),
        (F1, Key::F1),
        (DOWN, Key::Down),
    ] {
        walker.press(scan, key);
    }
    for (scan, key) in [(F3, Key::F3), (F4, Key::F4)] {
        walker.excursion(scan, key);
    }
}

#[test]
fn f2_on_the_too_large_screen_opens_dist_and_clear_goes_back() {
    let built = caps_built();
    let lib = lib_of(built);
    let first = lib.len() - CASES.len();
    let mut tried = 0;
    let mut large = 0;
    for (i, (label, over, _)) in CASES.into_iter().enumerate() {
        if !over {
            continue;
        }
        let mut walker = Walker::at(built, None, 0, first + i);
        walker.mirrored = false;
        assert_eq!(lib.label(first + i), label);
        assert_eq!(walker.var("TV_LARGE"), 1, "{label}: the too large screen");
        large += 1;
        open_dist(&mut walker);
        assert_eq!(app_rows(&mut walker), 0, "{label}: no pairs");
        tried += ignores(&mut walker);
        leave(&mut walker);
        assert_eq!(
            walker.var("TV_LARGE"),
            1,
            "{label}: the too large screen again"
        );
        for (scan, key) in [(F4, Key::F4)] {
            walker.excursion(scan, key);
        }
        walker.back();
    }
    assert_eq!(large, 4, "four ways past a cap");
    assert_eq!(tried, 36);
}
