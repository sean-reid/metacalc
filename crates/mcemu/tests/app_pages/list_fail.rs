//! What the user sees when the list's enter fails, on the push from Libraries and on the uncovering
//! from its notice, and what its rows show when the store refuses a mark.

use crate::app::*;
use crate::list_depth::alternating_marks;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table::VAR;
use crate::ui_support::same_frame;
use mcui::Key;

/// Every way list_enter answers a status other than 0: the reader's assertion count refusing inside
/// libs_counts, the reader's read of the library's CRC there, and the count list_enter asks for
/// itself.
const REFUSALS: [(&str, u8, u8); 3] = [
    ("far_lib_assertion_count", 1, 0x31),
    ("far_lib_read", 1, 0x32),
    ("far_lib_assertion_count", 2, 0x33),
];

fn error_frames(list: &mut Lister, depth: usize, status: u8, at: &str) {
    let frame = list.run.next_blit();
    list.run.run(300_000);
    same_frame(at, &frame, &model::error(status));
    assert_eq!(list.run.depth(), depth, "{at}: depth");
    let frames = list.run.frames();
    assert_eq!(
        frames[depth - 1][..3],
        [KIND_NOTICE, NOTICE_ERROR, status],
        "{at}: the error notice is on top"
    );
}

#[test]
fn a_list_that_fails_to_enter_on_a_push_leaves_libraries_with_the_error_notice_over_it() {
    let built = with_library();
    for (stub, call, status) in REFUSALS {
        let at = format!("{stub} call {call}");
        let mut list = Lister::at_stage(built, None, 0);
        let libraries = list.run.frames();
        list.fail_at(0, stub, call, status);
        list.tap(ENTER);
        error_frames(&mut list, 2, status, &at);
        assert!(
            list.fail_calls(0) >= u16::from(call),
            "{at}: the refusal ran"
        );
        assert_eq!(
            list.run.frames()[0],
            libraries[0],
            "{at}: Libraries is as it was, and the list's frame is gone"
        );
        assert!(list.run.depth() == 2 && list.run.top_kind() == KIND_NOTICE);
        list.tap(CLEAR);
        list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.run.depth(), 1, "{at}: the notice closes");
        same_frame(&at, &list.run.frame(), &model::frame(&list.ui));
    }
}

#[test]
fn a_list_that_fails_to_enter_on_an_uncovering_keeps_its_frame_under_the_error_notice() {
    let built = with_library();
    for (stub, call, status) in REFUSALS {
        let at = format!("{stub} call {call}");
        let mut list = Lister::open(built, None, 0);
        list.settle();
        list.key(RIGHT, Key::Right);
        list.settle();
        list.tap(F4);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(list.run.depth(), 3);
        let frames = list.run.frames();
        list.fail_at(0, stub, call, status);
        list.tap(CLEAR);
        error_frames(&mut list, 3, status, &at);
        assert_eq!(list.fail_calls(0), u16::from(call), "{at}: the refusal ran");
        assert_eq!(
            list.run.frames()[..2],
            frames[..2],
            "{at}: Libraries and the list are kept"
        );
        list.tap(CLEAR);
        let frame = list.run.next_blit();
        list.run.run(2_000);
        assert_eq!(list.run.depth(), 2, "{at}: the error notice closes");
        let pending = list.replayed();
        same_frame(&at, &frame, &model::frame(&pending));
        assert_eq!(
            list.run.frames()[..2],
            frames[..2],
            "{at}: cursor and top kept"
        );
    }
}

/// The frame without its title bar, whose counts are the enter's and not the rows'.
fn below_the_title(frame: &[u8]) -> Vec<u8> {
    let mut rest = frame.to_vec();
    rest[..96].fill(0);
    rest
}

/// A list of the fixture's three stages with every other assertion checked, opened and settled, then
/// `spoil`ed, then moved a row: its rows show every mark unchecked, as the model's do with no status,
/// which is what the header says a row shows when the store refuses its value. The title keeps the
/// counts the enter read.
fn refused_table(spoil: impl Fn(&mut Lister)) -> Lister {
    let built = crate::builds::three_stages();
    let (data, status) = alternating_marks(built);
    let mut list = Lister::open(built, Some((&data, &status)), 0);
    list.settle();
    assert_eq!(list.var("LV_TABLE"), 1, "the library's table is read");
    spoil(&mut list);
    list.status = Vec::new();
    list.ui = list.replayed();
    list.ui.press(Key::Down);
    list.log.push(Key::Down);
    list.tap(DOWN);
    let frame = list.run.next_blit();
    list.run.run(2_000);
    let want = list.shown();
    assert_ne!(
        below_the_title(&frame),
        below_the_title(&marked_rows(&data, &status, built)),
        "the rows are not the marked ones"
    );
    same_frame(
        "the rows of a table the store refuses",
        &below_the_title(&frame),
        &below_the_title(&want),
    );
    let mut spent = 0;
    while list.var("LV_PREVIEW") == PENDING {
        list.run.run(20_000);
        spent += 20_000;
        assert!(spent < LIMIT);
    }
    list.run.run(300_000);
    list.ui.idle();
    same_frame(
        "the settled rows",
        &below_the_title(&list.run.frame()),
        &below_the_title(&list.shown()),
    );
    list
}

/// The rows the model draws for the same list moved a row with its marks.
fn marked_rows(_data: &[u8], status: &[mcui::Status], built: &'static Built) -> Vec<u8> {
    let mut ui = crate::shellrig::model::ui(built, status);
    for key in [Key::Enter, Key::Down] {
        ui.press(key);
    }
    crate::shellrig::model::frame(&ui)
}

#[test]
fn a_table_the_store_refuses_after_the_enter_accepted_it_shows_every_row_unchecked() {
    let mut list = refused_table(|list| {
        let pointer = list.run.m().read_var(VAR).unwrap().pointer;
        list.run.m().load_ram(pointer + 2, &[0xFF]);
    });
    assert_eq!(
        list.var("LV_TABLE"),
        1,
        "the list still thinks it has a table"
    );
    let var = list.run.m().read_var(VAR).expect("the table is left");
    assert_eq!(var.data[0], 0xFF, "damaged, and not repaired or deleted");
}

#[test]
fn a_table_archived_after_the_enter_accepted_it_shows_every_row_unchecked() {
    let mut list = refused_table(|list| list.run.m().archive_var(VAR));
    let var = list.run.m().read_var(VAR).expect("the table is left");
    assert!(var.archived, "and still archived");
}

#[test]
fn a_table_archived_before_the_list_opens_is_no_table_and_stays_archived() {
    let built = crate::builds::three_stages();
    let (data, status) = alternating_marks(built);
    let mut list = Lister::at_stage(built, Some((&data, &status)), 0);
    list.run.m().archive_var(VAR);
    list.status = Vec::new();
    list.ui = list.replayed();
    assert!(list.key(ENTER, Key::Enter));
    list.settle();
    assert_eq!(list.var("LV_TABLE"), 0);
    let var = list.run.m().read_var(VAR).expect("the table is left");
    assert!(var.archived);
    assert_eq!(var.data, data, "with its bytes");
}
