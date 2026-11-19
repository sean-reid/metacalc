use crate::fixture;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH, TEXT_WIDTH, text_width};

const NARROW: usize = 20;

fn widths() -> [(usize, usize); 4] {
    let indent = text_width("3.") + 2;
    [
        (TEXT_WIDTH, 1),
        (STATEMENT_WIDTH, GUTTER),
        (STATEMENT_WIDTH - indent, GUTTER + indent),
        (NARROW, GUTTER),
    ]
}

#[test]
fn the_fixture_has_a_statement_of_255_symbols_and_a_token_wider_than_the_narrow_width() {
    let library = decoded(&fixture::library());
    let all = statements(&library);
    assert!(all.iter().any(|(_, _, t)| t.len() == 255));
    assert!(all.iter().any(|(_, _, t)| t.iter().any(|s| s == "setvar")));
    assert!(text_width("setvar") > NARROW);
    assert!(all.iter().any(|(_, k, _)| *k < 0xFF && *k > 5));
}

#[test]
fn fm_rows_is_the_number_of_lines_the_model_lays_out_at_every_width() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let mut fm = Fm::new(&bytes);
    for (n, k, tokens) in &all {
        let out = fm.statement(*n, *k);
        assert_eq!(out.status(), 0, "statement {k} of {n}");
        assert_eq!(count(out), tokens.len(), "tokens of statement {k} of {n}");
        for (width, _) in widths() {
            let out = fm.rows(width);
            assert_eq!(out.status(), 0);
            assert_eq!(
                (out.bc >> 8) as usize,
                model_rows(tokens, width),
                "rows of statement {k} of assertion {n} at width {width}"
            );
        }
    }
}

/// The windows and first rows a screen can ask for: the whole screen, a statement scrolled up
/// so that its first lines are above the window, one that starts below the top, and a window
/// that cuts it at both ends.
fn cases(rows: usize) -> Vec<(i16, (u8, u8))> {
    let rows = rows as i16;
    vec![
        (0, (0, 8)),
        (1, (1, 7)),
        (-1, (0, 8)),
        (-(rows / 2), (0, 8)),
        (-(rows - 1).max(0), (1, 7)),
        (2, (3, 5)),
        (-2, (2, 4)),
        (3, (0, 3)),
        (0, (4, 4)),
        (-rows, (0, 8)),
    ]
}

#[test]
fn fm_draw_leaves_the_frame_the_model_draws_for_the_same_statement() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let mut fm = Fm::new(&bytes);
    let mut drawn = 0;
    for (n, k, tokens) in &all {
        assert_eq!(fm.statement(*n, *k).status(), 0);
        for (width, x) in widths() {
            let rows = model_rows(tokens, width);
            for (row, window) in cases(rows) {
                fm.clear();
                let out = fm.draw(x, row, width, window);
                assert_eq!(out.status(), 0);
                let want = model_frame(
                    tokens,
                    width,
                    x,
                    isize::from(row),
                    isize::from(window.0)..isize::from(window.1),
                );
                let what = format!(
                    "statement {k} of {n}, width {width}, x {x}, row {row}, window {window:?}"
                );
                same_frame(&what, &fm.frame(), &want);
                drawn += 1;
            }
        }
    }
    assert!(drawn > 1000, "{drawn} frames compared");
}
