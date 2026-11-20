use crate::fixture;
use crate::folds::{fold_sets, hidden, model_frame_of, put, widths};
use crate::guard::{pattern, stray};
use crate::nodes::Model;
use crate::rig::*;
use crate::session::allowed;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH};
use mcui::fold::{Folds, layout};

fn assertion(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap() as u16
}

/// A statement with a foldable node and a node with nothing in it.
fn mixed(model: &Model, bytes: &[u8]) -> (u16, u8) {
    for (n, k, _) in statements(&decoded(bytes)) {
        let expr = model.expr(n, k);
        let leaf = (0..expr.len()).any(|i| !expr.foldable(i));
        if expr.len() > 3 && expr.foldable(0) && leaf {
            return (n, k);
        }
    }
    panic!("a statement with both");
}

#[test]
fn fm_fold_and_fm_folded_and_fm_cursor_answer_with_the_documented_statuses() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut closed = Fm::bare(&bytes);
    for out in [closed.fold(0, 1), closed.folded(0), closed.cursor(0, 1, 0, 94, (0, 8))] {
        assert_eq!(out.status(), code("FM_ERR_NOT_OPEN"));
    }
    let mut fm = Fm::new(&bytes);
    let index = code("FM_ERR_INDEX");
    assert_eq!(fm.fold(0, 1).status(), index, "nothing loaded");
    assert_eq!(fm.folded(0).status(), index);
    assert_eq!(fm.cursor(0, 1, 0, 94, (0, 8)).status(), index);

    let (n, k) = mixed(&model, &bytes);
    let expr = model.expr(n, k);
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(fm.header().nodes, 0, "no table yet");
    assert_eq!(fm.folded(0).status(), index, "no table: folded");
    assert_eq!(fm.cursor(0, 1, 0, 94, (0, 8)).status(), index, "no table: cursor");

    let out = fm.fold(0, 1);
    assert_eq!((out.status(), count(out)), (0, 1), "fm_fold builds the table");
    assert_eq!(usize::from(fm.header().nodes), expr.len());
    let out = fm.fold(0, 1);
    assert_eq!((out.status(), count(out)), (0, 0), "set twice");
    let out = fm.fold(0, 2);
    assert_eq!((out.status(), count(out)), (0, 1), "toggled off");
    let out = fm.fold(0, 2);
    assert_eq!((out.status(), count(out)), (0, 1), "toggled on");
    let out = fm.fold(0, 0);
    assert_eq!((out.status(), count(out)), (0, 1), "cleared");
    let out = fm.fold(0, 0);
    assert_eq!((out.status(), count(out)), (0, 0), "cleared twice");

    let leaf = (0..expr.len()).find(|&i| !expr.foldable(i)).unwrap();
    for mode in 0..3 {
        let out = fm.fold(leaf, mode);
        assert_eq!((out.status(), count(out)), (0, 0), "node {leaf} has nothing in it");
    }
    assert_eq!(count(fm.folded(leaf)), 0, "a refused fold sets nothing");
    let past = expr.len();
    assert_eq!(fm.fold(past, 1).status(), index);
    assert_eq!(fm.fold(255, 1).status(), index);
    assert_eq!(fm.fold(0, 3).status(), code("FM_ERR_BAD"));
    assert_eq!(fm.fold(0, 255).status(), code("FM_ERR_BAD"));
    assert_eq!(fm.folded(past).status(), index);
    assert_eq!(fm.cursor(past, 1, 0, 94, (0, 8)).status(), index);
    assert_eq!(fm.fold(0, 1).status(), 0);
    assert_eq!(fm.folded(past).status(), index, "the flag past the table stays out of reach");
}

#[test]
fn a_fold_changes_the_records_and_clearing_every_fold_gives_back_the_ones_fm_assertion_wrote() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let mut changed = 0;
    for label in ["many", "ax-2", "df-clab", "deep", "longest", "wide", "syl3"] {
        let n = assertion(&bytes, label);
        let out = fm.assertion(n);
        assert_eq!(out.status(), 0, "{label}");
        let statements = count(out);
        let tokens = usize::from(u16::from_le_bytes(fm.memory("FV_NTOK", 2).try_into().unwrap()));
        let fresh = fm.memory("FM_TOKENS", 4 * tokens);
        for index in 0..statements {
            let k = if index + 1 == statements { 0xFF } else { index as u8 };
            assert_eq!(fm.select(k).status(), 0);
            assert_eq!(fm.nodes().status(), 0);
            let expr = model.expr(n, k);
            let sets = fold_sets(&expr, 5 + index as u64, false);
            let mut now = Folds::none(&expr);
            let few = if expr.len() > 40 { 5 } else { sets.len() };
            for (name, folds) in sets.iter().filter(|(name, _)| name != "none").take(few) {
                put(&mut fm, &expr, &now, folds);
                assert_eq!(fm.rows(94).status(), 0);
                if fm.memory("FM_TOKENS", 4 * tokens) != fresh {
                    changed += 1;
                }
                put(&mut fm, &expr, folds, &Folds::none(&expr));
                now = Folds::none(&expr);
                assert_eq!(fm.rows(94).status(), 0);
                let got = fm.memory("FM_TOKENS", 4 * tokens);
                assert!(got == fresh, "{label} statement {k}, folds {name}: records differ");
            }
        }
    }
    assert!(changed > 20, "{changed} folds changed the records");
}

#[test]
fn the_first_call_after_a_fold_applies_it_whichever_call_it_is() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let n = assertion(&bytes, "many");
    for (k, node_count) in [(0u8, 4usize), (4, 8), (0xFF, 12)] {
        let expr = model.expr(n, k);
        assert_eq!(fm.statement(n, k).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
        let sets = fold_sets(&expr, 11, false);
        let mut now = Folds::none(&expr);
        for (i, (name, folds)) in sets.iter().skip(1).take(node_count).enumerate() {
            let first = if i % 2 == 0 { "draw" } else { "cursor" };
            put(&mut fm, &expr, &now, folds);
            now = folds.clone();
            let node = (0..expr.len()).rev().find(|&i| hidden(&expr, folds, i)).unwrap_or(0);
            let place = (GUTTER, 0, (0u8, 8u8));
            fm.clear();
            let want = if first == "draw" {
                assert_eq!(fm.draw(GUTTER, 0, STATEMENT_WIDTH, (0, 8)).status(), 0);
                model_frame_of(&expr, folds, STATEMENT_WIDTH, place, None)
            } else {
                // On a blank frame the cursor leaves what it flips: the model's frame with it
                // and without it, xor-ed.
                assert_eq!(fm.cursor(node, GUTTER, 0, STATEMENT_WIDTH, (0, 8)).status(), 0);
                let mut blank = model_frame_of(&expr, folds, STATEMENT_WIDTH, place, Some(node));
                let text = model_frame_of(&expr, folds, STATEMENT_WIDTH, place, None);
                for (b, t) in blank.iter_mut().zip(&text) {
                    *b ^= *t;
                }
                blank
            };
            same_frame(&format!("{k} {name}, {first} first"), &fm.frame(), &want);
        }
    }
}

#[test]
fn folds_survive_fm_select_and_are_gone_after_fm_assertion_fm_statement_and_fm_open() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let n = assertion(&bytes, "many");
    let mut fm = Fm::new(&bytes);
    let out = fm.assertion(n);
    assert_eq!(out.status(), 0);
    let statements = count(out);
    let ks: Vec<u8> = (0..statements as u8 - 1).chain([0xFF]).collect();
    let mut kept = Vec::new();
    for (i, &k) in ks.iter().enumerate() {
        assert_eq!(fm.select(k).status(), 0);
        assert_eq!(fm.nodes().status(), 0);
        let expr = model.expr(n, k);
        let sets = fold_sets(&expr, 3, false);
        let (_, folds) = sets[(1 + i % (sets.len().max(2) - 1)).min(sets.len() - 1)].clone();
        put(&mut fm, &expr, &Folds::none(&expr), &folds);
        kept.push(folds);
    }
    let mut seen = 0;
    for (&k, folds) in ks.iter().zip(&kept).rev() {
        let expr = model.expr(n, k);
        assert_eq!(fm.select(k).status(), 0);
        for node in 0..expr.len() {
            let out = fm.folded(node);
            assert_eq!(count(out) & 1, usize::from(folds.is_folded(node)), "{k} node {node}");
            seen += count(out) & 1;
        }
        assert_eq!(fm.rows(60).bc >> 8, layout(&expr, folds, 60).rows() as u16);
        fm.clear();
        fm.draw(GUTTER, 0, STATEMENT_WIDTH, (0, 8));
        let place = (GUTTER, 0, (0, 8));
        let want = model_frame_of(&expr, folds, STATEMENT_WIDTH, place, None);
        same_frame(&format!("statement {k} after other selects"), &fm.frame(), &want);
    }
    assert!(seen > 5, "{seen} flags came back");

    let reload = |fm: &mut Fm, what: &str| {
        for &k in &ks {
            assert_eq!(fm.select(k).status(), 0, "{what}");
            assert_eq!(fm.folded(0).status(), code("FM_ERR_INDEX"), "{what}: the table is gone");
            assert_eq!(fm.nodes().status(), 0);
            let expr = model.expr(n, k);
            for node in 0..expr.len() {
                assert_eq!(count(fm.folded(node)) & 1, 0, "{what}: {k} node {node}");
            }
            fm.clear();
            fm.draw(GUTTER, 0, STATEMENT_WIDTH, (0, 8));
            let plain = Folds::none(&expr);
            let want = model_frame_of(&expr, &plain, STATEMENT_WIDTH, (GUTTER, 0, (0, 8)), None);
            same_frame(&format!("{what}: statement {k}"), &fm.frame(), &want);
        }
    };
    assert_eq!(fm.assertion(n).status(), 0);
    reload(&mut fm, "fm_assertion");
    for (&k, folds) in ks.iter().zip(&kept) {
        assert_eq!(fm.select(k).status(), 0);
        put(&mut fm, &model.expr(n, k), &Folds::none(&model.expr(n, k)), folds);
    }
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.assertion(n).status(), 0);
    reload(&mut fm, "fm_open");
    let k = ks[ks.len() - 1];
    let expr = model.expr(n, k);
    assert_eq!(fm.nodes().status(), 0);
    put(&mut fm, &expr, &Folds::none(&expr), &fold_sets(&expr, 1, false)[1].1);
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    for node in 0..expr.len() {
        assert_eq!(count(fm.folded(node)) & 1, 0, "fm_statement: node {node}");
    }
}

#[test]
fn the_fold_calls_write_only_fms_memory() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::opened_primed(&bytes);
    let (n, k) = mixed(&model, &bytes);
    let expr = model.expr(n, k);
    assert_eq!(fm.statement(n, k).status(), 0);
    let tokens = fm.memory("FM_TOKENS", 4 * expr.len());
    for ei in [false, true] {
        fm.rig.ei = ei;
        assert_eq!(fm.fold(0, 1).status(), 0);
        assert_eq!(fm.folded(0).status(), 0);
        for width in widths() {
            assert_eq!(fm.rows(width).status(), 0);
            assert_eq!(fm.draw(1, -1, width, (0, 8)).status(), 0);
            for node in 0..expr.len() {
                assert_eq!(fm.cursor(node, 0, 0, width, (0, 8)).status(), 0);
                assert_eq!(fm.cursor(node, 90, -2, width, (1, 5)).status(), 0);
            }
        }
        assert_eq!(fm.fold(0, 0).status(), 0);
        assert_eq!(fm.fold(255, 0).status(), code("FM_ERR_INDEX"));
    }
    let found = stray(&mut fm.rig, &allowed());
    assert!(found.is_empty(), "stray writes at {found:x?}");
    let at = usize::from(fm.rig.label("FM_FOLDS") - SPARE_BASE);
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    assert!((0..expr.len()).all(|i| page[at + i] == 0), "the flags were cleared");
    assert_ne!(pattern(SPARE_PAGE, at), 0, "the priming pattern is not zero here");
    assert_eq!(fm.rows(94).status(), 0);
    assert_eq!(fm.memory("FM_TOKENS", 4 * expr.len()), tokens);
}
