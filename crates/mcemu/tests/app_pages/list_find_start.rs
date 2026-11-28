//! What a letter's search calls and where it starts: a prefix with an empty range calls neither the walk
//! nor the copy, and a second letter typed after the table was lost starts from the cursor.

use crate::list_find_arms::{Press, arms};
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind, State, Status};

const CASES: [(Kind, bool); 5] = [
    (Kind::All, false),
    (Kind::Theorems, false),
    (Kind::Axioms, false),
    (Kind::Unchecked, true),
    (Kind::Unchecked, false),
];

/// A list of the proofs library under `kind` whose reader calls the find makes are counted and never
/// refused: slot 0 the range, 1 the walk, 2 the copy.
fn counted(kind: Kind, table: bool) -> Lister {
    let marks = every_mark(proofs_three());
    let mut list = list_under(proofs_three(), 1, kind, table.then_some(&marks));
    list.fail_at(0, "far_lib_find_range", 99, 3);
    list.fail_at(1, "far_lib_find_walk", 99, 3);
    list.fail_at(2, "far_lib_find_copy", 99, 3);
    list
}

/// The character the named arm presses, from the labels the list shows.
fn arm_char(list: &Lister, name: &str) -> char {
    match arms(list).into_iter().find(|a| a.name == name).unwrap().press {
        Press::Char(c) => c,
        _ => panic!("{name} presses no character"),
    }
}

/// A character one press types that no label of the whole library starts with.
fn unused_first(list: &Lister) -> char {
    let labels = labels_of(list.built);
    ('a'..='z')
        .chain(['0', '3', '.'])
        .find(|&c| !labels.iter().any(|l| l.starts_with(c)))
        .expect("a character no label starts with")
}

#[test]
fn a_prefix_with_an_empty_range_calls_neither_the_walk_nor_the_copy() {
    for (kind, table) in CASES {
        let at = format!("{kind:?} table {table}");
        let mut list = counted(kind, table);
        let miss = unused_first(&list);
        list.press_char(miss);
        assert!(list.find().missed, "{at}: {miss:?} misses");
        let calls = (list.fail_calls(0), list.fail_calls(1), list.fail_calls(2));
        println!("{at}, a miss: range, walk, copy calls {calls:?}");
        assert_eq!(calls, (1, 0, 0), "{at}: an empty range calls the range only");

        let mut list = counted(kind, table);
        let hit = arm_char(&list, "letter, first");
        list.press_char(hit);
        assert!(!list.find().missed, "{at}: {hit:?} hits");
        let calls = (list.fail_calls(0), list.fail_calls(1), list.fail_calls(2));
        println!("{at}, a hit: range, walk, copy calls {calls:?}");
        assert_eq!(calls.0, 1, "{at}");
        if table && kind == Kind::Unchecked {
            assert_eq!((calls.1, calls.2 >= 1), (0, true), "{at}: the copy walks");
        } else {
            assert_eq!((calls.1, calls.2), (1, 0), "{at}: the walk answers");
        }
    }
}

/// Six axioms whose first two are checked: Unchecked shows the other four, the first of which, assertion
/// 2, is the first to start with "a" among them, and assertion 0, hidden, starts with "ax" too.
#[test]
fn a_second_letter_after_the_table_was_lost_searches_from_the_cursor_and_not_the_top() {
    let built = pattern_built("pages-find-start", "AAAAAA");
    let mut status = vec![Status::Unchecked; 6];
    status[0] = Status::Checked;
    status[1] = Status::Checked;
    let data = table_of(built, &status);
    let labels = labels_of(built);
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(list.items(), [2, 3, 4, 5], "the table hides the first two");
    assert!(labels[0].starts_with("ax") && labels[2].starts_with("ax"));
    list.type_settled("a");
    let cursor = |list: &mut Lister| {
        let frame = list.frame_bytes();
        u16::from_le_bytes([frame[2], frame[3]])
    };
    assert_eq!(cursor(&mut list), 2, "the first letter lands on assertion 2");
    assert_eq!(list.var("LV_TABLE"), 1);

    list.run.m().delete_var("MCSTAT");
    list.keep_waiting();
    list.tap(scans_for('x')[0]);
    list.ui.press(Key::Letter('x'));
    list.run.next_blit();
    list.run.run(300_000);
    let find = list.find();
    assert_eq!((find.len, find.missed, find.chars.as_str()), (2, false, "ax"));
    let (own, shown) = (list.pos(), list.items());
    let own = (shown[own.cursor], shown[own.top]);
    let frame = list.frame_bytes();
    let app = (
        u16::from_le_bytes([frame[2], frame[3]]),
        u16::from_le_bytes([frame[4], frame[5]]),
    );
    println!("after the second letter: the app (cursor, top) {app:?}, the model that kept its list {own:?}");
    assert_eq!(app.0, 2, "the search starts at the cursor, not at hidden assertion 0");
    assert_eq!(own.0, 2, "the model's own state is on the same assertion");

    let mut bare = crate::shellrig::model::ui(built, &[]);
    for key in [Key::Enter, Key::Letter('a'), Key::Letter('x'), Key::Letter('-'), Key::Letter('2')] {
        bare.press(key);
    }
    let State::List { cursor, top, .. } = bare.state() else {
        panic!("the model is on a list");
    };
    println!("the model that never had a table, after typing ax-2: (cursor, top) {:?}", (cursor, top));
    assert_eq!((cursor, top), (2, 1));
    assert_eq!((usize::from(app.0), usize::from(app.1)), (cursor, top), "the app is where a model with no table lands on assertion 2");
    assert_ne!(usize::from(app.1), own.1, "the kept list puts the first row where it hides two assertions");
}
