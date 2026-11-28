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
        let miss = match arms(&probe).remove(2).press {
            Press::Char(c) => c,
            _ => unreachable!(),
        };
        // The shortest prefix of the last shown label whose first hit is not the first shown assertion:
        // its last character moves the cursor from where the shorter prefix left it.
        let last = labels[*items.last().unwrap()].clone();
        let hit_of = |p: &str| {
            items
                .iter()
                .position(|&i| labels[i].starts_with(p))
                .unwrap()
        };
        let start = probe.pos().cursor;
        let cursor_at = |n: usize| if n == 0 { start } else { hit_of(&last[..n]) };
        let n = (1..=last.len().min(8))
            .find(|&n| hit_of(&last[..n]) != cursor_at(n - 1))
            .expect("a character that moves the cursor");

        let mut list = settled(kind, table);
        list.type_settled(&last[..n - 1]);
        list.skip_the_wait();
        list.settle();
        assert_eq!(list.var("LV_PREVIEW"), ROW, "{at}: settled before");
        let before = list.pos().cursor;
        assert_eq!(
            before,
            cursor_at(n - 1),
            "{at}: where the shorter prefix leaves the cursor"
        );
        list.press_char(last.as_bytes()[n - 1] as char);
        assert!(
            list.pos().cursor != before,
            "{at}: {last} moves the cursor at character {n}"
        );
        println!(
            "{at}: after a letter that moves, preview {} count {}",
            list.seen.0, list.seen.1
        );
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
            assert_eq!(
                list.pos().cursor,
                cursor,
                "{at}: {second:?} stays on the row"
            );
            assert_eq!(
                (list.seen.0, list.seen.1),
                (ROW, 0),
                "{at}: a hit on the cursor's row keeps the preview"
            );
        }

        let mut list = settled(kind, table);
        list.press_char(miss);
        assert!(list.find().missed);
        assert_eq!(
            (list.seen.0, list.seen.1),
            (ROW, 0),
            "{at}: a miss keeps the preview"
        );
    }
}
