mod common;

use common::*;
use mcedit::{AUTO_BUDGET, AUTO_DEPTH, Candidates, Op};

fn list(c: Candidates) -> Vec<(String, usize)> {
    let Candidates::List(found) = c else {
        panic!("expected a list");
    };
    found
        .iter()
        .map(|c| (c.assertion, c.new_goals))
        .map(|(a, n)| (a.to_string(), n))
        .collect()
}

fn named(db: &mcpack::mm::Database, items: &[(&str, usize)]) -> Vec<(String, usize)> {
    items
        .iter()
        .map(|&(l, n)| (id(db, l).to_string(), n))
        .collect()
}

#[test]
fn candidates_come_in_database_order_with_their_goal_counts() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tfwd");
    assert_eq!(s.show_goal(0).unwrap(), "|- ( ps -> ph )");
    let found = s.candidates(0, 99).unwrap();
    assert_eq!(
        list(found),
        named(
            &db,
            &[
                ("ax-mp", 2),
                ("a1i", 1),
                ("axfree", 1),
                ("t1", 2),
                ("tdum", 1),
                ("tdum-ok", 1),
                ("tnew", 1),
                ("tstuck", 1),
            ]
        )
    );
}

#[test]
fn candidates_stop_at_the_limit() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tfwd");
    assert_eq!(
        list(s.candidates(0, 2).unwrap()),
        named(&db, &[("ax-mp", 2), ("a1i", 1)])
    );
    assert_eq!(s.candidates(0, 0).unwrap(), Candidates::List(vec![]));
}

#[test]
fn candidates_leave_the_session_untouched() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tfwd");
    let (sizes, saved) = (s.sizes(), s.save());
    s.candidates(0, 99).unwrap();
    assert_eq!((s.sizes(), s.save()), (sizes, saved));
}

#[test]
fn a_goal_that_fits_only_a_few_assertions_lists_just_those() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tax5-ok");
    assert_eq!(
        list(s.candidates(0, 99).unwrap()),
        named(
            &db,
            &[
                ("ax-mp", 2),
                ("a1i", 1),
                ("ax-5", 0),
                ("axfree", 1),
                ("axdv", 1),
                ("t1", 2),
                ("tax5-bad", 0),
            ]
        )
    );
}

#[test]
fn a_bare_metavariable_is_unconstrained() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(s.show_goal(1).unwrap(), "|- ?0");
    assert_eq!(s.candidates(1, 3).unwrap(), Candidates::Unconstrained);
    assert!(matches!(s.candidates(2, 99).unwrap(), Candidates::List(_)));
}

#[test]
fn auto_finds_a_two_step_proof_and_leaves_the_session_as_it_was() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tauto");
    let saved = s.save();
    let a1i = id(&db, "a1i");
    let steps = s.auto(0, AUTO_DEPTH, AUTO_BUDGET).unwrap();
    assert_eq!(
        steps,
        [
            Op::Apply {
                goal: 0,
                assertion: a1i
            },
            Op::Apply {
                goal: 1,
                assertion: a1i
            },
            Op::Close {
                goal: 2,
                hyp: id(&db, "tauto.1")
            },
        ]
    );
    assert_eq!(s.save(), saved);
    s.play(&steps).unwrap();
    assert!(s.status().open.is_empty());
    let done = s.export().unwrap();
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn auto_respects_its_depth() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tauto");
    assert_eq!(s.auto(0, 1, AUTO_BUDGET), None);
    assert_eq!(s.auto(0, 0, AUTO_BUDGET), None);
    assert_eq!(s.auto(0, 3, AUTO_BUDGET).map(|v| v.len()), Some(3));
}

#[test]
fn auto_gives_up_within_its_budget() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tauto");
    assert_eq!(s.auto(0, AUTO_DEPTH, 1), None);
    assert_eq!(s.auto_nodes(), 1);
    assert_eq!(s.auto(0, AUTO_DEPTH, 0), None);
    assert_eq!(s.auto_nodes(), 0);
    assert_eq!(s.auto(0, AUTO_DEPTH, 5), None);
    assert_eq!(s.auto_nodes(), 5);
    assert!(s.auto(0, AUTO_DEPTH, 6).is_some());
    assert_eq!(s.auto_nodes(), 6);

    let mut hard = session(&cat, "tstuck");
    assert_eq!(hard.auto(0, AUTO_DEPTH, AUTO_BUDGET), None);
    assert!(hard.auto_nodes() > 0 && hard.auto_nodes() < AUTO_BUDGET);
    assert_eq!(hard.auto(0, AUTO_DEPTH, 4), None);
    assert_eq!(hard.auto_nodes(), 4);
}

#[test]
fn auto_declines_goals_with_metavariables() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(s.auto(1, AUTO_DEPTH, AUTO_BUDGET), None);
    assert_eq!(s.auto(2, AUTO_DEPTH, AUTO_BUDGET), None);
}

const TWO_PROOFS: &str = "
$c |- wff A $.
wa $a wff A $.
ax-1 $a |- A $.
ax-2 $a |- A $.
t $p |- A $= ax-1 $.
";

fn applied(ops: Option<Vec<Op>>) -> Vec<mcpack::mm::StmtId> {
    ops.expect("auto finds a proof")
        .iter()
        .map(|op| match op {
            Op::Apply { assertion, .. } => *assertion,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn auto_skips_what_the_session_is_told_to_and_finds_the_other_proof() {
    let db = mcpack::mm::parse(TWO_PROOFS.as_bytes().to_vec()).unwrap();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t");
    assert_eq!(
        applied(s.auto(0, AUTO_DEPTH, AUTO_BUDGET)),
        [id(&db, "ax-1")]
    );
    s.skip([id(&db, "ax-1")]);
    assert_eq!(
        applied(s.auto(0, AUTO_DEPTH, AUTO_BUDGET)),
        [id(&db, "ax-2")]
    );
    assert_eq!(
        list(s.candidates(0, 99).unwrap()),
        named(&db, &[("ax-2", 0)])
    );
    s.skip([]);
    assert_eq!(
        list(s.candidates(0, 99).unwrap()),
        named(&db, &[("ax-1", 0), ("ax-2", 0)])
    );
}

#[test]
fn auto_with_everything_skipped_finds_nothing_and_leaves_no_operation() {
    let db = mcpack::mm::parse(TWO_PROOFS.as_bytes().to_vec()).unwrap();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t");
    s.skip([id(&db, "ax-1"), id(&db, "ax-2")]);
    assert_eq!(s.auto(0, AUTO_DEPTH, AUTO_BUDGET), None);
    assert!(s.ops().is_empty());
}
