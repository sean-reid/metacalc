mod common;

use common::*;
use mcedit::{Clash, Head, Store};
use mcpack::mm::Sym;
use std::collections::HashMap;

fn store(db: &mcpack::mm::Database) -> Store<'_> {
    let mut types: HashMap<Sym, Sym> = HashMap::new();
    for s in &db.stmts {
        if s.kind == mcpack::mm::Kind::Float {
            types.insert(s.expr[1], s.expr[0]);
        }
    }
    Store::new(db, types)
}

#[test]
fn a_metavariable_takes_the_term_it_meets() {
    let db = db();
    let mut s = store(&db);
    let wff = sym(&db, "wff");
    let (a, b) = (s.new_meta(wff), s.new_meta(wff));
    let pattern = s.app(id(&db, "wi"), &[a, b]);
    let (ph, ps) = (s.var(sym(&db, "ph")), s.var(sym(&db, "ps")));
    let inner = s.app(id(&db, "wi"), &[ps, ph]);
    let target = s.app(id(&db, "wi"), &[ph, inner]);
    assert_eq!(s.unify(pattern, target), Ok(()));
    assert_eq!(s.show(a).as_deref(), Some("ph"));
    assert_eq!(s.show(b).as_deref(), Some("( ps -> ph )"));
    assert!(s.is_ground(pattern));
}

#[test]
fn different_syntax_or_variables_clash() {
    let db = db();
    let mut s = store(&db);
    let (ph, ps) = (s.var(sym(&db, "ph")), s.var(sym(&db, "ps")));
    let implication = s.app(id(&db, "wi"), &[ph, ps]);
    let negation = s.app(id(&db, "wn"), &[ph]);
    assert_eq!(s.unify(implication, negation), Err(Clash::Constructor));
    assert_eq!(s.unify(ph, ps), Err(Clash::Constructor));
    assert_eq!(s.unify(ph, implication), Err(Clash::Constructor));
    let other = s.var(sym(&db, "ph"));
    assert_eq!(s.unify(ph, other), Ok(()));
}

#[test]
fn a_metavariable_cannot_contain_itself() {
    let db = db();
    let mut s = store(&db);
    let a = s.new_meta(sym(&db, "wff"));
    let ps = s.var(sym(&db, "ps"));
    let wrapped = s.app(id(&db, "wi"), &[a, ps]);
    assert_eq!(s.unify(a, wrapped), Err(Clash::Occurs));
    assert_eq!(s.unify(wrapped, a), Err(Clash::Occurs));
    assert_eq!(s.unify(a, a), Ok(()));
}

#[test]
fn typecodes_are_respected() {
    let db = db();
    let mut s = store(&db);
    let wff = s.new_meta(sym(&db, "wff"));
    let setvar = s.new_meta(sym(&db, "setvar"));
    let x = s.var(sym(&db, "x"));
    assert_eq!(s.unify(wff, x), Err(Clash::Typecode));
    assert_eq!(s.unify(setvar, x), Ok(()));
    assert_eq!(s.unify(wff, setvar), Err(Clash::Typecode));
}

#[test]
fn two_metavariables_unify_and_follow_each_other() {
    let db = db();
    let mut s = store(&db);
    let wff = sym(&db, "wff");
    let (a, b) = (s.new_meta(wff), s.new_meta(wff));
    assert_eq!(s.unify(a, b), Ok(()));
    assert!(matches!(s.head(a), Some(Head::Meta(_))));
    assert_eq!(s.head(a), s.head(b));
    let ph = s.var(sym(&db, "ph"));
    assert_eq!(s.unify(b, ph), Ok(()));
    assert_eq!(s.show(a).as_deref(), Some("ph"));
    assert_eq!(s.unbound().count(), 0);
}

#[test]
fn rolling_back_drops_bindings_terms_and_metavariables() {
    let db = db();
    let mut s = store(&db);
    let a = s.new_meta(sym(&db, "wff"));
    let mark = s.mark();
    let before = s.len();
    let ph = s.var(sym(&db, "ph"));
    let extra = s.new_meta(sym(&db, "wff"));
    assert_eq!(s.unify(a, ph), Ok(()));
    assert_eq!(s.unify(extra, ph), Ok(()));
    s.rollback(mark);
    assert_eq!(s.len(), before);
    assert_eq!(s.show(a).as_deref(), Some("?0"));
}

#[test]
fn unknown_ids_are_no_term_rather_than_a_panic() {
    let db = db();
    let mut s = store(&db);
    let ph = s.var(sym(&db, "ph"));
    assert_eq!(s.head(99), None);
    assert_eq!(s.children(99), &[] as &[u32]);
    assert_eq!(s.typecode(99), None);
    assert_eq!(s.show(99), None);
    assert_eq!(s.meta_term(5), None);
    assert_eq!(s.meta_typecode(5), None);
    assert!(!s.is_ground(99));
    assert_eq!(s.walk(99), 99);
    assert_eq!(s.unify(ph, 99), Err(Clash::Unknown));
    assert_eq!(s.bind(5, ph), Err(Clash::Unknown));
    let stale = s.var(sym(&db, "ps"));
    let mark = s.mark();
    let late = s.new_meta(sym(&db, "wff"));
    s.rollback(mark);
    assert_eq!(s.show(stale).as_deref(), Some("ps"));
    assert_eq!(s.head(late), None);
    assert_eq!(s.meta_term(0), None);
    assert!(!s.is_unbound(0));
}
