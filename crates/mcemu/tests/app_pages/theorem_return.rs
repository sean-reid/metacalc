//! What happens around the theorem screen: the notices the soft keys push and the refit after them,
//! CLEAR back to the list and to Libraries, an auto power down, a theorem opened right after a check
//! and with a failing fm_open, and a frame the refit has to clamp.

use crate::app::*;
use crate::builds::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_frames::find;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::{Key, State};

/// Opens `n` of the fixture and brings the screen to one of the states a notice has to give back.
fn prepared(n: usize, how: &str) -> Walker {
    let mut walker = Walker::at(with_library(), None, 0, n);
    match how {
        "conclusion" => {}
        "hypothesis" => {
            for _ in 0..3 {
                walker.press(LEFT, Key::Left);
            }
            walker.press(DOWN, Key::Down);
        }
        "fold made" => {
            for _ in 0..2 {
                walker.press(DOWN, Key::Down);
            }
            walker.press(ENTER, Key::Enter);
            walker.press(RIGHT, Key::Right);
        }
        "scrolled away" => {
            for _ in 0..12 {
                walker.press(LEFT, Key::Left);
            }
            for _ in 0..3 {
                walker.press(DOWN, Key::Down);
            }
        }
        other => panic!("{other}"),
    }
    walker
}

#[test]
fn each_soft_key_pushes_the_notice_and_the_frame_after_it_is_the_models_refit() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    for how in ["conclusion", "hypothesis", "fold made", "scrolled away"] {
        for (scan, key) in [(F2, Key::F2), (F3, Key::F3), (F4, Key::F4)] {
            let mut walker = prepared(n, how);
            let before = walker.frame();
            println!("{how}, {scan:02X}: {before:?}");
            walker.excursion(scan, key);
            println!("  after: {:?}", walker.frame());
        }
    }
}

#[test]
fn clear_goes_back_to_the_list_where_it_was_with_a_pending_preview_and_then_to_libraries() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = Walker::on_list(built, None, 0);
    walker.goto(n);
    walker.list.settle();
    let before = walker.list.frame_bytes();
    assert_ne!((before[2], before[4]), (0, 0), "the cursor and the top have moved");
    walker.open();
    for (scan, key) in [(DOWN, Key::Down), (LEFT, Key::Left), (ENTER, Key::Enter)] {
        walker.press(scan, key);
    }
    walker.back();
    assert_eq!(walker.list.frame_bytes(), before, "cursor and top");
    assert_eq!(walker.list.seen, (PENDING, 11), "blank, the wait begins");
    assert!(walker.list.settle(), "the preview is laid out again");
    assert!(walker.list.key(CLEAR, Key::Clear), "Libraries is drawn");
    assert_eq!((walker.list.run.depth(), walker.list.run.top_kind()), (1, KIND_LIBRARIES));
}

#[test]
fn an_auto_power_down_on_the_screen_brings_back_the_models_refit() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, f) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    for how in ["conclusion", "hypothesis", "fold made", "scrolled away"] {
        let mut walker = prepared(n, how);
        let before = walker.frame();
        walker.list.run.run(600_000);
        walker.list.run.shell.power_down();
        let blits = walker.list.run.blits();
        let at = walker.list.run.shell.app.label("TV_TOTAL");
        walker.list.run.m().load_ram(at, &[0xFF, 0xFF]);
        walker.list.tap(LETTER);
        let frame = walker.list.run.next_blit();
        walker.list.run.run(2_000);
        assert_eq!(walker.var16("TV_TOTAL"), f.rows, "{how}: the enter ran");
        assert_eq!(walker.list.run.blits(), blits + 1, "{how}: one frame for the refresh");
        walker.list.ui.press(Key::F2);
        walker.list.ui.press(Key::Clear);
        walker.list.log.extend([Key::F2, Key::Clear]);
        same_frame(how, &frame, &walker.list.shown());
        walker.same_position(how);
        assert_eq!(walker.frame()[4..6], before[4..6], "{how}: the cursor kept");
    }
}

/// The model's node under the cursor.
fn node_of(walker: &Walker) -> usize {
    match walker.list.ui.state() {
        State::Theorem { node, .. } => node,
        other => panic!("{other:?}"),
    }
}

/// Moves the cursor to the next node in preorder, as far as the keys reach: Down to the first
/// child, else Right to a sibling, else Up and the same again. False at the last.
fn next_node(walker: &mut Walker) -> bool {
    let before = node_of(walker);
    if matches!(walker.list.ui.state(), State::Theorem { hidden: true, .. }) {
        walker.press(ENTER, Key::Enter);
    }
    walker.press(DOWN, Key::Down);
    if node_of(walker) != before {
        return true;
    }
    loop {
        let at = node_of(walker);
        if at == 0 {
            return false;
        }
        walker.press(RIGHT, Key::Right);
        if node_of(walker) != at {
            return true;
        }
        walker.press(UP, Key::Up);
    }
}

#[test]
fn a_cursor_the_frame_holds_past_the_statements_is_clamped_by_the_enter() {
    let built = with_library();
    let lib = lib_of(built);
    let (n, f) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let mut walker = Walker::at(built, None, 0, n);
    let last = lib.statement(n).conclusion.len() - 1;
    assert!(last >= 3, "the conclusion has nodes to go to");
    while node_of(&walker) != last {
        assert!(next_node(&mut walker), "the keys reach the conclusion's last node");
    }
    let stack = walker.list.run.shell.app.label("SCR_STACK") + 32;
    walker.list.run.m().load_ram(stack + 4, &[99, 250]);
    let depth = walker.list.run.depth();
    walker.list.tap(F2);
    walker.list.run.next_blit();
    walker.list.run.run(300_000);
    walker.list.tap(CLEAR);
    let frame = walker.list.run.next_blit();
    walker.list.run.run(2_000);
    assert_eq!(walker.list.run.depth(), depth);
    walker.list.ui.press(Key::F2);
    walker.list.ui.press(Key::Clear);
    walker.list.log.extend([Key::F2, Key::Clear]);
    same_frame("clamped", &frame, &walker.list.shown());
    assert_eq!(walker.frame()[4], f.hyps as u8, "the statement is clamped to the conclusion");
    assert_eq!(usize::from(walker.frame()[5]), last, "and the node to the last");
    walker.same_position("clamped");
}

#[test]
fn a_theorem_opened_right_after_a_check_opens_fm_again_once() {
    let built = with_library();
    let verdicts = crate::checkrig::host_verdicts(built.library.as_ref().unwrap());
    let mut run = crate::checkrig::Run::boot(built, None);
    run.m().press_key(F2);
    let values = loop {
        let (_, values) = run.next_frame();
        if let Some(values) = values
            && values.iter().all(|&v| v != 0)
        {
            break values;
        }
    };
    run.m().release_key(F2);
    assert_eq!(values.len(), verdicts.len());
    run.tap(CLEAR);
    run.next_blit();
    run.run(2_000_000);
    let status = crate::checkrig::status_of(&values);
    let mut list = Lister {
        run,
        ui: crate::shellrig::model::ui(built, &status),
        built,
        status,
        log: Vec::new(),
        seen: (0, 0),
    };
    assert!(list.key(ENTER, Key::Enter), "the list opens");
    list.settle();
    let ready = usize::from(list.run.shell.app.label("KV_READY") - 0xC000);
    list.run.shell.app.m.ram_page(3)[ready..ready + 2].fill(0);
    list.count_fm_calls();
    let mut walker = Walker { list, stage: 0, built };
    walker.open();
    assert_eq!(walker.list.fm_calls()[0], 1, "fm_open ran once, in the theorem's enter");
    assert_eq!(walker.var("TV_TABLE"), 1, "the check's table is the library's");
    walker.back();
    walker.goto(2);
    walker.open();
    assert_eq!(walker.list.fm_calls()[0], 1, "and not again while fm is open");
}

/// The statuses fm and the reader refuse the theorem's enter with, the call each is answered at, and
/// the status the user sees.
const REFUSALS: [(&str, u8, u8); 8] = [
    ("far_lib_assertion_count", 1, 0x31),
    ("far_lib_read", 1, 0x32),
    ("far_fm_assertion", 1, 0x33),
    ("far_fm_fit", 1, 0x34),
    ("far_fm_rows", 1, 0x35),
    ("far_fm_select", 1, 0x36),
    ("far_fm_span", 1, 0x37),
    ("far_fm_open", 1, 0x38),
];

fn error_frames(walker: &mut Walker, depth: usize, status: u8, at: &str) {
    let frame = walker.list.run.next_blit();
    walker.list.run.run(300_000);
    same_frame(at, &frame, &model::error(status));
    assert_eq!(walker.list.run.depth(), depth, "{at}: depth");
    let frames = walker.list.run.frames();
    assert_eq!(
        frames[depth - 1][..3],
        [KIND_NOTICE, NOTICE_ERROR, status],
        "{at}: the error notice is on top"
    );
}

/// Makes `stub` answer `status` the first time, and for far_fm_open also makes fm_assertion say that fm
/// is closed, so that fm_open is called.
fn refuse(walker: &mut Walker, stub: &str, status: u8) {
    if stub == "far_fm_open" {
        walker.list.fail_at(1, "far_fm_assertion", 1, 1);
    }
    walker.list.fail_at(0, stub, 1, status);
}

#[test]
fn a_theorem_that_fails_to_enter_on_a_push_leaves_the_list_with_the_error_notice_over_it() {
    let built = with_library();
    for (stub, _, status) in REFUSALS {
        let at = format!("{stub}");
        let mut walker = Walker::on_list(built, None, 0);
        walker.goto(2);
        let before = walker.list.run.frames();
        refuse(&mut walker, stub, status);
        walker.list.tap(ENTER);
        error_frames(&mut walker, 3, status, &at);
        assert!(walker.list.fail_calls(0) >= 1, "{at}: the refusal ran");
        assert_eq!(
            walker.list.run.frames()[..2],
            before[..2],
            "{at}: Libraries and the list are as they were"
        );
        walker.list.tap(CLEAR);
        walker.list.run.next_blit();
        walker.list.run.run(2_000);
        assert_eq!((walker.list.run.depth(), walker.list.run.top_kind()), (2, KIND_LIST), "{at}");
        same_frame(&at, &walker.list.run.frame(), &model::frame(&walker.list.replayed()));
    }
}

#[test]
fn a_theorem_that_fails_to_enter_on_an_uncovering_keeps_its_frame_under_the_error_notice() {
    let built = with_library();
    for (stub, _, status) in REFUSALS {
        if stub == "far_lib_assertion_count" || stub == "far_lib_read" || stub == "far_fm_span" {
            continue;
        }
        let at = format!("{stub}");
        let mut walker = Walker::at(built, None, 0, 4);
        walker.press(DOWN, Key::Down);
        walker.list.tap(F2);
        walker.list.run.next_blit();
        walker.list.run.run(300_000);
        let frames = walker.list.run.frames();
        refuse(&mut walker, stub, status);
        walker.list.tap(CLEAR);
        error_frames(&mut walker, 4, status, &at);
        assert_eq!(walker.list.run.frames()[..3], frames[..3], "{at}: the theorem's frame is kept");
    }
}

#[test]
fn a_status_from_fm_inside_a_key_pushes_the_error_notice_and_changes_nothing() {
    let built = with_library();
    for (stub, scan, status) in [
        ("far_fm_step", LEFT, 0x41),
        ("far_fm_select", DOWN, 0x42),
        ("far_fm_folded", DOWN, 0x43),
        ("far_fm_fold", ENTER, 0x44),
        ("far_fm_span", UP, 0x45),
    ] {
        let at = format!("{stub} on {scan:02X}");
        let mut walker = Walker::at(built, None, 0, 4);
        walker.press(DOWN, Key::Down);
        let frames = walker.list.run.frames();
        walker.list.fail_at(0, stub, 1, status);
        walker.list.tap(scan);
        error_frames(&mut walker, 4, status, &at);
        assert_eq!(walker.list.run.frames()[..3], frames[..3], "{at}: nothing changed");
    }
}
