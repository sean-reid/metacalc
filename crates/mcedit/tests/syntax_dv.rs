mod common;

use common::*;
use mcedit::{Catalog, Session, Violation, ViolationKind};
use mcpack::mm::{self, Database, Grammar, ProofError, Verifier};

fn v(db: &Database, rule: &str, x: &str, y: &str, kind: ViolationKind) -> Violation {
    let (a, b) = (sym(db, x), sym(db, y));
    Violation {
        assertion: id(db, rule),
        x: a.min(b),
        y: a.max(b),
        kind,
    }
}

fn mp<'a>(cat: &std::rc::Rc<Catalog<'a>>, name: &str) -> Session<'a> {
    let mut s = session(cat, name);
    s.apply(0, id(cat.db, "ax-mp")).unwrap();
    s
}

#[test]
fn a_variable_on_both_sides_of_a_pair_is_shared() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "tsh");
    let out = s.close(1, id(&db, "tsh.2")).unwrap();
    assert_eq!(
        out.violations,
        [v(&db, "wqq", "y", "y", ViolationKind::Shared)]
    );
    s.close(2, id(&db, "tsh.1")).unwrap();
    assert!(s.export().is_err());
    assert_eq!(
        Verifier::new(&db).verify(id(&db, "tsh")),
        Err(ProofError::DisjointViolation)
    );
}

#[test]
fn every_cross_pair_of_a_side_with_several_variables_is_owed() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "tmulti");
    let out = s.close(1, id(&db, "tmulti.2")).unwrap();
    let want = [
        v(&db, "wq", "y", "ch", ViolationKind::Undeclared),
        v(&db, "wq", "y", "th", ViolationKind::Undeclared),
    ];
    assert_eq!(out.violations.len(), 2);
    assert!(want.iter().all(|w| out.violations.contains(w)));

    let mut ok = mp(&cat, "tmulti-ok");
    assert_eq!(ok.close(1, id(&db, "tmulti-ok.2")).unwrap().violations, []);
    ok.close(2, id(&db, "tmulti-ok.1")).unwrap();
    assert_eq!(verified(&db, &ok.export().unwrap()), Ok(()));
}

#[test]
fn a_value_with_an_unbound_metavariable_owes_nothing_until_it_binds() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "tm");
    let meta = s.status().metas[0];
    assert_eq!(s.refine(meta, id(&db, "wq")).unwrap().violations, []);
    assert_eq!(s.status().violations, []);
    let wrong = v(&db, "wq", "y", "ch", ViolationKind::Undeclared);
    assert_eq!(s.close(1, id(&db, "tm.2")).unwrap().violations, [wrong]);
    assert_eq!(s.status().violations, [wrong]);
    s.undo().unwrap();
    assert_eq!(s.status().violations, []);
}

#[test]
fn undo_clears_a_violation_from_a_substituted_term() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "tm");
    s.close(1, id(&db, "tm.2")).unwrap();
    assert_eq!(s.status().violations.len(), 1);
    s.undo().unwrap();
    assert_eq!(s.status().violations, []);
}

#[test]
fn a_bare_variable_as_a_value_owes_nothing() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "t1");
    assert_eq!(s.close(1, id(&db, "t1.1")).unwrap().violations, []);
    s.close(2, id(&db, "t1.2")).unwrap();
    assert_eq!(s.status().violations, []);
    assert!(s.syntax_walk_stats().0 > 0);
    assert_eq!(verified(&db, &s.export().unwrap()), Ok(()));
}

#[test]
fn a_term_built_twice_is_reported_once() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "ttwice");
    let out = s.apply(0, id(&db, "axpair")).unwrap();
    assert_eq!(out.violations.len(), 1);
    s.close(1, id(&db, "ttwice.1")).unwrap();
    s.close(2, id(&db, "ttwice.1")).unwrap();
    assert_eq!(
        s.status().violations,
        [v(&db, "wq", "y", "ch", ViolationKind::Undeclared)]
    );
}

#[test]
fn a_second_copy_of_a_standing_violation_is_newly_broken() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tsolve");
    let wrong = v(&db, "wq", "y", "ch", ViolationKind::Undeclared);
    assert_eq!(s.apply(0, id(&db, "ax-mp")).unwrap().violations, [wrong]);
    assert_eq!(s.close(1, id(&db, "tsolve.1")).unwrap().violations, [wrong]);
    assert_eq!(s.apply(2, id(&db, "ax-1")).unwrap().violations, [wrong]);
}

#[test]
fn auto_does_not_accept_a_candidate_that_adds_a_second_copy() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tsolve");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.close(1, id(&db, "tsolve.1")).unwrap();
    assert_eq!(s.auto(2, 1, 500), None);
}

fn chain(steps: usize, with_dv: bool) -> String {
    let mut stmt = "ph".to_string();
    for _ in 0..steps {
        stmt = format!("( ps -> {stmt} )");
    }
    let dv = if with_dv {
        "${ $d x ph $. wq $a wff Q x ph $. $}"
    } else {
        ""
    };
    format!(
        "$c ( ) -> wff |- setvar Q $. $v ph ps x $.
         wph $f wff ph $. wps $f wff ps $. vx $f setvar x $.
         wi $a wff ( ph -> ps ) $. {dv}
         ${{ a1i.1 $e |- ph $. a1i $a |- ( ps -> ph ) $. $}}
         ${{ t.1 $e |- ph $. t $p |- {stmt} $= ? $. $}}"
    )
}

fn prove_chain(src: &str, steps: usize) -> (usize, usize, usize, usize) {
    let db = mm::parse(src.as_bytes().to_vec()).unwrap();
    let g = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let cat = Catalog::new(&db, &g, LIBRARY);
    let mut s = Session::start_in(cat, id(&db, "t")).unwrap();
    let mut worst = 0;
    for goal in 0..steps {
        let before = s.syntax_walk_stats().1;
        s.apply(goal as u32, id(&db, "a1i")).unwrap();
        worst = worst.max(s.syntax_walk_stats().1 - before);
    }
    s.close(steps as u32, id(&db, "t.1")).unwrap();
    let before = s.syntax_walk_stats().0;
    assert!(s.status().open.is_empty());
    let (walks, visits) = s.syntax_walk_stats();
    assert_eq!(walks - before > 0, src.contains("wq"));
    (walks, visits, worst, s.sizes().terms.nodes)
}

#[test]
fn a_database_without_a_syntax_pair_never_walks_the_proof() {
    let (walks, visits, ..) = prove_chain(&chain(120, false), 120);
    assert_eq!((walks, visits), (0, 0));
}

#[test]
fn the_walk_visits_each_term_once_per_pass() {
    let steps = 120;
    let (walks, visits, worst, nodes) = prove_chain(&chain(steps, true), steps);
    println!("walks {walks} visits {visits} worst per op {worst} store nodes {nodes}");
    assert!(walks >= 2 * steps);
    assert!(worst <= 2 * nodes);
    assert!(visits <= walks * nodes);
}
