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

fn open(src: &str) -> (&'static Database, Grammar<'static>) {
    let db: &'static Database = Box::leak(Box::new(mm::parse(src.as_bytes().to_vec()).unwrap()));
    (db, Grammar::new(db, &[("|-", "wff")]).unwrap())
}

/// Proves the chain with one step per level, returning the walks, the worst visits in one step
/// and the store's node count; `walked` says whether the database has a syntax pair.
fn prove_chain(steps: usize, walked: bool) -> (usize, usize, usize) {
    let (db, g) = open(&chain(steps, walked));
    let cat = Catalog::new(db, &g, LIBRARY);
    let mut s = Session::start_in(cat, id(db, "t")).unwrap();
    let mut worst = 0;
    for goal in 0..steps {
        let before = s.syntax_walk_stats().1;
        s.apply(goal as u32, id(db, "a1i")).unwrap();
        worst = worst.max(s.syntax_walk_stats().1 - before);
    }
    s.close(steps as u32, id(db, "t.1")).unwrap();
    let before = s.syntax_walk_stats().0;
    assert!(s.status().open.is_empty());
    let (walks, ..) = s.syntax_walk_stats();
    assert_eq!(walks > before, walked);
    (walks, worst, s.sizes().terms.nodes)
}

#[test]
fn a_database_without_a_syntax_pair_never_walks_the_proof() {
    let (walks, ..) = prove_chain(120, false);
    assert_eq!(walks, 0);
}

#[test]
fn a_database_with_a_syntax_pair_visits_each_term_once_per_step() {
    let (walks, worst, nodes) = prove_chain(120, true);
    println!("walks {walks} worst visits per step {worst} store nodes {nodes}");
    assert!(walks >= 240);
    assert!(worst <= 2 * nodes);
}

fn nested(levels: usize) -> String {
    let mut term = "ph".to_string();
    for _ in 0..levels {
        term = format!("Q x {term}");
    }
    format!(
        "$c ( ) -> wff |- setvar Q $. $v ph ps x $.
         wph $f wff ph $. wps $f wff ps $. vx $f setvar x $.
         wi $a wff ( ph -> ps ) $.
         ${{ $d x ph $. wq $a wff Q x ph $. $}}
         idd $a |- ( ph -> ph ) $.
         ${{ t $p |- ( {term} -> {term} ) $= ? $. $}}"
    )
}

/// The work and visits of the one step that builds the whole nested term.
fn nested_step(levels: usize) -> (usize, usize) {
    let src = nested(levels);
    let (db, g) = open(&src);
    let cat = Catalog::new(db, &g, LIBRARY);
    let mut s = Session::start_in(cat, id(db, "t")).unwrap();
    let (_, visits, work) = s.syntax_walk_stats();
    s.apply(0, id(db, "idd")).unwrap();
    let (_, after_visits, after_work) = s.syntax_walk_stats();
    assert!(!s.status().violations.is_empty());
    (after_visits - visits, after_work - work)
}

#[test]
fn nested_syntax_pairs_cost_work_linear_in_the_term() {
    let (small_visits, small) = nested_step(60);
    let (large_visits, large) = nested_step(120);
    println!(
        "60 levels: {small_visits} visits {small} work; 120 levels: {large_visits} visits {large} work"
    );
    assert!(large_visits <= 2 * 121 + 2);
    assert!(
        large * 10 <= small * 22,
        "work grew {small} -> {large}, more than 2.2 times"
    );
}

#[test]
fn a_proof_that_cannot_be_walked_is_not_clean_and_refuses_operations() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = mp(&cat, "tm-ok");
    s.close(1, id(&db, "tm-ok.2")).unwrap();
    assert!(s.status().clean());
    s.corrupt_metas();
    let status = s.status();
    assert!(status.corrupt && !status.clean());
    assert_eq!(s.export().err(), Some(mcedit::Error::Corrupt));
    let (ops, sizes) = (s.ops().len(), s.sizes());
    assert_eq!(s.close(2, id(&db, "tm-ok.1")), Err(mcedit::Error::Corrupt));
    assert_eq!((s.ops().len(), s.sizes()), (ops, sizes));
}
