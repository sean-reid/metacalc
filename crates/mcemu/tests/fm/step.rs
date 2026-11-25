use crate::fit::{essentials, oracle, widths_of};
use crate::fixture;
use crate::folds::{fold_sets, hidden, picks, put, widths};
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcui::canvas::STATEMENT_WIDTH;
use mcui::fold::{Folds, layout};

const SEED_A: u8 = 0xA5;
const SEED_BC: u16 = 0x7E7E;
const NONE: u8 = 0xFF;

impl Fm {
    /// fm_step of `node` in direction `dir`, from registers a bare `ret` would hand back as a
    /// failure: the node found is `bc >> 8`.
    pub fn step(&mut self, node: usize, dir: u8) -> Out {
        let regs = Regs {
            a: SEED_A,
            bc: SEED_BC,
            de: (u16::from(dir) << 8) | node as u16,
            ..Regs::default()
        };
        self.rig.call("FM_STEP", regs)
    }

    /// fm_span of `node` at `width`: the first line is `bc >> 8`, one past the last `bc & 255`.
    pub fn span(&mut self, node: usize, width: usize) -> Out {
        let regs = Regs {
            a: SEED_A,
            bc: SEED_BC,
            de: ((width as u16) << 8) | node as u16,
            ..Regs::default()
        };
        self.rig.call("FM_SPAN", regs)
    }
}

/// The model's answer for each direction: parent, first child, next and previous sibling.
fn model_step(expr: &mcui::expr::Expr, node: usize, dir: u8) -> Option<usize> {
    match dir {
        0 => expr.parent(node),
        1 => expr.children(node).first().copied(),
        2 => expr.sibling(node, true),
        _ => expr.sibling(node, false),
    }
}

#[derive(Default)]
struct Steps {
    statements: usize,
    found: [usize; 4],
    none: [usize; 4],
    ticks: Vec<u32>,
}

fn steps_of(bytes: &[u8], every: bool) -> Steps {
    let model = Model::new(bytes);
    let mut fm = Fm::new(bytes);
    let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let mut tally = Steps::default();
    let all = statements(&decoded(bytes));
    let largest = all.iter().map(|(_, _, t)| t.len()).max().unwrap();
    for (n, k, tokens) in &all {
        let expr = model.expr(*n, *k);
        let table = fm.built(*n, *k);
        assert_eq!(table.len(), expr.len());
        tally.statements += 1;
        let whole = every || tokens.len() == largest || expr.len() <= 40;
        let nodes: Vec<usize> = if whole {
            (0..expr.len()).collect()
        } else {
            picks(&expr, &Folds::none(&expr), 0)
        };
        for node in nodes {
            for dir in 0..4u8 {
                let want = model_step(&expr, node, dir);
                let out = fm.step(node, dir);
                let what = format!("statement {k} of assertion {n}, node {node}, direction {dir}");
                assert_eq!(out.status(), 0, "{what}");
                assert_eq!(out.af & 0x40, 0x40, "{what}: zero flag");
                assert_eq!(
                    (out.bc >> 8) as u8,
                    want.map_or(NONE, |w| w as u8),
                    "{what}"
                );
                match want {
                    Some(_) => tally.found[usize::from(dir)] += 1,
                    None => tally.none[usize::from(dir)] += 1,
                }
                if tokens.len() == largest {
                    tally.ticks.push(out.tstates - base);
                }
            }
        }
    }
    tally
}

#[test]
fn fm_step_is_the_models_parent_child_and_siblings_for_every_node_of_the_fixture() {
    let tally = steps_of(&fixture::nodes_library(), true);
    assert!(tally.statements > 90, "{} statements", tally.statements);
    for dir in 0..4 {
        assert!(tally.found[dir] > 200, "direction {dir}: {:?}", tally.found);
        assert!(tally.none[dir] > 90, "direction {dir}: {:?}", tally.none);
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn fm_step_is_the_models_on_the_shipped_libraries() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let tally = steps_of(&bytes, name == "prop");
        let mut ticks = tally.ticks.clone();
        ticks.sort_unstable();
        println!(
            "{name}: fm_step over every node of its largest statement: median {}, worst {}",
            ticks[ticks.len() / 2],
            ticks.last().unwrap()
        );
        for dir in 0..4 {
            assert!(tally.found[dir] > 100, "{name}: {:?}", tally.found);
            assert!(tally.none[dir] > 100, "{name}: {:?}", tally.none);
        }
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn fm_span_is_the_models_on_the_shipped_libraries() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let model = Model::new(&bytes);
        let mut fm = Fm::new(&bytes);
        let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        let all = statements(&decoded(&bytes));
        let largest = all.iter().map(|(_, _, t)| t.len()).max().unwrap();
        let (mut ticks, mut t) = (Vec::new(), Spans::default());
        for (n, k, tokens) in &all {
            let expr = model.expr(*n, *k);
            fm.built(*n, *k);
            let whole = tokens.len() == largest;
            let nodes = if whole {
                (0..expr.len()).collect()
            } else {
                picks(&expr, &Folds::none(&expr), 0)
            };
            let none = Folds::none(&expr);
            let what = format!("{name}: statement {k} of assertion {n}");
            check_span(
                &mut fm,
                &expr,
                &none,
                STATEMENT_WIDTH,
                &nodes,
                &what,
                &mut t,
            );
            if whole {
                for node in 0..expr.len() {
                    ticks.push(fm.span(node, STATEMENT_WIDTH).tstates - base);
                }
            }
        }
        ticks.sort_unstable();
        println!(
            "{name}: {} spans compared; fm_span over every node of its largest statement: median {}, worst {}",
            t.compared,
            ticks[ticks.len() / 2],
            ticks.last().unwrap()
        );
        assert!(
            t.compared > 1000 && t.multi > 20,
            "{name}: {} spans, {} over several lines",
            t.compared,
            t.multi
        );
    }
}

#[test]
fn fm_step_refuses_what_it_cannot_answer_and_a_failure_leaves_the_registers_it_was_given_no_answer_in()
 {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let all = statements(&decoded(&bytes));
    let (n, k, _) = all
        .iter()
        .find(|(n, k, _)| model.expr(*n, *k).len() > 3)
        .unwrap();
    let mut fm = Fm::bare(&bytes);
    assert_eq!(fm.step(0, 0).status(), code("FM_ERR_NOT_OPEN"), "closed");
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    for dir in 0..4 {
        let out = fm.step(0, dir);
        assert_eq!(out.status(), code("FM_ERR_INDEX"), "table not built, {dir}");
    }
    let out = fm.step(0, 4);
    assert_eq!(
        out.status(),
        code("FM_ERR_BAD"),
        "direction 4 before the table"
    );
    let out = fm.step(0, 0xFF);
    assert_eq!(out.status(), code("FM_ERR_BAD"), "direction FFh");
    let nodes = count(fm.nodes());
    for dir in 0..4 {
        assert_eq!(
            fm.step(nodes, dir).status(),
            code("FM_ERR_INDEX"),
            "node {nodes}"
        );
        assert_eq!(fm.step(255, dir).status(), code("FM_ERR_INDEX"), "node 255");
    }
    assert_eq!(fm.step(nodes - 1, 4).status(), code("FM_ERR_BAD"));
    assert_eq!(
        fm.step(nodes, 4).status(),
        code("FM_ERR_BAD"),
        "BAD before INDEX"
    );
    let before = (fm.header().nodes, fm.flags());
    assert_eq!(fm.step(1, 1).status(), 0);
    assert_eq!((fm.header().nodes, fm.flags()), before, "nothing written");
}

/// The model's first line and one past its last line for `node`.
fn model_span(expr: &mcui::expr::Expr, folds: &Folds, width: usize, node: usize) -> (usize, usize) {
    let rects = layout(expr, folds, width).cursor(expr, node);
    (rects.first().unwrap().row, rects.last().unwrap().row + 1)
}

#[derive(Default)]
struct Spans {
    compared: usize,
    hidden: usize,
    multi: usize,
    folded_sets: usize,
    after_fit: usize,
}

fn check_span(
    fm: &mut Fm,
    expr: &mcui::expr::Expr,
    folds: &Folds,
    width: usize,
    nodes: &[usize],
    what: &str,
    t: &mut Spans,
) {
    for &node in nodes {
        let out = fm.span(node, width);
        let want = model_span(expr, folds, width, node);
        let what = format!("{what}, width {width}, node {node}");
        assert_eq!(out.status(), 0, "{what}");
        assert_eq!(
            (usize::from((out.bc >> 8) as u8), usize::from(out.bc as u8)),
            want,
            "{what}"
        );
        t.compared += 1;
        t.hidden += usize::from(hidden(expr, folds, node));
        t.multi += usize::from(want.1 - want.0 > 1);
    }
}

#[test]
fn fm_span_is_the_models_first_and_last_cursor_line_unfolded_and_under_folds() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let mut t = Spans::default();
    for (n, k, _) in statements(&decoded(&bytes)) {
        let expr = model.expr(n, k);
        fm.built(n, k);
        let mut sets = fold_sets(&expr, u64::from(n) * 31 + u64::from(k), expr.len() <= 12);
        let big = expr.len() > 40;
        if big {
            sets.truncate(3);
        }
        let mut now = Folds::none(&expr);
        for (name, folds) in &sets {
            put(&mut fm, &expr, &now, folds);
            now = folds.clone();
            t.folded_sets += usize::from(name != "none");
            let nodes = picks(&expr, folds, 40);
            for width in widths().into_iter().take(if big { 2 } else { 4 }) {
                let what = format!("statement {k} of assertion {n}, folds {name}");
                check_span(&mut fm, &expr, folds, width, &nodes, &what, &mut t);
            }
        }
    }
    println!(
        "{} spans, {} hidden, {} over several lines, {} fold sets",
        t.compared, t.hidden, t.multi, t.folded_sets
    );
    assert!(t.compared > 3000, "{} spans", t.compared);
    assert!(t.hidden > 300, "{} hidden", t.hidden);
    assert!(t.multi > 300, "{} spanning lines", t.multi);
    assert!(t.folded_sets > 150, "{} fold sets", t.folded_sets);
}

#[test]
fn fm_span_follows_a_fit_that_folded_and_a_fold_of_an_ancestor_made_after_it() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    let mut t = Spans::default();
    let mut folded = 0;
    for n in 0..library.assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let hyps = essentials(&library, n);
        assert_eq!(fm.assertion(n).status(), 0);
        let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
        assert_eq!(fm.fit(6, hyp, concl).status(), 0);
        let want = oracle(&model, n, hyps, 6, STATEMENT_WIDTH);
        for s in 0..=hyps {
            let k = if s == hyps { 0xFF } else { s as u8 };
            assert_eq!(fm.select(k).status(), 0);
            let expr = &want.exprs[s];
            let width = want.widths[s];
            let nodes = picks(expr, &want.folds[s], 40);
            let what = format!("assertion {n}, statement {s} after the fit");
            check_span(&mut fm, expr, &want.folds[s], width, &nodes, &what, &mut t);
            folded += usize::from(want.folds[s].count() > 0);
            t.after_fit += nodes.len();
            let mut now = want.folds[s].clone();
            if let Some(node) = (0..expr.len())
                .rev()
                .find(|&m| expr.foldable(m) && !now.is_folded(m) && !hidden(expr, &now, m))
            {
                now.toggle(expr, node);
                assert_eq!(fm.fold(node, 1).status(), 0);
                let what =
                    format!("assertion {n}, statement {s}, node {node} folded after the fit");
                check_span(&mut fm, expr, &now, width, &nodes, &what, &mut t);
            }
        }
    }
    println!(
        "{folded} statements folded by the fit, {} spans after a fit, {} hidden",
        t.after_fit, t.hidden
    );
    assert!(folded >= 6, "{folded} statements the fit folded");
    assert!(t.after_fit > 500, "{} spans after a fit", t.after_fit);
    assert!(t.hidden > 50, "{} hidden", t.hidden);
}

#[test]
fn fm_span_refuses_what_it_cannot_answer() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let all = statements(&decoded(&bytes));
    let (n, k, _) = all
        .iter()
        .find(|(n, k, _)| model.expr(*n, *k).len() > 3)
        .unwrap();
    let mut fm = Fm::bare(&bytes);
    assert_eq!(fm.span(0, 94).status(), code("FM_ERR_NOT_OPEN"), "closed");
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    assert_eq!(
        fm.span(0, 94).status(),
        code("FM_ERR_INDEX"),
        "table not built"
    );
    let nodes = count(fm.nodes());
    assert_eq!(
        fm.span(nodes, 94).status(),
        code("FM_ERR_INDEX"),
        "past the table"
    );
    assert_eq!(fm.cursor(2).status(), 0);
    let at = fm.memory("FV_CURN", 1);
    assert_eq!(fm.span(1, 94).status(), 0);
    assert_eq!(fm.memory("FV_CURN", 1), at, "the cursor is left as it was");
    assert_eq!(fm.clip_to(1, 30), 0);
    assert_eq!(fm.span(0, 30).status(), code("FM_ERR_CLIPPED"), "clipped");
}

impl Fm {
    /// fm_clip of the selected statement to `rows` lines at `width`; returns the status.
    pub fn clip_to(&mut self, rows: usize, width: usize) -> u8 {
        let regs = Regs {
            bc: (rows as u16) << 8,
            de: (width as u16) << 8,
            ..Regs::default()
        };
        self.rig.call("FM_CLIP", regs).status()
    }
}

/// The bytes fm's private stack takes while `call` runs, from a statement of the fixture loaded and
/// its table built.
fn stack_used(call: impl Fn(&mut Fm, usize)) -> usize {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let all = statements(&decoded(&bytes));
    let mut deepest = 0;
    for mark in [0xA5u8, 0x5A] {
        let mut fm = Fm::new(&bytes);
        let top = fm.rig.label("FM_STACK_TOP") - SPARE_BASE;
        let low = usize::from(fm.rig.label("FM_STACK") - SPARE_BASE);
        for (n, k, _) in all.iter().step_by(3) {
            let nodes = model.expr(*n, *k).len();
            fm.built(*n, *k);
            fm.rig.m.ram_page(SPARE_PAGE)[low..usize::from(top)].fill(mark);
            for node in 0..nodes.min(30) {
                call(&mut fm, node);
            }
            let page = fm.rig.m.ram_page(SPARE_PAGE);
            let unused = page[low..usize::from(top)]
                .iter()
                .take_while(|&&b| b == mark)
                .count();
            deepest = deepest.max(usize::from(top) - low - unused);
        }
    }
    deepest
}

#[test]
fn fm_step_and_fm_span_write_this_much_of_fms_stack() {
    let step = stack_used(|fm, node| {
        for dir in 0..4 {
            assert_eq!(fm.step(node, dir).status(), 0);
        }
    });
    let span = stack_used(|fm, node| {
        for width in [94, 30] {
            assert_eq!(fm.span(node, width).status(), 0);
        }
    });
    println!("fm_step reaches {step} bytes of fm's stack, fm_span {span}");
    assert_eq!((step, span), (4, 6));
}
