use crate::fitcases::cases_library;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::{pack, same_frame};
use mcui::canvas::{Canvas, GUTTER, STATEMENT_WIDTH, TEXT_WIDTH};
use mcui::expr::Expr;
use mcui::fold::{Folds, Layout, Piece, fit_one, fitted, layout};
use std::collections::BTreeMap;

const ELLIPSIS_FLAG: u8 = 0x80;

impl Fm {
    /// fm_clip of the selected statement to `rows` lines at `width`; the status is in `af >> 8`
    /// and the lines the statement takes after it in `bc >> 8`.
    pub fn clip(&mut self, rows: usize, width: usize) -> Out {
        let regs = Regs {
            bc: (rows as u16) << 8,
            de: (width as u16) << 8,
            ..Regs::default()
        };
        self.rig.call("FM_CLIP", regs)
    }

    /// The token records of the selected statement: symbol, width, gap and flags.
    pub fn records(&mut self) -> Vec<[u8; 4]> {
        let at = u16::from_le_bytes(self.memory("FV_TOK", 2).try_into().unwrap());
        let count = usize::from(self.memory("FV_COUNT", 1)[0]);
        let page = self.rig.m.ram_page(SPARE_PAGE);
        let from = usize::from(at - SPARE_BASE);
        page[from..from + 4 * count]
            .chunks(4)
            .map(|r| r.try_into().unwrap())
            .collect()
    }

    /// Header byte 7 of the selected statement: bit 0 stale folds, bit 1 counted, bit 2 clipped.
    pub fn header_flags(&mut self) -> u8 {
        let at = u16::from_le_bytes(self.memory("FV_HDR", 2).try_into().unwrap());
        self.rig.m.ram_page(SPARE_PAGE)[usize::from(at - SPARE_BASE) + 7]
    }

    /// What the selected statement shows, in order: true for an ellipsis, false for a token. A
    /// hidden record has no gap.
    pub fn shown(&mut self) -> Vec<bool> {
        self.records()
            .iter()
            .filter(|r| r[2] != 0)
            .map(|r| r[3] & ELLIPSIS_FLAG != 0)
            .collect()
    }
}

pub fn index(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}")) as u16
}

pub fn kinds(laid: &Layout) -> Vec<bool> {
    laid.pieces.iter().map(|p| *p == Piece::Ellipsis).collect()
}

/// The frame the model draws for a laid out statement at the first text row.
pub fn frame_of(laid: &Layout) -> Vec<u8> {
    let mut canvas = Canvas::default();
    canvas.layout(laid, GUTTER, 0, 0..8);
    pack(&canvas.into_bitmap())
}

/// What the runs of one row count found.
#[derive(Default, Debug, Clone, Copy)]
pub struct Counts {
    pub runs: usize,
    pub clipped: usize,
    pub unchanged: usize,
    /// Clipped to the ellipsis alone, which is the model's n of 0.
    pub alone: usize,
    /// Clipped with the ellipsis alone on the last of several lines.
    pub own_line: usize,
    pub pieces_kept: usize,
}

/// What a pass over statements found, by the row count each run clipped to.
#[derive(Default, Debug)]
pub struct Tally {
    pub by_rows: BTreeMap<usize, Counts>,
}

impl Tally {
    /// The counts of every row count of `from` or more; row 0 clips every statement to the
    /// ellipsis alone, so the floors leave it out.
    pub fn sum_from(&self, from: usize) -> Counts {
        let mut sum = Counts::default();
        for c in self.by_rows.range(from..).map(|(_, c)| c) {
            sum.runs += c.runs;
            sum.clipped += c.clipped;
            sum.unchanged += c.unchanged;
            sum.alone += c.alone;
            sum.own_line += c.own_line;
            sum.pieces_kept += c.pieces_kept;
        }
        sum
    }
}

/// What fm_clip is called after: fm_rows, which counts the lines and applies the folds, or nothing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum After {
    Rows,
    Nothing,
}

/// Clips the selected statement in fm and compares with `want`, the model's clipped layout of
/// `plain`: the lines, what is shown and the frame. A statement that already fits keeps its
/// records.
pub fn check(
    fm: &mut Fm,
    what: &str,
    (plain, want): (&Layout, &Layout),
    (rows, width): (usize, usize),
    tally: &mut Tally,
) {
    check_after(fm, what, (plain, want), (rows, width), tally, After::Rows);
}

pub fn check_after(
    fm: &mut Fm,
    what: &str,
    (plain, want): (&Layout, &Layout),
    (rows, width): (usize, usize),
    tally: &mut Tally,
    after: After,
) {
    if after == After::Rows {
        fm.rows(width);
    }
    let before = fm.records();
    let out = fm.clip(rows, width);
    assert_eq!(out.status(), 0, "{what}: fm_clip");
    assert_eq!(count(out), want.rows(), "{what}: lines after the clip");
    let counted = fm.rows(width);
    assert_eq!(
        (counted.status(), count(counted)),
        (0, want.rows()),
        "{what}: fm_rows"
    );
    assert_eq!(fm.shown(), kinds(want), "{what}: what is shown");
    fm.clear();
    assert_eq!(fm.draw(GUTTER, 0, width, (0, 8)).status(), 0, "{what}");
    same_frame(what, &fm.frame(), &frame_of(want));
    let counts = tally.by_rows.entry(rows).or_default();
    counts.runs += 1;
    if plain.rows() <= rows {
        if after == After::Rows {
            assert_eq!(
                fm.records(),
                before,
                "{what}: a statement that fits is untouched"
            );
        }
        counts.unchanged += 1;
    } else {
        assert!(want.rows() <= rows.max(1), "{what}: the model's own clip");
        let last = want.pieces.len() - 1;
        counts.clipped += 1;
        counts.alone += usize::from(last == 0);
        counts.own_line +=
            usize::from(want.rows() > 1 && want.lines.last().is_some_and(|l| l.tokens.len() == 1));
        counts.pieces_kept += last;
    }
}

/// The widths of the sweeps: the screen's, the list's, two between and two too narrow for most
/// tokens to share a line, which leave a token wider than the row and no room for the ellipsis
/// after the first token.
pub const WIDTHS: [usize; 6] = [STATEMENT_WIDTH, TEXT_WIDTH, 60, 30, 20, 12];

/// Every statement of the library, loaded alone with no fold, clipped to each row count.
fn unfolded(bytes: &[u8], row_counts: &[usize]) -> Tally {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for (n, k, _) in statements(&library) {
        if !model.within(n) {
            continue;
        }
        let expr: Expr = model.expr(n, k);
        for &width in &WIDTHS {
            let plain = layout(&expr, &Folds::none(&expr), width);
            for &rows in row_counts {
                assert_eq!(fm.statement(n, k).status(), 0);
                let what =
                    format!("unfolded statement {k} of assertion {n}, {rows} rows, width {width}");
                let want = plain.clipped(width, rows);
                check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
            }
        }
    }
    tally
}

/// Every statement of the library with every third node folded by hand, from a start that differs
/// from one statement to the next, clipped to each row count: folds that fm_fit did not choose.
/// With `After::Nothing` the clip is the first call after the last fold.
fn folded_by_hand(bytes: &[u8], row_counts: &[usize], after: After) -> Tally {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for (n, k, _) in statements(&library) {
        if !model.within(n) {
            continue;
        }
        let expr: Expr = model.expr(n, k);
        let mut folds = Folds::none(&expr);
        for node in (usize::from(n % 3)..expr.len()).step_by(3) {
            folds.toggle(&expr, node);
        }
        for &width in &WIDTHS {
            let plain = layout(&expr, &folds, width);
            for &rows in row_counts {
                assert_eq!(fm.statement(n, k).status(), 0);
                assert_eq!(fm.nodes().status(), 0);
                for node in (usize::from(n % 3)..expr.len()).step_by(3) {
                    let out = fm.fold(node, 1);
                    assert_eq!(out.status(), 0);
                    assert_eq!(count(out) == 1, expr.foldable(node));
                }
                let what = format!(
                    "hand folded statement {k} of assertion {n}, {rows} rows, width {width}"
                );
                let want = plain.clipped(width, rows);
                let at = (rows, width);
                check_after(&mut fm, &what, (&plain, &want), at, &mut tally, after);
            }
        }
    }
    tally
}

/// Every conclusion of the library, fitted to each row count by fm_fit and then clipped, as the
/// list's preview does.
fn fitted_conclusions(bytes: &[u8], row_counts: &[usize], widths: &[usize]) -> Tally {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for n in 0..library.assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        for &width in widths {
            for &rows in row_counts {
                assert_eq!(fm.statement(n, 0xFF).status(), 0);
                let what = format!("conclusion of assertion {n}, {rows} rows, width {width}");
                let fit = fm.fit(rows, width, width);
                assert_eq!(fit.status(), 0, "{what}: fm_fit");
                let plain = layout(&expr, &fit_one(&expr, width, rows), width);
                let want = fitted(&expr, width, rows);
                assert_eq!(want, plain.clipped(width, rows));
                check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
            }
        }
    }
    tally
}

/// What a sweep must have reached, counted over the row counts of 1 or more: runs clipped, runs
/// left alone, runs cut to the ellipsis alone and runs that kept pieces.
pub struct Floors {
    pub clipped: usize,
    pub unchanged: usize,
    pub alone: usize,
    pub kept: usize,
}

/// Fails when the sweep did not reach every branch of the cut, for each of the row counts 1 to 3
/// it ran and not only in total.
pub fn floors(name: &str, t: &Tally, floors: Floors) {
    println!("{name}: {t:?}");
    let sum = t.sum_from(1);
    assert!(
        sum.clipped >= floors.clipped
            && sum.unchanged >= floors.unchanged
            && sum.alone >= floors.alone
            && sum.pieces_kept >= floors.kept,
        "{name}: {sum:?}"
    );
    for (rows, c) in t.by_rows.range(1..=3) {
        assert!(
            c.clipped > 0,
            "{name}: no statement clipped at {rows} rows: {c:?}"
        );
        assert!(
            c.unchanged > 0,
            "{name}: no statement kept at {rows} rows: {c:?}"
        );
    }
    for (rows, c) in &t.by_rows {
        assert_eq!(c.clipped + c.unchanged, c.runs, "{name} at {rows}");
    }
}

const ALL_ROWS: [usize; 4] = [0, 1, 2, 3];

#[test]
fn unfolded_fixture_statements_clip_as_the_model_does() {
    let t = unfolded(&fixture::nodes_library(), &ALL_ROWS);
    floors(
        "fixture unfolded",
        &t,
        Floors {
            clipped: 870,
            unchanged: 2550,
            alone: 270,
            kept: 4600,
        },
    );
}

#[test]
fn unfolded_cases_statements_clip_as_the_model_does() {
    let t = unfolded(&cases_library(), &ALL_ROWS);
    floors(
        "cases unfolded",
        &t,
        Floors {
            clipped: 600,
            unchanged: 550,
            alone: 20,
            kept: 5800,
        },
    );
}

#[test]
fn fitted_fixture_conclusions_clip_as_the_model_does() {
    let t = fitted_conclusions(&fixture::nodes_library(), &[1, 2, 3], &WIDTHS);
    floors(
        "fixture fitted",
        &t,
        Floors {
            clipped: 370,
            unchanged: 2800,
            alone: 270,
            kept: 430,
        },
    );
}

#[test]
fn fitted_cases_conclusions_clip_as_the_model_does() {
    let t = fitted_conclusions(&cases_library(), &[1, 2, 3], &WIDTHS);
    floors(
        "cases fitted",
        &t,
        Floors {
            clipped: 40,
            unchanged: 500,
            alone: 20,
            kept: 130,
        },
    );
}

#[test]
fn folded_fixture_statements_clip_as_the_model_does_after_rows() {
    let t = folded_by_hand(&fixture::nodes_library(), &ALL_ROWS, After::Rows);
    floors(
        "fixture folded",
        &t,
        Floors {
            clipped: 580,
            unchanged: 2850,
            alone: 270,
            kept: 1080,
        },
    );
}

#[test]
fn folded_cases_statements_clip_as_the_model_does_after_rows() {
    let t = folded_by_hand(&cases_library(), &ALL_ROWS, After::Rows);
    floors(
        "cases folded",
        &t,
        Floors {
            clipped: 280,
            unchanged: 880,
            alone: 20,
            kept: 1250,
        },
    );
}

#[test]
fn folded_fixture_statements_clip_as_the_model_does_straight_after_the_folds() {
    let t = folded_by_hand(&fixture::nodes_library(), &ALL_ROWS, After::Nothing);
    floors(
        "fixture folded, straight",
        &t,
        Floors {
            clipped: 580,
            unchanged: 2850,
            alone: 270,
            kept: 1080,
        },
    );
}

#[test]
fn folded_cases_statements_clip_as_the_model_does_straight_after_the_folds() {
    let t = folded_by_hand(&cases_library(), &ALL_ROWS, After::Nothing);
    floors(
        "cases folded, straight",
        &t,
        Floors {
            clipped: 280,
            unchanged: 880,
            alone: 20,
            kept: 1250,
        },
    );
}
