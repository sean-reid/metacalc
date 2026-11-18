mod common;

use common::*;
use mcedit::{Catalog, Clash, Error, Session};
use mcpack::mm::Kind;

fn put(out: &mut Vec<u8>, mut n: u32) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

type Ops = Vec<(u8, u32, u32)>;

/// A saved session written by hand: the theorem, the flag and operations as (tag, a, b).
fn bytes(theorem: u32, flag: u8, ops: &[(u8, u32, u32)]) -> Vec<u8> {
    let mut out = vec![1];
    put(&mut out, theorem);
    out.push(flag);
    for &(tag, a, b) in ops {
        out.push(tag);
        put(&mut out, a);
        put(&mut out, b);
    }
    out
}

#[test]
fn refine_cannot_cite_a_syntax_axiom_that_comes_after_the_theorem() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tfwd");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    let late = id(&db, "wlate");
    assert_eq!(s.refine(0, late), Err(Error::Forward));
    assert!(s.refine(0, id(&db, "wn")).is_ok());
    let mut after = session(&cat, "tafter");
    after.apply(0, id(&db, "ax-mp")).unwrap();
    assert!(after.refine(0, id(&db, "wn")).is_ok());

    let ax_mp = id(&db, "ax-mp");
    let tampered = bytes(id(&db, "tfwd"), 0, &[(0, 0, ax_mp), (3, 0, late)]);
    assert_eq!(
        Session::restore(catalog(&db, &g), &tampered).err(),
        Some(Error::Corrupt)
    );
}

#[test]
fn a_theorem_id_outside_the_database_is_an_error() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    for theorem in [db.stmts.len() as u32, 9999, u32::MAX] {
        assert_eq!(
            Session::start(&db, &g, theorem).err(),
            Some(Error::NotATheorem)
        );
        assert_eq!(
            Session::restore(cat.clone(), &bytes(theorem, 0, &[])).err(),
            Some(Error::Corrupt)
        );
    }
}

#[test]
fn stale_and_out_of_range_ids_are_answered_not_panicked_on() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.refine(0, id(&db, "wn")).unwrap();
    let stale_term = s.goal(1).unwrap().term;
    assert_eq!(s.children(0), 1..3);
    assert!(s.show_goal(2).is_some());
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(s.children(0), 0..0);
    assert_eq!(s.goal(1).map(|g| g.term), None);
    assert_eq!(s.show_goal(1), None);
    assert_eq!(s.children(1), 0..0);
    assert_eq!(s.show(stale_term), None);
    for goal in [99, u32::MAX] {
        assert_eq!(s.show_goal(goal), None);
        assert_eq!(s.children(goal), 0..0);
        assert_eq!(s.apply(goal, id(&db, "ax-mp")), Err(Error::NoSuchGoal));
        assert_eq!(s.close(goal, id(&db, "t1.1")), Err(Error::NoSuchGoal));
        assert!(s.candidates(goal, 1).is_err());
        assert_eq!(s.auto(goal, 1, 1), None);
    }
    for meta in [0, 99, u32::MAX] {
        assert_eq!(s.assign(meta, sym(&db, "ph")), Err(Error::NoSuchMeta));
        assert_eq!(s.refine(meta, id(&db, "wn")), Err(Error::NoSuchMeta));
    }
    assert_eq!(s.show(u32::MAX), None);
}

#[test]
fn a_varint_with_bits_beyond_thirty_two_is_corrupt() {
    let db = db();
    let g = grammar(&db);
    let ax_mp = id(&db, "ax-mp");
    let mut tampered = bytes(id(&db, "t1"), 0, &[]);
    tampered.push(0);
    tampered.extend([0x80, 0x80, 0x80, 0x80, 0x10]);
    put(&mut tampered, ax_mp);
    assert_eq!(
        Session::restore(catalog(&db, &g), &tampered).err(),
        Some(Error::Corrupt)
    );
    let mut valid = bytes(id(&db, "t1"), 0, &[(0, 0, ax_mp)]);
    assert!(Session::restore(catalog(&db, &g), &valid).is_ok());
    valid.extend([0, 0x80, 0x80, 0x80, 0x80, 0x01]);
    assert_eq!(
        Session::restore(catalog(&db, &g), &valid).err(),
        Some(Error::Corrupt)
    );
}

#[test]
fn permission_to_add_pairs_survives_a_save() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdum");
    s.allow_new_disjoint(true);
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tdum.1")).unwrap();
    let meta = s.status().metas[0];
    assert_eq!(s.assign(meta, sym(&db, "z")).unwrap().violations, []);
    let saved = s.save();
    assert_eq!(saved[..3], [1, id(&db, "tdum") as u8, 1]);
    let back = Session::restore(catalog(&db, &g), &saved).unwrap();
    assert_eq!(back.status(), s.status());
    assert_eq!(back.status().violations, []);
    assert_eq!(back.status().added_dv.len(), 1);
    assert_eq!(back.save(), saved);

    let strict = session(&cat, "tdum");
    assert_eq!(strict.save()[2], 0);
    let mut flag = saved.clone();
    flag[2] = 2;
    assert_eq!(
        Session::restore(catalog(&db, &g), &flag).err(),
        Some(Error::Corrupt)
    );
}

#[test]
fn only_variables_in_force_at_the_theorem_can_be_assigned() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    s.apply(1, id(&db, "ax-gen")).unwrap();
    s.close(2, id(&db, "tnew.1")).unwrap();
    let meta = s.status().metas[0];
    assert_eq!(s.assign(meta, sym(&db, "u")), Err(Error::NotAVariable));
    assert_eq!(s.assign(meta, 9999), Err(Error::NotAVariable));
    assert!(s.assign(meta, sym(&db, "w")).is_ok());
}

#[test]
fn assign_refuses_a_constant_and_a_variable_of_another_typecode() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    assert_eq!(s.store().meta_typecode(0), Some(sym(&db, "wff")));
    assert_eq!(s.assign(0, sym(&db, "->")), Err(Error::NotAVariable));
    assert_eq!(s.assign(0, sym(&db, "x")), Err(Error::NotAVariable));
    assert_eq!(s.status().metas, [0]);

    let mut dummy = session(&cat, "tnew");
    dummy.apply(0, id(&db, "axfree")).unwrap();
    assert_eq!(dummy.store().meta_typecode(1), Some(sym(&db, "setvar")));
    assert_eq!(dummy.assign(1, sym(&db, "ph")), Err(Error::NotAVariable));
    assert!(dummy.assign(1, sym(&db, "y")).is_ok());
}

#[test]
fn refine_refuses_the_wrong_typecode_and_a_statement_that_is_not_syntax() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    assert_eq!(s.refine(1, id(&db, "wi")), Err(Error::NotSyntax));
    let mut t = session(&cat, "t1");
    t.apply(0, id(&db, "ax-mp")).unwrap();
    for label in ["ax-1", "t1.1", "wph", "a1i"] {
        assert_eq!(
            t.refine(0, id(&db, label)),
            Err(Error::NotSyntax),
            "{label}"
        );
    }
    assert_eq!(t.refine(0, 9999), Err(Error::NotSyntax));
    assert_eq!(t.status().metas, [0]);
}

#[test]
fn an_occurs_check_failure_in_a_session_changes_nothing() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tocc");
    s.apply(0, id(&db, "axh")).unwrap();
    assert_eq!(
        s.show_goal(1).unwrap(),
        "|- ( ?0 -> ( ?1 -> ( ?0 -> ?1 ) ) )"
    );
    let (sizes, saved, status) = (s.sizes(), s.save(), s.status());
    assert_eq!(
        s.apply(1, id(&db, "ax-1")),
        Err(Error::Unify(Clash::Occurs))
    );
    assert_eq!((s.sizes(), s.save(), s.status()), (sizes, saved, status));
}

#[test]
fn restore_refuses_operations_that_are_well_formed_but_wrong() {
    let db = db();
    let g = grammar(&db);
    let t1 = id(&db, "t1");
    let ax_mp = id(&db, "ax-mp");
    let bad: [(u32, Ops); 4] = [
        (id(&db, "tfwd"), vec![(0, 0, id(&db, "tafter"))]),
        (t1, vec![(1, 0, id(&db, "tfwd.1"))]),
        (t1, vec![(0, 0, ax_mp), (0, 0, ax_mp)]),
        (t1, vec![(0, 5, ax_mp)]),
    ];
    for (theorem, ops) in bad {
        let tampered = bytes(theorem, 0, &ops);
        assert_eq!(
            Session::restore(catalog(&db, &g), &tampered).err(),
            Some(Error::Corrupt)
        );
    }
    let mut s = session(&catalog(&db, &g), "t1");
    assert_eq!(s.close(0, id(&db, "tfwd.1")), Err(Error::NotAHypothesis));
    assert_eq!(s.apply(0, id(&db, "tafter")), Err(Error::Forward));
}

#[test]
fn undo_takes_back_an_added_pair_and_a_violation() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);

    let mut open = session(&cat, "tnew");
    open.apply(0, id(&db, "axfree")).unwrap();
    open.apply(1, id(&db, "ax-gen")).unwrap();
    open.close(2, id(&db, "tnew.1")).unwrap();
    let before = open.status();
    let meta = before.metas[0];
    open.assign(meta, sym(&db, "z")).unwrap();
    assert_eq!(open.status().added_dv.len(), 1);
    open.undo().unwrap();
    assert_eq!(open.status(), before);
    assert_eq!(open.status().added_dv, []);

    let mut strict = session(&cat, "tdum");
    strict.apply(0, id(&db, "axfree")).unwrap();
    strict.apply(1, id(&db, "ax-gen")).unwrap();
    strict.close(2, id(&db, "tdum.1")).unwrap();
    let before = strict.status();
    strict.assign(before.metas[0], sym(&db, "z")).unwrap();
    assert_eq!(strict.status().violations.len(), 1);
    strict.undo().unwrap();
    assert_eq!(strict.status(), before);
    assert_eq!(strict.status().violations, []);
}

#[test]
fn a_session_starts_with_one_hypothesis_per_essential_hypothesis() {
    let db = db();
    let g = grammar(&db);
    let cat: std::rc::Rc<Catalog> = catalog(&db, &g);
    let mut checked = 0;
    for theorem in db.theorems().filter(|&t| db.stmt(t).label != "tbad") {
        let s = Session::start_in(cat.clone(), theorem).unwrap();
        let frame = &db.stmt(theorem).frame.hyps;
        let essential: Vec<_> = frame
            .iter()
            .copied()
            .filter(|&h| db.stmt(h).kind == Kind::Essential)
            .collect();
        let held: Vec<_> = s.hypotheses().iter().map(|h| h.0).collect();
        assert_eq!(held, essential, "{}", db.stmt(theorem).label);
        checked += 1;
    }
    assert!(checked > 10);
}

#[test]
fn a_pair_waits_through_a_refine_for_the_assign_that_completes_it() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tdv");
    s.apply(0, id(&db, "axdv")).unwrap();
    let out = s.refine(1, id(&db, "wn")).unwrap();
    assert_eq!(out.violations, []);
    assert_eq!(s.status().violations, []);
    let out = s.assign(out.metas.start, sym(&db, "th")).unwrap();
    assert_eq!(out.violations.len(), 1);
}

#[test]
fn a_theorem_whose_hypothesis_the_grammar_cannot_read_does_not_start() {
    let db = db();
    let g = grammar(&db);
    assert_eq!(
        Session::start(&db, &g, id(&db, "tbad")).err(),
        Some(Error::Syntax(mcpack::mm::SyntaxError::NoParse))
    );
}

#[test]
fn a_variable_in_force_whose_only_float_comes_later_is_refused() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let q = sym(&db, "q");
    let theorem = id(&db, "tnew");
    assert!(db.var_active(q, theorem));
    assert!(db.stmt(id(&db, "vq")).expr[1] == q && id(&db, "vq") > theorem);
    let mut s = session(&cat, "tnew");
    s.apply(0, id(&db, "axfree")).unwrap();
    assert_eq!(s.store().meta_typecode(1), Some(sym(&db, "setvar")));
    assert_eq!(s.assign(1, q), Err(Error::NotAVariable));
    assert!(s.vars().of_typecode(sym(&db, "setvar")).all(|v| v != q));
    assert!(s.assign(1, sym(&db, "y")).is_ok());
}

#[test]
fn a_saved_session_starts_with_its_format_byte() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let s = session(&cat, "t1");
    let saved = s.save();
    assert_eq!(saved[0], 1);
    assert!(Session::restore(cat.clone(), &saved).is_ok());
    for version in [0, 2, 255] {
        let mut other = saved.clone();
        other[0] = version;
        assert_eq!(
            Session::restore(cat.clone(), &other).err(),
            Some(Error::Corrupt)
        );
    }
    assert_eq!(Session::restore(cat, &[]).err(), Some(Error::Corrupt));
}

#[test]
fn stale_goal_ids_after_an_undo_are_refused_everywhere() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "t1");
    s.apply(0, id(&db, "ax-mp")).unwrap();
    s.undo().unwrap();
    let (sizes, saved) = (s.sizes(), s.save());
    for stale in [1, 2] {
        assert_eq!(s.apply(stale, id(&db, "ax-mp")), Err(Error::NoSuchGoal));
        assert_eq!(s.close(stale, id(&db, "t1.1")), Err(Error::NoSuchGoal));
        assert_eq!(s.candidates(stale, 9), Err(Error::NoSuchGoal));
        assert_eq!(s.auto(stale, 2, 50), None);
        assert_eq!(s.children(stale), 0..0);
        assert_eq!(s.show_goal(stale), None);
    }
    assert_eq!((s.sizes(), s.save()), (sizes, saved));
}
