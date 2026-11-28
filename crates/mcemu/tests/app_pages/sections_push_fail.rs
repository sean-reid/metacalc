//! What the user sees when the reader refuses a call the sections screen makes on its way in: the title
//! lookup of the push (the screen opens on its first title), and the assertion count in the screen's own
//! enter, on the push, after an auto power down and when the error notice over it closes. Only a build
//! whose one stage is the whole library asks the reader for the count in libs_list, so those tests use one.

use crate::app::Built;
use crate::built::around;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use crate::ui_support::same_frame;
use mcemu::Stop;
use mcui::{Key, Kind};

use std::sync::OnceLock;

const TITLE_FOR: &str = "far_lib_title_for";
const COUNT: &str = "far_lib_assertion_count";

/// The library `rich` holds, built with no stage list, so that its one stage is the whole library and
/// libs_list asks the reader for the count.
fn one_stage() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        around(
            "sections-one-stage",
            &sectioned(&rich_pattern(), &rich_titles()),
        )
    })
}

/// The list of `built`'s last stage under `kind` with its cursor on an assertion whose section is not the
/// first, settled.
fn in_a_later_section(built: &'static Built, stage: usize, kind: Kind) -> Walker {
    let mut walker = Walker::on_list(built, None, stage);
    walker.list.go_to_kind(kind);
    for _ in 0..2 {
        assert!(walker.list.key(RIGHT, Key::Right), "a page down");
        walker.list.settle();
    }
    walker
}

#[test]
fn a_refused_title_lookup_opens_the_screen_on_title_0_with_the_list_as_it_was() {
    for kind in [Kind::All, Kind::Axioms] {
        let mut walker = in_a_later_section(rich(), 5, kind);
        let list = &mut walker.list;
        let at = format!("{kind:?}");
        let section = rich_titles()
            .iter()
            .rposition(|t| t.assertion <= list.pos().cursor)
            .unwrap();
        assert!(section > 0, "{at}: the cursor is past the first title");
        let before = list.run.frames();
        let log = list.log.len();
        list.fail_at(0, TITLE_FOR, 1, 0x51);
        list.tap(F5);
        let drawn = list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(
            list.fail_calls(0),
            2,
            "{at}: the push's lookup was refused and the enter's was not"
        );
        let frames = list.run.frames();
        assert_eq!(
            frames.len(),
            3,
            "{at}: the screen is on the stack, no notice"
        );
        assert_eq!(
            frames[..2],
            before[..],
            "{at}: Libraries and the list are as they were"
        );
        let mut screen = [0u8; 16];
        screen[0] = KIND_SECTIONS;
        screen[1] = 5;
        assert_eq!(
            frames[2], screen,
            "{at}: scr_new holds title 0 and first title 0"
        );
        let mut first = Walker::on_list(rich(), None, 5);
        let want = first.list.open_sections();
        assert_eq!(
            (want.cursor, want.top),
            (0, 0),
            "{at}: the model's first title"
        );
        list.same_sections(&at, &want);
        same_frame(&at, &drawn, &first.list.shown());
        list.tap(CLEAR);
        let back = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.run.frames(), before, "{at}: CLEAR gives the list back");
        same_frame(
            &at,
            &back,
            &crate::shellrig::model::frame(&list.replayed_to(log)),
        );
    }
}

#[test]
fn a_refused_assertion_count_in_the_enter_on_the_push_leaves_the_list_under_the_error_notice() {
    for kind in [Kind::All, Kind::Axioms] {
        let mut walker = in_a_later_section(one_stage(), 0, kind);
        let list = &mut walker.list;
        let at = format!("{kind:?}");
        let before = list.run.frames();
        let log = list.log.len();
        let status = 0x61;
        list.fail_at(0, COUNT, 1, status);
        list.tap(F5);
        let drawn = list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(
            list.fail_calls(0),
            3,
            "{at}: the screen's enter was refused, then the list's enter made two calls of its own"
        );
        same_frame(&at, &drawn, &model::error(status));
        let frames = list.run.frames();
        assert_eq!(frames.len(), 3, "{at}: Libraries, the list and the notice");
        assert_eq!(
            frames[..2],
            before[..],
            "{at}: the sections frame is gone, the list's is kept"
        );
        assert_eq!(frames[2][..3], [KIND_NOTICE, NOTICE_ERROR, status], "{at}");
        list.tap(CLEAR);
        let back = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(
            list.run.frames(),
            before,
            "{at}: the notice closes on the list"
        );
        same_frame(
            &at,
            &back,
            &crate::shellrig::model::frame(&list.replayed_to(log)),
        );
        assert_eq!(list.run.top_kind(), KIND_LIST);
    }
}

#[test]
fn a_refused_assertion_count_in_the_enter_after_a_power_down_puts_the_error_notice_over_the_screen_and_drops_the_key()
 {
    let mut walker = in_a_later_section(one_stage(), 0, Kind::All);
    let list = &mut walker.list;
    list.open_sections();
    list.goto_title(20);
    list.run.run(600_000);
    let at = list.sections();
    let frames = list.run.frames();
    assert_eq!(frames.len(), 3);
    list.run.shell.power_down();
    let status = 0x62;
    list.fail_at(0, COUNT, 1, status);
    list.run.press(DOWN);
    let drawn = list.run.next_blit();
    list.run.release(DOWN);
    list.run.run(300_000);
    assert_eq!(list.fail_calls(0), 1, "the refresh's call was refused");
    same_frame("the notice", &drawn, &model::error(status));
    let now = list.run.frames();
    assert_eq!(now.len(), 4, "the notice is over the screen");
    assert_eq!(
        now[..3],
        frames[..],
        "which is as it was: the key moved nothing"
    );
    assert_eq!(now[3][..3], [KIND_NOTICE, NOTICE_ERROR, status]);
    list.tap(CLEAR);
    let back = list.run.next_blit();
    list.run.run(2_000);
    assert_eq!(list.run.frames(), frames, "the notice closes on the screen");
    list.same_sections("after the notice", &at);
    same_frame("the screen", &back, &list.shown());
}

#[test]
fn a_refused_assertion_count_when_the_notice_over_the_screen_closes_leaves_the_app() {
    let mut walker = in_a_later_section(one_stage(), 0, Kind::All);
    let list = &mut walker.list;
    list.open_sections();
    list.fail_at(0, "far_lib_title", 1, 0x63);
    list.tap(ENTER);
    let notice = list.run.next_blit();
    list.run.run(300_000);
    same_frame("the first notice", &notice, &model::error(0x63));
    assert_eq!(list.run.depth(), 4, "the notice is over the screen");
    list.fail_at(1, COUNT, 1, 0x64);
    list.tap(CLEAR);
    let stop = list.run.m().resume(3_000_000).stop;
    assert_eq!(
        stop,
        Stop::Exited,
        "a screen whose enter fails when the notice closes leaves"
    );
    assert_eq!(list.fail_calls(1), 1, "the enter's call was refused");
    list.run.shell.assert_regions_put_away();
}
