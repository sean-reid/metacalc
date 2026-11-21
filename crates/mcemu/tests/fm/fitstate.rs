use crate::fit::*;
use crate::fixture;
use crate::guard::stray;
use crate::nodes::Model;
use crate::rig::*;
use crate::session::allowed;
use crate::support::*;
use crate::ui_support::{pack, same_frame};
use mcui::canvas::{CONTENT_ROWS, Canvas, STATEMENT_WIDTH};
use mcui::fold::{Folds, layout};

fn label_at(library: &mcpack::library::Library, label: &str) -> u16 {
    library
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}")) as u16
}

/// An assertion of the fixture with hypotheses whose fit folds something.
fn folding(model: &Model, library: &mcpack::library::Library) -> (u16, usize) {
    for n in 0..library.assertions.len() as u16 {
        let hyps = essentials(library, n);
        if hyps > 1 && model.within(n) {
            let o = oracle(model, n, hyps, CONTENT_ROWS, STATEMENT_WIDTH);
            if o.folds.iter().any(|f| f.count() > 0) {
                return (n, hyps);
            }
        }
    }
    panic!("an assertion with hypotheses that folds");
}

fn fit_at(fm: &mut Fm, n: u16, hyps: usize, rows: usize) -> Out {
    assert_eq!(fm.assertion(n).status(), 0);
    let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
    fm.fit(rows, hyp, concl)
}

fn all_flags(fm: &mut Fm, hyps: usize) -> Vec<Vec<bool>> {
    (0..=hyps)
        .map(|s| {
            assert_eq!(
                fm.select(if s == hyps { 0xFF } else { s as u8 }).status(),
                0
            );
            fm.flags()
        })
        .collect()
}

#[test]
fn fm_fit_twice_gives_the_same_folds() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let (n, hyps) = folding(&model, &library);
    let mut fm = Fm::new(&bytes);
    let first = fit_at(&mut fm, n, hyps, CONTENT_ROWS);
    assert_eq!(first.status(), 0);
    let flags = all_flags(&mut fm, hyps);
    assert!(flags.iter().flatten().any(|&f| f), "something is folded");
    let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
    let second = fm.fit(CONTENT_ROWS, hyp, concl);
    assert_eq!((second.status(), second.hl), (0, first.hl));
    assert_eq!(all_flags(&mut fm, hyps), flags);
    let third = fm.fit(1, hyp, concl);
    let fewer = all_flags(&mut fm, hyps);
    assert_ne!(fewer, flags, "a different number of rows folds differently");
    let back = fm.fit(CONTENT_ROWS, hyp, concl);
    assert_eq!((back.status(), back.hl), (0, first.hl));
    assert_eq!(all_flags(&mut fm, hyps), flags, "{third:?}");
}

#[test]
fn folds_made_by_hand_before_fm_fit_are_gone_after_it() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let n = label_at(&library, "ax-2");
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(n).status(), 0);
    let nodes = fm.nodes();
    assert_eq!(nodes.status(), 0);
    let hand = fm.fold(0, 1);
    assert_eq!((hand.status(), count(hand)), (0, 1));
    assert_eq!(fm.rows(STATEMENT_WIDTH).bc >> 8, 1);
    let folded = fm.rows(30);
    let hand_rows = count(folded);
    let out = fm.fit(CONTENT_ROWS, STATEMENT_WIDTH, STATEMENT_WIDTH);
    assert_eq!(out.status(), 0);
    assert_eq!(out.hl, model_rows_of(&model, n, STATEMENT_WIDTH) as u16);
    assert_eq!(fm.flags(), vec![false; count(nodes)]);
    let after = fm.rows(30);
    assert_ne!(count(after), hand_rows, "the fold the hand made hid lines");
}

fn model_rows_of(model: &Model, n: u16, width: usize) -> usize {
    let expr = model.expr(n, 0xFF);
    layout(&expr, &mcui::fold::Folds::none(&expr), width).rows()
}

#[test]
fn after_fm_fit_the_conclusion_is_selected_and_the_cursor_is_cleared() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let (n, hyps) = folding(&model, &library);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(n).status(), 0);
    assert_eq!(fm.select(0).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    assert_eq!(fm.cursor(0).status(), 0);
    assert_eq!(fm.memory("FV_CURN", 1)[0], 0);
    let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
    assert_eq!(fm.fit(CONTENT_ROWS, hyp, concl).status(), 0);
    assert_eq!(
        fm.memory("FV_CURN", 1)[0],
        NO_CURSOR,
        "the cursor is cleared"
    );
    let o = oracle(&model, n, hyps, CONTENT_ROWS, STATEMENT_WIDTH);
    fm.clear();
    assert_eq!(fm.draw(3, 1, concl, (1, 7)).status(), 0);
    let mut canvas = Canvas::default();
    let laid = layout(&o.exprs[hyps], &o.folds[hyps], concl);
    canvas.layout(&laid, 3, 1, 1..7);
    same_frame(
        "the conclusion alone",
        &fm.frame(),
        &pack(&canvas.into_bitmap()),
    );
    assert_eq!(fm.select(0xFF).status(), 0);
    assert_eq!(
        fm.cursor(0).status(),
        0,
        "the table is built and the cursor can be set"
    );
}

#[test]
fn fm_fit_answers_with_the_documented_statuses() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let mut closed = Fm::bare(&bytes);
    assert_eq!(closed.fit(6, 90, 92).status(), code("FM_ERR_NOT_OPEN"));
    let mut fm = Fm::new(&bytes);
    assert_eq!(
        fm.fit(6, 90, 92).status(),
        code("FM_ERR_INDEX"),
        "nothing loaded"
    );
    let n = label_at(&library, "ax-2");
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    let alone = fm.fit(6, 90, 92);
    let model = Model::new(&bytes);
    let rows = model_rows_of(&model, n, 92) as u16;
    assert_eq!(
        (alone.status(), alone.hl),
        (0, rows),
        "a store of one conclusion fits"
    );
    let hyp = essentials(&library, label_at(&library, "syl3"));
    assert!(hyp > 0);
    assert_eq!(fm.statement(label_at(&library, "syl3"), 0).status(), 0);
    assert_eq!(
        fm.fit(6, 90, 92).status(),
        code("FM_ERR_INDEX"),
        "a store of one hypothesis has no conclusion"
    );
}

#[test]
fn fm_fit_writes_only_fms_memory_and_keeps_the_ports_the_stack_and_the_interrupt_state() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let (n, hyps) = folding(&model, &library);
    let mut fm = Fm::opened_primed(&bytes);
    for ei in [false, true] {
        fm.rig.ei = ei;
        let out = fit_at(&mut fm, n, hyps, CONTENT_ROWS);
        assert_eq!(out.status(), 0);
        assert_eq!(out.iff, u8::from(ei), "interrupt flag");
        assert_eq!((out.port5, out.port7), (0, 0x81));
        assert_eq!(fm.rig.m.regs().sp, fm.rig.sp, "SP");
        assert_ne!(out.af & 0x40, 0, "the zero flag is set");
    }
    let found = stray(&mut fm.rig, &allowed());
    assert!(found.is_empty(), "stray writes at {found:x?}");
}

#[test]
fn the_cached_row_count_follows_a_fold_a_width_and_a_reload() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let n = label_at(&library, "longest");
    let expr = model.expr(n, 0xFF);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fit_at(&mut fm, n, 0, CONTENT_ROWS).status(), 0);
    let fitted = oracle(&model, n, 0, CONTENT_ROWS, STATEMENT_WIDTH)
        .folds
        .remove(0);
    let rows_of = |folds: &Folds, width| layout(&expr, folds, width).rows();
    let node = (1..expr.len())
        .find(|&i| expr.foldable(i) && !fitted.is_folded(i))
        .expect("a foldable node the fit left open");
    let mut folds = fitted.clone();
    folds.toggle(&expr, node);
    assert_ne!(rows_of(&folds, 92), rows_of(&fitted, 92));
    assert_eq!(count(fm.rows(92)), rows_of(&fitted, 92), "read back");
    let set = fm.fold(node, 1);
    assert_eq!((set.status(), count(set)), (0, 1));
    assert_eq!(count(fm.rows(92)), rows_of(&folds, 92), "after a fold");
    assert_eq!(fm.fold(node, 0).status(), 0);
    assert_eq!(
        count(fm.rows(92)),
        rows_of(&fitted, 92),
        "after it is cleared"
    );
    for width in [60, 92, 30, 92] {
        assert_eq!(
            count(fm.rows(width)),
            rows_of(&fitted, width),
            "width {width}"
        );
    }
    let again = fit_at(&mut fm, n, 0, CONTENT_ROWS);
    assert_eq!(usize::from(again.hl), rows_of(&fitted, 92));
    let other = label_at(&library, "ax-2");
    assert_eq!(fm.assertion(other).status(), 0);
    assert_eq!(
        count(fm.rows(92)),
        model_rows_of(&model, other, 92),
        "another statement in the same place"
    );
}

#[test]
fn a_row_count_read_back_costs_a_fraction_of_one_laid_out() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let n = label_at(&library, "longest");
    let mut fm = Fm::new(&bytes);
    let zero = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    assert_eq!(fit_at(&mut fm, n, 0, CONTENT_ROWS).status(), 0);
    let hit = fm.rows(92).tstates - zero;
    let laid_out = fm.rows(60).tstates - zero;
    println!("fm_rows of the longest statement: read back {hit}, laid out {laid_out}");
    assert!(hit <= READ_BACK, "a read back took {hit}");
    assert!(laid_out > 10 * hit, "laid out {laid_out}, read back {hit}");
}

/// A little above what fm_rows costs when the count is cached.
const READ_BACK: u32 = 980;

#[test]
fn an_interrupt_in_fm_fit_changes_nothing() {
    let bytes = fixture::nodes_library();
    let (model, library) = (Model::new(&bytes), decoded(&bytes));
    let n = label_at(&library, "syl3");
    let hyps = essentials(&library, n);
    let mut fm = Fm::new(&bytes);
    fm.rig.install_handler();
    let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
    let regs = Regs {
        bc: 2 << 8,
        de: ((hyp as u16) << 8) | concl as u16,
        ..Regs::default()
    };
    assert!(model.within(n));
    fm.rig.ei = true;
    assert_eq!(fm.assertion(n).status(), 0);
    let want = fm.rig.call("FM_FIT", regs);
    assert_eq!(want.status(), 0);
    let flags = all_flags(&mut fm, hyps);
    let rows = fm.rows(94).bc;
    let (start, end) = crate::session::span(&mut fm, "FM_FIT", regs);
    let mut trials = 0;
    for after in (start..=start + 450).chain(end.saturating_sub(450)..=end) {
        assert_eq!(fm.assertion(n).status(), 0);
        let (out, taken) = fm.rig.call_interrupted("FM_FIT", regs, after);
        let what = format!("FM_FIT, interrupt after {after} of {start} to {end}");
        assert_eq!(taken, 1, "{what}: taken");
        assert_eq!(
            (out.af >> 8, out.bc, out.hl),
            (want.af >> 8, want.bc, want.hl),
            "{what}"
        );
        assert_eq!((out.port5, out.iff), (0, 1), "{what}");
        assert_eq!(fm.rig.m.regs().sp, fm.rig.sp, "{what}: SP");
        assert_eq!(all_flags(&mut fm, hyps), flags, "{what}: folds");
        assert_eq!(fm.rows(94).bc, rows, "{what}: rows");
        trials += 1;
    }
    assert!(trials > 800, "{trials} trials");
}
