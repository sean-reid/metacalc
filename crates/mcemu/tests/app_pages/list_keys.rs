//! The keys of the list that are not moves: the five that open the notice, F5 where the stage has
//! sections, the keys it ignores, CLEAR, and the list that comes back when the notice pops.

use crate::app::*;
use crate::builds::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Key;
use std::sync::OnceLock;

/// The fixture as three stages with a section title before ax-1 and one before the longest
/// statement: the first stage ends before the first title, the second after it and the third holds
/// both.
pub fn titled() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let rule = "#".repeat(30);
        let heading = |name: &str| format!("$(\n{rule}\n  {name}\n{rule}\n$)\n");
        let mut src = crate::fixture::source();
        for (at, name) in [("longest $a", "Long"), ("ax-1 $a", "Axioms")] {
            let i = src.find(at).expect("the statement is in the fixture");
            src.insert_str(i, &heading(name));
        }
        let library = crate::support::pack_all(&crate::support::parse(&src));
        let count = crate::table::header_of(&library).1 as usize;
        let ends: Vec<usize> = (1..=3).map(|n| n * count / 3).collect();
        staged("pages-titled", &library, &ends)
    })
}

/// Whether the library has a section title that starts before the stage's end, which is what the
/// model's `stage_titles` asks and when F5 does something.
fn has_sections(built: &Built, stage: usize) -> bool {
    let library = crate::support::decoded(built.library.as_ref().unwrap());
    let titles = library.sections.and_then(|s| s.titles).unwrap_or_default();
    titles
        .first()
        .is_some_and(|t| t.assertion < built.stages[stage].end)
}

/// The list of `stage` with its cursor taken a page and a row down, settled.
fn moved_list(built: &'static Built, stage: usize) -> Lister {
    let mut list = Lister::open(built, None, stage);
    list.settle();
    list.key(RIGHT, Key::Right);
    list.settle();
    list.key(DOWN, Key::Down);
    list.settle();
    list
}

#[test]
fn the_sections_key_is_live_exactly_where_the_stage_has_section_titles() {
    let built = titled();
    let want = [false, true, true];
    for (stage, want) in want.into_iter().enumerate() {
        assert_eq!(has_sections(built, stage), want, "the model, stage {stage}");
        let mut list = Lister::open(built, None, stage);
        list.settle();
        assert_eq!(
            list.var("LV_SECT"),
            u8::from(want),
            "the app, stage {stage}"
        );
    }
}

#[test]
fn the_function_keys_open_the_stages_notice_and_f5_only_where_there_are_sections() {
    for built in [three_stages(), titled()] {
        for stage in 0..3 {
            for scan in [F2, F4, F5] {
                let mut list = moved_list(built, stage);
                let at = format!("stage {stage}, key {scan:02X}");
                let before = list.frame_bytes();
                let blits = list.run.blits();
                list.tap(scan);
                let opens = scan != F5 || has_sections(built, stage);
                if opens {
                    let frame = list.run.next_blit();
                    list.run.run(2_000);
                    crate::ui_support::same_frame(
                        &at,
                        &frame,
                        &model::unavailable(built.stage_name(stage)),
                    );
                    assert_eq!(list.run.depth(), 3, "{at}");
                    let frames = list.run.frames();
                    assert_eq!(
                        frames[2][..3],
                        [KIND_NOTICE, NOTICE_LIST, stage as u8],
                        "{at}: the notice's frame"
                    );
                    assert_eq!(frames[1][..6], before, "{at}: the list is as it was");
                    assert_eq!(list.run.blits(), blits + 1, "{at}");
                } else {
                    list.run.run(300_000);
                    assert_eq!(list.run.depth(), 2, "{at}: F5 with no sections");
                    assert_eq!(list.run.blits(), blits, "{at}: draws nothing");
                    assert_eq!(list.frame_bytes(), before, "{at}");
                }
            }
        }
    }
}

#[test]
fn the_list_comes_back_from_its_notice_with_cursor_and_top_where_they_were_and_a_blank_preview() {
    let built = titled();
    for pop in [CLEAR, ENTER, F1] {
        for stage in 1..3 {
            let mut list = moved_list(built, stage);
            let at = format!("stage {stage}, pop with {pop:02X}");
            let before = list.frame_bytes();
            assert_ne!(
                (before[2], before[4]),
                (0, 0),
                "{at}: the cursor and the top have moved"
            );
            list.tap(F4);
            list.run.next_blit();
            list.run.run(300_000);
            assert_eq!(list.run.top_kind(), KIND_NOTICE, "{at}");
            list.tap(pop);
            let frame = list.run.next_blit();
            list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
            assert_eq!(list.seen, (PENDING, 11), "{at}: blank, the wait begins");
            list.run.run(2_000);
            assert_eq!((list.run.depth(), list.run.top_kind()), (2, KIND_LIST));
            assert_eq!(list.frame_bytes(), before, "{at}: cursor and top");
            let pending = list.replayed();
            crate::ui_support::same_frame(&at, &frame, &model::frame(&pending));
            list.ui = pending;
            assert!(list.settle(), "{at}: the preview is laid out again");
        }
    }
}

#[test]
fn keys_that_do_not_pop_leave_the_notice_and_the_list_under_it_alone() {
    let built = titled();
    for scan in [UP, DOWN, LEFT, RIGHT, F2, F3, F4, F5, LETTER] {
        let mut list = moved_list(built, 1);
        let before = list.run.frames();
        list.tap(F2);
        list.run.next_blit();
        list.run.run(300_000);
        let (blits, notice) = (list.run.blits(), list.run.frames());
        list.tap(scan);
        list.run.run(300_000);
        assert_eq!(list.run.depth(), 3, "{scan:02X}: the notice stays");
        assert_eq!(list.run.blits(), blits, "{scan:02X}: no frame");
        assert_eq!(list.run.frames(), notice, "{scan:02X}");
        assert_eq!(notice[..2], before[..2], "{scan:02X}: the list is under it");
    }
}

#[test]
fn clear_goes_back_to_libraries_with_the_stages_row_where_it_was() {
    let built = six_stages();
    for stage in 0..6 {
        let mut list = Lister::open(built, None, stage);
        list.settle();
        list.key(RIGHT, Key::Right);
        list.settle();
        assert!(list.key(CLEAR, Key::Clear), "stage {stage}: a frame");
        assert_eq!((list.run.depth(), list.run.top_kind()), (1, KIND_LIBRARIES));
        let frames = list.run.frames();
        let mut ui = crate::shellrig::model::ui(built, &[]);
        for _ in 0..stage {
            ui.press(Key::Down);
        }
        assert_eq!(
            frames[0][1],
            stage as u8,
            "the cursor is on the stage's row: {:?}",
            ui.state()
        );
    }
}

#[test]
fn a_modifier_and_the_key_after_it_do_nothing_on_the_list() {
    for built in [with_library(), titled()] {
        let mut list = moved_list(built, 0);
        let (frame, state) = (list.frame_bytes(), list.var("LV_PREVIEW"));
        let blits = list.run.blits();
        for scans in [&[SECOND, ONE][..]] {
            let before = list.run.scr_keys();
            if let [modifier, last] = scans {
                for (n, &scan) in [*modifier, *last].iter().enumerate() {
                    list.run.m().press_key(scan);
                    list.run.run(120_000);
                    list.run.m().release_key(scan);
                    list.run.run(480_000);
                    assert_eq!(
                        list.run.scr_keys(),
                        before + n as u16,
                        "{scan:02X}: 2nd is no key and the key after it reaches scr_key as none"
                    );
                }
            } else {
                list.tap(scans[0]);
            }
            list.run.run(600_000);
            assert_eq!(list.run.depth(), 2, "{scans:02X?}");
            assert_eq!(list.frame_bytes(), frame, "{scans:02X?}");
            assert_eq!(list.var("LV_PREVIEW"), state, "{scans:02X?}: still settled");
            assert_eq!(list.run.blits(), blits, "{scans:02X?}: no frame");
        }
    }
}
