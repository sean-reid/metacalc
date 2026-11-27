//! The Dist screen's routines traced one call at a time: the stack each writes on each path, counting the
//! caller's return address, whether interrupts and the speed are as they were at the call, and that the
//! stack comes back balanced. Each call is traced with interrupts masked for its length, so that no
//! handler's pushes are in the figure.

use crate::app::*;
use crate::dist_lib::{self, SEVEN};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_depth::{Trace, kept};
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::Key;
use std::collections::BTreeMap;

pub const SCR_STAY: u8 = 0;
pub const SCR_REDRAW: u8 = 1;
pub const SCR_PUSH: u8 = 2;
pub const SCR_POP: u8 = 3;

/// The Dist screen of `label` with the keys not yet pressed: the theorem screen of it, on `built`.
fn on_theorem(built: &'static Built, label: &str) -> Walker {
    let n = index_of(&lib_of(built), label);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

/// A key traced on `routine`, then let finish and the model told: the frame the app shows is the model's.
pub fn traced(walker: &mut Walker, routine: &str, scan: u8, key: Key, speed: u8) -> Trace {
    walker.list.run.press(scan);
    let t = walker.trace(routine, true, speed);
    walker.list.run.release(scan);
    walker.list.run.run(3_000_000);
    walker.list.ui.press(key);
    walker.list.log.push(key);
    same_frame(
        &format!("after the traced {scan:02X}"),
        &walker.list.run.frame(),
        &walker.list.shown(),
    );
    t
}

/// What each routine of the screen did on each path, at `speed`, on the app in `built` with the
/// assertion `label` of 7 rows and `few` of fewer.
pub fn figures_on(built: &'static Built, label: &str, speed: u8) -> BTreeMap<String, (u16, u8)> {
    let mut found = BTreeMap::new();
    let mut put = |name: &str, t: Trace| {
        let depth = kept(name, &[t], speed, true);
        found.insert(name.to_string(), (depth, t.action));
    };
    let mut walker = on_theorem(built, label);
    walker.list.run.press(F2);
    put(
        "key f2 on the theorem screen",
        walker.trace("theorem_key", true, speed),
    );
    put("enter", walker.trace("dist_enter", true, speed));
    put("draw", walker.trace("dist_draw", true, speed));
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    walker.list.ui.press(Key::F2);
    walker.list.log.push(Key::F2);
    same_frame(
        "after the traced F2",
        &walker.list.run.frame(),
        &walker.list.shown(),
    );
    assert_eq!(walker.list.run.top_kind(), KIND_DIST);
    for (name, scan, key) in [
        ("key down, a move", DOWN, Key::Down),
        ("key down, at the end", DOWN, Key::Down),
        ("key up, a move", UP, Key::Up),
        ("key up, at the start", UP, Key::Up),
        ("key f1", F1, Key::F1),
        ("key letter", LETTER, Key::Letter('A')),
        ("key clear", CLEAR, Key::Clear),
    ] {
        put(name, traced(&mut walker, "dist_key", scan, key, speed));
    }
    found
}

/// The figures the header gives, each as its stack depth and its answer.
pub fn expected() -> BTreeMap<String, (u16, u8)> {
    [
        ("key f2 on the theorem screen", (8, SCR_PUSH)),
        ("enter", (22, 0)),
        ("draw", (26, 0)),
        ("key down, a move", (2, SCR_REDRAW)),
        ("key down, at the end", (2, SCR_STAY)),
        ("key up, a move", (2, SCR_REDRAW)),
        ("key up, at the start", (2, SCR_STAY)),
        ("key f1", (2, SCR_STAY)),
        ("key letter", (2, SCR_STAY)),
        ("key clear", (2, SCR_POP)),
    ]
    .map(|(name, figure)| (name.to_string(), figure))
    .into()
}

#[test]
fn the_routines_return_with_interrupts_and_speed_as_they_were_and_the_stack_balanced() {
    let built = dist_lib::built();
    let mut seen = BTreeMap::new();
    for speed in [0, 1] {
        let got = figures_on(built, SEVEN, speed);
        println!("stub, speed {speed}: {got:?}");
        seen.insert(speed, got);
    }
    assert_eq!(seen[&0], seen[&1], "the depths do not depend on the speed");
    assert_eq!(seen[&0], expected());
}
