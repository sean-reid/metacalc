use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::{pack, same_frame};
use mcpack::library::{Hypothesis, Library};
use mcui::canvas::{CONTENT_ROWS, Canvas, GUTTER, STATEMENT_WIDTH, TEXT_WIDTH, text_width};
use mcui::expr::Expr;
use mcui::fold::{Fit, Folds, fit, fit_one, layout};
use mcui::{Installed, Key, State, Status, Statuses, Ui};

impl Fm {
    /// fm_fit for `rows` lines, `hyp` pixels for a hypothesis and `concl` for the conclusion; the
    /// status is in `af >> 8` and the total lines in `hl`.
    pub fn fit(&mut self, rows: usize, hyp: usize, concl: usize) -> Out {
        let regs = Regs {
            bc: (rows as u16) << 8,
            de: ((hyp as u16) << 8) | concl as u16,
            ..Regs::default()
        };
        self.rig.call("FM_FIT", regs)
    }

    /// The fold flags of every node of the selected statement, read from the node table.
    pub fn flags(&mut self) -> Vec<bool> {
        let header = self.header();
        let base = usize::from(header.nodes_at);
        let plane = self.memory("FM_FOLDS", base + usize::from(header.nodes));
        plane[base..].iter().map(|&b| b != 0).collect()
    }
}

/// How many pixels the theorem screen gives a hypothesis and the conclusion out of `width`, and
/// where each starts: Theorem::fitted and Theorem::indent in crates/mcui/src/screens/theorem.rs.
pub fn widths_of(hyps: usize, width: usize) -> (usize, usize) {
    let indent = match hyps {
        0 => 0,
        _ => text_width(&format!("{hyps}.")) + 2,
    };
    (width - indent, width)
}

/// The statements of one assertion as the theorem screen fits them.
pub struct Oracle {
    pub exprs: Vec<Expr>,
    pub widths: Vec<usize>,
    pub xs: Vec<usize>,
    pub folds: Vec<Folds>,
}

pub fn oracle(model: &Model, n: u16, hyps: usize, rows: usize, width: usize) -> Oracle {
    let mut exprs: Vec<Expr> = (0..hyps).map(|k| model.expr(n, k as u8)).collect();
    exprs.push(model.expr(n, 0xFF));
    let (hyp, concl) = widths_of(hyps, width);
    let widths: Vec<usize> = (0..=hyps)
        .map(|s| if s == hyps { concl } else { hyp })
        .collect();
    let xs = widths.iter().map(|w| GUTTER + width - w).collect();
    let items: Vec<Fit> = exprs
        .iter()
        .zip(&widths)
        .map(|(expr, &width)| Fit { expr, width })
        .collect();
    let mut folds = fit(&items, rows);
    if items.len() > rows {
        let last = items.len() - 1;
        folds[last] = fit_one(items[last].expr, items[last].width, rows);
    }
    Oracle {
        exprs,
        widths,
        xs,
        folds,
    }
}

impl Oracle {
    pub fn rows_of(&self, s: usize) -> usize {
        layout(&self.exprs[s], &self.folds[s], self.widths[s]).rows()
    }

    pub fn total(&self) -> usize {
        (0..self.exprs.len()).map(|s| self.rows_of(s)).sum()
    }

    /// The first text row of each statement with the view at the top.
    pub fn starts(&self) -> Vec<usize> {
        let mut row = 1;
        (0..self.exprs.len())
            .map(|s| {
                let at = row;
                row += self.rows_of(s);
                at
            })
            .collect()
    }

    /// The statements drawn as the screen draws them, less the numbers and the marker, with the
    /// first line of the first statement on text row 1 - `scroll`.
    pub fn bitmap(&self, scroll: usize) -> mcglyph::Bitmap {
        let mut canvas = Canvas::default();
        for (s, at) in self.starts().into_iter().enumerate() {
            let laid = layout(&self.exprs[s], &self.folds[s], self.widths[s]);
            let row = at as isize - scroll as isize;
            canvas.layout(&laid, self.xs[s], row, 1..1 + CONTENT_ROWS as isize);
        }
        canvas.into_bitmap()
    }

    /// fm drawing each statement where the screen does, on a cleared canvas.
    pub fn drawn_by_fm(&self, fm: &mut Fm, scroll: usize) -> Vec<u8> {
        fm.clear();
        let last = self.exprs.len() - 1;
        for (s, at) in self.starts().into_iter().enumerate() {
            assert_eq!(
                fm.select(if s == last { 0xFF } else { s as u8 }).status(),
                0
            );
            let row = at as i16 - scroll as i16;
            let window = (1, 1 + CONTENT_ROWS as u8);
            assert_eq!(fm.draw(self.xs[s], row, self.widths[s], window).status(), 0);
        }
        fm.frame()
    }
}

/// How the statements of an assertion came out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// fm_assertion or fm_fit refused it and the model says too large.
    TooLarge,
    /// Nothing was folded.
    Unfolded,
    /// Folded to fit the rows.
    Fitted,
    /// More statements than rows, each on one line and the conclusion refitted.
    Alone,
}

/// What a pass over assertions found.
#[derive(Default, Debug)]
pub struct Tally {
    pub assertions: usize,
    pub too_large: usize,
    pub refused_at_load: usize,
    pub unfolded: usize,
    pub fitted: usize,
    /// Assertions where the fit-alone fallback runs.
    pub alone: usize,
    /// Assertions with more statements than rows, whose conclusion is refitted.
    pub refit: usize,
    /// Assertions where a statement at one line was left alone while a later depth folded another.
    pub skip_mattered: usize,
    pub folded_nodes: usize,
    pub frames: usize,
}

pub fn essentials(library: &Library, n: u16) -> usize {
    library.assertions[usize::from(n)]
        .hypotheses
        .iter()
        .filter(|h| matches!(h, Hypothesis::Essential(_)))
        .count()
}

/// How much one comparison checks beyond the folds and the total.
#[derive(Clone, Copy)]
pub struct Check {
    /// fm_folded for every node, not the node table's plane.
    pub bits: bool,
    pub frame: bool,
}

pub const FULL: Check = Check {
    bits: true,
    frame: true,
};
pub const FLAGS: Check = Check {
    bits: false,
    frame: false,
};

/// Loads assertion `n`, fits it in fm and in the model and compares the folds of every node, the
/// rows every statement takes, the total and the frame.
pub fn compare(
    fm: &mut Fm,
    model: &Model,
    library: &Library,
    (n, rows, width): (u16, usize, usize),
    check: Check,
    tally: &mut Tally,
) -> Outcome {
    let hyps = essentials(library, n);
    let what = format!(
        "assertion {n} ({}), {rows} rows, width {width}",
        library.assertions[usize::from(n)].label
    );
    tally.assertions += 1;
    let within = model.within(n);
    let loaded = fm.assertion(n);
    if loaded.status() == code("FM_ERR_LIMIT") {
        assert!(!within, "{what}: fm_assertion refuses what the model holds");
        tally.too_large += 1;
        tally.refused_at_load += 1;
        return Outcome::TooLarge;
    }
    assert_eq!(loaded.status(), 0, "{what}: fm_assertion");
    assert_eq!(count(loaded), hyps + 1, "{what}: statements");
    let (hyp, concl) = widths_of(hyps, width);
    let out = fm.fit(rows, hyp, concl);
    if !within {
        assert_eq!(out.status(), code("FM_ERR_LIMIT"), "{what}: too large");
        tally.too_large += 1;
        return Outcome::TooLarge;
    }
    assert_eq!(out.status(), 0, "{what}: fm_fit");
    let want = oracle(model, n, hyps, rows, width);
    assert_eq!(usize::from(out.hl), want.total(), "{what}: total rows");
    let last = hyps;
    let selected = fm.rows(concl);
    assert_eq!(
        (selected.status(), count(selected)),
        (0, want.rows_of(last)),
        "{what}: the conclusion is selected"
    );
    let mut folded_statements = 0;
    for s in 0..=hyps {
        let k = if s == last { 0xFF } else { s as u8 };
        assert_eq!(fm.select(k).status(), 0, "{what}: select {s}");
        let expr = &want.exprs[s];
        let flags: Vec<bool> = if check.bits {
            let built = fm.nodes();
            assert_eq!((built.status(), count(built)), (0, expr.len()), "{what}");
            (0..expr.len())
                .map(|node| {
                    let bits = fm.folded(node);
                    assert_eq!(bits.status(), 0, "{what}: fm_folded {node}");
                    assert_eq!(
                        count(bits) & 2 != 0,
                        expr.foldable(node),
                        "{what}: foldable {node}"
                    );
                    count(bits) & 1 != 0
                })
                .collect()
        } else {
            fm.flags()
        };
        let model_flags: Vec<bool> = (0..expr.len())
            .map(|node| want.folds[s].is_folded(node))
            .collect();
        assert_eq!(flags, model_flags, "{what}: folds of statement {s}");
        let counted = fm.rows(want.widths[s]);
        assert_eq!(
            (counted.status(), count(counted)),
            (0, want.rows_of(s)),
            "{what}: rows of statement {s}"
        );
        tally.folded_nodes += flags.iter().filter(|&&f| f).count();
        folded_statements += usize::from(flags.iter().any(|&f| f));
    }
    if check.frame {
        let got = want.drawn_by_fm(fm, 0);
        same_frame(&what, &got, &pack(&want.bitmap(0)));
        tally.frames += 1;
    }
    let walked = walk(&want, rows);
    tally.refit += usize::from(hyps + 1 > rows);
    tally.alone += usize::from(hyps + 1 > rows || !walked.fits);
    tally.skip_mattered += usize::from(walked.skip_mattered);
    if folded_statements == 0 {
        tally.unfolded += 1;
        Outcome::Unfolded
    } else if hyps + 1 > rows {
        Outcome::Alone
    } else {
        tally.fitted += 1;
        Outcome::Fitted
    }
}

/// The widths of the screen, of the text row and of two narrow ones, and the rows tried.
pub const WIDTHS: [usize; 4] = [STATEMENT_WIDTH, TEXT_WIDTH, 60, 30];
pub const ROW_COUNTS: [usize; 5] = [CONTENT_ROWS, 1, 2, 3, 8];

pub fn sweep(bytes: &[u8], rows: usize, width: usize, part: (usize, usize), check: Check) -> Tally {
    let model = Model::new(bytes);
    let library = decoded(bytes);
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for n in (part.0..library.assertions.len()).step_by(part.1) {
        compare(
            &mut fm,
            &model,
            &library,
            (n as u16, rows, width),
            check,
            &mut tally,
        );
    }
    tally
}

fn fixture_part(part: usize) {
    let bytes = fixture::nodes_library();
    let mut total = Tally::default();
    for rows in ROW_COUNTS {
        for width in WIDTHS {
            let full = rows == CONTENT_ROWS && width == STATEMENT_WIDTH;
            let check = if full || (rows + width) % 4 == part {
                FULL
            } else {
                FLAGS
            };
            let t = sweep(&bytes, rows, width, (part, 4), check);
            total.assertions += t.assertions;
            total.too_large += t.too_large;
            total.unfolded += t.unfolded;
            total.fitted += t.fitted;
            total.alone += t.alone;
            total.refit += t.refit;
            total.skip_mattered += t.skip_mattered;
            total.folded_nodes += t.folded_nodes;
            total.frames += t.frames;
        }
    }
    println!("fixture part {part}: {total:?}");
    assert!(total.fitted > 20 && total.unfolded > 20);
    assert!(total.folded_nodes > 200 && total.frames > 50);
}

/// The model's fold_levels over the statements of `o`, noting whether the rule that leaves a
/// statement of one line alone held one back from a fold while a later depth went on folding.
pub struct Walk {
    pub fits: bool,
    pub skip_mattered: bool,
}

pub fn walk(o: &Oracle, rows: usize) -> Walk {
    let lay = |s: usize, f: &Folds| layout(&o.exprs[s], f, o.widths[s]).rows();
    let mut folds: Vec<Folds> = o.exprs.iter().map(Folds::none).collect();
    let total = |f: &[Folds]| (0..f.len()).map(|s| lay(s, &f[s])).sum::<usize>();
    if total(&folds) <= rows {
        return Walk {
            fits: true,
            skip_mattered: false,
        };
    }
    let deepest = o
        .exprs
        .iter()
        .flat_map(|e| (0..e.len()).map(|n| e.depth(n)))
        .max()
        .unwrap_or(0);
    let (mut skipped_at, mut mattered) = (None, false);
    for depth in (0..=deepest).rev() {
        for s in 0..o.exprs.len() {
            let expr = &o.exprs[s];
            let nodes: Vec<usize> = (0..expr.len())
                .filter(|&n| expr.depth(n) == depth)
                .collect();
            if lay(s, &folds[s]) <= 1 {
                let open = nodes
                    .iter()
                    .any(|&n| expr.foldable(n) && !folds[s].is_folded(n));
                if open {
                    skipped_at.get_or_insert(depth);
                }
                continue;
            }
            let mut changed = false;
            for n in nodes {
                if !folds[s].is_folded(n) {
                    changed |= folds[s].toggle(expr, n);
                }
            }
            mattered |= changed && skipped_at.is_some_and(|d| depth < d);
            if changed && total(&folds) <= rows {
                return Walk {
                    fits: true,
                    skip_mattered: mattered,
                };
            }
        }
    }
    Walk {
        fits: false,
        skip_mattered: mattered,
    }
}

#[test]
fn the_fixture_sweeps_reach_every_branch_of_the_rule() {
    let (mut refit, mut fallback, mut skip, mut fitted) = (0, 0, 0, 0);
    let mut skip_in_fixture = 0;
    for (which, bytes) in [fixture::nodes_library(), crate::fitcases::cases_library()]
        .iter()
        .enumerate()
    {
        let (model, library) = (Model::new(bytes), decoded(bytes));
        for rows in ROW_COUNTS {
            for width in WIDTHS {
                for n in 0..library.assertions.len() as u16 {
                    if !model.within(n) {
                        continue;
                    }
                    let hyps = essentials(&library, n);
                    let o = oracle(&model, n, hyps, rows, width);
                    let walked = walk(&o, rows);
                    refit += usize::from(hyps + 1 > rows);
                    fallback += usize::from(hyps + 1 > rows || !walked.fits);
                    skip += usize::from(walked.skip_mattered);
                    skip_in_fixture += usize::from(walked.skip_mattered && which == 0);
                    fitted += usize::from(
                        hyps + 1 <= rows && walked.fits && o.folds.iter().any(|f| f.count() > 0),
                    );
                }
            }
        }
    }
    println!(
        "sweeps: {fitted} fitted, {fallback} fit-alone fallbacks, {refit} conclusion refits, {skip} skips that mattered ({skip_in_fixture} in the nodes fixture)"
    );
    assert!(
        refit >= FLOOR_REFIT && fallback >= FLOOR_FALLBACK,
        "{refit} {fallback}"
    );
    assert!(
        skip >= FLOOR_SKIP && fitted >= FLOOR_FITTED,
        "{skip} {fitted}"
    );
}

const FLOOR_REFIT: usize = 150;
const FLOOR_FALLBACK: usize = 250;
const FLOOR_SKIP: usize = 10;
const FLOOR_FITTED: usize = 380;

#[test]
fn fixture_fits_match_the_model_in_part_0_of_4() {
    fixture_part(0);
}

#[test]
fn fixture_fits_match_the_model_in_part_1_of_4() {
    fixture_part(1);
}

#[test]
fn fixture_fits_match_the_model_in_part_2_of_4() {
    fixture_part(2);
}

#[test]
fn fixture_fits_match_the_model_in_part_3_of_4() {
    fixture_part(3);
}

/// The frame the real theorem screen draws for `label` and the scroll it opens at.
fn on_screen(bytes: &[u8], label: &str) -> (mcglyph::Bitmap, usize) {
    let status = vec![Status::Unchecked; decoded(bytes).assertions.len()];
    let installed = Installed {
        name: "library".into(),
        bytes: bytes.to_vec(),
    };
    let mut ui = Ui::new(
        vec![installed],
        Statuses {
            libraries: vec![status],
            my_proofs: 0,
        },
    )
    .expect("the screen loads the library");
    for key in [Key::Enter, Key::F2] {
        ui.press(key);
    }
    for c in label.chars() {
        ui.press(Key::Letter(c));
    }
    for _ in 0..2000 {
        if matches!(ui.state(), State::List { label: Some(ref l), .. } if l == label) {
            break;
        }
        ui.press(Key::Down);
    }
    ui.press(Key::Enter);
    match ui.state() {
        State::Theorem { scroll, .. } => (ui.frame(), scroll),
        other => panic!("{label}: not a theorem screen: {other:?}"),
    }
}

/// fm's statements against the theorem screen's own frame: the pixels right of each statement's
/// first column on every text row it takes, which leaves out the numbers and the marker.
fn same_as_screen(bytes: &[u8], labels: &[&str]) -> (usize, Vec<String>) {
    let (model, library) = (Model::new(bytes), decoded(bytes));
    let mut fm = Fm::new(bytes);
    let (mut compared, mut bad) = (0, Vec::new());
    for label in labels {
        let n = library
            .assertions
            .iter()
            .position(|a| a.label == *label)
            .unwrap_or_else(|| panic!("no assertion {label}")) as u16;
        let hyps = essentials(&library, n);
        let (screen, scroll) = on_screen(bytes, label);
        assert_eq!(fm.assertion(n).status(), 0);
        let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
        assert_eq!(fm.fit(CONTENT_ROWS, hyp, concl).status(), 0);
        let o = oracle(&model, n, hyps, CONTENT_ROWS, STATEMENT_WIDTH);
        let drawn = o.drawn_by_fm(&mut fm, scroll);
        let starts = o.starts();
        for s in 0..=hyps {
            for line in 0..o.rows_of(s) {
                let row = (starts[s] + line) as isize - scroll as isize;
                if !(1..1 + CONTENT_ROWS as isize).contains(&row) {
                    continue;
                }
                for y in 8 * row as usize..8 * row as usize + 8 {
                    for x in o.xs[s]..96 {
                        let ink = drawn[y * 12 + x / 8] & (0x80 >> (x % 8)) != 0;
                        compared += 1;
                        if ink != screen.get(x, y) && !bad.contains(&label.to_string()) {
                            bad.push(label.to_string());
                        }
                    }
                }
            }
        }
    }
    (compared, bad)
}

#[test]
fn the_statements_fm_draws_are_the_ones_the_theorem_screen_draws() {
    let fixture_labels = ["ax-2", "syl3", "many", "longest", "deep", "alimi"];
    let (mut compared, mut bad) = same_as_screen(&fixture::nodes_library(), &fixture_labels);
    let (more, worse) =
        same_as_screen(&crate::fitcases::cases_library(), &["e8", "f1", "f2", "g1"]);
    compared += more;
    bad.extend(worse);
    assert!(compared > 10_000, "{compared} pixels");
    assert!(bad.is_empty(), "the screen and fm differ for {bad:?}");
}
