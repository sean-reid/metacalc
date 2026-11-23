use crate::clip::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{CONTENT_ROWS, GUTTER, TEXT_WIDTH};
use mcui::fold::{Folds, fit_one, layout};

/// Everything a refused call could have written: the records, the cursor variable, the header's
/// last byte and each node's fold bits.
#[derive(Debug, PartialEq)]
struct Snapshot {
    records: Vec<[u8; 4]>,
    cursor: u8,
    flags: u8,
    folds: Vec<u8>,
}

fn snapshot(fm: &mut Fm, nodes: usize) -> Snapshot {
    let folds = (0..nodes)
        .map(|node| {
            let out = fm.folded(node);
            assert_eq!(out.status(), 0);
            (out.bc >> 8) as u8
        })
        .collect();
    Snapshot {
        records: fm.records(),
        cursor: fm.memory("FV_CURN", 1)[0],
        flags: fm.header_flags(),
        folds,
    }
}

/// Loads the "many" conclusion and builds its node table: the node count and the nodes that fold.
fn loaded_many(fm: &mut Fm, bytes: &[u8]) -> (usize, Vec<usize>) {
    assert_eq!(fm.statement(index(bytes, "many"), 0xFF).status(), 0);
    let nodes = count(fm.nodes());
    let foldable = (1..nodes)
        .filter(|&node| fm.folded(node).bc >> 9 & 1 == 1)
        .collect();
    (nodes, foldable)
}

/// The "many" conclusion cut to one line of `CUT` pixels.
const CUT: usize = 40;

fn cut(fm: &mut Fm) {
    let out = fm.clip(1, CUT);
    assert_eq!((out.status(), count(out)), (0, 1));
}

#[test]
fn a_refused_cursor_or_fold_on_a_clipped_statement_writes_nothing() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let (nodes, foldable) = loaded_many(&mut fm, &bytes);
    assert!(foldable.len() >= 3, "{foldable:?}");
    assert_eq!(fm.cursor(2).status(), 0);
    cut(&mut fm);
    let before = snapshot(&mut fm, nodes);
    assert_eq!(before.cursor, NO_CURSOR, "the clip cleared the cursor");
    let refused = code("FM_ERR_CLIPPED");
    for node in 1..nodes {
        assert_eq!(fm.cursor(node as u8).status(), refused, "cursor {node}");
        assert_eq!(snapshot(&mut fm, nodes), before, "cursor {node} wrote");
    }
    for &node in &foldable {
        for mode in [1, 2] {
            let out = fm.fold(node, mode);
            assert_eq!(out.status(), refused, "fold {node} mode {mode}");
            assert_eq!(snapshot(&mut fm, nodes), before, "fold {node} wrote");
        }
    }
}

#[test]
fn a_fold_that_changes_no_flag_on_a_clipped_statement_returns_zero() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let (nodes, foldable) = loaded_many(&mut fm, &bytes);
    cut(&mut fm);
    let before = snapshot(&mut fm, nodes);
    for &node in &foldable {
        let out = fm.fold(node, 0);
        assert_eq!((out.status(), count(out)), (0, 0), "clear {node}");
        assert_eq!(snapshot(&mut fm, nodes), before, "clear {node} wrote");
    }
    let flat = (1..nodes).find(|node| !foldable.contains(node));
    if let Some(node) = flat {
        for mode in [0, 1, 2] {
            let out = fm.fold(node, mode);
            assert_eq!((out.status(), count(out)), (0, 0), "unfoldable {node}");
        }
        assert_eq!(snapshot(&mut fm, nodes), before);
    }
}

#[test]
fn a_node_folded_before_the_clip_is_set_again_and_not_cleared() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let (nodes, foldable) = loaded_many(&mut fm, &bytes);
    let node = foldable[0];
    assert_eq!(count(fm.fold(node, 1)), 1);
    cut(&mut fm);
    let before = snapshot(&mut fm, nodes);
    let again = fm.fold(node, 1);
    assert_eq!((again.status(), count(again)), (0, 0), "set again");
    assert_eq!(snapshot(&mut fm, nodes), before);
    for mode in [0, 2] {
        let out = fm.fold(node, mode);
        assert_eq!(out.status(), code("FM_ERR_CLIPPED"), "mode {mode}");
        assert_eq!(snapshot(&mut fm, nodes), before, "mode {mode} wrote");
    }
}

#[test]
fn a_clipped_statement_takes_no_cursor_but_the_none_and_reports_a_missing_node_as_missing() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let (nodes, _) = loaded_many(&mut fm, &bytes);
    cut(&mut fm);
    let before = snapshot(&mut fm, nodes);
    let none = fm.cursor(NO_CURSOR);
    assert_eq!(none.status(), 0, "no cursor is allowed");
    assert_eq!(snapshot(&mut fm, nodes), before);
    for node in [nodes as u8, nodes as u8 + 1, 0xFE] {
        let out = fm.cursor(node);
        assert_eq!(out.status(), code("FM_ERR_INDEX"), "cursor {node}");
        assert_eq!(snapshot(&mut fm, nodes), before, "cursor {node} wrote");
    }
    let out = fm.fold(nodes, 1);
    assert_eq!(out.status(), code("FM_ERR_INDEX"), "fold past the count");
    assert_eq!(snapshot(&mut fm, nodes), before);
}

#[test]
fn fm_open_drops_the_clip_and_the_header_it_was_in() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    loaded_many(&mut fm, &bytes);
    cut(&mut fm);
    assert_eq!(fm.open().status(), 0);
    let index_error = code("FM_ERR_INDEX");
    assert_eq!(fm.clip(1, CUT).status(), index_error, "not CLIPPED");
    assert_eq!(fm.cursor(1).status(), index_error);
    assert_eq!(fm.fold(1, 1).status(), index_error);
    assert_eq!(fm.header_flags(), 0, "no flag is left to read");
    let n = index(&bytes, "many");
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    cut(&mut fm);
}

#[test]
fn loading_the_assertion_again_drops_the_clip_of_its_conclusion() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let (n, width) = (index(&bytes, "many"), CUT);
    assert_eq!(fm.assertion(n).status(), 0);
    assert_eq!(fm.select(0xFF).status(), 0);
    assert_eq!(fm.clip(1, width).status(), 0);
    assert_eq!(count(fm.rows(width)), 1);
    assert_eq!(fm.assertion(n).status(), 0);
    assert_eq!(fm.select(0xFF).status(), 0);
    let expr = model.expr(n, 0xFF);
    let plain = layout(&expr, &Folds::none(&expr), width);
    assert!(plain.rows() > 1);
    assert_eq!(
        count(fm.rows(width)),
        plain.rows(),
        "the rows are the whole ones"
    );
    fm.clear();
    assert_eq!(fm.draw(GUTTER, 0, width, (0, 8)).status(), 0);
    same_frame("reloaded", &fm.frame(), &frame_of(&plain));
    assert_eq!(fm.nodes().status(), 0);
    assert_eq!(fm.cursor(1).status(), 0);
    let out = fm.clip(1, width);
    assert_eq!((out.status(), count(out)), (0, 1), "and it clips again");
}

#[test]
fn a_clip_at_another_width_than_the_fit_cuts_as_the_model_does_there() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let mut tally = Tally::default();
    let library = decoded(&bytes);
    for n in 0..library.assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        for (fitted, width) in [
            (TEXT_WIDTH, 30),
            (TEXT_WIDTH, 60),
            (30, TEXT_WIDTH),
            (60, 20),
        ] {
            for rows in [1, 2] {
                assert_eq!(fm.statement(n, 0xFF).status(), 0);
                assert_eq!(fm.fit(rows, fitted, fitted).status(), 0);
                let plain = layout(&expr, &fit_one(&expr, fitted, rows), width);
                let want = plain.clipped(width, rows);
                let what =
                    format!("assertion {n}, fitted at {fitted}, clipped at {width}, {rows} rows");
                let at = (rows, width);
                check_after(
                    &mut fm,
                    &what,
                    (&plain, &want),
                    at,
                    &mut tally,
                    After::Nothing,
                );
            }
        }
    }
    let sum = tally.sum_from(1);
    println!("clip at another width: {tally:?}");
    assert!(sum.clipped > 100 && sum.unchanged > 100, "{sum:?}");
}

#[test]
fn a_clip_that_does_not_cut_keeps_the_cursor() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let n = index(&bytes, "ax-2");
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    let width = TEXT_WIDTH;
    assert_eq!(fm.fit(CONTENT_ROWS, width, width).status(), 0);
    assert_eq!(fm.cursor(3).status(), 0);
    fm.clear();
    assert_eq!(fm.draw(GUTTER, 0, width, (0, 8)).status(), 0);
    let with_cursor = fm.frame();
    assert_eq!(fm.cursor(NO_CURSOR).status(), 0);
    fm.clear();
    assert_eq!(fm.draw(GUTTER, 0, width, (0, 8)).status(), 0);
    assert_ne!(with_cursor, fm.frame(), "the cursor shows");
    assert_eq!(fm.cursor(3).status(), 0);
    let out = fm.clip(CONTENT_ROWS, width);
    assert_eq!(out.status(), 0);
    assert_eq!(fm.memory("FV_CURN", 1)[0], 3, "the cursor stays");
    fm.clear();
    assert_eq!(fm.draw(GUTTER, 0, width, (0, 8)).status(), 0);
    same_frame("cursor after a clip that fits", &fm.frame(), &with_cursor);
    let out = fm.clip(1, CUT);
    assert_eq!((out.status(), fm.memory("FV_CURN", 1)[0]), (0, NO_CURSOR));
}

#[test]
fn a_clip_that_fits_repeats_harmlessly_and_a_cut_clip_cuts_once() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let n = index(&bytes, "ax-1");
    let width = TEXT_WIDTH;
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.fit(CONTENT_ROWS, width, width).status(), 0);
    let before = fm.records();
    for _ in 0..3 {
        let out = fm.clip(CONTENT_ROWS, width);
        assert_eq!(out.status(), 0);
        assert_eq!(fm.records(), before);
        assert_eq!(fm.header_flags() & 4, 0, "no clipped mark");
    }
    let expr = model.expr(n, 0xFF);
    let fit = fit_one(&expr, width, CONTENT_ROWS);
    let (narrow, mut tally) = (20, Tally::default());
    let plain = layout(&expr, &fit, narrow);
    let want = plain.clipped(narrow, 1);
    assert_ne!(want, plain, "the narrow width cuts");
    check_after(
        &mut fm,
        "narrow",
        (&plain, &want),
        (1, narrow),
        &mut tally,
        After::Nothing,
    );
    assert_eq!(fm.header_flags() & 4, 4, "clipped now");
    assert_eq!(fm.clip(1, narrow).status(), code("FM_ERR_CLIPPED"));
    assert_eq!(
        fm.clip(CONTENT_ROWS, narrow).status(),
        code("FM_ERR_CLIPPED")
    );
}

#[test]
fn a_statement_of_the_typecode_alone_clips_as_the_model_does() {
    let bytes = fixture::custom("typecode-only $a |- $.\n");
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let n = index(&bytes, "typecode-only");
    let expr = model.expr(n, 0xFF);
    let mut tally = Tally::default();
    for width in [4, 8, 20, TEXT_WIDTH] {
        for rows in [0, 1, 2] {
            let plain = layout(&expr, &Folds::none(&expr), width);
            let want = plain.clipped(width, rows);
            assert_eq!(fm.statement(n, 0xFF).status(), 0);
            let what = format!("typecode alone, {rows} rows, width {width}");
            check(&mut fm, &what, (&plain, &want), (rows, width), &mut tally);
        }
    }
}
