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

/// The marks of a library of `n` axioms with the assertions in `checked` checked, the rest not.
fn marks(n: usize, checked: &[usize]) -> Vec<Status> {
    (0..n)
        .map(|i| {
            if checked.contains(&i) {
                Status::Checked
            } else {
                Status::Unchecked
            }
        })
        .collect()
}

/// One way the marks change under an open Unchecked list. The app is brought to a place under the
/// old marks, where every assertion is unchecked, by `keys`; the table is swapped for the new marks, a
/// notice is pushed and popped, and the list is entered again. The model is built with the new marks
/// and driven to the same place: through F3 to Axioms, `down` rows to the cursor's assertion, and F3
/// into Unchecked, which is the model's own landing; for a list whose cursor and top stay shown it is
/// driven by `keys` under Unchecked, where nothing lands.
fn marks_change(name: &str, keys: &[(u8, Key)], checked: &[usize], lands: bool) {
    let built = pattern_built("pages-kinds-marks", &"A".repeat(30));
    let n = count(built);
    let old = marks(n, &[]);
    let new = marks(n, checked);
    let (data_old, data_new) = (table_of(built, &old), table_of(built, &new));
    let mut list = Lister::open(built, Some((&data_old, &old)), 0);
    list.settle();
    list.go_to_kind(Kind::Unchecked);
    for &(scan, key) in keys {
        list.key(scan, key);
        list.settle();
    }
    let was = list.run.frames()[1].clone();
    let (cursor, top) = (list.pos().cursor, list.pos().top);
    let shown = |i: usize| !checked.contains(&i);
    assert_eq!(
        (shown(cursor), shown(top)),
        match name {
            "cursor lost" => (false, true),
            "top lost" => (true, false),
            "both lost" => (false, false),
            _ => (true, true),
        },
        "{name}: the precondition"
    );
    if !lands {
        assert_ne!(
            top,
            cursor.saturating_sub(4),
            "{name}: the top is not the landing's"
        );
    }
    list.run.m().delete_var("MCSTAT");
    table::install(list.run.m(), &data_new);
    list.tap(F4);
    list.run.next_blit();
    list.run.run(300_000);
    list.tap(CLEAR);
    let frame = list.run.next_blit();
    list.run.run(2_000);
    assert_eq!(list.var("LV_TABLE"), 1);
    let mut model = crate::shellrig::model::ui(built, &new);
    model.press(Key::Enter);
    if lands {
        model.press(Key::F3);
        model.press(Key::F3);
        for _ in 0..cursor {
            model.press(Key::Down);
        }
        model.press(Key::F3);
    } else {
        for _ in 0..3 {
            model.press(Key::F3);
        }
        for &(_, key) in keys {
            model.press(key);
        }
    }
    crate::ui_support::same_frame(name, &frame, &crate::shellrig::model::frame(&model));
    list.ui = model;
    list.status = new;
    list.same_kind_position(name);
    let now = list.run.frames()[1].clone();
    assert_eq!(now[6], 3);
    assert_eq!(now != was, lands, "{name}: the frame {was:?} then {now:?}");
    assert!(list.settle(), "{name}");
}

const TWELVE: [(u8, Key); 12] = [(DOWN, Key::Down); 12];

#[test]
fn marks_that_changed_under_a_list_are_met_as_f3_would_meet_them_when_it_is_entered_again() {
    marks_change("cursor lost", &TWELVE, &[12], true);
    marks_change("top lost", &TWELVE, &[8], true);
    marks_change("both lost", &TWELVE, &[8, 12], true);
    marks_change(
        "both kept",
        &[(RIGHT, Key::Right), (RIGHT, Key::Right), (UP, Key::Up)],
        &[20, 25],
        false,
    );
}
