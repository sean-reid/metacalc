use crate::clip::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcui::expr::Expr;
use mcui::fold::{Folds, Layout, Piece, layout};

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

/// The first conclusion of three pieces or more for which some width below its natural one cuts
/// only its last piece, which keeps more than the one token a cut to the first piece keeps.
fn drops_only_its_last(bytes: &[u8]) -> (String, u8) {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    for (n, assertion) in library.assertions.iter().enumerate() {
        let n = n as u16;
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        let plain = layout(&expr, &Folds::none(&expr), 255);
        if plain.rows() != 1 {
            continue;
        }
        let nat = natural(&expr);
        for width in (1..nat).rev() {
            let narrow = layout(&expr, &Folds::none(&expr), width);
            let want = narrow.clipped(width, 1);
            if narrow.rows() > 1
                && want.pieces.len() == plain.pieces.len()
                && plain.pieces.len() >= 3
            {
                let mut one = narrow.pieces[..1].to_vec();
                one.push(Piece::Ellipsis);
                assert_ne!(
                    want.pieces, one,
                    "{}: the cut keeps one token",
                    assertion.label
                );
                return (assertion.label.clone(), 0xFF);
            }
        }
    }
    panic!("no statement has a width that cuts only its last piece");
}

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

/// What the cuts of a sweep kept: the cuts that keep one token before the ellipsis and the cuts that drop only the last piece.
#[derive(Default, Debug)]
struct Cuts {
    one_kept: usize,
    last_dropped: usize,
}

impl Cuts {
    fn note(&mut self, plain: &Layout, want: &Layout) {
        let kept = want.pieces.len() - 1;
        self.one_kept += usize::from(kept == 1);
        self.last_dropped += usize::from(kept > 1 && kept + 1 == plain.pieces.len());
    }
}

fn sweep(bytes: &[u8], statements: &[(&str, u8)], row_counts: &[usize]) -> (Tally, Cuts) {
    let model = Model::new(bytes);
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    let mut cuts = Cuts::default();
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
                if plain.rows() > rows {
                    cuts.note(&plain, &want);
                }
            }
        }
    }
    (tally, cuts)
}

#[test]
fn clips_at_the_widths_where_the_ellipsis_just_fits_and_one_pixel_under() {
    let (tally, cuts) = sweep(&fixture::nodes_library(), &SHAPES, &[1, 2, 3]);
    println!("{tally:?} {cuts:?}");
    assert!(
        cuts.one_kept > 0,
        "no cut keeps exactly one token: {cuts:?}"
    );

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

#[test]
fn clips_every_width_of_a_statement_whose_cut_drops_only_the_last_token() {
    let bytes = fixture::nodes_library();
    let (label, k) = drops_only_its_last(&bytes);
    let (model, n) = (Model::new(&bytes), index(&bytes, &label));
    let mut fm = Fm::new(&bytes);
    let expr = model.expr(n, k);
    let pieces = layout(&expr, &Folds::none(&expr), 255).pieces.len();
    assert!(pieces >= 3, "{label} has {pieces} pieces");
    let mut tally = Tally::default();
    let mut cuts = Cuts::default();
    for width in 1..=natural(&expr) {
        let plain = layout(&expr, &Folds::none(&expr), width);
        let want = plain.clipped(width, 1);
        assert_eq!(fm.statement(n, k).status(), 0);
        let what = format!("{label}, 1 row, width {width}");
        check(&mut fm, &what, (&plain, &want), (1, width), &mut tally);
        if plain.rows() > 1 {
            cuts.note(&plain, &want);
        }
    }
    println!("{label}: {tally:?} {cuts:?}");
    assert!(
        cuts.last_dropped > 0,
        "{label}: no cut drops only the last token: {cuts:?}"
    );
}
