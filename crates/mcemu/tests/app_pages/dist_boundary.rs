//! The Dist screen on a library packed for the edges of mcglyph's place: a row a pair exactly fills and
//! one a pixel over, six rows and seven, the title's count at 9, 10 and past 255, and the scroll from
//! the first row to the last and back.

use crate::dist_lib::{self, *};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;

fn line(row: &[usize]) -> usize {
    row.iter().sum::<usize>() + 3 * (row.len() - 1)
}

#[test]
fn a_row_a_pair_exactly_fills_and_a_row_a_pixel_over_are_the_models() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let fill = rows_of(&lib, index_of(&lib, FILL));
    assert!(fill[0].len() >= 5, "the row holds several pairs");
    assert_eq!(
        line(&fill[0]),
        94,
        "the last pair ends on column 94 exactly"
    );
    assert_eq!(fill.len(), 2, "the next pair starts the next row");
    let over = rows_of(&lib, index_of(&lib, OVER));
    assert_eq!(over.len(), 2);
    assert_eq!(over[1].len(), 1, "one pair does not fit and starts a row");
    assert_eq!(
        line(&over[0]) + 3 + over[1][0],
        95,
        "line + gap + width is one over"
    );
    for (label, rows) in [(FILL, fill), (OVER, over)] {
        let mut walker = dist_of(built, label);
        assert_eq!(app_rows(&mut walker), rows.len(), "{label}: rows");
        assert_eq!(
            walker.var16("DV_NPAIRS"),
            rows.iter().map(Vec::len).sum::<usize>()
        );
        leave(&mut walker);
    }
}

#[test]
fn six_rows_stay_put_and_seven_scroll_once() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let mut walker = dist_of(built, SIX);
    assert_eq!(rows_of(&lib, index_of(&lib, SIX)).len(), 6);
    assert_eq!(app_rows(&mut walker), 6);
    assert!(
        !press(&mut walker, DOWN, Key::Down),
        "Down stays on six rows"
    );
    assert!(!press(&mut walker, UP, Key::Up), "Up stays at the start");
    leave(&mut walker);
    walker.back();
    let mut walker = dist_of(built, SEVEN);
    assert_eq!(rows_of(&lib, index_of(&lib, SEVEN)).len(), 7);
    assert_eq!(app_rows(&mut walker), 7);
    assert!(
        press(&mut walker, DOWN, Key::Down),
        "Down moves once on seven"
    );
    assert_eq!(model_top(&walker), 1);
    assert!(!press(&mut walker, DOWN, Key::Down), "and then stays");
    assert!(press(&mut walker, UP, Key::Up));
    assert!(!press(&mut walker, UP, Key::Up));
    leave(&mut walker);
}

#[test]
fn the_titles_count_at_9_10_and_past_255_is_the_models() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    for (label, count) in [(NINE, 9), (TEN, 10), (SINGLE, 1), (ALL, 276)] {
        let i = index_of(&lib, label);
        assert_eq!(lib.dist(i).len(), count, "{label}: the model's pairs");
        let mut walker = dist_of(built, label);
        assert_eq!(walker.var16("DV_NPAIRS"), count, "{label}");
        leave(&mut walker);
        walker.back();
    }
}

#[test]
fn the_scroll_runs_to_the_last_row_and_back_with_every_frame_the_models() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let rows = rows_of(&lib, index_of(&lib, ALL)).len();
    assert!(rows > 14, "the assertion takes {rows} rows");
    let most = (0..lib.len())
        .map(|n| rows_of(&lib, n).len())
        .max()
        .unwrap();
    assert_eq!(rows, most, "it has the most rows of the library");
    let mut walker = dist_of(built, ALL);
    assert_eq!(app_rows(&mut walker), rows);
    let last = last_of(rows);
    for step in 1..=last {
        assert!(press(&mut walker, DOWN, Key::Down), "Down {step}");
        assert_eq!(model_top(&walker), step);
    }
    for _ in 0..3 {
        assert!(!press(&mut walker, DOWN, Key::Down), "Down past the end");
        assert_eq!(model_top(&walker), last);
    }
    for step in (0..last).rev() {
        assert!(press(&mut walker, UP, Key::Up), "Up to {step}");
        assert_eq!(model_top(&walker), step);
    }
    for _ in 0..3 {
        assert!(!press(&mut walker, UP, Key::Up), "Up past the start");
    }
    leave(&mut walker);
}

#[test]
fn the_assertion_the_scroll_costs_are_measured_on_holds_six_pairs_on_every_full_row() {
    let lib = lib_of(dist_lib::built());
    let rows = rows_of(&lib, index_of(&lib, SCROLL));
    assert_eq!(rows.len(), 16);
    for (i, row) in rows[..15].iter().enumerate() {
        assert!(row.len() >= PER_ROW, "row {i} holds {} pairs", row.len());
        assert!(line(row) <= 94);
    }
    println!(
        "the scroll assertion: {} rows, pairs a row {:?}",
        rows.len(),
        rows.iter().map(Vec::len).collect::<Vec<_>>()
    );
}
