//! The model's caps on a goal tree and the walk's, on the same databases: each case is read by
//! `Lib::goals` and by the Z80, and the tree the model's outline builds is counted to put the case
//! on the boundary.

use crate::caps::{Case, at_the_boundary, doubling, fanout};
use crate::limits::chain;
use crate::proofs::*;
use crate::rig::*;
use crate::shape::model_of;
use crate::support::*;
use mcui::caps;

const TOO_LARGE: &str = "its proof is too large";

/// What the model's outline holds for the case before any cap: distinct goals, links, the most
/// subgoals of one goal, and nodes.
struct Tree {
    goals: usize,
    links: usize,
    widest: usize,
    total: u64,
}

fn tree(case: &Case) -> Tree {
    let root = outline_of(&case.bytes, usize::from(case.n)).expect("the outline has a tree");
    let (goals, links, widest) = tree_counts(&root);
    Tree {
        goals,
        links,
        widest,
        total: nodes_under(&root, &mut Default::default()),
    }
}

/// The model's answer and the Z80's status for `case`: Ok and 0, or the model's error and the
/// status of the cap the case is one past.
fn compare(case: &Case, status: Option<u8>) {
    let answer = model_of("case", &case.bytes)
        .goals(usize::from(case.n))
        .map(|_| ());
    match status {
        None => assert_eq!(answer, Ok(())),
        Some(_) => assert_eq!(answer, Err(TOO_LARGE)),
    }
    at_the_boundary(case, status.unwrap_or(ST_OK));
}

#[test]
fn the_models_caps_are_the_walks() {
    let rig = Rig::with_image(PW);
    assert_eq!(usize::from(rig.label("PW_MAX_GOALS")), caps::MAX_GOALS);
    assert_eq!(
        usize::from(rig.label("PW_MAX_SUBGOALS")),
        caps::MAX_SUBGOALS
    );
    assert_eq!(usize::from(rig.label("PW_MAX_LINKS")), caps::MAX_LINKS);
    assert_eq!(caps::MAX_PROOF_NODES, (1 << 24) - 1);
}

#[test]
fn a_goal_with_31_subgoals_is_read_by_both_and_one_with_32_by_neither() {
    let at = fanout(&[31]);
    assert_eq!(tree(&at).widest, caps::MAX_SUBGOALS, "the precondition");
    compare(&at, None);
    let past = fanout(&[32]);
    assert_eq!(
        tree(&past).widest,
        caps::MAX_SUBGOALS + 1,
        "the precondition"
    );
    compare(&past, Some(SUBGOALS));
}

#[test]
fn a_tree_of_255_distinct_goals_is_read_by_both_and_one_of_256_by_neither() {
    let (bytes, n) = chain(127, false);
    let at = Case { bytes, n };
    assert_eq!(tree(&at).goals, caps::MAX_GOALS, "the precondition");
    compare(&at, None);
    let (bytes, n) = chain(128, true);
    let past = Case { bytes, n };
    assert_eq!(tree(&past).goals, caps::MAX_GOALS + 1, "the precondition");
    compare(&past, Some(GOALS));
}

#[test]
fn a_tree_of_857_links_is_read_by_both_and_one_of_858_by_neither() {
    let links = |total: usize| {
        let full = total / caps::MAX_SUBGOALS;
        let mut arities = vec![caps::MAX_SUBGOALS; full];
        arities.insert(0, total - caps::MAX_SUBGOALS * full);
        fanout(&arities)
    };
    let at = links(caps::MAX_LINKS);
    let t = tree(&at);
    assert_eq!((t.links, t.widest), (caps::MAX_LINKS, caps::MAX_SUBGOALS));
    assert!(t.goals < caps::MAX_GOALS && t.total < caps::MAX_PROOF_NODES);
    compare(&at, None);
    let past = links(caps::MAX_LINKS + 1);
    let t = tree(&past);
    assert_eq!(
        (t.links, t.widest),
        (caps::MAX_LINKS + 1, caps::MAX_SUBGOALS)
    );
    assert!(t.goals < caps::MAX_GOALS && t.total < caps::MAX_PROOF_NODES);
    compare(&past, Some(LINKS));
}

#[test]
fn a_total_of_16777215_is_read_by_both_and_one_of_16777216_by_neither() {
    let at = doubling(false);
    let t = tree(&at);
    assert_eq!(t.total, caps::MAX_PROOF_NODES, "the precondition");
    assert!(t.goals < caps::MAX_GOALS && t.links < caps::MAX_LINKS);
    compare(&at, None);
    let past = doubling(true);
    let t = tree(&past);
    assert_eq!(t.total, caps::MAX_PROOF_NODES + 1, "the precondition");
    assert!(t.goals < caps::MAX_GOALS && t.links < caps::MAX_LINKS);
    compare(&past, Some(TOTAL));
}
