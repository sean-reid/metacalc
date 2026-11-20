use crate::folds::put;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH};
use mcui::fold::Folds;

const BUDGET: u32 = 50 * 6000;

/// T-state ceilings, a little above what each took.
pub struct Ceilings {
    pub cursor_root: u32,
    pub cursor_child: u32,
    pub apply_one: u32,
    pub apply_all: u32,
    pub fold: u32,
}

pub const FIXTURE: Ceilings = Ceilings {
    cursor_root: 65_000,
    cursor_child: 65_000,
    apply_one: 128_500,
    apply_all: 81_500,
    fold: 2_900,
};
pub const RING: Ceilings = Ceilings {
    cursor_root: 63_500,
    cursor_child: 62_000,
    apply_one: 51_000,
    apply_all: 32_500,
    fold: 2_900,
};

fn base(fm: &mut Fm) -> u32 {
    fm.rig.call("CALL_NOP", Regs::default()).tstates
}

/// What fm_cursor costs on node 0 and on the child of node 0 with the most tokens, drawn after
/// fm_draw of the statement, and what the whole frame comes to with it.
pub fn cursor_costs(name: &str, bytes: &[u8], at: (u16, u8), frame: u32, ceilings: &Ceilings) {
    let mut fm = Fm::new(bytes);
    let zero = base(&mut fm);
    let table = fm.built(at.0, at.1);
    let child = (1..table.len())
        .filter(|&i| table[i].2 == 1)
        .max_by_key(|&i| table[i].1 - table[i].0)
        .unwrap_or(0);
    let mut costs = Vec::new();
    for node in [0, child] {
        fm.clear();
        assert_eq!(fm.draw(GUTTER, 1, STATEMENT_WIDTH, (1, 7)).status(), 0);
        let out = fm.cursor(node, GUTTER, 1, STATEMENT_WIDTH, (1, 7));
        assert_eq!(out.status(), 0);
        costs.push(out.tstates - zero);
    }
    println!(
        "{name}: fm_cursor on node 0 {}, on node {child} (the largest child, {} tokens) {}; the frame \
         with the cursor on node 0 is {} T-states, with the child {}, against {BUDGET}",
        costs[0],
        table[child].1 - table[child].0 + 1,
        costs[1],
        frame + costs[0],
        frame + costs[1]
    );
    assert!(costs[0] <= ceilings.cursor_root, "cursor on node 0 took {}", costs[0]);
    assert!(costs[1] <= ceilings.cursor_child, "cursor on the child took {}", costs[1]);
}

/// The pass that applies folds, on the statement with the most tokens: the rows call after one
/// fold less the same call with nothing to apply, the same with every foldable node folded, and
/// fm_fold alone.
pub fn apply_costs(name: &str, bytes: &[u8], ceilings: &Ceilings) {
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
    let mut spent = Vec::new();
    for folds in [&one, &every] {
        put(&mut fm, &expr, &none, folds);
        let out = fm.rows(94);
        assert_eq!(out.status(), 0);
        spent.push(out.tstates - zero - plain);
        put(&mut fm, &expr, folds, &none);
        fm.rows(94);
    }
    let fold = fm.fold(0, 2).tstates - zero;
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
    assert!(fold <= ceilings.fold, "fm_fold: {fold}");
}
