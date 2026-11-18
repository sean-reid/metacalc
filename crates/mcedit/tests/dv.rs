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
    assert_eq!(s.show_goal(1), "|- ( ps -> ?1 )");
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
