//! Frames no key builds and a reader that refuses a read in the middle of a walk: the enter and the
//! moves leave the first row on a shown assertion at or above the cursor, never on FFFFh.

use crate::app::*;
use crate::list_kind_keys::marks;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};

/// An axiom and a theorem alternate, so Theorems shows the odd assertions.
fn alternating() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| pattern_built("pages-kinds-edges", &"AT".repeat(15)))
}

fn thirty_axioms() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| pattern_built("pages-kinds-edges-lost", &"A".repeat(30)))
}

/// The list under Theorems, a notice over it, and the frame under the notice rewritten to `cursor` and
/// `top`, the numbers of shown assertions: CLEAR then runs list_enter on it.
fn list_under_a_notice(cursor: u16, top: u16) -> Lister {
    let mut list = Lister::open(alternating(), None, 0);
    list.settle();
    list.go_to_kind(Kind::Theorems);
    list.tap(F4);
    list.run.next_blit();
    list.run.run(300_000);
    let at = list.run.shell.app.label("SCR_STACK") + 16 + 2;
    let [c0, c1] = cursor.to_le_bytes();
    let [t0, t1] = top.to_le_bytes();
    list.run.m().load_ram(at, &[c0, c1, t0, t1]);
    let frame = list.run.frames()[1].clone();
    assert_eq!(frame[2..6], [c0, c1, t0, t1], "the frame is written");
    assert_eq!(frame[6], 1, "under Theorems");
    list
}

fn clear(list: &mut Lister) -> Vec<u8> {
    list.tap(CLEAR);
    let frame = list.run.next_blit();
    list.run.run(2_000);
    assert_eq!(list.run.top_kind(), KIND_LIST);
    frame
}

fn number(frame: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([frame[at], frame[at + 1]])
}

#[test]
fn an_enter_with_the_cursor_above_the_first_row_puts_the_first_row_on_the_cursor() {
    let mut met = 0;
    for (cursor, top) in [(1, 7), (7, 15), (19, 29), (3, 5)] {
        let at = format!("cursor {cursor}, first row {top}");
        assert!(
            cursor < top && cursor % 2 == 1 && top % 2 == 1,
            "{at}: both shown, the cursor above"
        );
        let mut list = list_under_a_notice(cursor, top);
        let frame = clear(&mut list);
        let now = list.run.frames()[1].clone();
        assert_eq!(
            (number(&now, 2), number(&now, 4)),
            (cursor, cursor),
            "{at}: the frame"
        );
        assert_eq!(now[6], 1, "{at}: the kind");
        same_frame(&at, &frame, &first_row_on(cursor));
        met += 1;
    }
    assert_eq!(met, 4);
}

/// The model's frame under Theorems with the first row on the shown assertion `cursor`, an odd number
/// at least four shown assertions short of the last, so that the model can scroll that far.
fn first_row_on(cursor: u16) -> Vec<u8> {
    let mut model = crate::shellrig::model::ui(alternating(), &[]);
    model.press(Key::Enter);
    model.press(Key::F3);
    for _ in 0..usize::from(cursor / 2) + 4 {
        model.press(Key::Down);
    }
    for _ in 0..4 {
        model.press(Key::Up);
    }
    crate::shellrig::model::frame(&model)
}

#[test]
fn an_enter_with_the_cursor_on_the_first_row_keeps_the_frame() {
    let mut met = 0;
    for cursor in [1, 7, 9, 11, 15, 21] {
        let at = format!("cursor and first row {cursor}");
        assert!(cursor % 2 == 1, "{at}: shown under Theorems");
        let mut list = list_under_a_notice(cursor, cursor);
        let frame = clear(&mut list);
        let now = list.run.frames()[1].clone();
        assert_eq!(
            (number(&now, 2), number(&now, 4)),
            (cursor, cursor),
            "{at}: the frame"
        );
        same_frame(&at, &frame, &first_row_on(cursor));
        met += usize::from(cursor / 2 >= 5);
    }
    assert_eq!(met, 3, "three with at least five shown assertions above");
}

#[test]
fn a_refused_read_for_the_cursors_predecessor_leaves_the_first_row_on_the_cursor_in_the_enter() {
    let (cursor, top) = (21, 1);
    let mut list = list_under_a_notice(cursor, top);
    list.fail_at(0, "far_lib_kind_prev", 1, 4);
    clear(&mut list);
    let now = list.run.frames()[1].clone();
    assert_eq!(
        list.fail_calls(0),
        1,
        "the one call after the walk's four steps was refused"
    );
    assert_eq!(
        (number(&now, 2), number(&now, 4)),
        (cursor, cursor),
        "the frame"
    );
}

#[test]
fn a_refused_read_for_the_cursors_predecessor_leaves_the_first_row_on_the_cursor_in_a_move() {
    let mut list = Lister::open(alternating(), None, 0);
    list.settle();
    list.go_to_kind(Kind::Theorems);
    for _ in 0..4 {
        list.key(DOWN, Key::Down);
        list.skip_the_wait();
        list.settle();
    }
    let before = list.run.frames()[1].clone();
    assert_eq!(
        (number(&before, 2), number(&before, 4)),
        (9, 1),
        "the cursor on the fifth, the first row on the first"
    );
    list.fail_at(0, "far_lib_kind_prev", 1, 4);
    list.tap(RIGHT);
    list.run.next_blit();
    list.run.run(300_000);
    let now = list.run.frames()[1].clone();
    assert_eq!(
        list.fail_calls(0),
        1,
        "the call after the walk's four steps was refused"
    );
    assert_eq!((number(&now, 2), number(&now, 4)), (19, 19), "the frame");
}

/// What the enter's walk answers when the table is lost before the store call number `lost_before`
/// (the calls counted from the enter's first, 1 based): the rules of lmk_below, with every assertion
/// unchecked from that call on. Returns the first row and, in order, whether each call asked for the
/// next (true) or the previous assertion.
fn walk_after_a_loss(
    n: usize,
    checked: &[usize],
    top: usize,
    cursor: usize,
    lost_before: usize,
) -> (usize, Vec<bool>) {
    fn ask(
        ops: &mut Vec<bool>,
        next: bool,
        from: usize,
        n: usize,
        checked: &[usize],
        lost: usize,
    ) -> usize {
        ops.push(next);
        let unchecked = |i: usize| ops.len() >= lost || !checked.contains(&i);
        if next {
            (from..n).find(|&i| unchecked(i)).unwrap_or(n)
        } else {
            (0..from)
                .rev()
                .find(|&i| unchecked(i))
                .unwrap_or(usize::MAX)
        }
    }
    let mut ops = vec![true, true];
    let (mut at, mut first) = (top, 0);
    for step in 0..4 {
        at = ask(&mut ops, true, at + 1, n, checked, lost_before);
        if step == 0 {
            first = at;
        }
        if at == cursor {
            return (top, ops);
        }
    }
    let before = ask(&mut ops, false, cursor, n, checked, lost_before);
    if before == at {
        return (first, ops);
    }
    if before == usize::MAX {
        return (cursor, ops);
    }
    let mut x = before;
    for _ in 0..3 {
        let p = ask(&mut ops, false, x, n, checked, lost_before);
        if p == usize::MAX {
            break;
        }
        x = p;
    }
    (x, ops)
}

/// The enter of a frame (`cursor`, first row 0) under Unchecked, with the marks that check `checked`,
/// loses its table before its store call number `lost_before`.
fn lost_in_the_walk(checked: &[usize], cursor: u16, lost_before: usize) {
    let built = thirty_axioms();
    let n = count(built);
    let (none, new) = (marks(n, &[]), marks(n, checked));
    let (data_none, data_new) = (table_of(built, &none), table_of(built, &new));
    let at = format!("cursor {cursor}, lost before call {lost_before}");
    let (first, ops) = walk_after_a_loss(n, checked, 0, usize::from(cursor), lost_before);
    assert!(
        lost_before <= ops.len(),
        "{at}: the loss falls inside the walk's {} calls",
        ops.len()
    );
    assert!(
        !checked.contains(&usize::from(cursor)) && !checked.contains(&0),
        "{at}: both shown"
    );
    let (kept, _) = walk_after_a_loss(n, checked, 0, usize::from(cursor), usize::MAX);
    assert_ne!(first, kept, "{at}: the loss changes the answer");
    let mut list = Lister::open(built, Some((&data_none, &none)), 0);
    list.settle();
    list.go_to_kind(Kind::Unchecked);
    list.run.m().delete_var("MCSTAT");
    crate::table::install(list.run.m(), &data_new);
    list.tap(F4);
    list.run.next_blit();
    list.run.run(300_000);
    let place = list.run.shell.app.label("SCR_STACK") + 16 + 2;
    let [c0, c1] = cursor.to_le_bytes();
    list.run.m().load_ram(place, &[c0, c1, 0, 0]);
    list.tap(CLEAR);
    for call in 1..=lost_before {
        let label = if ops[call - 1] {
            "store_next_not_one"
        } else {
            "store_prev_not_one"
        };
        list.run.run_to(label, 2_000_000);
        if call < lost_before {
            list.run.m().resume(1);
        }
    }
    list.run.m().delete_var("MCSTAT");
    list.run.next_blit();
    list.run.run(2_000);
    let now = list.run.frames()[1].clone();
    assert_eq!(
        list.var("LV_TABLE"),
        1,
        "{at}: the screen still thinks it has the table"
    );
    assert_eq!(
        (number(&now, 2), number(&now, 4)),
        (cursor, first as u16),
        "{at}: the frame"
    );
}

#[test]
fn a_table_lost_between_two_store_calls_of_the_enters_walk_is_met_as_no_table() {
    for lost_before in 3..=5 {
        lost_in_the_walk(&[2, 3, 5, 6, 7, 9], 8, lost_before);
    }
    for lost_before in [4, 6, 7, 8] {
        lost_in_the_walk(&[2, 3, 5, 6, 7, 9, 17, 18], 20, lost_before);
    }
}
