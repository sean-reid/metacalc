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
        let saved = s.save();
        match s.refine(meta, wn) {
            Ok(out) => meta = out.metas.start,
            Err(e) => {
                assert_eq!(s.save(), saved);
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
    Session::start(&db, &g, db.find("t").unwrap()).map(drop)
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
