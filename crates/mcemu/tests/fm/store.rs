use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{GUTTER, STATEMENT_WIDTH, text_width};

type Statements = Vec<(u8, Vec<String>)>;

/// The width and x a theorem screen gives statement `k` of an assertion with `hyps` hypotheses:
/// hypotheses are indented by the width of their last number, the conclusion is not.
fn screen(hyps: usize, k: u8) -> (usize, usize) {
    if k == 0xFF {
        return (STATEMENT_WIDTH, GUTTER);
    }
    let indent = text_width(&format!("{hyps}.")) + 2;
    (STATEMENT_WIDTH - indent, GUTTER + indent)
}

/// The statements of each assertion as (number, tokens), in load order.
fn by_assertion(bytes: &[u8]) -> Vec<(u16, Statements)> {
    let mut out: Vec<(u16, Statements)> = Vec::new();
    for (n, k, tokens) in statements(&decoded(bytes)) {
        match out.last_mut() {
            Some((last, all)) if *last == n => all.push((k, tokens)),
            _ => out.push((n, vec![(k, tokens)])),
        }
    }
    out
}

/// Three orders over `m` statements: the first two differ from the load order from two statements
/// on, the third from three.
fn orders(m: usize) -> [Vec<usize>; 3] {
    [
        (0..m).rev().collect(),
        (0..m).map(|i| (i + 1) % m).collect(),
        (0..m).step_by(2).chain((1..m).step_by(2)).collect(),
    ]
}

/// Loads assertion `n`, then selects its statements in three orders that are not the load order,
/// building the node tables of every other one in the first pass and the rest in the second, and
/// checks rows, nodes and with `frames` pixels against the model each time.
fn check_assertion(fm: &mut Fm, model: &Model, n: u16, all: &Statements, frames: bool) {
    let hyps = all.len() - 1;
    let out = fm.assertion(n);
    assert_eq!((out.status(), count(out)), (0, all.len()), "assertion {n}");
    let last = &all[hyps];
    let (width, _) = screen(hyps, 0xFF);
    let rows = fm.rows(width);
    assert_eq!(
        count(rows),
        model_rows(&last.1, width),
        "assertion {n}: the conclusion is selected"
    );
    let invalid = fm.select(hyps as u8);
    assert_eq!(
        (invalid.status(), count(invalid)),
        (code("FM_ERR_INDEX"), 0)
    );
    assert_eq!(
        count(fm.rows(width)),
        count(rows),
        "a refused select keeps it"
    );
    for (pass, order) in orders(all.len()).iter().enumerate() {
        for (step, &i) in order.iter().enumerate() {
            let (k, tokens) = &all[i];
            let what = format!("assertion {n}, statement {k}, pass {pass}");
            let out = fm.select(*k);
            assert_eq!((out.status(), count(out)), (0, tokens.len()), "{what}");
            let (width, x) = screen(hyps, *k);
            let out = fm.rows(width);
            assert_eq!(count(out), model_rows(tokens, width), "{what}: rows");
            if frames {
                fm.clear();
                assert_eq!(fm.draw(x, 0, width, (0, 8)).status(), 0, "{what}");
                let want = model_frame(tokens, width, x, 0, 0..8);
                same_frame(&what, &fm.frame(), &want);
            }
            if pass > 0 || step % 2 == 0 {
                let out = fm.nodes();
                assert_eq!(out.status(), 0, "{what}: nodes");
                assert_eq!(fm.table(count(out)), model.table(n, *k), "{what}: nodes");
            }
        }
    }
}

/// Whether an assertion has several statements that take more than six rows together on the
/// theorem screen: the ones whose frames scroll and hide a stale pointer.
fn tall(all: &Statements) -> bool {
    let hyps = all.len() - 1;
    let rows: usize = all
        .iter()
        .map(|(k, t)| model_rows(t, screen(hyps, *k).0))
        .sum();
    all.len() > 1 && rows > 6
}

/// Checks every assertion, with pixels for those `frames` picks.
fn check_library(name: &str, bytes: &[u8], frames: &dyn Fn(&Statements) -> bool) {
    let model = Model::new(bytes);
    let mut fm = Fm::new(bytes);
    let all = by_assertion(bytes);
    let mut drawn = 0;
    for (n, stmts) in &all {
        let with = frames(stmts);
        drawn += usize::from(with);
        check_assertion(&mut fm, &model, *n, stmts, with);
    }
    println!(
        "{name}: {} assertions loaded and selected, {drawn} with frames",
        all.len()
    );
}

#[test]
fn every_statement_of_every_assertion_of_the_fixture_selects_in_any_order() {
    let bytes = fixture::nodes_library();
    assert!(by_assertion(&bytes).iter().any(|(_, s)| s.len() > 10));
    check_library("fixture", &bytes, &|_| true);
}

fn shipped(name: &str) -> Vec<u8> {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads")
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn every_statement_of_every_assertion_of_prop_selects_in_any_order_and_draws_as_the_model_does() {
    check_library("prop", &shipped("prop"), &|_| true);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding pred.mclib and ring.mclib>"]
fn every_statement_of_every_assertion_of_pred_and_ring_selects_in_any_order_and_the_tall_ones_draw()
{
    for name in ["pred", "ring"] {
        check_library(name, &shipped(name), &tall);
    }
}

/// `|- ` and `tildes` prefix operators in front of a variable: 2 + tildes tokens and 1 + tildes
/// nodes.
fn chain(tildes: usize) -> String {
    format!("|- {}ph", "~ ".repeat(tildes))
}

/// One assertion `thm` whose hypotheses and conclusion are chains of the given lengths.
fn chains(hyps: &[usize], conclusion: usize) -> Vec<u8> {
    let mut src = String::from("$c ~ $.\nwun $a wff ~ ph $.\n${\n");
    for (i, tildes) in hyps.iter().enumerate() {
        src += &format!("h{i} $e {} $.\n", chain(*tildes));
    }
    src += &format!("thm $a {} $.\n$}}\n", chain(conclusion));
    fixture::custom(&src)
}

/// `|-` and `constants` copies of F, which a syntax axiom of that many constants makes one node.
fn wide(constants: usize) -> String {
    format!("|- {}", "F ".repeat(constants).trim_end())
}

/// The syntax axioms that make `wide(k)` one node for each k.
fn wide_axioms(lengths: &[usize]) -> String {
    let mut src = String::from("$c F $.\n");
    for k in lengths {
        src += &format!("wf{k} $a wff {} $.\n", "F ".repeat(*k).trim_end());
    }
    src
}

/// A library of the fixture's header, `axioms`, and one assertion for each part: its label and
/// its statements, the hypotheses and then the conclusion.
fn library_of(axioms: &str, parts: &[(&str, Vec<String>)]) -> Vec<u8> {
    let mut src = axioms.to_string();
    for (label, statements) in parts {
        src += "${\n";
        let (conclusion, hyps) = statements.split_last().unwrap();
        for (i, h) in hyps.iter().enumerate() {
            src += &format!("{label}.{i} $e {h} $.\n");
        }
        src += &format!("{label} $a {conclusion} $.\n$}}\n");
    }
    fixture::custom(&src)
}

fn named(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap() as u16
}

fn thm(bytes: &[u8]) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == "thm")
        .unwrap() as u16
}

/// What is loaded after a refused fm_assertion: nothing selectable, nothing to draw.
fn assert_nothing_loaded(fm: &mut Fm, what: &str) {
    let index = code("FM_ERR_INDEX");
    for k in [0, 1, 0xFF] {
        assert_eq!(fm.select(k).status(), index, "{what}: select {k}");
    }
    assert_eq!(fm.nodes().status(), index, "{what}: nodes");
    let rows = fm.rows(STATEMENT_WIDTH);
    assert_eq!((rows.status(), count(rows)), (0, 0), "{what}: rows");
    fm.clear();
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    assert_eq!(fm.frame(), vec![0; FRAME_LEN], "{what}: nothing drawn");
}

/// What fm_assertion does with a refused load after a good one in the same machine: the earlier
/// store is gone, whatever was selected and built.
fn refused_after_a_good_load(bytes: &[u8], good: &str, bad: &str, fits: usize, what: &str) {
    let limit = code("FM_ERR_LIMIT");
    let model = Model::new(bytes);
    let (good, bad) = (named(bytes, good), named(bytes, bad));
    assert!(model.within(good), "the model's caps hold the first");
    assert!(!model.within(bad), "and not the second");
    let mut fm = Fm::new(bytes);
    let out = fm.assertion(good);
    assert_eq!((out.status(), count(out)), (0, fits), "{what}");
    assert_eq!(fm.select(0).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    fm.clear();
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    assert_ne!(fm.frame(), vec![0; FRAME_LEN], "something is on screen");
    let out = fm.assertion(bad);
    assert_eq!((out.status(), count(out)), (limit, 0), "{what}");
    assert_nothing_loaded(&mut fm, what);
}

#[test]
fn an_assertion_of_768_tokens_loads_and_one_of_769_is_refused_with_nothing_left() {
    let hyps = |k: usize| vec![wide(254), wide(254), wide(254), wide(k)];
    let bytes = library_of(
        &wide_axioms(&[254, 2, 3]),
        &[("fits", hyps(2)), ("over", hyps(3))],
    );
    let tokens = |label: &str| -> usize {
        by_assertion(&bytes)[usize::from(named(&bytes, label))]
            .1
            .iter()
            .map(|(_, t)| t.len())
            .sum()
    };
    assert_eq!((tokens("fits"), tokens("over")), (768, 769));
    refused_after_a_good_load(&bytes, "fits", "over", 4, "769 tokens");
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(named(&bytes, "fits")).status(), 0);
    assert_eq!(count(fm.select(0xFF)), 3);
}

#[test]
fn an_assertion_of_32_statements_loads_and_one_of_33_is_refused_with_nothing_left() {
    let hyps = |count: usize| vec!["|- ph".to_string(); count + 1];
    let bytes = library_of("", &[("fits", hyps(31)), ("over", hyps(32))]);
    refused_after_a_good_load(&bytes, "fits", "over", 32, "33 statements");
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(named(&bytes, "fits")).status(), 0);
    for k in (0..31).chain([0xFF]) {
        assert_eq!(fm.select(k).status(), 0, "statement {k}");
    }
    assert_eq!(fm.select(31).status(), code("FM_ERR_INDEX"));
}

/// The node counts of each statement in a library built by `chains`, from the model.
fn node_counts(bytes: &[u8]) -> Vec<usize> {
    let model = Model::new(bytes);
    let n = thm(bytes);
    by_assertion(bytes)[usize::from(n)]
        .1
        .iter()
        .map(|(k, _)| model.table(n, *k).len())
        .collect()
}

#[test]
fn tables_of_384_nodes_build_and_a_385th_node_refuses_that_table_and_keeps_the_rest() {
    let limit = code("FM_ERR_LIMIT");
    let fits = chains(&[127, 127], 127);
    assert_eq!(node_counts(&fits), [128, 128, 128]);
    let over = chains(&[127, 127], 128);
    assert_eq!(node_counts(&over), [128, 128, 129]);
    for (bytes, want) in [(&fits, 0), (&over, limit)] {
        let (model, n) = (Model::new(bytes), thm(bytes));
        assert_eq!(model.within(n), want == 0, "the model's caps agree");
        let mut fm = Fm::new(bytes);
        assert_eq!(fm.assertion(n).status(), 0);
        for k in [0, 1] {
            fm.select(k);
            let out = fm.nodes();
            assert_eq!(out.status(), 0);
            assert_eq!(fm.table(count(out)), model.table(n, k));
        }
        fm.select(0xFF);
        let out = fm.nodes();
        assert_eq!((out.status(), count(out) > 0), (want, want == 0));
        if want == 0 {
            assert_eq!(fm.table(count(out)), model.table(n, 0xFF));
        } else {
            assert_eq!(fm.header().nodes, 0, "the table is not built");
            assert_eq!(fm.nodes().status(), limit, "and a second call refuses too");
            let tokens = &by_assertion(bytes)[usize::from(n)].1[2].1;
            let rows = model_rows(tokens, STATEMENT_WIDTH);
            assert_eq!(count(fm.rows(STATEMENT_WIDTH)), rows, "it is still loaded");
        }
        for k in [1, 0, 0xFF] {
            if k == 0xFF && want != 0 {
                continue;
            }
            fm.select(k);
            let out = fm.nodes();
            assert_eq!(fm.table(count(out)), model.table(n, k), "statement {k}");
        }
    }
}

#[test]
fn a_statement_of_255_tokens_and_254_nodes_still_loads_and_builds_in_a_store_of_one() {
    let bytes = fixture::nodes_library();
    let n = decoded(&bytes)
        .assertions
        .iter()
        .position(|a| a.label == "deep")
        .unwrap() as u16;
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let out = fm.assertion(n);
    assert_eq!((out.status(), count(out)), (0, 1));
    assert_eq!(count(fm.select(0xFF)), 255);
    let out = fm.nodes();
    assert_eq!((out.status(), count(out)), (0, 254));
    assert_eq!(fm.table(count(out)), model.table(n, 0xFF));
}

/// T-states from the first instruction at label `from` to the first at label `to` for a call,
/// found by stepping it; the call then runs to its end, so the session restores SP, the ports and
/// the interrupt state before the next one.
fn between(fm: &mut Fm, target: &str, regs: Regs, from: &str, to: &str) -> u32 {
    fm.rig.prepare(target, regs);
    let (from, to) = (fm.rig.label(from), fm.rig.label(to));
    let done = fm.rig.label("CALL_DONE");
    let (mut spent, mut start, mut found) = (0, None, None);
    let mut run = fm.rig.m.run(fm.rig.label("CALL_ENTRY"), 1);
    loop {
        spent += run.tstates;
        let pc = fm.rig.m.pc();
        if start.is_none() && pc == from {
            start = Some(spent);
        }
        if found.is_none() && pc == to {
            found = Some(spent - start.expect("the body ran"));
        }
        if pc == done {
            return found.expect("the body ended");
        }
        run = fm.rig.m.resume(1);
    }
}

/// T-state ceilings per library: fm_assertion, the median and the largest, and fm_select's body.
struct Ceilings {
    name: &'static str,
    median: u32,
    largest: u32,
}

const CEILINGS: [Ceilings; 3] = [
    Ceilings {
        name: "prop",
        median: 44595,
        largest: 105341,
    },
    Ceilings {
        name: "pred",
        median: 38532,
        largest: 116886,
    },
    Ceilings {
        name: "ring",
        median: 35571,
        largest: 196555,
    },
];

/// What fm_select costs beyond a session's entry and exit, and in all, about 5 percent over the
/// largest of the three libraries.
const SELECT_BODY: u32 = 352;
const SELECT_TOTAL: u32 = 849;

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_load_every_assertion_and_select_every_statement_within_the_ceilings() {
    for ceiling in CEILINGS {
        let name = ceiling.name;
        let bytes = shipped(name);
        let mut fm = Fm::bare(&bytes);
        let base = fm.rig.call("CALL_NOP", Regs::default()).tstates;
        assert_eq!(fm.open().status(), 0, "{name}: fm_open");
        let all = statements(&decoded(&bytes));
        let distinct = |t: &Vec<String>| t.iter().collect::<std::collections::BTreeSet<_>>().len();
        let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| distinct(t)).unwrap();
        assert_eq!(fm.open().status(), 0);
        let cold = fm.statement(*n, *k).tstates - base;
        let warm = fm.statement(*n, *k).tstates - base;
        println!(
            "{name}: fm_statement of {} tokens with {} symbols, all new: {cold} after fm_open, {warm} when measured",
            tokens.len(),
            distinct(tokens)
        );
        let (mut loads, mut selects, mut body) = (Vec::new(), 0, 0);
        for (n, all) in by_assertion(&bytes) {
            let out = fm.assertion(n);
            assert_eq!(out.status(), 0, "{name}: assertion {n}");
            loads.push(out.tstates - base);
            for (k, _) in &all {
                let regs = Regs {
                    de: u16::from(*k),
                    ..Regs::default()
                };
                body = body.max(between(
                    &mut fm,
                    "FM_SELECT",
                    regs,
                    "SELECT_BODY",
                    "FM_LEAVE",
                ));
                selects = selects.max(fm.select(*k).tstates - base);
            }
        }
        loads.sort_unstable();
        let (median, largest) = (loads[loads.len() / 2], *loads.last().unwrap());
        println!(
            "{name}: fm_assertion median {median}, largest {largest}\n\
             {name}: fm_select largest {selects} in all, {body} beyond the session"
        );
        if name == "ring" {
            assert_header_agrees("fm_assertion of its largest assertion, ", largest);
        }
        assert!(body <= SELECT_BODY, "{name}: fm_select body took {body}");
        assert!(selects <= SELECT_TOTAL, "{name}: fm_select took {selects}");
        assert!(
            median <= ceiling.median,
            "{name}: fm_assertion median {median}"
        );
        assert!(
            largest <= ceiling.largest,
            "{name}: fm_assertion largest {largest}"
        );
    }
}

#[test]
fn fms_caps_are_the_models() {
    use mcui::caps::*;
    for (name, model) in [
        ("FM_MAX_STMTS", MAX_STATEMENTS),
        ("FM_MAX_TOKENS", MAX_TOKENS),
        ("FM_MAX_NODES", MAX_NODES),
        ("FM_MAX_STMT_TOKENS", MAX_STATEMENT_TOKENS),
        ("FM_MAX_STMT_NODES", MAX_STATEMENT_NODES),
    ] {
        assert_eq!(constant_of(WITH_FM.labels, name) as usize, model, "{name}");
    }
}
