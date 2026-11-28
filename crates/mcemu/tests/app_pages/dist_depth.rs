//! The Dist screen's routines traced one call at a time: the stack each writes on each path, counting the
//! caller's return address, whether interrupts and the speed are as they were at the call, and that the
//! stack comes back balanced. Each call is traced with interrupts masked for its length, so that no
//! handler's pushes are in the figure.

use crate::dist_lib::{self, EMPTY, SEVEN};
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

/// The theorem screen of `label` on the stub OS, with the keys not yet pressed.
fn on_theorem_stub(label: &str) -> Walker {
    let built = dist_lib::built();
    let n = index_of(&lib_of(built), label);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

/// A key traced on `routine`, then let finish and the model told: the frame the app shows is the model's.
pub fn traced(
    walker: &mut Walker,
    routine: &str,
    scan: u8,
    key: Key,
    (speed, mask): (u8, bool),
) -> Trace {
    walker.list.run.press(scan);
    let t = walker.trace(routine, mask, speed);
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

/// What each routine of the screen did on each path, at `speed`, with the interrupts masked for each call
/// or on at it: `on_theorem` gives the theorem screen of an assertion by label, `label` being the
/// assertion of 7 rows, and `on_large` the too large screen of an assertion that has pairs.
pub fn figures_on(
    on_theorem: &dyn Fn(&str) -> Walker,
    on_large: &dyn Fn() -> Walker,
    label: &str,
    speed: u8,
    mask: bool,
) -> BTreeMap<String, (u16, u8)> {
    let mut found = BTreeMap::new();
    let mut put = |name: &str, t: Trace| {
        let depth = kept(name, &[t], speed, mask);
        found.insert(name.to_string(), (depth, t.action));
    };
    let mut walker = on_large();
    assert_eq!(walker.var("TV_LARGE"), 1, "the too large screen");
    walker.list.run.press(F2);
    put(
        "key f2 on the too large screen",
        walker.trace("theorem_key", mask, speed),
    );
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    assert_eq!(
        walker.list.run.top_kind(),
        KIND_DIST,
        "F2 pushed the screen"
    );
    let mut walker = on_theorem(label);
    walker.list.run.press(F2);
    put(
        "key f2 on the theorem screen",
        walker.trace("theorem_key", mask, speed),
    );
    put("enter", walker.trace("dist_enter", mask, speed));
    put("draw", walker.trace("dist_draw", mask, speed));
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
        put(
            name,
            traced(&mut walker, "dist_key", scan, key, (speed, mask)),
        );
    }
    for (name, stub, call) in [
        ("enter refused at the record", "far_lib_record", 1),
        ("enter refused at a read", "far_lib_read", 1),
        ("enter refused at a glyph", "far_lib_glyph", 7),
    ] {
        let mut walker = on_theorem(label);
        walker.list.fail_at(0, stub, call, 0x6A);
        walker.list.run.press(F2);
        walker.trace("theorem_key", mask, speed);
        let t = walker.trace("dist_enter", mask, speed);
        assert_eq!(t.action, 0x6A, "{name}: the reader's status");
        put(name, t);
        walker.list.run.release(F2);
        walker.list.run.run(3_000_000);
    }
    for (name, stub, call) in [
        ("draw refused at the record", "far_lib_record", 1),
        ("draw refused at the label", "far_lib_label", 1),
        ("draw refused at a read", "far_lib_read", 1),
        ("draw refused at a glyph", "far_lib_glyph", 3),
    ] {
        let mut walker = on_theorem(label);
        walker.list.run.press(F2);
        walker.trace("theorem_key", mask, speed);
        walker.trace("dist_enter", mask, speed);
        walker.list.fail_at(0, stub, call, 0x6A);
        put(name, walker.trace("dist_draw", mask, speed));
        walker.list.run.release(F2);
        walker.list.run.run(3_000_000);
        assert!(walker.list.fail_calls(0) >= u16::from(call), "{name}");
    }
    let mut walker = on_theorem(EMPTY);
    walker.list.run.press(F2);
    walker.trace("theorem_key", mask, speed);
    put(
        "enter with no pair",
        walker.trace("dist_enter", mask, speed),
    );
    put("draw with no pair", walker.trace("dist_draw", mask, speed));
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    let cold = |run: &mut crate::checkrig::Run| {
        let at = run.shell.app.label("lr_dn");
        run.m().load_ram(at, &[0xFF, 0xFF]);
    };
    let mut walker = on_theorem(label);
    walker.list.run.press(F2);
    walker.trace("theorem_key", mask, speed);
    put(
        "enter with a cold descriptor",
        walker.trace_after("dist_enter", mask, speed, cold),
    );
    put(
        "draw with a cold descriptor",
        walker.trace_after("dist_draw", mask, speed, cold),
    );
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    let mut walker = on_theorem(label);
    walker.list.run.press(F2);
    walker.trace("theorem_key", mask, speed);
    let past = lib_of(walker.built).len() as u16;
    let t = walker.trace_after("dist_enter", mask, speed, |run| {
        let hl = run.m().regs().hl;
        run.m().load_ram(hl + 2, &past.to_le_bytes());
    });
    assert_ne!(
        t.action, 0,
        "the reader refuses an assertion past the count"
    );
    put("enter refused at a record the reader refuses", t);
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    found
}

/// The figures the header gives, each as its stack depth and its answer, with interrupts masked for the call.
pub fn expected() -> BTreeMap<String, (u16, u8)> {
    [
        ("key f2 on the theorem screen", (8, SCR_PUSH)),
        ("key f2 on the too large screen", (8, SCR_PUSH)),
        ("enter", (22, 0)),
        ("draw", (26, 0)),
        ("enter with a cold descriptor", (22, 0)),
        ("draw with a cold descriptor", (26, 0)),
        ("enter refused at a record the reader refuses", (12, 5)),
        ("key down, a move", (2, SCR_REDRAW)),
        ("key down, at the end", (2, SCR_STAY)),
        ("key up, a move", (2, SCR_REDRAW)),
        ("key up, at the start", (2, SCR_STAY)),
        ("key f1", (2, SCR_STAY)),
        ("key letter", (2, SCR_STAY)),
        ("key clear", (2, SCR_POP)),
        ("enter refused at the record", (10, 0x6A)),
        ("enter refused at a read", (14, 0x6A)),
        ("enter refused at a glyph", (22, 0x6A)),
        ("draw refused at the record", (26, 0)),
        ("draw refused at the label", (24, 0)),
        ("draw refused at a read", (26, 0)),
        ("draw refused at a glyph", (26, 0)),
        ("enter with no pair", (12, 0)),
        ("draw with no pair", (26, 0)),
    ]
    .map(|(name, figure)| (name.to_string(), figure))
    .into()
}

/// The most bytes an interrupt handler adds to a call it lands in: 4 on the stub OS, 6 on the reference OS.
pub const fn handler(real: bool) -> u16 {
    if real { 6 } else { 4 }
}

/// `got` against the header's figures. Masked, every depth is exact. With interrupts on at the call a key
/// and the F2 that pushes a screen are as short as masked, which is exact, and a handler that lands in the
/// enter or the draw adds its bytes where the stack is deepest, so those lie from the masked figure to it
/// plus the handler's most; the answers and the interrupt state are exact either way.
pub fn same_figures(got: &BTreeMap<String, (u16, u8)>, mask: bool, real: bool, what: &str) {
    let want = expected();
    assert_eq!(
        got.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>(),
        "{what}: the paths"
    );
    for (name, &(depth, answer)) in got {
        let (masked, status) = want[name];
        assert_eq!(answer, status, "{what}: {name}: the answer");
        if mask || name.starts_with("key") {
            assert_eq!(depth, masked, "{what}: {name}");
        } else {
            assert!(
                (masked..=masked + handler(real)).contains(&depth),
                "{what}: {name}: {depth} is not {masked} to {masked} and a handler"
            );
        }
    }
}

fn on_large_stub() -> Walker {
    let built = crate::dist_large::built();
    let n = index_of(&lib_of(built), crate::dist_large::LARGE_SEVEN);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

#[test]
fn the_routines_return_with_interrupts_and_speed_as_they_were_and_the_stack_balanced() {
    for mask in [true, false] {
        let mut seen = BTreeMap::new();
        for speed in [0, 1] {
            let got = figures_on(&on_theorem_stub, &on_large_stub, SEVEN, speed, mask);
            println!("stub, speed {speed}, masked {mask}: {got:?}");
            seen.insert(speed, got);
        }
        for speed in [0, 1] {
            same_figures(
                &seen[&speed],
                mask,
                false,
                &format!("stub, masked {mask}, speed {speed}"),
            );
        }
    }
}
