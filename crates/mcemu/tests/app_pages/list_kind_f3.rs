//! F3 with the preview pending and F3 twice running: on the stub against the model, and on the
//! reference OS the gap between two presses at which the shell takes the second one at once.

use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Key;

/// F3 twice with no settle between, from `at` rows down a list of alternating theorems and axioms
/// whose preview is settled: every blit's frame is the model's, and the preview and the wait are the
/// model's too. Returns how many of the two F3 kept the preview and how many blanked it.
fn twice(at: usize, settled: bool) -> (usize, usize) {
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
    let (mut kept, mut blanked) = (0, 0);
    for n in 0..2 {
        let before = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        let cursor = list.frame_bytes()[2];
        list.key(F3, Key::F3);
        list.same_kind_position(&format!("{name}, F3 {n}"));
        let moved = list.frame_bytes()[2] != cursor;
        let want = if list.pos().pending {
            blanked += 1;
            (PENDING, 11)
        } else {
            kept += 1;
            before
        };
        assert_eq!(list.seen, want, "{name}, F3 {n}: the preview and the wait");
        changed.push((moved, list.pos().pending));
    }
    println!("{name}: moved and pending after each F3: {changed:?}");
    let pending = list.pos().pending;
    assert_eq!(
        list.settle(),
        pending,
        "{name}: the model draws what settles"
    );
    assert_eq!(
        list.var("LV_PREVIEW"),
        ROW,
        "{name}: every conclusion here fits"
    );
    (kept, blanked)
}

#[test]
fn f3_twice_with_no_settle_between_keeps_or_blanks_the_preview_as_the_model_does() {
    let (mut kept, mut blanked) = (0, 0);
    for (at, settled) in [
        (0, true),
        (1, true),
        (4, true),
        (5, true),
        (0, false),
        (3, false),
    ] {
        let (k, b) = twice(at, settled);
        kept += k;
        blanked += b;
    }
    println!("F3 presses that kept the preview {kept}, that blanked it {blanked}");
    assert!(kept > 0 && blanked > 0, "{kept} kept, {blanked} blanked");
}

/// What the shell does with a second F3 pressed `gap` T-states after the first was released: how long
/// it was held before scr_key took it, or None when it was not taken in `BOUND`, and how many times
/// scr_key took it while it stayed down for `HOLD` more.
const BOUND: u32 = 3_000_000;
/// The greatest gap pinned as never taken and the least pinned as always taken. The table has 280,000
/// never taken and 300,000 taken once, and the gaps between 260,000 and 340,000 are left out of the
/// pins because which side of the OS's debounce they fall on depends on the timer's phase.
const EDGE: (u32, u32) = (260_000, 340_000);

fn second_press(list: &mut Lister, gap: u32) -> (Option<u32>, u16) {
    const STEP: u32 = 20_000;
    const HOLD: u32 = 300_000;
    list.run.run(2_000_000);
    list.tap(F3);
    list.run.allow_raw_press();
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
    assert!(
        rows.iter()
            .filter(|r| r.0 <= EDGE.0)
            .all(|r| r.1.is_none() && r.2 == 0),
        "a second F3 within {} T-states of the release is never taken: {rows:?}",
        EDGE.0
    );
    assert!(
        rows.iter()
            .filter(|r| r.0 >= EDGE.1)
            .all(|r| r.1.is_some() && r.2 == 1),
        "a second F3 from {} T-states on is taken once: {rows:?}",
        EDGE.1
    );
    assert!(
        edge.is_some_and(|e| (EDGE.0..=EDGE.1).contains(&e)),
        "{edge:?}"
    );
}

/// After a second F3 the OS never reports, released, a Down is taken and its frame is the model's.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_second_f3_that_is_ignored_leaves_the_app_taking_the_next_key() {
    for gap in [0, 100_000, EDGE.0] {
        let mut list = Lister::on_rom(crate::list_real::PROP);
        list.open_on_rom();
        list.run.run(2_000_000);
        list.tap(F3);
        list.run.allow_raw_press();
        list.ui.press(Key::F3);
        list.log.push(Key::F3);
        list.run.run(gap);
        let before = list.run.scr_keys();
        list.run.m().press_key(F3);
        list.run.run(BOUND);
        assert_eq!(
            list.run.scr_keys(),
            before,
            "gap {gap}: the second F3 was held and not taken"
        );
        list.run.m().release_key(F3);
        list.run.run(600_000);
        list.ui.idle();
        let moved = list.key(DOWN, Key::Down);
        assert!(moved, "gap {gap}: Down moved the cursor");
        list.same_kind_position(&format!("gap {gap}, Down after an ignored F3"));
        assert_eq!(
            list.run.scr_keys(),
            before + 1,
            "gap {gap}: Down is the next key taken"
        );
        list.settle();
    }
}
