use crate::fitcases::cases_library;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::{pack, same_frame};
use mcui::canvas::{Canvas, GUTTER, STATEMENT_WIDTH, TEXT_WIDTH};
use mcui::expr::Expr;
use mcui::fold::{Folds, Layout, Piece, fit_one, fitted, layout};

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

pub fn kinds(laid: &Layout) -> Vec<bool> {
    laid.pieces.iter().map(|p| *p == Piece::Ellipsis).collect()
}

/// The frame the model draws for a laid out statement at the first text row.
pub fn frame_of(laid: &Layout) -> Vec<u8> {
    let mut canvas = Canvas::default();
    canvas.layout(laid, GUTTER, 0, 0..8);
    pack(&canvas.into_bitmap())
}

/// What a pass over statements found.
#[derive(Default, Debug)]
pub struct Tally {
    pub runs: usize,
    pub clipped: usize,
    pub unchanged: usize,
    /// Clipped to the ellipsis alone, which is the model's n of 0.
    pub alone: usize,
    pub pieces_kept: usize,
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
    fm.rows(width);
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
    tally.runs += 1;
    if plain.rows() <= rows {
        assert_eq!(
            fm.records(),
            before,
            "{what}: a statement that fits is untouched"
        );
        tally.unchanged += 1;
    } else {
        assert!(want.rows() <= rows.max(1), "{what}: the model's own clip");
        tally.clipped += 1;
        tally.alone += usize::from(want.pieces.len() == 1);
        tally.pieces_kept += want.pieces.len() - 1;
    }
}

pub const WIDTHS: [usize; 4] = [STATEMENT_WIDTH, TEXT_WIDTH, 60, 30];

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
fn folded_by_hand(bytes: &[u8], row_counts: &[usize]) -> Tally {
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
                check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
            }
        }
    }
    tally
}

/// Every conclusion of the library, fitted to each row count by fm_fit and then clipped, as the
/// list's preview does.
fn fitted_conclusions(bytes: &[u8], row_counts: &[usize]) -> Tally {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for n in 0..library.assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        for &width in &WIDTHS {
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

/// Fails when the sweep did not reach every branch of the cut.
fn floors(name: &str, t: &Tally, (clipped, unchanged, alone): (usize, usize, usize)) {
    println!("{name}: {t:?}");
    assert!(
        t.clipped >= clipped && t.unchanged >= unchanged && t.alone >= alone,
        "{name}: {t:?}"
    );
    assert_eq!(t.clipped + t.unchanged, t.runs, "{name}");
}

#[test]
fn unfolded_fixture_statements_clip_as_the_model_does() {
    let t = unfolded(&fixture::nodes_library(), &[0, 1, 2, 3]);
    floors("fixture unfolded", &t, (1000, 1900, 700));
}

#[test]
fn unfolded_cases_statements_clip_as_the_model_does() {
    let t = unfolded(&cases_library(), &[0, 1, 2, 3]);
    floors("cases unfolded", &t, (550, 400, 230));
}

#[test]
fn fitted_fixture_conclusions_clip_as_the_model_does() {
    let t = fitted_conclusions(&fixture::nodes_library(), &[1, 2, 3]);
    floors("fixture fitted", &t, (60, 2000, 0));
}

#[test]
fn fitted_cases_conclusions_clip_as_the_model_does() {
    let t = fitted_conclusions(&cases_library(), &[1, 2, 3]);
    floors("cases fitted", &t, (10, 300, 0));
}

#[test]
fn folded_fixture_statements_clip_as_the_model_does() {
    let t = folded_by_hand(&fixture::nodes_library(), &[1, 3]);
    floors("fixture folded", &t, (0, 0, 0));
}

#[test]
fn folded_cases_statements_clip_as_the_model_does() {
    let t = folded_by_hand(&cases_library(), &[0, 1, 2, 3]);
    floors("cases folded", &t, (0, 0, 0));
}

/// T-state ceilings per library and width, a little above what each call took: fm_clip, fm_fit to
/// one row with fm_clip for a preview, the median and the worst of each, and the worst fm_draw of
/// the preview.
struct Ceilings {
    name: &'static str,
    width: usize,
    clip: (u32, u32),
    preview: (u32, u32),
    draw: u32,
}

const NARROW: usize = 30;

const CEILINGS: [Ceilings; 4] = [
    Ceilings {
        name: "prop",
        width: TEXT_WIDTH,
        clip: (1200, 1200),
        preview: (49000, 305000),
        draw: 30000,
    },
    Ceilings {
        name: "pred",
        width: TEXT_WIDTH,
        clip: (1200, 1200),
        preview: (51200, 305000),
        draw: 31300,
    },
    Ceilings {
        name: "ring",
        width: TEXT_WIDTH,
        clip: (1200, 1200),
        preview: (55300, 1103000),
        draw: 41500,
    },
    Ceilings {
        name: "ring",
        width: NARROW,
        clip: (1200, 2800),
        preview: (89400, 1233000),
        draw: 24500,
    },
];

fn median_and_worst(mut all: Vec<u32>) -> (u32, u32) {
    all.sort_unstable();
    (all[all.len() / 2], *all.last().unwrap())
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_preview_as_the_model_does_within_the_budget() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for ceiling in CEILINGS {
        let (name, width) = (ceiling.name, ceiling.width);
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let (model, library) = (Model::new(&bytes), decoded(&bytes));
        let mut fm = Fm::new(&bytes);
        let zero = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        let (mut clip, mut preview, mut draw, mut cut_draw) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut tally = Tally::default();
        for n in 0..library.assertions.len() as u16 {
            if !model.within(n) {
                continue;
            }
            let expr = model.expr(n, 0xFF);
            let want = fitted(&expr, width, 1);
            let label = &library.assertions[usize::from(n)].label;
            let what = format!("{name} at {width}: conclusion of {label}");
            assert_eq!(fm.statement(n, 0xFF).status(), 0, "{what}");
            let fit = fm.fit(1, width, width);
            assert_eq!(fit.status(), 0, "{what}: fm_fit");
            let plain = layout(&expr, &fit_one(&expr, width, 1), width);
            let out = fm.clip(1, width);
            assert_eq!((out.status(), count(out)), (0, want.rows()), "{what}");
            assert_eq!(fm.shown(), kinds(&want), "{what}: what is shown");
            fm.clear();
            let drawn = fm.draw(GUTTER, 0, width, (0, 8));
            assert_eq!(drawn.status(), 0, "{what}");
            same_frame(&what, &fm.frame(), &frame_of(&want));
            clip.push(out.tstates - zero);
            preview.push(fit.tstates + out.tstates - 2 * zero);
            draw.push(drawn.tstates - zero);
            tally.runs += 1;
            if plain.rows() > 1 {
                tally.clipped += 1;
                tally.alone += usize::from(want.pieces.len() == 1);
                cut_draw.push(drawn.tstates - zero);
            } else {
                tally.unchanged += 1;
            }
        }
        if width == NARROW {
            assert!(tally.clipped >= 20, "{name}: {tally:?}");
        }
        let (clip, preview, draw) = (
            median_and_worst(clip),
            median_and_worst(preview),
            median_and_worst(draw),
        );
        println!(
            "{name} at {width}: {tally:?}\n\
             {name} at {width}: fm_clip median {}, worst {}\n\
             {name} at {width}: fm_fit to one row then fm_clip median {}, worst {}\n\
             {name} at {width}: fm_draw of the preview median {}, worst {}; of the {} clipped ones worst {:?}",
            clip.0,
            clip.1,
            preview.0,
            preview.1,
            draw.0,
            draw.1,
            cut_draw.len(),
            cut_draw.iter().max()
        );
        for (what, got, ceiling) in [
            ("fm_clip median", clip.0, ceiling.clip.0),
            ("fm_clip worst", clip.1, ceiling.clip.1),
            ("preview median", preview.0, ceiling.preview.0),
            ("preview worst", preview.1, ceiling.preview.1),
            ("fm_draw worst", draw.1, ceiling.draw),
        ] {
            assert!(
                got <= ceiling,
                "{name} at {width}: {what} took {got}, ceiling {ceiling}"
            );
        }
    }
}
