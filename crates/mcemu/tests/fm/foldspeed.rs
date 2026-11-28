use crate::folds::put;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH};
use mcui::fold::Folds;

const BUDGET: u32 = 50 * 6000;

/// T-state ceilings, a little above what each took.
pub struct Ceilings {
    pub set: u32,
    pub frame_worst: u32,
    /// The worst frame's exact cost when it is over the 50 ms. The fixture's is 301,727 where it was
    /// 296,171 before the canvas marked its rows: the blit's list costs 902, the marks of ui_text,
    /// ui_title and ui_soft_keys 3,772 and the cursor's six ui_invert marks 882.
    pub over_budget: Option<u32>,
    pub apply_one: u32,
    pub apply_all: u32,
    pub apply_depth: u32,
    pub fold: u32,
}

pub const FIXTURE: Ceilings = Ceilings {
    set: 730,
    frame_worst: 303_196,
    over_budget: Some(301_727),
    apply_one: 128_500,
    apply_all: 81_500,
    apply_depth: 128_500,
    fold: 3_300,
};
pub const RING: Ceilings = Ceilings {
    set: 730,
    frame_worst: 296_284,
    over_budget: None,
    apply_one: 51_000,
    apply_all: 32_500,
    apply_depth: 50_500,
    fold: 3_300,
};

fn base(fm: &mut Fm) -> u32 {
    fm.rig.call("CALL_NOP", Regs::default()).tstates
}

/// What fm_cursor costs alone, and the frame `frame` comes to when the cursor is on the node that
/// makes fm_draw dearest, found by trying every node of the three heaviest six-row statements.
/// `draw` is the share of `frame` that is fm_draw of the heaviest statement.
pub fn cursor_costs(
    name: &str,
    bytes: &[u8],
    (frame, draw): (u32, u32),
    ceilings: &Ceilings,
) -> (u16, u8, u8) {
    let mut fm = Fm::new(bytes);
    let zero = base(&mut fm);
    let mut worst = (0, 0, 0, 0);
    for (n, k) in heaviest_six_row_statements(bytes, 3) {
        let (node, spent) = worst_cursor(&mut fm, n, k);
        println!(
            "{name}: statement {k} of assertion {n}: fm_draw is dearest with the cursor on node {node}, {spent}"
        );
        if spent > worst.3 {
            worst = (n, k, node, spent);
        }
    }
    let (n, k, node, drawn) = worst;
    fm.built(n, k);
    let set = fm.cursor(node);
    assert_eq!(set.status(), 0);
    let set = set.tstates - zero;
    assert_eq!(fm.cursor(NO_CURSOR).status(), 0);
    fm.clear();
    let top = heaviest_six_row_statements(bytes, 1)[0];
    fm.built(top.0, top.1);
    let plain = fm.draw(GUTTER, 1, STATEMENT_WIDTH, (1, 7)).tstates - zero;
    assert_eq!(plain, draw, "fm_draw with no cursor");
    let total = frame - draw + set + drawn;
    println!(
        "{name}: fm_cursor alone {set}; the worst cursor is node {node} of statement {k} of \
         assertion {n}: fm_draw {drawn} against {draw} with none; the frame with it is {total} \
         T-states against {BUDGET}"
    );
    assert!(set <= ceilings.set, "fm_cursor took {set}");
    match ceilings.over_budget {
        Some(over) => assert_eq!(
            total, over,
            "the frame with the cursor on node {node} is over {BUDGET}"
        ),
        None => assert!(
            total < BUDGET,
            "the frame with the cursor on node {node}: {total}"
        ),
    }
    assert!(
        total <= ceilings.frame_worst,
        "the frame with the cursor on node {node} took {total}, over {}",
        ceilings.frame_worst
    );
    (n, k, node)
}

/// The pass that applies folds, on the statement with the most tokens: the rows call after one
/// fold less the same call with nothing to apply, the same with every foldable node folded, and
/// fm_fold alone. Returns how many disjoint folds the one-depth measurement set.
pub fn apply_costs(name: &str, bytes: &[u8], ceilings: &Ceilings) -> usize {
    let model = Model::new(bytes);
    let all = statements(&decoded(bytes));
    let (n, k, _) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    let expr = model.expr(*n, *k);
    let mut fm = Fm::new(bytes);
    let zero = base(&mut fm);
    fm.built(*n, *k);
    let plain = fm.rows(94).tstates - zero;
    let mut one = Folds::none(&expr);
    let last = (0..expr.len()).rev().find(|&i| expr.foldable(i)).unwrap();
    one.toggle(&expr, last);
    let mut every = Folds::none(&expr);
    for node in (0..expr.len()).filter(|&i| expr.foldable(i)) {
        every.toggle(&expr, node);
    }
    let none = Folds::none(&expr);
    let widest = (0..expr.len())
        .map(|d| {
            (0..expr.len())
                .filter(|&i| expr.depth(i) == d && expr.foldable(i))
                .count()
        })
        .enumerate()
        .max_by_key(|&(_, c)| c)
        .unwrap()
        .0;
    let mut level = Folds::none(&expr);
    for node in (0..expr.len()).filter(|&i| expr.depth(i) == widest && expr.foldable(i)) {
        level.toggle(&expr, node);
    }
    let mut spent = Vec::new();
    for folds in [&one, &every, &level] {
        put(&mut fm, &expr, &none, folds);
        let out = fm.rows(94);
        assert_eq!(out.status(), 0);
        spent.push(out.tstates - zero - plain);
        put(&mut fm, &expr, folds, &none);
        fm.rows(94);
    }
    let fold = fm.fold(0, 2).tstates - zero;
    println!(
        "{name}: {} disjoint folds of depth {widest}: {}",
        level.count(),
        spent[2]
    );
    println!(
        "{name}: the statement of {} tokens and {} nodes: apply pass with one fold {}, with {} nodes folded {}; fm_fold alone {fold}",
        expr.tokens().len(),
        expr.len(),
        spent[0],
        every.count(),
        spent[1]
    );
    assert!(spent[0] <= ceilings.apply_one, "one fold: {}", spent[0]);
    assert!(spent[1] <= ceilings.apply_all, "every fold: {}", spent[1]);
    assert!(
        spent[2] <= ceilings.apply_depth,
        "one depth folded: {}",
        spent[2]
    );
    assert!(fold <= ceilings.fold, "fm_fold: {fold}");
    level.count()
}
