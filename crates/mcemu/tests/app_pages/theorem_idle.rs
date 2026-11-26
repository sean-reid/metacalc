//! The theorem row's idle slot is the shell's no-op, not the list's: the screen is opened in the state
//! where the list's idle, read over the theorem's variables, would lay out a preview.

use crate::app::*;
use crate::idle::{CARRY, SCR_PUSH, idles_to_nothing, snapshot};
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_keys::{built_of, cursor, index_of};
use crate::theorem_rig::*;
use mcui::Key;

#[test]
fn a_theorem_the_lists_idle_would_work_on_idles_to_nothing_and_changes_no_byte() {
    let built = built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    let mut walker = Walker::at(built, None, 0, index_of(built, "rows8"));
    for _ in 0..7 {
        assert!(walker.press(LEFT, Key::Left));
    }
    assert_eq!(
        cursor(&walker),
        (0, 0),
        "statement 0, a hypothesis of one node"
    );
    assert!(
        walker.press(DOWN, Key::Down),
        "the view scrolls, the cursor stays"
    );
    assert_eq!(walker.frame()[8], 0, "byte 8 is 0");
    let pending = walker.list.run.shell.app.label("LVP_PENDING") as u8;
    assert_eq!(
        walker.var("LV_PREVIEW"),
        pending,
        "the list reads a pending preview"
    );
    assert_eq!(walker.var("LV_COUNT"), 0, "no ticks to wait");
    assert_eq!(walker.var("LV_NOFM"), 0, "fm is not marked closed");
    let depth = walker.list.run.depth();
    let app = &mut walker.list.run.shell.app;
    idles_to_nothing(app, false, "the theorem");
    let kind = app.label("SCR_STACK") + 16 * (depth as u16 - 1);
    assert_eq!(app.m.read_ram(kind, 1), [KIND_THEOREM]);
    app.m.load_ram(kind, &[6]);
    let before = snapshot(app);
    let at = app.label("scr_idle");
    let done = app.call(
        BASE,
        at,
        In {
            a: SCR_PUSH,
            f: CARRY,
            ..In::default()
        },
    );
    assert_eq!(
        done.status(),
        1,
        "the list's idle on these bytes asks for a redraw"
    );
    assert_ne!(
        snapshot(app),
        before,
        "and writes: the precondition has teeth"
    );
}
