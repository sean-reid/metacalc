//! A hypothesis the window starts inside of: its number is drawn only when its first row is in the window,
//! so a hypothesis that starts at the row just above the window, or higher, has none.

use crate::app::*;
use crate::shellrig::*;
use crate::theorem_keys::{cursor, index_of};
use crate::theorem_return::next_node;
use crate::theorem_rig::*;
use mcui::canvas::{STATEMENT_WIDTH, text_width};
use mcui::fold::{Folds, layout};
use mcui::{Key, State};

/// Seven hypotheses and a conclusion, the hypotheses wide enough to take three rows or more unfolded and
/// one row each when the fit folds them.
fn straddling() -> Vec<u8> {
    let wide = "( ( ph -> ps ) -> ( ( ch -> th ) -> ( ( ta -> ph ) -> ( ps -> ( ch -> ( th -> ( ta -> ph ) ) ) ) ) ) )";
    let mut src = String::from("${\n");
    for i in 0..7 {
        src += &format!("st.{i} $e |- {wide} $.\n");
    }
    src += "st $a |- ph $.\n$}\n";
    crate::fixture::custom(&src)
}

fn scroll_of(walker: &Walker) -> usize {
    match walker.list.ui.state() {
        State::Theorem { scroll, .. } => scroll,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_hypothesis_the_window_starts_inside_has_no_number_and_the_rows_the_model_draws() {
    let built = around("theorem-straddle", &straddling());
    let n = index_of(built, "st");
    let lib = lib_of(built);
    let statement = lib.statement(n);
    let indent = text_width("7.") + 2;
    let unfolded = layout(
        statement.at(0),
        &Folds::none(statement.at(0)),
        STATEMENT_WIDTH - indent,
    )
    .rows();
    assert!(unfolded >= 3, "a hypothesis takes {unfolded} rows unfolded");
    let mut above: Vec<isize> = Vec::new();
    for k in [2usize, 3] {
        let mut walker = Walker::at(built, None, 0, n);
        for _ in 0..7 - k {
            assert!(walker.press(LEFT, Key::Left));
        }
        assert_eq!(cursor(&walker), (k, 0));
        while next_node(&mut walker) {}
        assert_eq!(
            cursor(&walker),
            (k, 0),
            "every node of hypothesis {k} is unfolded"
        );
        for _ in 0..7 - k {
            assert!(walker.press(RIGHT, Key::Right));
        }
        assert_eq!(cursor(&walker).0, 7, "the conclusion");
        let (starts, total) = walker.mirror().starts();
        let scroll = scroll_of(&walker);
        assert_eq!(
            scroll,
            total - 6,
            "the view follows the conclusion to the bottom"
        );
        assert!(
            starts[k] < scroll && starts[k + 1] > scroll,
            "hypothesis {k} straddles the top"
        );
        above.push(starts[k] as isize + 1 - scroll as isize);
    }
    println!("TV_ROW of the hypotheses that start above the window: {above:?}");
    assert!(
        above.iter().all(|&r| r <= 0) && above.iter().any(|&r| r < 0),
        "a first row above the window's row 0: {above:?}"
    );
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
