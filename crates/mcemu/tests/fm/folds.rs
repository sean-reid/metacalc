use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::{pack, same_frame};
use mcglyph::Font;
use mcui::canvas::{Canvas, GUTTER, STATEMENT_WIDTH, TEXT_WIDTH};
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};

/// The widths a screen asks for, and one narrower than the ellipsis.
pub fn widths() -> [usize; 4] {
    let ellipsis = Font::builtin().glyph("ellipsis").unwrap().width();
    [94, 60, 30, ellipsis - 1]
}

pub fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// The fold sets tried on a statement: none, each foldable node alone when there are few nodes,
/// every foldable node of each depth, everything foldable, and seeded random ones, which nest.
pub fn fold_sets(expr: &Expr, seed: u64, singles: bool) -> Vec<(String, Folds)> {
    let mut out = vec![("none".to_string(), Folds::none(expr))];
    let foldable: Vec<usize> = (0..expr.len()).filter(|&n| expr.foldable(n)).collect();
    if singles {
        for &n in &foldable {
            let mut f = Folds::none(expr);
            f.toggle(expr, n);
            out.push((format!("node {n}"), f));
        }
    }
    let deepest = (0..expr.len()).map(|n| expr.depth(n)).max().unwrap_or(0);
    for depth in 0..=deepest {
        let mut f = Folds::none(expr);
        for &n in foldable.iter().filter(|&&n| expr.depth(n) == depth) {
            f.toggle(expr, n);
        }
        if f.count() > 0 {
            out.push((format!("depth {depth}"), f));
        }
    }
    let mut all = Folds::none(expr);
    for &n in &foldable {
        all.toggle(expr, n);
    }
    if all.count() > 1 {
        out.push(("every foldable node".to_string(), all));
    }
    let mut state = seed | 1;
    for round in 0..3 {
        let mut f = Folds::none(expr);
        for &n in &foldable {
            if next(&mut state).is_multiple_of(3) {
                f.toggle(expr, n);
            }
        }
        if f.count() > 0 {
            out.push((format!("random {round}"), f));
        }
    }
    out
}

/// Takes fm from the folds `from` to `to` with fm_fold, setting, clearing and toggling, and checks
/// what it says of each change.
pub fn put(fm: &mut Fm, expr: &Expr, from: &Folds, to: &Folds) {
    for node in 0..expr.len() {
        let (had, want) = (from.is_folded(node), to.is_folded(node));
        let mode = if had != want && node % 2 == 0 {
            2
        } else {
            u8::from(want)
        };
        let out = fm.fold(node, mode);
        assert_eq!(out.status(), 0, "fold {node}");
        assert_eq!(
            count(out),
            usize::from(had != want),
            "fold {node} to {want}"
        );
    }
}

/// Whether the model hides `node`: some node above it is folded.
pub fn hidden(expr: &Expr, folds: &Folds, node: usize) -> bool {
    let mut at = expr.parent(node);
    while let Some(parent) = at {
        if folds.is_folded(parent) {
            return true;
        }
        at = expr.parent(parent);
    }
    false
}

/// The nodes a cursor is tried on: all of a statement of up to `whole` nodes, else a spread, with
/// hidden ones.
pub fn picks(expr: &Expr, folds: &Folds, whole: usize) -> Vec<usize> {
    let len = expr.len();
    if len <= whole {
        return (0..len).collect();
    }
    let mut out: Vec<usize> = (0..len)
        .filter(|n| n % 5 == 0 || *n < 3 || *n + 2 > len)
        .collect();
    out.extend((0..len).filter(|&n| hidden(expr, folds, n)).take(6));
    out.sort_unstable();
    out.dedup();
    out
}

/// The windows and first rows a screen can ask for: a statement that starts below the top, one
/// scrolled up with its first lines above the window, and a window that cuts it at both ends.
pub fn cases(rows: usize) -> Vec<(i16, (u8, u8))> {
    let rows = rows as i16;
    vec![
        (0, (0, 8)),
        (1, (1, 7)),
        (-(rows / 2), (0, 8)),
        (2, (3, 5)),
        (-2, (2, 4)),
    ]
}

/// The origins tried with their widths: the screens', one that starts on column 0, one narrower
/// than the ellipsis, where every token and the ellipsis sit alone on a line, and two more widths.
pub const ORIGINS: [(usize, usize); 6] = [
    (GUTTER, STATEMENT_WIDTH),
    (1, TEXT_WIDTH),
    (0, 96),
    (1, 4),
    (1, 60),
    (1, 30),
];

type Place = (usize, i16, (u8, u8));

/// The model's frame for a folded statement, with the cursor on `node` when there is one.
pub fn model_frame_of(
    expr: &Expr,
    folds: &Folds,
    width: usize,
    (x, row, window): Place,
    node: Option<usize>,
) -> Vec<u8> {
    let laid = layout(expr, folds, width);
    let window = isize::from(window.0)..isize::from(window.1);
    let mut canvas = Canvas::default();
    canvas.layout(&laid, x, isize::from(row), window.clone());
    if let Some(node) = node {
        let rects = laid.cursor(expr, node);
        canvas.cursor(&rects, x, isize::from(row), window);
    }
    pack(&canvas.into_bitmap())
}

/// What a pass over statements found, so that a test cannot pass with nothing compared.
#[derive(Default)]
pub struct Tally {
    pub rows: usize,
    pub frames: usize,
    pub cursors: usize,
    pub bits: usize,
    pub folded: usize,
    pub nested: usize,
    pub to_edge: usize,
    pub hidden_cursors: usize,
}

/// How much of a statement is compared: one of up to `whole` nodes gets every fold set and a cursor
/// on every node, a larger one at most `sets` sets and a spread of nodes.
#[derive(Clone, Copy)]
pub struct Depth {
    pub whole: usize,
    pub sets: usize,
}

const FIXTURE_DEPTH: Depth = Depth { whole: 40, sets: 6 };

/// Compares fm with the model for one statement under the fold sets.
pub fn compare(fm: &mut Fm, model: &Model, at: (u16, u8), tally: &mut Tally, depth: Depth) {
    let (n, k) = at;
    let expr = &model.expr(n, k);
    let what = format!("statement {k} of assertion {n}");
    let table = fm.built(n, k);
    assert_eq!(table, model.table(n, k), "{what}: nodes");
    let small = expr.len() <= 12;
    let mut sets = fold_sets(expr, u64::from(n) * 977 + u64::from(k), small);
    if expr.len() > depth.whole {
        let first = |name: &str| sets.iter().position(|(n, _)| n == name);
        let mut front = vec![0];
        front.extend(first("every foldable node"));
        front.extend(first("random 0"));
        let rest: Vec<usize> = (0..sets.len()).filter(|i| !front.contains(i)).collect();
        sets = front
            .into_iter()
            .chain(rest)
            .map(|i| sets[i].clone())
            .collect();
        sets.truncate(depth.sets.max(3));
    }
    let mut now = Folds::none(expr);
    for (name, folds) in &sets {
        put(fm, expr, &now, folds);
        now = folds.clone();
        let what = format!("{what}, folds {name}");
        let marked: Vec<usize> = (0..expr.len()).filter(|&n| folds.is_folded(n)).collect();
        tally.folded += marked.len();
        tally.nested += marked.iter().filter(|&&n| hidden(expr, folds, n)).count();
        for width in widths() {
            let out = fm.rows(width);
            assert_eq!(out.status(), 0, "{what}: rows");
            let rows = layout(expr, folds, width).rows();
            assert_eq!(count(out), rows, "{what}: rows at {width}");
            tally.rows += 1;
        }
        for node in 0..expr.len() {
            let bits = u8::from(folds.is_folded(node))
                | u8::from(expr.foldable(node)) << 1
                | u8::from(hidden(expr, folds, node)) << 2;
            let out = fm.folded(node);
            assert_eq!(
                (out.status(), count(out)),
                (0, usize::from(bits)),
                "{what}: {node}"
            );
            tally.bits += 1;
        }
        let several = if expr.len() > 40 { 4 } else { 5 };
        for (o, (x, width)) in ORIGINS.into_iter().enumerate() {
            let laid = layout(expr, folds, width);
            let used = match o {
                0 => several,
                1 | 2 => 2,
                _ => 1,
            };
            for (c, (row, window)) in cases(laid.rows()).into_iter().take(used).enumerate() {
                let place = (x, row, window);
                assert_eq!(fm.cursor(NO_CURSOR).status(), 0);
                fm.clear();
                let out = fm.draw(x, row, width, window);
                assert_eq!(out.status(), 0, "{what}: draw");
                let want = model_frame_of(expr, folds, width, place, None);
                let label = format!("{what}, width {width}, x {x}, row {row}, window {window:?}");
                let drawn = fm.frame();
                same_frame(&label, &drawn, &want);
                tally.frames += 1;
                let nodes = if c == 0 && (o == 0 || o == 2) {
                    picks(expr, folds, depth.whole)
                } else {
                    vec![(o * 7 + c * 3 + marked.len()) % expr.len()]
                };
                for node in nodes {
                    let out = fm.cursor(node as u8);
                    assert_eq!((out.status(), count(out)), (0, 0), "{label}: cursor {node}");
                    fm.clear();
                    assert_eq!(fm.draw(x, row, width, window).status(), 0, "{label}: draw");
                    let mask = model_frame_of(expr, folds, width, place, Some(node));
                    same_frame(&format!("{label}, cursor on {node}"), &fm.frame(), &mask);
                    tally.cursors += 1;
                    tally.hidden_cursors += usize::from(hidden(expr, folds, node));
                    let rects = laid.cursor(expr, node);
                    tally.to_edge += usize::from(rects.iter().any(|r| x + r.x + r.width >= 95));
                    assert_eq!(fm.cursor(NO_CURSOR).status(), 0);
                    fm.clear();
                    fm.draw(x, row, width, window);
                    same_frame(
                        &format!("{label}, cursor on {node} then cleared"),
                        &fm.frame(),
                        &drawn,
                    );
                }
            }
        }
    }
}

pub fn run_library(bytes: &[u8], every: usize, offset: usize, depth: Depth) -> Tally {
    let model = Model::new(bytes);
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for (i, (n, k, _)) in statements(&decoded(bytes)).iter().enumerate() {
        if i % every == offset {
            compare(&mut fm, &model, (*n, *k), &mut tally, depth);
        }
    }
    tally
}

fn fixture_part(part: usize) {
    let tally = run_library(&fixture::nodes_library(), 4, part, FIXTURE_DEPTH);
    println!(
        "fixture part {part}: {} row counts, {} fold bit sets, {} frames, {} cursors, {} folded \
         nodes, {} of them nested, {} cursors on hidden nodes, {} cursors to the last column",
        tally.rows,
        tally.bits,
        tally.frames,
        tally.cursors,
        tally.folded,
        tally.nested,
        tally.hidden_cursors,
        tally.to_edge
    );
    assert!(tally.rows > 400 && tally.frames > 200 && tally.cursors > 500);
    assert!(tally.nested > 5 && tally.hidden_cursors > 5 && tally.to_edge > 2);
}

#[test]
fn fixture_folds_match_the_model_in_part_0_of_4() {
    fixture_part(0);
}

#[test]
fn fixture_folds_match_the_model_in_part_1_of_4() {
    fixture_part(1);
}

#[test]
fn fixture_folds_match_the_model_in_part_2_of_4() {
    fixture_part(2);
}

#[test]
fn fixture_folds_match_the_model_in_part_3_of_4() {
    fixture_part(3);
}

#[test]
fn the_longest_and_the_deepest_fixture_statements_get_nested_folds_and_hidden_cursors() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    for label in ["longest", "deep", "t255"] {
        let n = library
            .assertions
            .iter()
            .position(|a| a.label == label)
            .unwrap() as u16;
        let mut tally = Tally::default();
        compare(&mut fm, &model, (n, 0xFF), &mut tally, FIXTURE_DEPTH);
        assert!(model.expr(n, 0xFF).len() > 40, "{label}");
        assert!(tally.nested > 0 && tally.hidden_cursors > 0, "{label}");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_fold_and_draw_a_cursor_as_the_model_does() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for (name, every) in [("prop", 12), ("pred", 30), ("ring", 40)] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let tally = run_library(&bytes, every, 0, Depth { whole: 0, sets: 3 });
        println!(
            "{name}: {} row counts, {} frames, {} cursors, {} folded nodes, {} of them nested, \
             {} cursors on hidden nodes",
            tally.rows,
            tally.frames,
            tally.cursors,
            tally.folded,
            tally.nested,
            tally.hidden_cursors
        );
        assert!(tally.frames > 100 && tally.cursors > 100, "{name}");
        assert!(tally.nested > 0 && tally.hidden_cursors > 0, "{name}");
    }
}
