//! What fm_step and fm_span leave alone: fm's tables, folds, selection and cursor, snapshotted from
//! the build's label file around a good call and a refused one, and the order of their statuses on a
//! closed fm.

use crate::fixture;
use crate::folds::hidden;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcui::canvas::STATEMENT_WIDTH;
use mcui::fold::Folds;

/// fm's tables, node planes, token records, selection and cursor, as the label file places them.
fn snapshot(fm: &mut Fm) -> Vec<(&'static str, Vec<u8>)> {
    [
        ("FM_HDR", 0x100),
        ("FM_FIRST", 0x180),
        ("FM_LAST", 0x180),
        ("FM_DEPTH", 0x180),
        ("FM_FOLDS", 0x180),
        ("FM_TOKENS", 0xC00),
        ("FV_HDR", 2),
        ("FV_CURN", 1),
    ]
    .into_iter()
    .map(|(name, len)| (name, fm.memory(name, len)))
    .collect()
}

/// A statement of the fixture with several nodes, loaded, built, with two folds applied and a cursor.
fn prepared() -> (Fm, usize) {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let all = statements(&decoded(&bytes));
    let (n, k, _) = all
        .iter()
        .find(|(n, k, _)| {
            let expr = model.expr(*n, *k);
            (0..expr.len()).filter(|&m| expr.foldable(m)).count() >= 3
        })
        .expect("a statement with folds to make");
    let mut fm = Fm::new(&bytes);
    let nodes = fm.built(*n, *k).len();
    let expr = model.expr(*n, *k);
    let foldable: Vec<usize> = (0..expr.len()).filter(|&m| expr.foldable(m)).collect();
    for &node in &foldable[1..3] {
        assert_eq!(fm.fold(node, 1).status(), 0);
    }
    assert_eq!(fm.rows(94).status(), 0, "the folds are applied");
    assert_eq!(fm.cursor(2).status(), 0);
    (fm, nodes)
}

#[test]
fn a_good_call_and_a_refused_call_of_fm_step_and_fm_span_leave_the_tables_folds_selection_and_cursor()
 {
    let (mut fm, nodes) = prepared();
    let before = snapshot(&mut fm);
    assert!(
        before
            .iter()
            .any(|(_, bytes)| bytes.iter().any(|&b| b != 0))
    );
    assert_eq!(fm.step(1, 1).status(), 0);
    assert_eq!(snapshot(&mut fm), before, "a good fm_step");
    assert_eq!(fm.span(0, 94).status(), 0);
    assert_eq!(snapshot(&mut fm), before, "a good fm_span");
    let out = fm.step(1, 4);
    assert_eq!(out.status(), code("FM_ERR_BAD"));
    assert_eq!(snapshot(&mut fm), before, "fm_step with D above 3");
    assert_eq!(fm.step(nodes, 0).status(), code("FM_ERR_INDEX"));
    assert_eq!(snapshot(&mut fm), before, "fm_step past the table");
    assert_eq!(fm.span(nodes, 94).status(), code("FM_ERR_INDEX"));
    assert_eq!(snapshot(&mut fm), before, "fm_span past the table");
}

#[test]
fn a_closed_fm_answers_not_open_to_fm_step_before_it_looks_at_the_direction() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::bare(&bytes);
    assert_eq!(
        fm.step(0, 4).status(),
        code("FM_ERR_NOT_OPEN"),
        "D = 4, closed"
    );
    assert_eq!(
        fm.step(0, 0).status(),
        code("FM_ERR_NOT_OPEN"),
        "D = 0, closed"
    );
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.step(0, 4).status(), code("FM_ERR_BAD"), "D = 4, open");
}

/// The bytes of fm's stack one call of `call` takes after `prepare`, over the statements of the
/// fixture with at least two folds of their own to make.
fn stack_after(
    prepare: impl Fn(&mut Fm, &mcui::expr::Expr, &[usize]),
    call: impl Fn(&mut Fm, &mcui::expr::Expr, &[usize]),
) -> (usize, usize) {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut deepest = 0;
    let mut cases = 0;
    for (n, k, _) in statements(&decoded(&bytes)) {
        let expr = model.expr(n, k);
        let mut outer: Vec<usize> = Vec::new();
        let mut skip_to = 0;
        for node in 1..expr.len() {
            if node >= skip_to && expr.foldable(node) {
                outer.push(node);
                skip_to = (node + 1..expr.len())
                    .find(|&m| expr.depth(m) <= expr.depth(node))
                    .unwrap_or(expr.len());
            }
        }
        if outer.len() < 2 {
            continue;
        }
        let mut fm = Fm::new(&bytes);
        fm.built(n, k);
        prepare(&mut fm, &expr, &outer[..2]);
        let top = fm.rig.label("FM_STACK_TOP") - SPARE_BASE;
        let low = usize::from(fm.rig.label("FM_STACK") - SPARE_BASE);
        fm.rig.m.ram_page(SPARE_PAGE)[low..usize::from(top)].fill(0xA5);
        call(&mut fm, &expr, &outer[..2]);
        let page = fm.rig.m.ram_page(SPARE_PAGE);
        let unused = page[low..usize::from(top)]
            .iter()
            .take_while(|&&b| b == 0xA5)
            .count();
        deepest = deepest.max(usize::from(top) - low - unused);
        cases += 1;
    }
    (deepest, cases)
}

#[test]
fn fm_span_writes_this_much_of_fms_stack_with_a_hidden_node_and_with_stale_folds() {
    let fold_two = |fm: &mut Fm, _: &mcui::expr::Expr, outer: &[usize]| {
        for &node in outer {
            assert_eq!(fm.fold(node, 1).status(), 0);
        }
    };
    let (stale, cases) = stack_after(fold_two, |fm, _, _| {
        assert_eq!(fm.span(0, 94).status(), 0);
    });
    let (hidden_node, hidden_cases) = stack_after(
        |fm, expr, outer| {
            fold_two(fm, expr, outer);
            assert_eq!(fm.rows(94).status(), 0);
        },
        |fm, expr, outer| {
            let mut hit = 0;
            for node in outer[0] + 1..expr.len() {
                if hidden(expr, &folds_of(expr, outer), node) {
                    assert_eq!(fm.span(node, 94).status(), 0);
                    hit += 1;
                }
            }
            assert!(hit > 0, "a node is hidden");
        },
    );
    let (both, both_cases) = stack_after(fold_two, |fm, expr, outer| {
        let mut hit = 0;
        for node in outer[0] + 1..expr.len() {
            if hidden(expr, &folds_of(expr, outer), node) {
                assert_eq!(fm.span(node, 94).status(), 0);
                hit += 1;
            }
        }
        assert!(hit > 0, "a node is hidden");
    });
    println!(
        "fm_span: {stale} bytes with two outermost folds stale ({cases} statements), {hidden_node} with a node hidden ({hidden_cases}), {both} with both ({both_cases})"
    );
    assert_eq!(
        (cases, hidden_cases, both_cases),
        (11, 11, 11),
        "the statements measured"
    );
    assert_eq!((stale, hidden_node, both), (10, 10, 10));
}

fn folds_of(expr: &mcui::expr::Expr, outer: &[usize]) -> Folds {
    let mut folds = Folds::none(expr);
    for &node in outer {
        folds.toggle(expr, node);
    }
    folds
}

/// The most T-states fm_span takes over every node of the largest statement after two outermost folds
/// were cleared and set again (stale), and then again with the folds applied and the node hidden.
pub fn folded_spans(fm: &mut Fm, bytes: &[u8], base: u32) -> (u32, u32) {
    let model = Model::new(bytes);
    let all = statements(&decoded(bytes));
    let largest = all.iter().map(|(_, _, t)| t.len()).max().unwrap();
    let (n, k, _) = all
        .iter()
        .find(|(_, _, t)| t.len() == largest)
        .expect("a largest statement");
    let expr = model.expr(*n, *k);
    fm.built(*n, *k);
    let mut outer: Vec<usize> = Vec::new();
    let mut skip_to = 1;
    for node in 1..expr.len() {
        if node >= skip_to && expr.foldable(node) {
            outer.push(node);
            skip_to = (node + 1..expr.len())
                .find(|&m| expr.depth(m) <= expr.depth(node))
                .unwrap_or(expr.len());
        }
    }
    assert!(!outer.is_empty(), "an outermost fold to make");
    let outer = &outer[..outer.len().min(2)];
    let mut folds = Folds::none(&expr);
    for &node in outer {
        folds.toggle(&expr, node);
    }
    let (mut stale, mut hidden_worst, mut hidden_seen) = (0, 0, 0);
    for node in 0..expr.len() {
        for &o in outer {
            assert_eq!(fm.fold(o, 0).status(), 0);
            assert_eq!(fm.fold(o, 1).status(), 0);
        }
        stale = stale.max(fm.span(node, STATEMENT_WIDTH).tstates - base);
        let again = fm.span(node, STATEMENT_WIDTH).tstates - base;
        if hidden(&expr, &folds, node) {
            hidden_worst = hidden_worst.max(again);
            hidden_seen += 1;
        }
    }
    assert!(hidden_seen > 0, "the largest statement has hidden nodes");
    (stale, hidden_worst)
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_a_statement_outside_the_window_costs_fm_to_select_and_draw() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let all = statements(&decoded(&bytes));
        let largest = all.iter().map(|(_, _, t)| t.len()).max().unwrap();
        let (n, k, _) = all
            .iter()
            .find(|(_, _, t)| t.len() == largest)
            .expect("a largest statement");
        let mut fm = Fm::new(&bytes);
        let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        fm.built(*n, *k);
        assert_eq!(fm.rows(STATEMENT_WIDTH).status(), 0);
        let select = fm.select(if *k == 0xFF { 0xFF } else { *k }).tstates - base;
        let out = fm.draw(3, 100, STATEMENT_WIDTH, (1, 7));
        assert_eq!(out.status(), 0);
        let inside = fm.draw(3, 1, STATEMENT_WIDTH, (1, 7)).tstates - base;
        println!(
            "{name}: the largest statement: fm_select {select}, fm_draw below the window {}, inside it {inside}",
            out.tstates - base
        );
    }
}
