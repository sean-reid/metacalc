//! The list's find on the stub OS against the host model: every frame a letter or an ending key
//! draws is the model's, the frame's bytes 7 to 15 say what the model says, and the cursor and the
//! first row land where the model's do, under every kind with a table and without.

use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind};

/// The table of the proofs library with every mark, and the marks.
fn marks() -> (Vec<u8>, Vec<mcui::Status>) {
    every_mark(proofs_three())
}

/// Under `kind` and, for Unchecked, with the table or without.
fn cases() -> Vec<(Kind, bool)> {
    vec![
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
    ]
}

fn open_under(stage: usize, kind: Kind, table: bool) -> Lister {
    let marks = marks();
    list_under(proofs_three(), stage, kind, table.then_some(&marks))
}

/// The index in `items` of the first label from `from` on that starts with `prefix`.
fn first_hit(labels: &[String], items: &[usize], prefix: &str, from: usize) -> Option<usize> {
    items[from.min(items.len())..]
        .iter()
        .position(|&i| labels[i].starts_with(prefix))
        .map(|at| at + from)
}

#[test]
fn a_letter_two_and_a_run_to_the_cap_are_the_models_frames_and_a_ninth_is_ignored() {
    let built = proofs_three();
    let labels = labels_of(built);
    let long = labels
        .iter()
        .position(|l| l.len() >= 8)
        .expect("a label of eight characters");
    let stage = Lister::stage_of(built, long);
    for (kind, table) in cases() {
        let mut list = open_under(stage, kind, table);
        let label = &labels[long];
        for (n, c) in label.chars().take(8).enumerate() {
            list.press_char(c);
            list.same_find(&format!("{kind:?}, {} typed", n + 1));
        }
        let before = list.run.frames().last().cloned().unwrap();
        let (preview, blits) = (list.var("LV_PREVIEW"), list.run.blits());
        for c in ['x', 'a', '-', '9'] {
            assert!(!list.press_char(c), "{kind:?}: a ninth {c} draws nothing");
            assert_eq!(list.run.frames().last().cloned().unwrap(), before);
            assert_eq!(list.var("LV_PREVIEW"), preview);
            assert_eq!(list.run.blits(), blits);
            list.same_find(&format!("{kind:?}, ninth {c}"));
        }
        assert_eq!(list.find().len, 8);
    }
}

#[test]
fn a_hit_on_the_cursors_own_row_keeps_a_settled_preview() {
    let built = proofs_three();
    let labels = labels_of(built);
    let mut kept = 0;
    for stage in 0..3 {
        let mut list = open_under(stage, Kind::All, false);
        let items = list.items();
        for (at, &i) in items.iter().enumerate() {
            let label = &labels[i];
            for k in 1..label.len().min(8) {
                let (p, q) = (&label[..k], &label[..k + 1]);
                if first_hit(&labels, &items, p, 0) != Some(at)
                    || first_hit(&labels, &items, q, at) != Some(at)
                {
                    continue;
                }
                list.f2();
                list.type_settled(p);
                assert_eq!(list.pos().cursor, at);
                assert_eq!(list.var("LV_PREVIEW"), ROW, "{label}: settled before");
                list.press_char(label.as_bytes()[k] as char);
                assert_ne!(list.seen.0, PENDING, "{label}: {q} kept the preview");
                assert!(!list.pos().pending);
                list.same_find(&format!("{label}: {q}"));
                kept += 1;
            }
        }
    }
    assert!(kept >= 20, "{kept} cases of a hit on the cursor's row");
    println!("a hit on the cursor's own row: {kept} cases");
}

/// Every prefix of one to three characters of the library's labels typed from every place the
/// previous find left the cursor, under each kind: the places the hits land are counted by where
/// they are against the view.
#[test]
fn hits_below_above_and_among_the_last_four_land_as_the_model_lands_them() {
    let built = proofs_three();
    let labels = labels_of(built);
    let mut prefixes: Vec<String> = labels
        .iter()
        .flat_map(|l| (1..=l.len().min(3)).map(move |k| l[..k].to_string()))
        .collect();
    prefixes.sort();
    prefixes.dedup();
    for (kind, table) in cases() {
        let mut list = open_under(1, kind, table);
        let items = list.items();
        let (mut below, mut above, mut last_four, mut own, mut misses) = (0, 0, 0, 0, 0);
        for p in &prefixes {
            list.f2();
            let (old_cursor, old_top) = (list.pos().cursor, list.pos().top);
            list.type_text(p);
            let pos = list.pos();
            if list.find().missed {
                misses += 1;
                assert_eq!((pos.cursor, pos.top), (old_cursor, old_top), "{p}: a miss");
                continue;
            }
            assert!(first_hit(&labels, &items, p, 0).is_some());
            below += usize::from(pos.cursor >= old_top + 5);
            above += usize::from(pos.cursor < old_top);
            own += usize::from(pos.cursor == old_cursor);
            if pos.len >= 5 && pos.cursor + 4 >= pos.len {
                last_four += 1;
                assert_eq!(pos.top, pos.len - 5, "{kind:?} {p}: the first row walks back");
            }
        }
        println!(
            "{kind:?} table {table}: below {below}, above {above}, last four {last_four}, own row {own}, misses {misses}"
        );
        assert!(below >= 1 && above >= 1 && own >= 1 && misses >= 1, "{kind:?}");
        if kind != Kind::Axioms {
            assert!(last_four >= 1, "{kind:?}: {last_four}");
        }
    }
}

#[test]
fn a_miss_then_letters_then_f2_and_a_new_prefix_are_the_models() {
    for (kind, table) in cases() {
        let mut list = open_under(0, kind, table);
        list.type_text("a");
        let at = list.frame_bytes();
        list.type_text("qzq");
        assert!(list.find().missed, "{kind:?}");
        assert_eq!(list.frame_bytes(), at, "a miss keeps the cursor and the top");
        list.type_text("-1");
        assert!(list.find().missed);
        assert!(list.f2());
        list.same_find("F2 after a miss");
        assert_eq!(list.find().raw[0], 0, "byte 7 is clear");
        list.type_settled("s");
        assert!(!list.find().missed || kind != Kind::All);
        assert!(!list.f2() || list.find().len == 0);
    }
}

#[test]
fn f2_with_no_prefix_changes_nothing_and_with_one_redraws() {
    for (kind, table) in cases() {
        let mut list = open_under(1, kind, table);
        list.settle();
        let (frame, preview, blits) = (
            list.run.frames().last().cloned().unwrap(),
            list.var("LV_PREVIEW"),
            list.run.blits(),
        );
        assert!(!list.f2(), "{kind:?}: no frame");
        assert_eq!(list.run.blits(), blits);
        assert_eq!(list.run.frames().last().cloned().unwrap(), frame);
        assert_eq!(list.var("LV_PREVIEW"), preview);
        list.type_settled("p");
        assert_ne!(list.find().len, 0);
        let preview = list.var("LV_PREVIEW");
        assert!(list.f2(), "{kind:?}: a frame");
        list.same_find("F2 with a prefix");
        assert_eq!(list.var("LV_PREVIEW"), preview, "the preview is kept");
        assert_ne!(list.var("LV_PREVIEW"), PENDING);
    }
}

#[test]
fn an_arrow_ends_the_find_and_at_an_end_it_still_redraws() {
    for (kind, table) in cases() {
        let built = proofs_three();
        let labels = labels_of(built);
        for arrow in [(UP, Key::Up), (DOWN, Key::Down), (LEFT, Key::Left), (RIGHT, Key::Right)] {
            let mut list = open_under(1, kind, table);
            let items = list.items();
            assert!(items.len() >= 6, "{kind:?}: {} shown", items.len());
            let up = matches!(arrow.1, Key::Up | Key::Left);
            let target = if up { items[0] } else { items[items.len() - 1] };
            let label = &labels[target];
            let k = (1..=label.len().min(8))
                .find(|&k| first_hit(&labels, &items, &label[..k], 0) == Some(if up { 0 } else { items.len() - 1 }))
                .expect("a prefix the end's label owns");
            list.type_settled(&label[..k]);
            assert_eq!(list.pos().cursor, if up { 0 } else { items.len() - 1 });
            let preview = list.var("LV_PREVIEW");
            assert!(list.press(arrow.0, arrow.1, &format!("{kind:?} {arrow:?} at an end")));
            assert_eq!(list.find().raw[0], 0);
            assert_eq!(list.var("LV_PREVIEW"), preview, "at an end the preview is kept");
            let mut list = open_under(1, kind, table);
            list.type_settled(&labels[items[2]][..1]);
            assert!(list.press(arrow.0, arrow.1, &format!("{kind:?} {arrow:?} inside")));
            assert_eq!(list.find().len, 0);
        }
    }
}

#[test]
fn clear_ends_a_find_first_and_pops_without_one() {
    for (kind, table) in cases() {
        let mut list = open_under(1, kind, table);
        list.type_text("p");
        list.type_text("zz");
        assert!(list.find().missed);
        assert!(list.press(CLEAR, Key::Clear, "CLEAR after a miss"));
        assert_eq!(list.find().raw[0], 0);
        assert_eq!(list.run.depth(), 2, "still on the list");
        list.type_text("a");
        assert!(list.press(CLEAR, Key::Clear, "CLEAR after a hit"));
        assert_eq!(list.run.depth(), 2);
        assert!(list.key(CLEAR, Key::Clear));
        assert_eq!(list.run.depth(), 1, "{kind:?}: the second CLEAR leaves");
        assert!(list.key(ENTER, Key::Enter));
        assert_eq!(list.find().len, 0, "back in with no prefix");
        assert_eq!(list.find().raw, [0u8; 9], "a push starts with bytes 7 to 15 zero");
    }
}

#[test]
fn f3_with_a_prefix_lands_from_the_first_shown_assertion_of_the_new_kind() {
    let built = proofs_three();
    let labels = labels_of(built);
    for (kind, table) in cases() {
        for prefix in ["p", "a", "m", "pm2.", "ax-", "zz", "syl"] {
            let mut list = open_under(1, kind, table);
            list.type_text(prefix);
            for step in 0..5 {
                let at = format!("{kind:?} {prefix:?} F3 {step}");
                list.key(F3, Key::F3);
                list.same_find(&at);
                let items = list.items();
                let want = first_hit(&labels, &items, prefix, 0);
                assert_eq!(list.find().missed, want.is_none(), "{at}");
                if let Some(w) = want {
                    assert_eq!(list.pos().cursor, w, "{at}");
                }
                list.skip_the_wait();
                list.settle();
            }
        }
    }
}

#[test]
fn enter_and_f1_open_the_found_theorem_and_the_prefix_is_in_the_frame_on_the_way_back() {
    let built = proofs_three();
    for scan in [ENTER, F1] {
        for missed in [false, true] {
            let mut list = open_under(1, Kind::All, false);
            list.type_settled("pm");
            if missed {
                list.type_text("q");
            }
            let before = list.find();
            assert_eq!(before.missed, missed);
            list.tap(scan);
            list.run.next_blit();
            list.run.run(300_000);
            assert_eq!(list.run.depth(), 3, "{scan:02X}: the theorem is open");
            assert_eq!(list.find_under(), before, "the list's frame keeps its find");
            list.tap(CLEAR);
            let frame = list.run.next_blit();
            list.run.run(2_000);
            assert_eq!(list.find(), before, "bytes 7 to 15 kept");
            let pending = list.replayed();
            crate::ui_support::same_frame(
                &format!("{scan:02X}, missed {missed}: back from the theorem"),
                &frame,
                &model::frame(&pending),
            );
            list.ui = pending;
            let _ = built;
            list.same_find("back from the theorem");
            list.settle();
        }
    }
}

#[test]
fn the_notice_over_a_list_with_a_prefix_returns_with_the_prefix_and_the_title_drawn_again() {
    for missed in [false, true] {
        let mut list = open_under(1, Kind::All, false);
        list.type_settled("pm");
        if missed {
            list.type_text("q");
        }
        let before = list.find();
        list.tap(F4);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(list.run.top_kind(), KIND_NOTICE);
        assert_eq!(list.find_under(), before);
        list.tap(CLEAR);
        let frame = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.find(), before, "bytes 7 to 15 kept through the enter");
        let pending = list.replayed();
        crate::ui_support::same_frame("back from the notice", &frame, &model::frame(&pending));
        list.ui = pending;
        list.same_find("back from the notice");
    }
}

#[test]
fn an_auto_power_down_with_a_prefix_keeps_it_through_the_refresh() {
    for missed in [false, true] {
        let mut list = open_under(1, Kind::All, false);
        list.type_settled("pm");
        if missed {
            list.type_text("q");
        }
        let before = list.find();
        list.run.shell.power_down();
        list.tap(PLUS);
        let frame = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.find(), before);
        let pending = list.replayed();
        crate::ui_support::same_frame("after the power down", &frame, &model::frame(&pending));
        list.ui = pending;
        list.same_find("after the power down");
        assert!(list.settle() || list.pos().len == 0);
    }
}
