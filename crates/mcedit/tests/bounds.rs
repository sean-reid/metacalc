mod common;

use common::*;
use mcedit::{Error, MAX_TERM_DEPTH, MAX_TERM_NODES, Session, Store, TermId};
use mcpack::mm::{self, Grammar};

fn nodes(store: &Store, term: TermId) -> usize {
    1 + store
        .children(term)
        .iter()
        .map(|&k| nodes(store, k))
        .sum::<usize>()
}

fn largest(s: &Session) -> usize {
    (0..s.goal_count() as u32)
        .map(|g| nodes(s.store(), s.goal(g).unwrap().term))
        .max()
        .unwrap()
}

/// One round of doubling: `ax-mp` splits the goal, then `axd` makes its first child `( T -> T )`.
fn double(s: &mut Session, goal: u32) -> Result<u32, Error> {
    let split = s.apply(goal, id(s.db(), "ax-mp"))?;
    s.apply(split.goals.start + 1, id(s.db(), "axd"))?;
    Ok(split.goals.start)
}

#[test]
fn a_term_that_would_double_past_the_bound_is_refused_and_the_session_survives() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tbig");
    let mut goal = 0;
    let mut rounds = 0;
    let refused = loop {
        let (sizes, saved, ops) = (s.sizes(), s.save(), s.ops().len());
        match double(&mut s, goal) {
            Ok(next) => {
                goal = next;
                rounds += 1;
                assert!(largest(&s) <= MAX_TERM_NODES);
            }
            Err(e) => {
                while s.ops().len() > ops {
                    s.undo().unwrap();
                }
                assert_eq!((s.sizes(), s.save()), (sizes, saved));
                break e;
            }
        }
        assert!(rounds < 40, "the bound never applied");
    };
    assert_eq!(refused, Error::TooLarge);
    assert_eq!(rounds, 10);
    assert!(s.show_goal(goal).is_some());
    assert!(s.peak().terms.nodes < 100);
}

#[test]
fn a_chain_of_refines_stops_at_the_depth_bound() {
    let db = db();
    let g = grammar(&db);
    let cat = catalog(&db, &g);
    let mut s = session(&cat, "tbig");
    let mut meta = s.apply(0, id(&db, "ax-mp")).unwrap().metas.start;
    let mut refines = 0;
    let wn = id(&db, "wn");
    let refused = loop {
        let (sizes, saved) = (s.sizes(), s.save());
        match s.refine(meta, wn) {
            Ok(out) => meta = out.metas.start,
            Err(e) => {
                assert_eq!((s.sizes(), s.save()), (sizes, saved));
                break e;
            }
        }
        refines += 1;
        assert!(refines < 4 * MAX_TERM_DEPTH, "the bound never applied");
    };
    assert_eq!(refused, Error::TooLarge);
    assert_eq!(refines, MAX_TERM_DEPTH - 2);
}

fn tree(levels: usize) -> String {
    match levels {
        0 => "ph".into(),
        n => format!("( {0} -> {0} )", tree(n - 1)),
    }
}

fn database(statement: &str) -> mm::Database {
    let src = format!(
        "$c ( ) -> -. wff |- $.\n$v ph ps $.\nwph $f wff ph $.\nwps $f wff ps $.\n\
         wi $a wff ( ph -> ps ) $.\nwn $a wff -. ph $.\n${{ t $p |- {statement} $= ? $. $}}\n"
    );
    mm::parse(src.into_bytes()).unwrap()
}

fn starts(statement: &str) -> Result<(), Error> {
    let db = database(statement);
    let g = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    Session::start_in(catalog(&db, &g), db.find("t").unwrap()).map(drop)
}

#[test]
fn a_statement_at_the_depth_bound_starts_and_one_level_deeper_does_not() {
    let chain = |levels: usize| format!("{}ph{}", "( ph -> ".repeat(levels), " )".repeat(levels));
    assert_eq!(starts(&chain(MAX_TERM_DEPTH - 1)), Ok(()));
    assert_eq!(starts(&chain(MAX_TERM_DEPTH)), Err(Error::TooLarge));
}

#[test]
fn a_statement_at_the_node_bound_starts_and_one_node_more_does_not() {
    assert_eq!(MAX_TERM_NODES, 4096);
    assert_eq!(starts(&format!("-. {}", tree(11))), Ok(()));
    assert_eq!(starts(&format!("-. -. {}", tree(11))), Err(Error::TooLarge));
}

fn sized(n: usize) -> String {
    match n {
        1 => "ph".into(),
        2 => "-. ph".into(),
        _ => {
            let left = (n - 1) / 2;
            format!("( {} -> {} )", sized(left), sized(n - 1 - left))
        }
    }
}

/// A theorem whose conclusion has `size` nodes, with hypotheses of 5 nodes and of 1 node.
fn edge(size: usize) -> mm::Database {
    let src = format!(
        "$c ( ) -> -. wff |- setvar Q $.\n$v ph ps x $.\nwph $f wff ph $.\nwps $f wff ps $.\n\
         vx $f setvar x $.\nwi $a wff ( ph -> ps ) $.\nwn $a wff -. ph $.\n\
         ${{ $d x ph $. wq $a wff Q x ph $. $}}\n\
         ${{ min $e |- ph $. maj $e |- ( ph -> ps ) $. ax-mp $a |- ps $. $}}\n\
         ${{ t.1 $e |- {} $. t.2 $e |- ph $. t $p |- {} $= ? $. $}}\n",
        sized(5),
        sized(size)
    );
    mm::parse(src.into_bytes()).unwrap()
}

#[test]
fn refused_steps_at_the_node_bound_leave_the_session_exactly_as_it_was() {
    let db = edge(MAX_TERM_NODES - 3);
    let g = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let mut s = Session::start_in(catalog(&db, &g), db.find("t").unwrap()).unwrap();
    let meta = s.apply(0, db.find("ax-mp").unwrap()).unwrap().metas.start;
    assert_eq!(largest(&s), MAX_TERM_NODES - 1);
    s.refine(meta, db.find("wn").unwrap()).unwrap();
    assert_eq!(largest(&s), MAX_TERM_NODES);
    s.undo().unwrap();

    let snapshot = |s: &Session| (s.sizes(), s.status(), s.save());
    let before = snapshot(&s);
    assert_eq!(s.refine(meta, db.find("wq").unwrap()), Err(Error::TooLarge));
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.close(1, db.find("t.1").unwrap()), Err(Error::TooLarge));
    assert_eq!(snapshot(&s), before);

    assert!(s.close(1, db.find("t.2").unwrap()).is_ok());
    assert_eq!(largest(&s), MAX_TERM_NODES - 1);
}

#[test]
fn a_wide_refine_reaches_the_node_bound_and_stops_there() {
    let db = edge(1);
    let g = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let mut s = Session::start_in(catalog(&db, &g), db.find("t").unwrap()).unwrap();
    let meta = s.apply(0, db.find("ax-mp").unwrap()).unwrap().metas.start;
    let wi = db.find("wi").unwrap();
    let mut queue = std::collections::VecDeque::from([meta]);
    let refused = loop {
        let at = queue.pop_front().unwrap();
        let (sizes, saved) = (s.sizes(), s.save());
        match s.refine(at, wi) {
            Ok(out) => queue.extend(out.metas),
            Err(e) => {
                assert_eq!((s.sizes(), s.save()), (sizes, saved));
                break e;
            }
        }
        assert!(largest(&s) <= MAX_TERM_NODES);
    };
    assert_eq!(refused, Error::TooLarge);
    assert_eq!(largest(&s), MAX_TERM_NODES - 1);
}
