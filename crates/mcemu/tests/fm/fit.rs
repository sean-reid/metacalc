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
    /// Assertions with more statements than rows, where fm_fit fits each statement alone.
    pub alone: usize,
    /// Of those, the ones whose conclusion keeps more than one line after the refit.
    pub refit: usize,
    /// Assertions that end with a statement at one line holding an unfolded foldable node while
    /// another statement has a folded node of a shallower depth.
    pub skip_mattered: usize,
    pub folded_nodes: usize,
    pub frames: usize,
    /// A sum over the compared runs of a hash of fm's flags and total, to see two builds agree.
    pub digest: u64,
}

impl Tally {
    pub fn add(&mut self, t: &Tally) {
        self.assertions += t.assertions;
        self.too_large += t.too_large;
        self.refused_at_load += t.refused_at_load;
        self.unfolded += t.unfolded;
        self.fitted += t.fitted;
        self.alone += t.alone;
        self.refit += t.refit;
        self.skip_mattered += t.skip_mattered;
        self.folded_nodes += t.folded_nodes;
        self.frames += t.frames;
        self.digest = self.digest.wrapping_add(t.digest);
    }
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
    let (mut all_flags, mut all_rows) = (Vec::new(), Vec::new());
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
        all_flags.push(flags);
        all_rows.push(count(counted));
    }
    if check.frame {
        let got = want.drawn_by_fm(fm, 0);
        same_frame(&what, &got, &pack(&want.bitmap(0)));
        tally.frames += 1;
    }
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(
        &(n, rows, width, out.hl, &all_flags, &all_rows),
        &mut hasher,
    );
    tally.digest = tally
        .digest
        .wrapping_add(std::hash::Hasher::finish(&hasher));
    let alone = count(loaded) > rows;
    tally.alone += usize::from(alone);
    tally.refit += usize::from(alone && all_rows[last] > 1);
    tally.skip_mattered += usize::from(skip_mattered(&want, &all_flags, &all_rows));
    if folded_statements == 0 {
        tally.unfolded += 1;
        Outcome::Unfolded
    } else if alone {
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
            total.add(&t);
        }
    }
    println!("fixture part {part}: {total:?}");
    assert!(total.fitted > 20 && total.unfolded > 20);
    assert!(total.folded_nodes > 200 && total.frames > 50);
}

/// Whether fm left a statement at one line with an unfolded foldable node while another
/// statement has a folded node of a shallower depth: the flags and rows are fm's.
fn skip_mattered(o: &Oracle, flags: &[Vec<bool>], rows: &[usize]) -> bool {
    (0..o.exprs.len()).any(|s| {
        let expr = &o.exprs[s];
        let open = (0..expr.len())
            .filter(|&n| expr.foldable(n) && !flags[s][n])
            .map(|n| expr.depth(n))
            .max();
        rows[s] == 1
            && open.is_some_and(|depth| {
                (0..o.exprs.len()).filter(|&t| t != s).any(|t| {
                    (0..o.exprs[t].len()).any(|n| flags[t][n] && o.exprs[t].depth(n) < depth)
                })
            })
    })
}

#[test]
fn the_cases_library_through_the_z80_reaches_every_branch_of_the_rule() {
    let bytes = crate::fitcases::cases_library();
    let started = std::time::Instant::now();
    let mut total = Tally::default();
    let mut pairs = 0;
    for rows in ROW_COUNTS {
        for width in WIDTHS {
            total.add(&sweep(&bytes, rows, width, (0, 1), FLAGS));
            pairs += 1;
        }
    }
    println!(
        "cases sweep: {pairs} row and width pairs, {} runs of fm_fit compared with the model ({} refused as too large), digest {:x}, {} fitted, {} fitting each statement alone, {} with the conclusion refitted, {} skips that mattered, in {:?}",
        total.assertions,
        total.too_large,
        total.digest,
        total.fitted,
        total.alone,
        total.refit,
        total.skip_mattered,
        started.elapsed()
    );
    assert!(
        total.refit >= FLOOR_REFIT && total.alone >= FLOOR_ALONE,
        "{total:?}"
    );
    assert!(
        total.skip_mattered >= FLOOR_SKIP && total.fitted >= FLOOR_FITTED,
        "{total:?}"
    );
}

const FLOOR_REFIT: usize = 18;
const FLOOR_ALONE: usize = 125;
const FLOOR_SKIP: usize = 14;
const FLOOR_FITTED: usize = 140;

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
