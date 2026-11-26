//! F3 with the preview pending and F3 twice running: on the stub against the model, and on the
//! reference OS the gap between two presses at which the shell takes the second one at once.

use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Key;

/// F3 twice with no settle between, from `at` rows down a list of alternating theorems and axioms
/// whose preview is settled: every blit's frame is the model's, and the preview and the wait are the
/// model's too.
fn twice(at: usize, settled: bool) {
    let built = pattern_built("pages-kinds-f3", "TATATATATATATATA");
    let mut list = Lister::open(built, None, 0);
    list.settle();
    for _ in 0..at {
        list.key(DOWN, Key::Down);
        list.settle();
    }
    if !settled {
        list.key(DOWN, Key::Down);
        assert_eq!(list.seen.0, PENDING);
    }
    let name = format!("row {at}, settled {settled}");
    let mut changed = Vec::new();
    for n in 0..2 {
        let before = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        let cursor = list.frame_bytes()[2];
        list.key(F3, Key::F3);
        list.same_kind_position(&format!("{name}, F3 {n}"));
        let moved = list.frame_bytes()[2] != cursor;
        let want = if list.pos().pending {
            (PENDING, 11)
        } else {
            before
        };
        assert_eq!(list.seen, want, "{name}, F3 {n}: the preview and the wait");
        changed.push((moved, list.pos().pending));
    }
    println!("{name}: moved and pending after each F3: {changed:?}");
    assert!(list.settle() || list.pos().len == 0 || !list.pos().pending);
}

#[test]
fn f3_twice_with_no_settle_between_keeps_or_blanks_the_preview_as_the_model_does() {
    twice(0, true);
    twice(1, true);
    twice(4, true);
    twice(5, true);
    twice(0, false);
    twice(3, false);
}

/// What the shell does with a second F3 pressed `gap` T-states after the first was released: how long
/// it was held before scr_key took it, or None when it was not taken in `BOUND`, and how many times
/// scr_key took it while it stayed down for `HOLD` more.
const BOUND: u32 = 3_000_000;

fn second_press(list: &mut Lister, gap: u32) -> (Option<u32>, u16) {
    const STEP: u32 = 20_000;
    const HOLD: u32 = 300_000;
    list.run.run(2_000_000);
    list.tap(F3);
    list.run.run(gap);
    let before = list.run.scr_keys();
    list.run.m().press_key(F3);
    let mut held = 0;
    while list.run.scr_keys() == before && held < BOUND {
        list.run.run(STEP);
        held += STEP;
    }
    let taken = (held < BOUND).then_some(held);
    list.run.run(HOLD);
    let presses = list.run.scr_keys() - before;
    list.run.m().release_key(F3);
    list.run.run(HOLD);
    (taken, presses)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_second_f3_is_taken_at_once_after_a_gap_and_when_held_before_it() {
    let mut list = Lister::on_rom(crate::list_real::PROP);
    list.open_on_rom();
    let gaps: Vec<u32> = (0..=400_000)
        .step_by(20_000)
        .chain([600_000, 1_000_000, 2_000_000])
        .collect();
    let mut rows = Vec::new();
    for gap in gaps {
        let (taken, presses) = second_press(&mut list, gap);
        rows.push((gap, taken, presses));
    }
    println!(
        "gap, T-states held before scr_key took the second F3 (bound {BOUND}), presses while held: {rows:?}"
    );
    let edge = rows
        .iter()
        .find(|r| r.1.is_some_and(|h| h <= 40_000))
        .map(|r| r.0);
    println!("the shortest gap at which the second F3 is taken at once: {edge:?}");
    assert!(edge.is_some());
    assert!(
        rows.iter()
            .filter(|r| r.0 >= 340_000)
            .all(|r| r.1.is_some() && r.2 == 1),
        "{rows:?}"
    );
}
