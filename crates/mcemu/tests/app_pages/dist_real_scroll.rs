//! The Dist screen on the reference OS through the real shell's keys, on the packed library: a scroll that
//! moves, to the end and back, and the keys that stay where it could move.

use crate::dist_keys::{FIRST, SCR_STAY, key_none};
use crate::dist_lib::{self, ALL};
use crate::dist_rig::*;
use crate::dist_screens::{IGNORED, ignores};
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;

/// The packed library's list on the reference OS with ALL's Dist screen open, and the model beside it.
fn open_all() -> (Walker, usize) {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let n = index_of(&lib, ALL);
    let most = (0..lib.len())
        .map(|n| rows_of(&lib, n).len())
        .max()
        .unwrap();
    let rows = rows_of(&lib, n).len();
    assert_eq!(rows, most, "ALL has the most rows of the library");
    let mut list = Lister::on_rom_of(built, 0);
    list.open_on_rom();
    let mut walker = Walker::new(list, 0, built);
    walker.gap = 600_000;
    walker.mirrored = false;
    walker.goto(n);
    walker.hold_preview();
    walker.open();
    open_dist(&mut walker);
    assert_eq!(app_rows(&mut walker), rows);
    (walker, rows)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn through_the_real_shell_the_scroll_moves_to_the_end_and_back_with_every_frame_the_models() {
    let (mut walker, rows) = open_all();
    let last = last_of(rows);
    assert!(last >= 10, "{last} first rows");
    let mut moved = 0;
    for step in 1..=last {
        assert!(press(&mut walker, DOWN, Key::Down), "Down to {step}");
        moved += 1;
        assert_eq!(model_top(&walker), step);
    }
    assert!(
        !press(&mut walker, DOWN, Key::Down),
        "Down at the end stays"
    );
    assert_eq!(model_top(&walker), last);
    for step in (0..last).rev() {
        assert!(press(&mut walker, UP, Key::Up), "Up to {step}");
        moved += 1;
        assert_eq!(model_top(&walker), step);
    }
    assert!(!press(&mut walker, UP, Key::Up), "Up at the start stays");
    assert_eq!(moved, 2 * last, "the frames that moved");
    println!("ROM shell: {rows} rows, {moved} frames moved");
    leave(&mut walker);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn through_the_real_shell_every_other_key_stays_at_a_first_row_a_move_could_leave() {
    let (mut walker, rows) = open_all();
    assert!(0 < FIRST && FIRST < last_of(rows));
    for step in 1..=FIRST {
        assert!(press(&mut walker, DOWN, Key::Down), "Down {step}");
    }
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
    assert_eq!(frame_of(&mut walker), frame);
    assert_eq!(self::scratch(&mut walker), scratch);
    assert_eq!(walker.list.run.frame(), picture);
    assert_eq!(model_top(&walker), FIRST);
    assert!(press(&mut walker, DOWN, Key::Down), "Down still moves");
    leave(&mut walker);
}
