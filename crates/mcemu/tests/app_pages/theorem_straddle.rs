//! A hypothesis the window starts inside of: its number is drawn only when its first row is in the window,
//! so a hypothesis that starts at the row just above the window, or higher, has none.

use crate::app::*;
use crate::shellrig::*;
use crate::theorem_keys::{cursor, index_of};
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, text_width};
use mcui::{Key, State};

fn scroll_of(walker: &Walker) -> usize {
    match walker.list.ui.state() {
        State::Theorem { scroll, .. } => scroll,
        other => panic!("{other:?}"),
    }
}

/// Two assertions of hypotheses that are one row each but the third, which takes two rows: `edge0` has four
/// more after it, so the window at the conclusion starts one row into the third, and `edge1` has three, so
/// the window starts at its first row.
fn edges() -> Vec<u8> {
    let mut src = String::from("$c WWWWWWWWWW $.\nwedge $a wff WWWWWWWWWW WWWWWWWWWW $.\n");
    for (name, after) in [("edge0", 4), ("edge1", 3)] {
        src += "${\n";
        for i in 0..2 {
            src += &format!("{name}.{i} $e |- ph $.\n");
        }
        src += &format!("{name}.2 $e |- WWWWWWWWWW WWWWWWWWWW $.\n");
        for i in 0..after {
            src += &format!("{name}.{} $e |- ph $.\n", 3 + i);
        }
        src += &format!("{name} $a |- ph $.\n$}}\n");
    }
    crate::fixture::custom(&src)
}

/// Whether any pixel is set in the gutter's columns on the window's row `row`.
fn number_drawn(frame: &[u8], row: usize) -> bool {
    let width = text_width("3.");
    (8 * row..8 * row + 8)
        .any(|y| (GUTTER..GUTTER + width).any(|x| frame[y * 12 + x / 8] & (0x80 >> (x % 8)) != 0))
}

#[test]
fn a_hypothesis_starting_one_row_above_the_window_has_no_number_and_one_at_its_first_row_has() {
    let built = around("theorem-edges", &edges());
    for (label, after, row, drawn) in [("edge0", 4usize, 0isize, false), ("edge1", 3, 1, true)] {
        let n = index_of(built, label);
        let mut walker = Walker::at(built, None, 0, n);
        assert_eq!(cursor(&walker), (3 + after, 0), "{label}: the conclusion");
        let (starts, total) = walker.mirror().starts();
        let scroll = scroll_of(&walker);
        assert_eq!(
            starts[3] - starts[2],
            2,
            "{label}: the third takes two rows"
        );
        assert_eq!(scroll, total - 6, "{label}: the view is at the bottom");
        let got = starts[2] as isize + 1 - scroll as isize;
        println!("{label}: {total} rows, scroll {scroll}, TV_ROW of the third hypothesis {got}");
        assert_eq!(got, row, "{label}: TV_ROW");
        let frame = walker.list.run.frame();
        same_frame(label, &frame, &walker.list.shown());
        assert_eq!(
            number_drawn(&frame, 1),
            drawn,
            "{label}: the number at row 1"
        );
    }
}

/// A hypothesis of 251 tokens, each too wide to share a row with another and none a node to fold, then four of
/// one row and a conclusion: the first takes 251 rows and the window at the conclusion starts 250 rows in, so
/// its last row is the window's first.
fn tall() -> Vec<u8> {
    let tokens = ["WWWWWWWWWW"; 251].join(" ");
    let mut src =
        format!("$c WWWWWWWWWW $.\nwtall $a wff {tokens} $.\n${{\ntall.0 $e |- {tokens} $.\n");
    for i in 1..5 {
        src += &format!("tall.{i} $e |- ph $.\n");
    }
    src += "tall $a |- ph $.\n$}\n";
    crate::fixture::custom(&src)
}

#[test]
fn a_hypothesis_of_251_rows_the_window_starts_250_rows_inside_has_no_number() {
    let built = around("theorem-tall", &tall());
    let n = index_of(built, "tall");
    let mut walker = Walker::at(built, None, 0, n);
    assert_eq!(cursor(&walker), (5, 0), "the conclusion");
    let (starts, total) = walker.mirror().starts();
    let scroll = scroll_of(&walker);
    let row = starts[0] as isize + 1 - scroll as isize;
    println!("{total} rows, scroll {scroll}, TV_ROW of the first hypothesis {row}");
    assert_eq!(
        scroll,
        total - 6,
        "the view follows the conclusion to the bottom"
    );
    assert!(
        (-255..=-249).contains(&row),
        "the row's low byte is 1 to 7: {row}"
    );
    assert!(walker.press(LEFT, Key::Left));
    assert!(walker.press(RIGHT, Key::Right));
    assert_eq!(scroll_of(&walker), scroll, "back at the conclusion");
}
