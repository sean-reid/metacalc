//! Keys the Dist screen answers with a stay where a move is possible, and the fold the theorem screen
//! keeps across F2.

use crate::dist_frames::targets;
use crate::dist_lib::{self, ALL};
use crate::dist_rig::*;
use crate::dist_screens::{IGNORED, ignores};
use crate::list_rig::proofs_three;
use crate::shellrig::*;
use crate::theorem_depth::trace_here;
use crate::theorem_rig::*;
use mcui::{Key, State};

/// The first row the other keys are pressed at: above 0 and below the last.
pub const FIRST: usize = 3;

pub const SCR_STAY: u8 = 0;

/// `dist_key` called with KEY_NONE, which no scan code gives the shell: a key is held until the shell
/// calls the routine, A is set to 0 at its entry and the call runs to its return.
pub fn key_none(walker: &mut Walker) -> u8 {
    let run = &mut walker.list.run;
    run.press(LETTER);
    let at = run.shell.app.label("dist_key");
    run.run_to_addr("dist_key", at, crate::checkrig::WAIT);
    let mut regs = run.m().regs();
    regs.af &= 0x00FF;
    run.m().set_regs(&regs);
    let t = trace_here(run, "dist_key", true, 0);
    run.release(LETTER);
    run.run(600_000);
    t.action
}

#[test]
fn every_other_key_stays_on_a_screen_that_could_move_with_nothing_changed() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let most = (0..lib.len())
        .map(|n| rows_of(&lib, n).len())
        .max()
        .unwrap();
    assert_eq!(rows_of(&lib, index_of(&lib, ALL)).len(), most);
    assert!(
        0 < FIRST && FIRST < last_of(most),
        "a move is possible both ways"
    );
    let mut walker = dist_of(built, ALL);
    for step in 1..=FIRST {
        assert!(press(&mut walker, DOWN, Key::Down), "Down {step}");
    }
    assert_eq!(model_top(&walker), FIRST);
    let frame = frame_of(&mut walker);
    assert_eq!(frame[4..6], [FIRST as u8, 0]);
    let scratch = scratch(&mut walker);
    let picture = walker.list.run.frame();
    let mut tried = ignores(&mut walker);
    assert_eq!(tried, IGNORED.len());
    let blits = walker.list.run.blits();
    assert_eq!(key_none(&mut walker), SCR_STAY, "KEY_NONE stays");
    assert_eq!(walker.list.run.blits(), blits, "KEY_NONE draws nothing");
    tried += 1;
    assert_eq!(tried, 10);
    assert_eq!(frame_of(&mut walker), frame, "the frame is as it was");
    assert_eq!(
        self::scratch(&mut walker),
        scratch,
        "the scratch is as it was"
    );
    assert_eq!(walker.list.run.frame(), picture, "the picture is as it was");
    assert_eq!(model_top(&walker), FIRST);
    assert!(press(&mut walker, DOWN, Key::Down), "Down still moves");
    assert!(press(&mut walker, UP, Key::Up), "and Up");
    leave(&mut walker);
}

/// ENTER at each position a walk of the keys reaches, until one folds: the model's picture before it, if
/// one does.
fn fold_somewhere(walker: &mut Walker) -> Option<Vec<u8>> {
    let folds = |walker: &Walker| match walker.list.ui.state() {
        State::Theorem { folded, .. } => folded,
        other => panic!("the model is on {other:?}"),
    };
    let moves = [
        (DOWN, Key::Down),
        (RIGHT, Key::Right),
        (DOWN, Key::Down),
        (DOWN, Key::Down),
    ];
    for (scan, key) in moves.into_iter().cycle().take(60) {
        walker.press(scan, key);
        let (unfolded, before) = (walker.list.shown(), folds(walker));
        if walker.press(ENTER, Key::Enter) && folds(walker) > before {
            return Some(unfolded);
        }
    }
    None
}

#[test]
fn a_fold_made_before_f2_is_forgotten_by_the_refit_after_clear_and_the_cursor_is_kept() {
    let built = proofs_three();
    let lib = lib_of(built);
    let with = targets(&lib, lib.len(), true);
    assert_eq!(with.len(), 2, "two assertions have pairs");
    for n in with {
        let mut walker = Walker::at(built, None, 2, n);
        let Some(unfolded) = fold_somewhere(&mut walker) else {
            continue;
        };
        let folded = walker.list.shown();
        assert_ne!(
            folded, unfolded,
            "the folded picture differs from the unfolded one"
        );
        let State::Theorem { folded: kept, .. } = walker.list.ui.state() else {
            panic!("the model is not on a theorem");
        };
        assert!(kept > 0, "the model has a folded node");
        let cursor = walker.frame();
        walker.mirrored = false;
        open_dist(&mut walker);
        leave(&mut walker);
        assert_eq!(
            walker.frame()[..FRAME],
            cursor[..FRAME],
            "the frame, with the cursor"
        );
        assert_eq!(
            walker.list.shown(),
            unfolded,
            "the model's refit forgets a fold the user made: the picture after CLEAR is the unfolded one"
        );
        assert_ne!(
            walker.list.shown(),
            folded,
            "the folded picture is not shown"
        );
        let State::Theorem { folded: after, .. } = walker.list.ui.state() else {
            panic!("the model is not on a theorem");
        };
        assert!(
            after < kept,
            "the model has the fit's folds only: {after} of {kept}"
        );
        return;
    }
    panic!("no assertion with pairs has a node ENTER folds");
}
