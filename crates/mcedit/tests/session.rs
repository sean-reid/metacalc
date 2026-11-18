mod common;

use common::*;
use mcedit::{Candidates, Clash, Error, GoalState, Op, Session};

#[test]
fn modus_ponens_leaves_a_metavariable_that_a_close_resolves() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    assert_eq!(s.show_goal(0), "|- ps");
    let out = s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(out.goals, 1..3);
    assert_eq!(out.metas, 0..2);
    assert_eq!(s.show_goal(1), "|- ?0");
    assert_eq!(s.show_goal(2), "|- ( ?0 -> ps )");
    assert_eq!(s.status().open, [1, 2]);
    assert_eq!(s.status().metas, [0]);

    s.close(2, id(&db, "t1.2")).unwrap();
    assert_eq!(s.show_goal(1), "|- ph");
    assert!(s.status().metas.is_empty());
    assert_eq!(s.status().open, [1]);
    s.close(1, id(&db, "t1.1")).unwrap();
    assert!(s.status().open.is_empty());

    let done = s.export().unwrap();
    assert_eq!(labels(&db, &done), "wph wps t1.1 t1.2 ax-mp");
    assert_eq!(verified(&db, &done), Ok(()));
}

#[test]
fn a_failed_apply_changes_nothing() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tax5-ok");
    let (sizes, saved) = (s.sizes(), s.save());
    assert_eq!(
        s.apply(0, id(&db, "ax-1")).unwrap_err(),
        Error::Unify(Clash::Constructor)
    );
    assert_eq!(s.sizes(), sizes);
    assert_eq!(s.save(), saved);
    assert_eq!(s.show_goal(0), "|- ( ps -> A. y ps )");
    assert!(matches!(s.goal(0).unwrap().state, GoalState::Open));
    assert!(s.apply(0, id(&db, "ax-5")).is_ok());
}

#[test]
fn operations_on_the_wrong_things_are_refused() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    assert_eq!(s.apply(7, id(&db, "ax-mp")), Err(Error::NoSuchGoal));
    assert_eq!(s.apply(0, id(&db, "wi")), Err(Error::NotAnAssertion));
    assert_eq!(s.apply(0, id(&db, "t1.1")), Err(Error::NotAnAssertion));
    assert_eq!(s.close(0, id(&db, "ax-1")), Err(Error::NotAHypothesis));
    s.close(0, id(&db, "t1.1")).unwrap_err();
    s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(s.apply(0, id(&db, "ax-mp")), Err(Error::GoalNotOpen));
    assert_eq!(s.assign(9, 0), Err(Error::NoSuchMeta));
    assert_eq!(
        Session::start_in(cat.clone(), id(&db, "ax-mp")).err(),
        Some(Error::NotATheorem)
    );
}

#[test]
fn only_assertions_before_the_theorem_may_be_cited() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut early = session(&cat, "tfwd");
    assert_eq!(early.apply(0, id(&db, "tafter")), Err(Error::Forward));
    assert_eq!(early.apply(0, id(&db, "tfwd")), Err(Error::Forward));
    assert!(early.apply(0, id(&db, "a1i")).is_ok());
    let Candidates::List(found) = session(&cat, "tfwd").candidates(0, 99).unwrap() else {
        panic!("a constrained goal lists candidates");
    };
    assert!(found.iter().all(|c| c.assertion < id(&db, "tfwd")));

    let mut late = session(&cat, "tafter");
    assert!(late.apply(0, id(&db, "tfwd")).is_ok());
}

#[test]
fn assign_picks_a_declared_variable_of_the_right_typecode() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(s.assign(0, sym(&db, "x")), Err(Error::NotAVariable));
    assert_eq!(s.assign(0, sym(&db, "->")), Err(Error::NotAVariable));
    s.assign(0, sym(&db, "th")).unwrap();
    assert_eq!(s.show_goal(1), "|- th");
    assert_eq!(s.assign(0, sym(&db, "ph")), Err(Error::AlreadyBound));
}

#[test]
fn undo_reverses_every_kind_of_operation() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    let mut snapshots = vec![(s.save(), s.status(), s.sizes())];
    let steps = [
        Op::Apply {
            goal: 0,
            assertion: id(&db, "ax-mp"),
        },
        Op::Refine {
            meta: 0,
            syntax: id(&db, "wi"),
        },
        Op::Assign {
            meta: 2,
            var: sym(&db, "ph"),
        },
        Op::Assign {
            meta: 3,
            var: sym(&db, "ps"),
        },
        Op::Close {
            goal: 1,
            hyp: id(&db, "t1.2"),
        },
    ];
    for op in steps {
        s.run(op).unwrap();
        snapshots.push((s.save(), s.status(), s.sizes()));
    }
    assert_eq!(s.show_goal(1), "|- ( ph -> ps )");
    for (bytes, status, sizes) in snapshots.iter().rev().skip(1) {
        s.undo().unwrap();
        assert_eq!(&s.save(), bytes);
        assert_eq!(&s.status(), status);
        assert_eq!(&s.sizes(), sizes);
    }
    assert_eq!(s.undo(), Err(Error::NothingToUndo));
    assert_eq!(s.show_goal(0), "|- ps");
    s.play(&steps).unwrap();
    assert_eq!(s.ops(), steps);
}

#[test]
fn undo_after_a_failed_operation_goes_back_one_successful_step() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.apply(2, id(&db, "ax-5")).unwrap_err();
    assert_eq!(
        s.undo(),
        Ok(Op::Apply {
            goal: 0,
            assertion: id(&db, "ax-mp")
        })
    );
}

#[test]
fn a_session_round_trips_through_its_bytes() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.apply(2, id(&db, "ax-1")).unwrap_err();
    s.close(2, id(&db, "t1.2")).unwrap();
    s.undo().unwrap();
    s.refine(0, id(&db, "wi")).unwrap();
    s.assign(2, sym(&db, "ph")).unwrap();
    s.assign(3, sym(&db, "ps")).unwrap();
    s.close(1, id(&db, "t1.2")).unwrap();
    let bytes = s.save();

    let fresh = catalog(&db, &g);
    let back = Session::restore(fresh, &bytes).unwrap();
    assert_eq!(back.ops(), s.ops());
    assert_eq!(back.status(), s.status());
    assert_eq!(back.sizes(), s.sizes());
    assert_eq!(back.save(), bytes);
    for goal in 0..s.goal_count() as u32 {
        assert_eq!(back.show_goal(goal), s.show_goal(goal));
    }
    assert!(bytes.len() < 24, "{} bytes", bytes.len());
    for cut in 0..bytes.len() {
        let _ = Session::restore(catalog(&db, &g), &bytes[..cut]);
    }
    assert_eq!(
        Session::restore(catalog(&db, &g), &[]).err(),
        Some(Error::Corrupt)
    );
    let mut bad = bytes.clone();
    bad.push(9);
    assert_eq!(
        Session::restore(catalog(&db, &g), &bad).err(),
        Some(Error::Corrupt)
    );
}

#[test]
fn export_refuses_an_unfinished_proof() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    assert_eq!(
        s.export().unwrap_err(),
        Error::Unfinished {
            open: 1,
            metas: 0,
            violations: 0
        }
    );
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.close(1, id(&db, "t1.1")).unwrap();
    assert_eq!(
        s.export().unwrap_err(),
        Error::Unfinished {
            open: 1,
            metas: 0,
            violations: 0
        }
    );
    s.close(2, id(&db, "t1.2")).unwrap();
    assert!(s.export().is_ok());
}

#[test]
fn export_refuses_while_a_metavariable_remains() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tnew.1")).unwrap();
    assert!(s.status().open.is_empty());
    assert_eq!(
        s.export().unwrap_err(),
        Error::Unfinished {
            open: 0,
            metas: 1,
            violations: 0
        }
    );
}
