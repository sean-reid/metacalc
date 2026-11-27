//! The characters the find takes and the ones it refuses, a library with no label order section, and a
//! table lost or a reader call refused under a find.

use crate::built::staged_with;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};

#[test]
fn every_key_a_calculator_can_type_adds_its_character_as_the_model_does() {
    let built = proofs_three();
    let chars: Vec<char> = ('a'..='z').chain('0'..='9').chain(['-', '.']).collect();
    let (mut hits, mut misses) = (0, 0);
    let mut list = Lister::open(built, None, 1);
    list.settle();
    for &c in &chars {
        list.f2();
        list.press_char(c);
        list.same_find(&format!("{c:?}"));
        assert_eq!(list.find().raw[1], c as u8, "{c:?}: the character is in byte 8");
        if list.find().missed {
            misses += 1;
        } else {
            hits += 1;
        }
        list.skip_the_wait();
        list.settle();
    }
    println!("typed {} characters: {hits} hits, {misses} misses", chars.len());
    assert_eq!(hits + misses, 38);
    assert!(hits >= 5 && misses >= 20, "{hits} hits, {misses} misses");
}

#[test]
fn every_label_of_the_proofs_library_is_typed_with_digits_dots_and_hyphens_as_the_model_takes_it() {
    let built = proofs_three();
    let labels = labels_of(built);
    for stage in 0..3 {
        let mut list = Lister::open(built, None, stage);
        list.settle();
        for label in &labels {
            list.f2();
            let typed: String = label.chars().take(8).collect();
            list.type_text(&typed);
            assert_eq!(list.find().chars, typed);
        }
    }
    assert!(labels.iter().any(|l| l.contains('-')));
    assert!(labels.iter().any(|l| l.contains('.')));
    assert!(labels.iter().any(|l| l.chars().any(|c| c.is_ascii_digit())));
}

/// A list on stage 1 with the prefix "p" typed, under `kind`.
fn typed_p(kind: Kind, table: bool) -> Lister {
    let marks = every_mark(proofs_three());
    let mut list = list_under(proofs_three(), 1, kind, table.then_some(&marks));
    list.type_settled("p");
    list
}

/// The writes of `call` in bank B and C, outside the driver's own RAM.
fn writes_of(list: &mut Lister, call: impl FnOnce(&mut Lister)) -> Vec<(u16, u8)> {
    list.run.shell.app.m.watch_writes(0x8000, 0x8000);
    call(list);
    let writes = list.run.shell.app.m.take_writes();
    list.run.shell.app.m.unwatch_writes();
    let own = list.run.shell.app.scratch.clone();
    writes
        .iter()
        .map(|w| (w.addr, w.value))
        .filter(|(a, _)| !own.iter().any(|&(s, e)| (s..e).contains(a)))
        .collect()
}

#[test]
fn a_character_the_model_refuses_stays_and_writes_nothing() {
    let refused = [0x20, b'/', b':', b'@', b'[', b'`', b'{', 0x80, 0xFF, 0x00, b'!', b'~'];
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
    ] {
        for b in refused {
            let mut list = typed_p(kind, table);
            let before = list.frame_now();
            assert_eq!(before[7], 1, "{kind:?}: one character typed");
            let mut done = None;
            let writes = writes_of(&mut list, |l| done = Some(l.call_key(12, b)));
            assert_eq!(done.unwrap().status(), 0, "{kind:?} {b:02X}: SCR_STAY");
            assert!(writes.is_empty(), "{kind:?} {b:02X}: wrote {writes:02X?}");
            assert_eq!(list.frame_now(), before, "{kind:?} {b:02X}");
        }
    }
}

#[test]
fn a_ninth_character_called_in_stays_and_writes_nothing() {
    let built = proofs_three();
    let mut list = Lister::open(built, None, 1);
    list.settle();
    list.type_text("pm2.61d1");
    assert_eq!(list.find().len, 8);
    let before = list.frame_now();
    for b in [b'x', b'1', b'-', b'P', b'_'] {
        let mut done = None;
        let writes = writes_of(&mut list, |l| done = Some(l.call_key(12, b)));
        assert_eq!(done.unwrap().status(), 0, "{b:02X}");
        assert!(writes.is_empty(), "{b:02X}: wrote {writes:02X?}");
        assert_eq!(list.frame_now(), before);
    }
}

#[test]
fn a_capital_and_its_lowercase_give_the_same_frame_and_the_underscore_is_taken() {
    for (kind, table) in [(Kind::All, false), (Kind::Unchecked, true)] {
        let mut frames = Vec::new();
        for b in [b'S', b's'] {
            let marks = every_mark(proofs_three());
            let mut list = list_under(proofs_three(), 1, kind, table.then_some(&marks));
            let done = list.call_key(12, b);
            assert_eq!(done.status(), 1, "{kind:?} {b:02X}: SCR_REDRAW");
            frames.push(list.frame_now());
        }
        assert_eq!(frames[0], frames[1], "{kind:?}");
        assert_eq!(frames[0][7] & 0x7F, 1);
        assert_eq!(frames[0][8], b's');
    }
    let mut list = Lister::open(proofs_three(), None, 1);
    list.settle();
    let done = list.call_key(12, b'_');
    assert_eq!(done.status(), 1);
    let frame = list.frame_now();
    assert_eq!((frame[7] & 0x0F, frame[8]), (1, b'_'));
}

/// The library with its label order section's row cut out, the count in the trailer one lower, and the
/// header's length and CRC fixed.
fn without_order(bytes: &[u8]) -> Vec<u8> {
    use mcpack::library::{ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION, crc32};
    let count = usize::from(bytes[bytes.len() - TRAILER_LEN]);
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    let row = (0..count)
        .find(|&i| bytes[first + i * ENTRY_LEN] == mcpack::library::LABEL_ORDER)
        .expect("a label order row");
    let at = first + row * ENTRY_LEN;
    let mut out = bytes[..at].to_vec();
    out.extend(&bytes[at + ENTRY_LEN..bytes.len() - TRAILER_LEN]);
    out.extend([(count - 1) as u8, TRAILER_VERSION]);
    out.extend(TRAILER_MAGIC);
    let len = out.len() as u32;
    out[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = crc32(&out[8..]);
    out[4..8].copy_from_slice(&crc.to_le_bytes());
    let sections = mcpack::library::decode(&out).unwrap().sections.unwrap();
    assert!(sections.label_order.is_none() && sections.kinds.is_some());
    out
}

#[test]
fn a_library_with_no_label_order_section_keeps_the_notice_on_f2_and_ignores_letters() {
    let library = without_order(&pattern_library("TATATATATAT"));
    let end = crate::table::header_of(&library).1 as usize;
    let built = staged_with(
        "pages-find-stripped",
        &library,
        &[mcui::Stage {
            name: "Kinds".into(),
            title: "Kinds".into(),
            end,
        }],
    );
    let mut list = Lister::open(built, None, 0);
    list.settle();
    assert_eq!(list.var("LV_HASO"), 0);
    assert_eq!(list.var("LV_HASK"), 1);
    for place in 0..3 {
        for _ in 0..place * 3 {
            list.key(RIGHT, Key::Right);
            list.settle();
        }
        let before = list.run.frames()[1].clone();
        let writes = writes_of(&mut list, |l| {
            for b in [b'a', b'T', b'-', b'1'] {
                assert_eq!(l.call_key(12, b).status(), 0);
            }
        });
        assert!(writes.is_empty(), "letters write nothing: {writes:02X?}");
        assert_eq!(list.run.peek(list.frame_addr(), 16), before);
        list.tap(F2);
        let frame = list.run.next_blit();
        list.run.run(300_000);
        same_frame("F2", &frame, &model::unavailable("Kinds"));
        assert_eq!(list.run.depth(), 3, "the notice is on top");
        list.tap(CLEAR);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(list.run.frames()[1][7..], before[7..], "byte 7 stays 0");
        assert_eq!(list.run.frames()[1][7], 0);
        list.ui = list.replayed();
        list.settle();
    }
    for _ in 0..3 {
        list.key(F3, Key::F3);
        list.same_kind_position("F3 with no label order");
        list.settle();
    }
    list.walk_kind("a stripped library");
}
