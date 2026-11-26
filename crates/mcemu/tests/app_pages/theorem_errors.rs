//! The theorem screen's error exits at the call that fails, not only the first: the nth call of a
//! stub, in the enter, in a key and inside the draw, with the frame each leaves, and the interrupts,
//! speed and stack of one exit of the enter and one of a key.

use crate::app::*;
use crate::list_rig::proofs_three;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_depth::kept;
use crate::theorem_frames::find;
use crate::theorem_return::{error_frames, prepared};
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::canvas::Call;
use mcui::{Key, State};

const SCR_PUSH: u8 = 2;

fn many() -> (&'static Built, usize, Facts) {
    let built = with_library();
    let (n, f) = find(&lib_of(built), |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    (built, n, f)
}

#[test]
fn a_refusal_at_a_later_call_of_the_enter_shows_the_error_notice_and_leaves_the_list() {
    let (built, n, _) = many();
    for (stub, call, status) in [
        ("far_fm_rows", 2, 0x51),
        ("far_fm_rows", 5, 0x52),
        ("far_fm_select", 2, 0x53),
        ("far_fm_select", 7, 0x54),
    ] {
        let at = format!("{stub} at call {call}");
        let mut walker = Walker::on_list(built, None, 0);
        walker.goto(n);
        let before = walker.list.run.frames();
        walker.list.fail_at(0, stub, call, status);
        walker.tap(ENTER);
        error_frames(&mut walker, 3, status, &at);
        assert!(
            walker.list.fail_calls(0) >= u16::from(call),
            "{at}: the call ran"
        );
        assert_eq!(walker.list.run.frames()[..2], before[..2], "{at}");
    }
}

#[test]
fn a_refusal_in_an_uncovering_enter_at_a_later_call_keeps_the_frame_under_the_notice() {
    let (_, n, _) = many();
    for (stub, call, status) in [
        ("far_fm_rows", 2, 0x55),
        ("far_fm_select", 3, 0x56),
        ("far_fm_nodes", 1, 0x57),
        ("far_fm_span", 1, 0x58),
    ] {
        let at = format!("{stub} at call {call}");
        let mut walker = prepared(n, "hypothesis");
        walker.tap(F2);
        walker.list.run.next_blit();
        walker.list.run.run(300_000);
        let frames = walker.list.run.frames();
        walker.list.fail_at(0, stub, call, status);
        walker.tap(CLEAR);
        error_frames(&mut walker, 4, status, &at);
        assert!(
            walker.list.fail_calls(0) >= u16::from(call),
            "{at}: the call ran"
        );
        assert_eq!(
            walker.list.run.frames()[..3],
            frames[..3],
            "{at}: the frame is kept"
        );
    }
}

#[test]
fn a_refusal_inside_a_key_at_a_later_call_pushes_the_error_notice_and_changes_nothing() {
    let (built, n, _) = many();
    for (stub, keys, scan, call, status) in [
        ("far_fm_nodes", vec![DOWN, UP], UP, 1, 0x59),
        ("far_fm_rows", vec![DOWN], ENTER, 1, 0x5A),
        ("far_fm_select", vec![DOWN], DOWN, 2, 0x5B),
    ] {
        let at = format!("{stub} on {scan:02X}");
        let mut walker = Walker::at(built, None, 0, n);
        for &key in &keys {
            let model = match key {
                DOWN => Key::Down,
                UP => Key::Up,
                _ => Key::Left,
            };
            walker.press(key, model);
        }
        let frames = walker.list.run.frames();
        walker.list.fail_at(0, stub, call, status);
        walker.tap(scan);
        error_frames(&mut walker, 4, status, &at);
        assert!(
            walker.list.fail_calls(0) >= u16::from(call),
            "{at}: the call ran"
        );
        assert_eq!(
            walker.list.run.frames()[..3],
            frames[..3],
            "{at}: nothing changed"
        );
    }
}

/// The model's frame with `mutate` applied to the packed bytes.
fn after_key(walker: &mut Walker, scan: u8, key: Key) -> (Vec<u8>, Vec<Call>) {
    tap(&mut walker.list, scan);
    walker.list.ui.press(key);
    walker.list.log.push(key);
    let frame = walker.list.run.next_blit();
    walker.list.run.run(300_000);
    (frame, walker.list.ui.frame_calls().1)
}

fn state_of(walker: &Walker) -> (usize, usize, usize) {
    match walker.list.ui.state() {
        State::Theorem {
            statement,
            node,
            scroll,
            ..
        } => (statement, node, scroll),
        other => panic!("{other:?}"),
    }
}

/// The statements with a row in the window of the model's screen.
fn visible(walker: &mut Walker) -> Vec<usize> {
    let (_, _, scroll) = state_of(walker);
    let (starts, rows) = walker.mirror().starts();
    (0..starts.len())
        .filter(|&s| {
            let end = starts.get(s + 1).copied().unwrap_or(rows);
            end > scroll && starts[s] < scroll + 6
        })
        .collect()
}

fn blank_rows(frame: &mut [u8], rows: std::ops::Range<usize>, scroll: usize) {
    for row in rows {
        let text = (row + 1)
            .checked_sub(scroll)
            .filter(|r| (1..=6).contains(r));
        if let Some(text) = text {
            frame[8 * text * 12..8 * (text + 1) * 12].fill(0);
        }
    }
}

#[test]
fn a_refused_select_inside_the_draw_leaves_that_statement_blank_and_the_rest_the_models() {
    let (built, n, _) = many();
    let prepare = || {
        let mut walker = Walker::at(built, None, 0, n);
        walker.press(LEFT, Key::Left);
        walker
    };
    let mut walker = prepare();
    walker.list.fail_at(1, "far_fm_select", 255, 1);
    walker.press(LEFT, Key::Left);
    let total = walker.list.fail_calls(1);
    let count = visible(&mut walker).len();
    assert!(
        count >= 3 && total > count as u16,
        "{total} selects, {count} drawn"
    );
    for back in [0usize, 1] {
        let mut walker = prepare();
        walker
            .list
            .fail_at(0, "far_fm_select", total as u8 - back as u8, 0x5C);
        let (frame, _) = after_key(&mut walker, LEFT, Key::Left);
        assert!(walker.list.fail_calls(0) >= total - back as u16);
        let (_, _, scroll) = state_of(&walker);
        let shown = visible(&mut walker);
        let s = shown[shown.len() - 1 - back];
        let (starts, rows) = walker.mirror().starts();
        let end = starts.get(s + 1).copied().unwrap_or(rows);
        let mut want = walker.list.shown();
        blank_rows(&mut want, starts[s]..end, scroll);
        same_frame(&format!("select refused for statement {s}"), &frame, &want);
        assert_eq!(walker.list.run.depth(), walker.list.ui.depth());
    }
}

#[test]
fn a_refused_cursor_inside_the_draw_leaves_the_node_with_no_rectangles() {
    let (built, n, _) = many();
    let mut walker = Walker::at(built, None, 0, n);
    walker.list.fail_at(0, "far_fm_cursor", 1, 0x5D);
    let (frame, calls) = after_key(&mut walker, DOWN, Key::Down);
    assert_eq!(
        walker.list.fail_calls(0),
        1,
        "the draw asked for the cursor once"
    );
    let mut want = walker.list.shown();
    let mut inverted = 0;
    for call in calls {
        if let Call::Invert {
            x,
            y,
            width,
            height,
        } = call
        {
            inverted += 1;
            for yy in y..(y + height).min(64) {
                for xx in x..(x + width).min(96) {
                    want[yy * 12 + xx / 8] ^= 0x80 >> (xx % 8);
                }
            }
        }
    }
    assert!(inverted > 0, "the model inverts the cursor's rectangles");
    same_frame("cursor refused", &frame, &want);
}

#[test]
fn one_error_exit_of_the_enter_and_one_of_a_key_keep_interrupts_and_speed_and_write_these_depths() {
    let (built, n, _) = many();
    for speed in [0u8, 1] {
        let mut walker = Walker::on_list(built, None, 0);
        walker.goto(n);
        walker.list.fail_at(0, "far_fm_fit", 1, 0x5E);
        walker.list.run.m().press_key(ENTER);
        let pushed = walker.trace("list_key", true, speed);
        let entered = walker.trace("theorem_enter", true, speed);
        walker.list.run.m().release_key(ENTER);
        assert_eq!(entered.action, 0x5E, "the enter answers the refusal");
        let enter = kept("enter error exit", &[entered], speed, true);
        let mut walker = Walker::at(built, None, 0, n);
        walker.list.fail_at(0, "far_fm_step", 1, 0x5F);
        walker.list.run.m().press_key(LEFT);
        let t = walker.trace("theorem_key", true, speed);
        walker.list.run.m().release_key(LEFT);
        assert_eq!(t.action, SCR_PUSH, "the key pushes the error notice");
        let key = kept("key error exit", &[t], speed, true);
        println!(
            "speed {speed}: enter error exit {enter} bytes, key error exit {key} bytes, list_key {pushed:?}"
        );
        assert_eq!((enter, key), (24, 10), "speed {speed}");
    }
}

/// A theorem of the proofs library the table marks as checked, and the walker open on its list with the
/// table installed.
fn a_checked_theorem() -> (Walker, usize) {
    let built = proofs_three();
    let lib = lib_of(built);
    let (data, status) = crate::theorem_floors::marked_table(built);
    let values = crate::table::values(built, &status);
    let i = (0..lib.len())
        .find(|&i| lib.is_theorem(i) && values[i] == 1)
        .expect("a theorem the table marks as checked");
    let mut walker = Walker::on_list(built, Some((&data, &status)), 2);
    walker.goto(i);
    walker.hold_preview();
    (walker, i)
}

/// The glyph codes of `text`, ended by 0.
fn codes(walker: &Walker, text: &str) -> Vec<u8> {
    let app = &walker.list.run.shell.app;
    let mut codes: Vec<u8> = text
        .chars()
        .map(|c| app.label(&format!("UI_FONT_CH_{:X}", c as u32)) as u8)
        .collect();
    codes.push(0);
    codes
}

#[test]
fn a_refused_call_while_the_title_is_built_leaves_a_plain_title_and_no_notice() {
    let check = mcglyph::CHECK;
    for (what, right, empty_label) in [
        ("far_lib_label", format!("{check} thm"), true),
        ("far_lib_record", format!("{check} ax"), false),
        ("store_get", "? thm".to_string(), false),
    ] {
        let (mut walker, _) = a_checked_theorem();
        match what {
            "store_get" => {
                let at = walker.list.run.shell.app.label("store_get");
                assert!(
                    (0x4000..0x8000).contains(&at),
                    "store_get is on the entry page"
                );
                let ret = [0x3E, 0x06, 0xC9];
                walker
                    .list
                    .run
                    .m()
                    .load_flash(BASE, usize::from(at - 0x4000), &ret);
            }
            stub => walker.list.fail_at(0, stub, 1, 0x61),
        }
        walker.tap(ENTER);
        walker.list.run.next_blit();
        walker.list.run.run(300_000);
        if what != "store_get" {
            assert_eq!(walker.list.fail_calls(0), 1, "{what}: the refusal ran");
        }
        assert_eq!(
            walker.list.run.depth(),
            3,
            "{what}: the list and the theorem"
        );
        assert_eq!(
            walker.list.run.top_kind(),
            KIND_THEOREM,
            "{what}: no notice"
        );
        let (cut, tail) = {
            let app = &walker.list.run.shell.app;
            (app.label("TV_CUT"), app.label("TV_RIGHT"))
        };
        let want = codes(&walker, &right);
        let got = walker.list.run.peek(tail, want.len());
        assert_eq!(got, want, "{what}: the mark and the kind");
        let first = walker.list.run.peek(cut, 1)[0];
        assert_eq!(
            first == 0,
            empty_label,
            "{what}: the label is empty: {empty_label}"
        );
    }
}
