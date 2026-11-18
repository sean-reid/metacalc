mod common;

use common::*;
use mcedit::{Violation, ViolationKind};

fn violation(
    db: &mcpack::mm::Database,
    rule: &str,
    x: &str,
    y: &str,
    kind: ViolationKind,
) -> Violation {
    let (a, b) = (sym(db, x), sym(db, y));
    Violation {
        assertion: id(db, rule),
        x: a.min(b),
        y: a.max(b),
        kind,
    }
}

#[test]
fn an_undeclared_pair_is_reported_when_the_apply_completes_it() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tax5-bad");
    let out = s.apply(0, id(&db, "ax-5")).unwrap();
    assert_eq!(
        out.violations,
        [violation(&db, "ax-5", "y", "ps", ViolationKind::Undeclared)]
    );
    assert_eq!(s.status().violations, out.violations);
    assert!(s.status().open.is_empty());
    assert!(s.export().is_err());
    s.undo().unwrap();
    assert!(s.status().violations.is_empty());
}

#[test]
fn a_declared_pair_raises_nothing() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tax5-ok");
    let out = s.apply(0, id(&db, "ax-5")).unwrap();
    assert_eq!(out.violations, []);
    let done = s.export().unwrap();
    assert_eq!(labels(&db, &done), "wps vy ax-5");
    assert_eq!(done.added_dv, []);
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_pair_with_an_unknown_term_waits_for_the_step_that_resolves_it() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdv");
    let out = s.apply(0, id(&db, "axdv")).unwrap();
    assert_eq!(out.violations, []);
    assert_eq!(s.show_goal(1).unwrap(), "|- ( ps -> ?1 )");
    let out = s.close(1, id(&db, "tdv.1")).unwrap();
    assert_eq!(
        out.violations,
        [violation(&db, "axdv", "y", "th", ViolationKind::Undeclared)]
    );
    s.undo().unwrap();
    assert!(s.status().violations.is_empty());
}

#[test]
fn the_same_proof_is_clean_when_the_theorem_declares_the_pair() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdv-ok");
    s.apply(0, id(&db, "axdv")).unwrap();
    let out = s.close(1, id(&db, "tdv-ok.1")).unwrap();
    assert_eq!(out.violations, []);
    let done = s.export().unwrap();
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_variable_in_both_terms_is_a_violation_even_when_declared() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdv-shared");
    s.apply(0, id(&db, "axdv")).unwrap();
    let out = s.close(1, id(&db, "tdv-shared.1")).unwrap();
    assert_eq!(
        out.violations,
        [violation(&db, "axdv", "y", "y", ViolationKind::Shared)]
    );
}

#[test]
fn a_dummy_variable_needs_a_declared_pair_in_a_theorem_that_has_a_proof() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdum");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tdum.1")).unwrap();
    let meta = s.status().metas[0];
    let out = s.assign(meta, sym(&db, "z")).unwrap();
    assert_eq!(
        out.violations,
        [violation(
            &db,
            "axfree",
            "ps",
            "z",
            ViolationKind::Undeclared
        )]
    );
    assert!(s.export().is_err());
}

#[test]
fn a_dummy_variable_is_clean_when_the_theorem_declares_its_pair() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdum-ok");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tdum-ok.1")).unwrap();
    let meta = s.status().metas[0];
    assert_eq!(s.assign(meta, sym(&db, "z")).unwrap().violations, []);
    let done = s.export().unwrap();
    assert_eq!(done.added_dv, []);
    assert_eq!(labels(&db, &done), "wps vz wps vz tdum-ok.1 ax-gen axfree");
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_new_proof_may_add_the_pairs_its_dummy_needs_and_reports_them() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tnew.1")).unwrap();
    let meta = s.status().metas[0];
    let out = s.assign(meta, sym(&db, "z")).unwrap();
    let pair = (sym(&db, "ps"), sym(&db, "z"));
    assert_eq!(out.violations, []);
    assert_eq!(s.status().added_dv, [pair]);

    let done = s.export().unwrap();
    assert_eq!(done.added_dv, [pair]);
    assert_eq!(done.dv, [pair]);
    assert_eq!(done.dummies.len(), 1);
    assert_eq!(
        (done.dummies[0].label.as_str(), done.dummies[0].declared),
        ("vz", false)
    );
    assert_eq!(
        done.statement(&db),
        "${\n  $d ps z $.\n  tnew $p |- ps $= wps vz wps vz tnew.1 ax-gen axfree $.\n$}\n"
    );
    assert_eq!(verified(&db, &done), Ok(()));

    s.allow_new_disjoint(false);
    assert_eq!(s.status().violations.len(), 1);
}

#[test]
fn a_dummy_with_an_active_hypothesis_reuses_it() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tnew.1")).unwrap();
    let meta = s.status().metas[0];
    s.assign(meta, sym(&db, "x")).unwrap();
    let done = s.export().unwrap();
    assert_eq!(labels(&db, &done), "wps vx wps vx tnew.1 ax-gen axfree");
    assert_eq!(done.dv, [(sym(&db, "ps"), sym(&db, "x"))]);
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_dummy_without_an_active_hypothesis_gets_a_new_one() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tnew.1")).unwrap();
    let meta = s.status().metas[0];
    s.assign(meta, sym(&db, "w")).unwrap();
    let done = s.export().unwrap();
    assert_eq!(
        (done.dummies[0].label.as_str(), done.dummies[0].declared),
        ("tnew.d1", true)
    );
    assert_eq!(
        done.statement(&db),
        "${\n  $d ps w $.\n  tnew.d1 $f setvar w $.\n  tnew $p |- ps $= wps tnew.d1 wps tnew.d1 tnew.1 ax-gen axfree $.\n$}\n"
    );
    assert_eq!(verified(&db, &done), Ok(()));
    assert_eq!(
        done.text(&db),
        "${\n  $d ps w $.\n  wps $f wff ps $.\n  tnew.1 $e |- ps $.\n  tnew.d1 $f setvar w $.\n  tnew $p |- ps $= wps tnew.d1 wps tnew.d1 tnew.1 ax-gen axfree $.\n$}\n"
    );
}

fn repeated(db: &mcpack::mm::Database) -> Violation {
    violation(db, "axq", "y", "ps", ViolationKind::Undeclared)
}

#[test]
fn a_step_that_repeats_a_violation_reports_it_and_the_verifier_agrees() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdup");
    s.apply(0, id(&db, "axpair")).unwrap();
    let first = s.apply(1, id(&db, "axq")).unwrap();
    assert_eq!(first.violations, [repeated(&db)]);
    let second = s.apply(2, id(&db, "axq")).unwrap();
    assert_eq!(second.violations, [repeated(&db)]);
    assert!(s.export().is_err());
    assert_eq!(
        mcpack::mm::Verifier::new(&db).verify(id(&db, "tdup")),
        Err(mcpack::mm::ProofError::DisjointViolation)
    );
}

#[test]
fn the_same_steps_are_clean_when_the_theorem_declares_the_pair() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdup-ok");
    s.apply(0, id(&db, "axpair")).unwrap();
    assert_eq!(s.apply(1, id(&db, "axq")).unwrap().violations, []);
    assert_eq!(s.apply(2, id(&db, "axq")).unwrap().violations, []);
    let done = s.export().unwrap();
    assert_eq!(verified(&db, &done), Ok(()));
    assert_eq!(
        mcpack::mm::Verifier::new(&db).verify(id(&db, "tdup-ok")),
        Ok(())
    );
}

#[test]
fn auto_does_not_return_a_step_that_repeats_a_standing_violation() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdup");
    s.apply(0, id(&db, "axpair")).unwrap();
    s.apply(1, id(&db, "axq")).unwrap();
    assert_eq!(s.auto(2, 1, 500), None);

    let mut ok = session(&cat, "tdup-ok");
    ok.apply(0, id(&db, "axpair")).unwrap();
    ok.apply(1, id(&db, "axq")).unwrap();
    let ops = ok.auto(2, 1, 500).unwrap();
    ok.play(&ops).unwrap();
    assert_eq!(verified(&db, &ok.export().unwrap()), Ok(()));
}

#[test]
fn a_refined_syntax_axiom_records_its_disjoint_pair() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tq");
    let meta = s.apply(0, id(&db, "ax-mp")).unwrap().metas.start;
    let out = s.refine(meta, id(&db, "wq")).unwrap();
    assert_eq!(out.violations, []);
    let out = s.close(1, id(&db, "tq.2")).unwrap();
    assert_eq!(
        out.violations,
        [violation(&db, "wq", "y", "ch", ViolationKind::Undeclared)]
    );
    s.close(2, id(&db, "tq.1")).unwrap();
    assert_eq!(s.status().violations, out.violations);
    assert!(s.export().is_err());
    assert_eq!(
        mcpack::mm::Verifier::new(&db).verify(id(&db, "tq")),
        Err(mcpack::mm::ProofError::DisjointViolation)
    );
}

#[test]
fn a_refined_syntax_axiom_is_clean_when_the_theorem_declares_the_pair() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tq-ok");
    let meta = s.apply(0, id(&db, "ax-mp")).unwrap().metas.start;
    s.refine(meta, id(&db, "wq")).unwrap();
    assert_eq!(s.close(1, id(&db, "tq-ok.2")).unwrap().violations, []);
    s.close(2, id(&db, "tq-ok.1")).unwrap();
    let done = s.export().unwrap();
    assert_eq!(labels(&db, &done), "wch vy wq wps tq-ok.2 tq-ok.1 ax-mp");
    assert_eq!(verified(&db, &done), Ok(()));
    assert_eq!(
        mcpack::mm::Verifier::new(&db).verify(id(&db, "tq-ok")),
        Ok(())
    );
}

fn scope_session<'a>(cat: &std::rc::Rc<mcedit::Catalog<'a>>, dummy: &str) -> mcedit::Session<'a> {
    let db = cat.db;
    let mut s = session(cat, "tscope");
    s.apply(0, id(db, "axfree")).unwrap();
    s.apply(1, id(db, "ax-gen")).unwrap();
    s.close(2, id(db, "tscope.1")).unwrap();
    let meta = s.status().metas[0];
    s.assign(meta, sym(db, dummy)).unwrap();
    s
}

#[test]
fn a_pair_a_question_mark_theorem_declares_is_in_scope_for_a_new_proof() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = scope_session(&cat, "w");
    s.allow_new_disjoint(false);
    assert_eq!(s.status().violations, []);
    let done = s.export().unwrap();
    assert_eq!(done.added_dv, []);
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_pair_a_question_mark_theorem_does_not_declare_is_refused_and_the_verifier_agrees() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = scope_session(&cat, "z");
    s.allow_new_disjoint(false);
    let wrong = violation(&db, "axfree", "ps", "z", ViolationKind::Undeclared);
    assert_eq!(s.status().violations, [wrong]);
    assert!(s.export().is_err());

    s.allow_new_disjoint(true);
    let mut done = s.export().unwrap();
    assert_eq!(done.added_dv, [(sym(&db, "ps"), sym(&db, "z"))]);
    assert_eq!(verified(&db, &done), Ok(()));
    done.added_dv.clear();
    assert_eq!(
        verified(&db, &done),
        Err(mcedit::oracle::Failure::Rejected(
            mcpack::mm::ProofError::DisjointViolation
        ))
    );
}
