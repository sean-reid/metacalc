//! The list's enter failing under the jump: the sections screen is gone, the list's frame is as it was
//! and has not been landed, and the error notice is over it, as scr.z80's header and the shell say.

use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};

/// Every way list_enter answers a status other than 0, as list_fail.rs has them: the reader's assertion
/// count refusing inside libs_counts, the read of the library's CRC there, and the count list_enter asks
/// for itself.
const REFUSALS: [(&str, u8, u8); 3] = [
    ("far_lib_assertion_count", 1, 0x31),
    ("far_lib_read", 1, 0x32),
    ("far_lib_assertion_count", 2, 0x33),
];

#[test]
fn a_list_whose_enter_fails_under_the_jump_keeps_its_frame_under_the_error_notice() {
    for kind in [Kind::All, Kind::Axioms] {
        for (stub, call, status) in REFUSALS {
            let at = format!("{kind:?}, {stub} call {call}");
            let mut walker = Walker::on_list(rich(), None, 5);
            walker.list.go_to_kind(kind);
            walker.goto(4);
            walker.list.settle();
            let list = &mut walker.list;
            let log = list.log.len();
            list.open_sections();
            list.poke_below(&[0x85, 1, 2, 3, 4, 5, 6, 7, 8]);
            list.goto_title(30);
            let frames = list.run.frames();
            assert_eq!(frames.len(), 3);
            list.fail_at(0, stub, call, status);
            list.tap(ENTER);
            let frame = list.run.next_blit();
            list.run.run(300_000);
            same_frame(&at, &frame, &model::error(status));
            assert_eq!(list.fail_calls(0), u16::from(call), "{at}: the refusal ran");
            let now = list.run.frames();
            assert_eq!(list.run.depth(), 3, "{at}: depth");
            assert_eq!(now[0], frames[0], "{at}: Libraries");
            assert_eq!(
                now[1], frames[1],
                "{at}: the list's frame, find bytes included: land did not run"
            );
            assert_eq!(
                now[2][..3],
                [KIND_NOTICE, NOTICE_ERROR, status],
                "{at}: the error notice is on top, the sections frame is gone"
            );
            list.tap(CLEAR);
            let frame = list.run.next_blit();
            list.run.run(2_000);
            assert_eq!(list.run.depth(), 2, "{at}: the notice closes");
            assert_eq!(
                list.run.frames()[..2],
                frames[..2],
                "{at}: both frames kept"
            );
            let before_f5 = list.replayed_to(log);
            same_frame(&at, &frame, &crate::shellrig::model::frame(&before_f5));
            assert_eq!(list.run.top_kind(), KIND_LIST);
        }
    }
}

#[test]
fn a_refused_read_of_the_cursors_title_pushes_the_error_notice_and_clear_brings_the_screen_back() {
    for (stub, status) in [("far_lib_title", 0x41u8)] {
        let mut walker = Walker::on_list(rich(), None, 5);
        walker.goto(30);
        let list = &mut walker.list;
        list.open_sections();
        list.goto_title(20);
        let frames = list.run.frames();
        list.fail_at(0, stub, 1, status);
        list.tap(ENTER);
        let frame = list.run.next_blit();
        list.run.run(300_000);
        same_frame("the error notice", &frame, &model::error(status));
        assert_eq!(list.fail_calls(0), 1, "the refusal ran");
        let now = list.run.frames();
        assert_eq!(now.len(), 4, "the notice is over the sections screen");
        assert_eq!(now[..3], frames[..], "which is as it was");
        assert_eq!(now[3][..3], [KIND_NOTICE, NOTICE_ERROR, status]);
        list.tap(CLEAR);
        let frame = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.run.frames(), frames);
        same_frame("back on the screen", &frame, &list.shown());
    }
}

#[test]
fn a_title_the_reader_refuses_is_an_empty_row_and_the_rest_of_the_screen_is_the_models() {
    for refused in [1usize, 3, 4, 5, 6] {
        let mut walker = Walker::on_list(rich(), None, 5);
        let list = &mut walker.list;
        list.open_sections();
        list.fail_at(0, "far_lib_title", refused as u8, 0x42);
        list.tap(DOWN);
        list.ui.press(Key::Down);
        list.log.push(Key::Down);
        let frame = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.fail_calls(0), 6, "the draw read six titles");
        let mut want = list.shown();
        assert_ne!(
            refused, 2,
            "row 2 holds the cursor, which would be inverted"
        );
        let band = 96 * refused..96 * (refused + 1);
        assert!(
            want[band.clone()].iter().any(|&b| b != 0),
            "the row has text"
        );
        want[band].fill(0);
        same_frame(&format!("title {refused} refused"), &frame, &want);
    }
}
