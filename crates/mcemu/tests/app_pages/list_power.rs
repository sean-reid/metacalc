//! An auto power down on the list: the OS copies the screen's pixels over saveSScreen, which holds
//! the list's derived state, so the next key makes the shell enter the list again. The cursor and the
//! top are in the frame, the preview is pending again and the wait begins afresh.

use crate::app::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::Key;

/// Opens the fixture's list, moves it a page, optionally lets the preview settle, powers down, taps a
/// key the list ignores and returns the list stopped at the blit of the frame that follows.
fn powered_down(settled: bool) -> (Lister, [u8; 6]) {
    let mut list = Lister::open(with_library(), None, 0);
    list.count_fm_calls();
    list.settle();
    list.key(RIGHT, Key::Right);
    if settled {
        list.settle();
        assert_eq!(list.var("LV_PREVIEW"), ROW);
    } else {
        assert_eq!(list.seen.0, PENDING);
        list.run.run(100_000);
        assert_eq!(list.var("LV_PREVIEW"), PENDING, "the wait is not over");
    }
    let before = list.frame_bytes();
    list.run.shell.power_down();
    let (blits, fits) = (list.run.blits(), list.fm_calls()[2]);
    list.tap(LETTER);
    let frame = list.run.next_blit();
    list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
    list.run.run(2_000);
    assert_eq!(list.run.blits(), blits + 1, "one frame for the refresh");
    assert_eq!(list.fm_calls()[2], fits, "no fit yet");
    let pending = list.replayed();
    same_frame("after the power down", &frame, &model::frame(&pending));
    list.ui = pending;
    (list, before)
}

#[test]
fn a_power_down_with_a_settled_preview_brings_back_the_list_with_a_blank_preview() {
    let (mut list, before) = powered_down(true);
    assert_eq!(list.seen, (PENDING, 11), "pending, the wait begins again");
    assert_eq!(
        list.frame_bytes(),
        before,
        "the cursor and the top are in the frame"
    );
    assert_eq!(list.var("LV_TABLE"), 0);
    let fits = list.fm_calls()[2];
    assert!(list.settle(), "the preview is laid out again");
    assert_eq!(list.fm_calls()[2], fits + 1, "by one fit");
}

#[test]
fn a_power_down_with_a_pending_preview_keeps_it_pending_and_it_settles_after_a_full_wait() {
    let (mut list, before) = powered_down(false);
    assert_eq!(
        list.seen,
        (PENDING, 11),
        "the wait begins again, not where it was"
    );
    assert_eq!(list.frame_bytes(), before);
    let waited = list.wait_for_the_fit();
    assert_eq!(waited.interrupts, crate::list_wait::WAIT_TICKS);
    assert!(list.settle());
}
