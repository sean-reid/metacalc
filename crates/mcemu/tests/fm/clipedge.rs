use crate::clip::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};

/// Statements of different shapes: short, nested, braced, many operators, quantifiers, words in
/// place of symbols and a hypothesis.
const SHAPES: [(&str, u8); 8] = [
    ("ax-1", 0xFF),
    ("ax-2", 0xFF),
    ("df-clab", 0xFF),
    ("many", 0xFF),
    ("ax-class", 0xFF),
    ("wide", 0xFF),
    ("alnex", 0xFF),
    ("syl3", 0),
];

/// The widest the statement is when it takes one line, which is the width where it just fits.
fn natural(expr: &Expr) -> usize {
    let laid = layout(expr, &Folds::none(expr), 255);
    assert_eq!(laid.rows(), 1, "the statement fits one line of 255 pixels");
    laid.lines[0].width
}

/// The most pieces the model keeps for each width from 1 to 255, at `rows` lines.
fn kept_by_width(expr: &Expr, rows: usize) -> Vec<usize> {
    (1..=255)
        .map(|width| {
            let plain = layout(expr, &Folds::none(expr), width);
            plain.clipped(width, rows).pieces.len()
        })
        .collect()
}

/// The widths around each width where the pieces kept change, which is where the ellipsis just
/// fits after the last kept token, and the width one pixel under it.
fn around_changes(kept: &[usize]) -> Vec<usize> {
    let mut widths = Vec::new();
    for at in 1..kept.len() {
        if kept[at] != kept[at - 1] {
            let width = at + 1;
            widths.extend(
                [width - 1, width, width + 1]
                    .into_iter()
                    .filter(|&w| w <= 255),
            );
        }
    }
    widths.sort_unstable();
    widths.dedup();
    widths
}

fn sweep(bytes: &[u8], statements: &[(&str, u8)], row_counts: &[usize]) -> Tally {
    let model = Model::new(bytes);
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for &(label, k) in statements {
        let n = index(bytes, label);
        let expr = model.expr(n, k);
        let nat = natural(&expr);
        for &rows in row_counts {
            let kept = kept_by_width(&expr, rows);
            let changes = around_changes(&kept);
            assert!(changes.len() >= 3, "{label} at {rows} rows has no boundary");
            let mut widths: Vec<usize> = (nat - 2..=nat + 2).collect();
            widths.extend(changes);
            widths.sort_unstable();
            widths.dedup();
            for width in widths {
                let plain = layout(&expr, &Folds::none(&expr), width);
                let want = plain.clipped(width, rows);
                assert_eq!(fm.statement(n, k).status(), 0);
                let what = format!("{label}, {rows} rows, width {width} (natural {nat})");
                check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
            }
        }
    }
    tally
}

#[test]
fn clips_at_the_widths_where_the_ellipsis_just_fits_and_one_pixel_under() {
    let tally = sweep(&fixture::nodes_library(), &SHAPES, &[1, 2, 3]);
    floors(
        "boundary widths",
        &tally,
        Floors {
            clipped: 640,
            unchanged: 120,
            alone: 6,
            kept: 5200,
        },
    );
    for rows in [2, 3] {
        assert!(tally.by_rows[&rows].own_line > 0, "{rows} rows: {tally:?}");
    }
}

#[test]
fn clips_a_statement_at_two_pixels_either_side_of_its_natural_width() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let mut tally = Tally::default();
    for (label, k) in SHAPES {
        let n = index(&bytes, label);
        let expr = model.expr(n, k);
        let nat = natural(&expr);
        for rows in [1, 2] {
            for width in nat - 2..=nat + 2 {
                let plain = layout(&expr, &Folds::none(&expr), width);
                assert_eq!(plain.rows() == 1, width >= nat, "{label} at {width}");
                let want = plain.clipped(width, rows);
                assert_eq!(fm.statement(n, k).status(), 0);
                let what = format!("{label}, {rows} rows, width {width} (natural {nat})");
                check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
            }
        }
    }
    let one = tally.by_rows[&1];
    assert!(
        one.clipped >= SHAPES.len() && one.unchanged >= 3 * SHAPES.len(),
        "{one:?}"
    );
}
