//! The preview's wait after a letter: a letter that moves the cursor makes the preview pending with the
//! full wait, and one that leaves the cursor on its row, or misses, leaves a settled preview as it was.
//! The rig's press_char starts a pending preview's wait again itself, so these read the app's own
//! variables from a settled list.

use crate::list_find_arms::{Press, arms};
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::list_wait::WAIT_TICKS;
use mcui::Kind;

const CASES: [(Kind, bool); 8] = [
    (Kind::All, false),
    (Kind::Theorems, false),
    (Kind::Axioms, false),
    (Kind::Unchecked, true),
    (Kind::Unchecked, false),
    (Kind::All, true),
    (Kind::Theorems, true),
    (Kind::Axioms, true),
];

fn settled(kind: Kind, table: bool) -> Lister {
    let marks = every_mark(proofs_three());
    let list = list_under(proofs_three(), 1, kind, table.then_some(&marks));
    let mut list = list;
    assert_eq!(list.var("LV_PREVIEW"), ROW, "{kind:?}: settled");
    assert_eq!(list.var("LV_COUNT"), 0);
    list.skip_the_wait();
    list
}

#[test]
fn a_letter_that_moves_the_cursor_starts_the_wait_and_the_others_leave_a_settled_preview() {
    let labels = labels_of(proofs_three());
    for (kind, table) in CASES {
        let at = format!("{kind:?} table {table}");
        let probe = settled(kind, table);
        let items = probe.items();
        let first = labels[items[0]].clone();
        let far = items
            .iter()
            .map(|&i| labels[i].chars().next().unwrap())
            .find(|&c| c != first.chars().next().unwrap())
            .expect("a label that starts with another character");
        let miss = match arms(&probe).remove(2).press {
            Press::Char(c) => c,
            _ => unreachable!(),
        };

        let mut list = settled(kind, table);
        let before = list.pos().cursor;
        list.press_char(far);
        assert!(list.pos().cursor > before, "{at}: {far:?} moves the cursor");
        println!("{at}: after a letter that moves, preview {} count {}", list.seen.0, list.seen.1);
        assert_eq!(list.seen.0, PENDING, "{at}: the preview is pending");
        assert_eq!(u32::from(list.seen.1), WAIT_TICKS, "{at}: the full wait");

        let mut list = settled(kind, table);
        list.press_char(first.chars().next().unwrap());
        list.skip_the_wait();
        list.settle();
        assert_eq!(list.var("LV_PREVIEW"), ROW);
        if first.len() > 1 {
            let second = first.chars().nth(1).unwrap();
            let cursor = list.pos().cursor;
            list.press_char(second);
            assert_eq!(list.pos().cursor, cursor, "{at}: {second:?} stays on the row");
            assert_eq!((list.seen.0, list.seen.1), (ROW, 0), "{at}: a hit on the cursor's row keeps the preview");
        }

        let mut list = settled(kind, table);
        list.press_char(miss);
        assert!(list.find().missed);
        assert_eq!((list.seen.0, list.seen.1), (ROW, 0), "{at}: a miss keeps the preview");
    }
}
