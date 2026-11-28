//! The Dist screen when the reader refuses a call: in the enter the error notice goes over the theorem
//! screen, in the draw what the refusal leaves out is left out and the rest is the model's.

use crate::dist_lib::{self, SEVEN};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_return::error_frames;
use crate::theorem_rig::*;
use crate::ui_support::{pack, same_frame};
use mcui::Key;
use mcui::canvas::Canvas;

fn on_theorem() -> Walker {
    let built = dist_lib::built();
    let n = index_of(&lib_of(built), SEVEN);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

/// The model's frame with its title bar taken from a title of `left` and `right`.
pub fn with_title(frame: &[u8], left: &str, right: &str) -> Vec<u8> {
    let mut canvas = Canvas::default();
    canvas.title(left, right);
    let bar = pack(&canvas.into_bitmap());
    let mut out = frame.to_vec();
    out[..8 * 12].copy_from_slice(&bar[..8 * 12]);
    out
}

/// The model's frame with the rows under the title blank.
pub fn without_rows(frame: &[u8]) -> Vec<u8> {
    let mut out = frame.to_vec();
    out[8 * 12..56 * 12].fill(0);
    out
}

#[test]
fn a_refusal_in_the_enter_pushes_the_error_notice_over_the_theorem_screen() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let pairs = lib.dist(index_of(&lib, SEVEN)).len();
    assert!(pairs > 32, "{pairs} pairs take more than two runs");
    let mut refused = 0;
    for (stub, call, status) in [
        ("far_lib_record", 1, 0x61),
        ("far_lib_read", 1, 0x62),
        ("far_lib_read", 2, 0x63),
        ("far_lib_glyph", 1, 0x64),
        ("far_lib_glyph", 2, 0x65),
        ("far_lib_glyph", 41, 0x66),
    ] {
        let at = format!("{stub} at call {call}");
        let mut walker = on_theorem();
        let before = walker.list.run.frames();
        walker.list.fail_at(0, stub, call, status);
        walker.tap(F2);
        error_frames(&mut walker, 4, status, &at);
        assert!(
            walker.list.fail_calls(0) >= u16::from(call),
            "{at}: the call ran"
        );
        assert_eq!(
            walker.list.run.frames()[..3],
            before[..3],
            "{at}: the screens below"
        );
        refused += 1;
        walker.tap(CLEAR);
        let frame = walker.list.run.next_blit();
        walker.list.run.run(300_000);
        walker.list.ui.press(Key::F2);
        walker.list.ui.press(Key::Clear);
        walker.list.log.extend([Key::F2, Key::Clear]);
        same_frame(&at, &frame, &walker.list.shown());
        assert_eq!(
            walker.list.run.top_kind(),
            KIND_THEOREM,
            "{at}: the theorem is back"
        );
        assert_eq!(walker.list.run.depth(), 3);
    }
    assert_eq!(refused, 6);
}

#[test]
fn a_refusal_inside_the_draw_leaves_out_what_it_refused_and_draws_the_rest() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let n = index_of(&lib, SEVEN);
    let label = lib.label(n).to_string();
    let count = format!("dv {}", lib.dist(n).len());
    for stub in [
        "far_lib_record",
        "far_lib_label",
        "far_lib_read",
        "far_lib_glyph",
    ] {
        let mut walker = on_theorem();
        open_dist(&mut walker);
        walker.list.fail_at(0, stub, 1, 0x67);
        let blits = walker.list.run.blits();
        tap(&mut walker.list, DOWN);
        let frame = walker.list.run.next_blit();
        walker.list.run.run(300_000);
        assert_eq!(walker.list.run.blits(), blits + 1, "{stub}: one frame");
        assert!(walker.list.fail_calls(0) >= 1, "{stub}: the refusal ran");
        walker.list.ui.press(Key::Down);
        let model = walker.list.shown();
        assert_eq!(model_top(&walker), 1);
        let want = match stub {
            "far_lib_record" => without_rows(&with_title(&model, &label, "")),
            "far_lib_label" => with_title(&model, "", &count),
            _ => without_rows(&model),
        };
        same_frame(stub, &frame, &want);
        assert_eq!(walker.list.run.top_kind(), KIND_DIST, "{stub}");
    }
}
