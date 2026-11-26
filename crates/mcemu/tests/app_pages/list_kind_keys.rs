//! What happens around a filtered list: the notice and the way back, CLEAR and in again, marks that
//! change under it, an auto power down, and a library with no kinds section.

use crate::app::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use crate::ui_support::same_frame;
use mcui::{Key, Kind, Status};

#[test]
fn the_notice_keys_and_the_way_back_keep_the_kind_the_cursor_and_the_top() {
    let built = crate::list_rig::proofs_three();
    let (data, status) = every_mark(built);
    let mut looked = 0;
    for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        for scan in [ENTER, F1, F2, F4] {
            let mut list = Lister::open(built, Some((&data, &status)), 1);
            list.settle();
            list.go_to_kind(kind);
            list.key(RIGHT, Key::Right);
            list.settle();
            let at = format!("{kind:?}, key {scan:02X}");
            let before = list.run.frames()[1].clone();
            assert_eq!(before[6], code(kind), "{at}");
            list.tap(scan);
            list.run.next_blit();
            list.run.run(300_000);
            assert_eq!(list.run.top_kind(), KIND_NOTICE, "{at}");
            list.tap(CLEAR);
            let frame = list.run.next_blit();
            list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
            assert_eq!(list.seen, (PENDING, 11), "{at}: blank, the wait begins");
            list.run.run(2_000);
            assert_eq!(list.run.frames()[1], before, "{at}: the frame");
            let pending = list.replayed();
            same_frame(&at, &frame, &model::frame(&pending));
            list.ui = pending;
            list.same_kind_position(&at);
            assert!(list.settle() || list.pos().len == 0, "{at}");
            looked += 1;
        }
    }
    assert_eq!(looked, 12);
}

#[test]
fn clear_to_libraries_and_in_again_starts_at_all() {
    let built = crate::list_rig::proofs_three();
    for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        let mut list = Lister::open(built, None, 0);
        list.settle();
        list.go_to_kind(kind);
        list.key(RIGHT, Key::Right);
        list.settle();
        assert!(list.key(CLEAR, Key::Clear));
        assert_eq!(list.run.top_kind(), KIND_LIBRARIES);
        assert!(list.key(ENTER, Key::Enter));
        assert_eq!(list.run.top_kind(), KIND_LIST);
        assert_eq!(list.kind(), Kind::All);
        list.same_kind_position(&format!("{kind:?} then in again"));
        assert_eq!(list.kind_byte(), 0);
        list.settle();
    }
}

/// The library with its kinds section's table row cut out, the count in the trailer one lower, and the
/// header's length and CRC fixed.
fn without_kinds(bytes: &[u8]) -> Vec<u8> {
    use mcpack::library::{ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION, crc32};
    let count = usize::from(bytes[bytes.len() - TRAILER_LEN]);
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    let last = first + (count - 1) * ENTRY_LEN;
    assert_eq!(
        bytes[last],
        mcpack::library::KINDS,
        "the kinds row is the last"
    );
    let mut out = bytes[..last].to_vec();
    out.extend([(count - 1) as u8, TRAILER_VERSION]);
    out.extend(TRAILER_MAGIC);
    let len = out.len() as u32;
    out[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = crc32(&out[8..]);
    out[4..8].copy_from_slice(&crc.to_le_bytes());
    assert!(
        mcpack::library::decode(&out)
            .unwrap()
            .sections
            .unwrap()
            .kinds
            .is_none()
    );
    out
}

#[test]
fn a_library_with_no_kinds_section_keeps_the_notice_on_f3_and_works_under_all() {
    let library = without_kinds(&pattern_library("TATATATATAT"));
    let end = table::header_of(&library).1 as usize;
    let built = staged_with(
        "pages-kinds-stripped",
        &library,
        &[mcui::Stage {
            name: "Kinds".into(),
            title: "Kinds".into(),
            end,
        }],
    );
    let mut list = Lister::open(built, None, 0);
    list.settle();
    assert_eq!(list.var("LV_HASK"), 0);
    for place in 0..3 {
        for _ in 0..place * 3 {
            list.key(RIGHT, Key::Right);
            list.settle();
        }
        let before = list.run.frames()[1].clone();
        list.tap(F3);
        let frame = list.run.next_blit();
        list.run.run(300_000);
        same_frame("F3", &frame, &model::unavailable("Kinds"));
        assert_eq!(list.run.depth(), 3, "the notice is on top");
        list.tap(CLEAR);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(
            list.run.frames()[1][..7],
            before[..7],
            "kind 0, cursor and top kept"
        );
        assert_eq!(list.run.frames()[1][6], 0);
        list.ui = list.replayed();
        list.settle();
        list.same_position("under All, after the notice");
    }
    let (len, stays) = (list.pos().len, 3);
    for _ in 0..len + stays {
        let drew = list.key(UP, Key::Up);
        if drew {
            list.settle();
        }
    }
}

#[test]
fn marks_that_changed_under_a_list_are_met_as_f3_would_meet_them_when_it_is_entered_again() {
    let built = crate::list_rig::proofs_three();
    let n = count(built);
    let before: Vec<Status> = (0..n)
        .map(|i| {
            if i % 3 == 0 {
                Status::Unchecked
            } else {
                Status::Checked
            }
        })
        .collect();
    let after: Vec<Status> = (0..n)
        .map(|i| {
            if i % 3 == 0 && i < 15 {
                Status::Checked
            } else {
                before[i]
            }
        })
        .collect();
    let (data_before, data_after) = (table_of(built, &before), table_of(built, &after));
    let mut list = Lister::open(built, Some((&data_before, &before)), 0);
    list.settle();
    list.go_to_kind(Kind::Unchecked);
    for _ in 0..5 {
        list.key(DOWN, Key::Down);
        list.settle();
    }
    let was = list.run.frames()[1].clone();
    let cursor = usize::from(u16::from_le_bytes([was[2], was[3]]));
    assert!(
        (6..15).contains(&cursor) && cursor % 3 == 0,
        "the cursor is on {cursor}"
    );
    list.run.m().delete_var("MCSTAT");
    table::install(list.run.m(), &data_after);
    list.tap(F4);
    list.run.next_blit();
    list.run.run(300_000);
    list.tap(CLEAR);
    list.run.next_blit();
    list.run.run(2_000);
    assert_eq!(list.var("LV_TABLE"), 1);
    let frame = list.run.frames()[1].clone();
    let end = built.stages[0].end;
    assert!(end > 17, "the stage is long enough: {end}");
    let values = table::values(built, &after);
    let items: Vec<usize> = (0..end).filter(|&i| values[i] != 1).collect();
    assert!(
        !items.contains(&cursor),
        "the cursor's assertion is not unchecked now"
    );
    let mut model = crate::shellrig::model::ui(built, &after);
    model.press(Key::Enter);
    for _ in 0..3 {
        model.press(Key::F3);
    }
    match model.state() {
        mcui::State::List { kind, len, .. } => {
            assert_eq!((kind, len), (Kind::Unchecked, items.len()))
        }
        other => panic!("{other:?}"),
    }
    let at = items
        .iter()
        .position(|&i| i >= cursor)
        .unwrap_or(items.len() - 1);
    let top = if at < 5 { 0 } else { at - 4 };
    let got = |at: usize| usize::from(u16::from_le_bytes([frame[at], frame[at + 1]]));
    assert_eq!((got(2), got(4)), (items[at], items[top]), "F3's rule");
    assert_ne!(got(2), cursor);
    assert_eq!(frame[6], 3);
}
