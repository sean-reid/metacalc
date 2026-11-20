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
    pub frame_root: u32,
    pub frame_child: u32,
    pub apply_one: u32,
    pub apply_all: u32,
    pub apply_depth: u32,
    pub fold: u32,
}

pub const FIXTURE: Ceilings = Ceilings {
    set: 1_000,
    frame_root: 298_100,
    frame_child: 299_400,
    apply_one: 128_500,
    apply_all: 81_500,
    apply_depth: 128_500,
    fold: 3_300,
};
pub const RING: Ceilings = Ceilings {
    set: 1_000,
    frame_root: 292_900,
    frame_child: 293_000,
    apply_one: 51_000,
    apply_all: 32_500,
    apply_depth: 50_500,
    fold: 3_300,
};

fn base(fm: &mut Fm) -> u32 {
    fm.rig.call("CALL_NOP", Regs::default()).tstates
}

/// What fm_cursor costs alone, and the frame `frame` comes to when `draw` of it draws the cursor
/// as well, on node 0 and on the child of node 0 with the most tokens, in the fm_draw of the
/// statement the screens call. The frame is the calls of a screen, clear to blit, and `draw` is
/// the share of it that is fm_draw.
pub fn cursor_costs(
    name: &str,
    bytes: &[u8],
    at: (u16, u8),
    (frame, draw): (u32, u32),
    ceilings: &Ceilings,
) {
    let mut fm = Fm::new(bytes);
    let zero = base(&mut fm);
    let table = fm.built(at.0, at.1);
    let child = (1..table.len())
        .filter(|&i| table[i].2 == 1)
        .max_by_key(|&i| table[i].1 - table[i].0)
        .unwrap_or(0);
    let mut costs = Vec::new();
    for node in [0, child] {
        let set = fm.cursor(node as u8);
        assert_eq!(set.status(), 0);
        fm.clear();
        let out = fm.draw(GUTTER, 1, STATEMENT_WIDTH, (1, 7));
        assert_eq!(out.status(), 0);
        costs.push((set.tstates - zero, out.tstates - zero));
    }
    assert_eq!(fm.cursor(NO_CURSOR).status(), 0);
    fm.clear();
    let plain = fm.draw(GUTTER, 1, STATEMENT_WIDTH, (1, 7)).tstates - zero;
    assert_eq!(plain, draw, "fm_draw with no cursor");
    let frames: Vec<u32> = costs
        .iter()
        .map(|&(set, drawn)| frame - draw + set + drawn)
        .collect();
    println!(
        "{name}: fm_cursor alone {}; fm_draw with the cursor on node 0 {} and on node {child} (the \
         largest child, {} tokens) {}, against {draw} with none; the frame with the cursor on node 0 \
         is {} T-states, with the child {}, against {BUDGET}",
        costs[0].0,
        costs[0].1,
        table[child].1 - table[child].0 + 1,
        costs[1].1,
        frames[0],
        frames[1]
    );
    assert!(
        costs[0].0 <= ceilings.set && costs[1].0 <= ceilings.set,
        "fm_cursor took {costs:?}"
    );
    for (what, took, ceiling) in [
        ("node 0", frames[0], ceilings.frame_root),
        ("the child", frames[1], ceilings.frame_child),
    ] {
        assert!(took < BUDGET, "the frame with the cursor on {what}: {took}");
        assert!(
            took <= ceiling,
            "the frame with the cursor on {what} took {took}, over {ceiling}"
        );
    }
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
