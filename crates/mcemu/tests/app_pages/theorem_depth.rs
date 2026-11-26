//! The theorem screen's routines traced one call at a time on the app's own path: the stack each
//! writes, counting the caller's return address, whether interrupts and the speed are as they were at
//! the call, and that the stack comes back balanced. Each call is traced with interrupts masked for
//! its length, so that no handler's pushes are in the figure.

use crate::app::*;
use crate::checkrig::{Run, WAIT};
use crate::shellrig::*;
use crate::theorem_frames::find;
use crate::theorem_rig::*;
use mcui::Key;
use std::collections::BTreeMap;

/// What one traced call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trace {
    /// Bytes written below the stack pointer at the entry, the return address added.
    pub depth: u16,
    pub iff: (u8, u8),
    pub speed: (u8, u8),
    pub action: u8,
}

fn set_interrupts(run: &mut Run, on: u8) {
    let mut regs = run.m().regs();
    regs.iff1 = on;
    regs.iff2 = on;
    run.m().set_regs(&regs);
}

impl Walker {
    /// Runs to the entry of the routine at `label`, runs it to its return and reports it. With `mask`
    /// interrupts are off for the call and on again after it.
    pub fn trace(&mut self, label: &str, mask: bool, speed: u8) -> Trace {
        let run = &mut self.list.run;
        let at = run.shell.app.label(label);
        run.run_to_addr(label, at, WAIT);
        trace_here(run, label, mask, speed)
    }
}

/// `trace` for a machine that is at the entry of `label` already. The speed port holds `speed` at the
/// call and is written back to 6 MHz after it.
pub fn trace_here(run: &mut Run, label: &str, mask: bool, speed: u8) -> Trace {
    run.m().out_port(0x20, speed);
    let sp = run.m().regs().sp;
    let ret = {
        let b = run.m().read_ram(sp, 2);
        u16::from_le_bytes([b[0], b[1]])
    };
    if mask {
        set_interrupts(run, 0);
    }
    let kept = run.m().read_ram(sp - 256, 256);
    let fill: Vec<u8> = (0..=255u8).map(|i| i.wrapping_mul(7) | 1).collect();
    run.m().load_ram(sp - 256, &fill);
    let before = (run.m().regs().iff1, run.m().in_port(0x20) & 1);
    let back = match label {
        "list_key" | "theorem_key" => run.shell.app.label("scr_key") + 5,
        _ => ret,
    };
    run.run_to_addr(&format!("the return of {label}"), back, WAIT);
    let regs = run.m().regs();
    assert_eq!(regs.sp, sp + 2, "{label}: the stack is balanced");
    let after = run.m().read_ram(sp - 256, 256);
    run.m().load_ram(sp - 256, &kept);
    let lowest = after
        .iter()
        .zip(&fill)
        .position(|(a, f)| a != f)
        .map_or(0, |i| 256 - i as u16);
    let trace = Trace {
        depth: lowest + 2,
        iff: (before.0, regs.iff1),
        speed: (before.1, run.m().in_port(0x20) & 1),
        action: (regs.af >> 8) as u8,
    };
    if mask {
        set_interrupts(run, 1);
    }
    run.m().out_port(0x20, 0);
    trace
}

/// The greatest depth over `calls`, asserting that every call kept interrupts as they were at the call
/// and returned with the speed port as it was set to `speed`.
pub fn kept(what: &str, calls: &[Trace], speed: u8, mask: bool) -> u16 {
    assert!(!calls.is_empty(), "{what}: no call was traced");
    for (n, t) in calls.iter().enumerate() {
        assert_eq!(
            t.iff.0,
            u8::from(!mask),
            "{what}, call {n}: the state at the call"
        );
        assert_eq!(
            t.iff.1, t.iff.0,
            "{what}, call {n}: interrupts at the return"
        );
        assert_eq!(t.speed, (speed, speed), "{what}, call {n}: the speed");
    }
    calls.iter().map(|t| t.depth).max().unwrap()
}

/// The depths of each routine, in bytes, at `speed`.
pub fn figures(speed: u8, mask: bool) -> BTreeMap<String, u16> {
    let built = with_library();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let (plain, _) = find(&lib, |f| f.hyps == 0 && !f.large);
    figures_of(
        &|n| {
            let mut walker = Walker::on_list(built, None, 0);
            walker.goto(n);
            walker
        },
        (n, plain),
        speed,
        mask,
    )
}

/// `figures` for a library: `before(n)` gives both on the list with the cursor on assertion `n`, `n`
/// being an assertion with several statements and `plain` one with none but the conclusion. The keys that
/// move the cursor are traced by path in theorem_paths.rs.
pub fn figures_of(
    before: &dyn Fn(usize) -> Walker,
    (n, plain): (usize, usize),
    speed: u8,
    mask: bool,
) -> BTreeMap<String, u16> {
    let at = |n: usize| {
        let mut walker = before(n);
        walker.open();
        walker
    };
    let mut found = BTreeMap::new();
    let mut put = |name: &str, calls: &[Trace]| {
        found.insert(name.to_string(), kept(name, calls, speed, mask));
    };
    let mut walker = before(n);
    walker.list.run.m().press_key(ENTER);
    let pushed = walker.trace("list_key", mask, speed);
    assert_eq!(pushed.action, 2, "the list's ENTER pushes");
    let entered = walker.trace("theorem_enter", mask, speed);
    assert_eq!(entered.action, 0, "the enter is built");
    walker.list.run.m().release_key(ENTER);
    walker.list.run.run(1_000_000);
    put("list_key enter", &[pushed]);
    put("enter on a push", &[entered]);
    walker.list.run.m().press_key(DOWN);
    let draw = walker.trace("theorem_draw", mask, speed);
    walker.list.run.m().release_key(DOWN);
    walker.list.run.run(1_000_000);
    put("draw", &[draw]);
    for (name, scan) in [
        ("f2", F2),
        ("f3", F3),
        ("f4", F4),
        ("f5", F5),
        ("clear", CLEAR),
        ("letter", LETTER),
    ] {
        let mut walker = at(n);
        for _ in 0..4 {
            walker.press(LEFT, Key::Left);
        }
        walker.press(DOWN, Key::Down);
        walker.list.run.m().press_key(scan);
        let t = walker.trace("theorem_key", mask, speed);
        walker.list.run.m().release_key(scan);
        put(&format!("key {name}"), &[t]);
    }
    let mut walker = at(n);
    walker.tap(F2);
    walker.list.run.run(300_000);
    walker.list.run.m().press_key(CLEAR);
    let t = walker.trace("theorem_enter", mask, speed);
    walker.list.run.m().release_key(CLEAR);
    put("enter on an uncovering", &[t]);
    let mut walker = at(n);
    for _ in 0..4 {
        walker.press(LEFT, Key::Left);
    }
    let mut guard = 0;
    while walker.frame()[5] == 0 {
        guard += 1;
        assert!(guard < 12, "the keys reach a node inside a statement");
        if !walker.press(DOWN, Key::Down) {
            walker.press(LEFT, Key::Left);
        }
    }
    assert_ne!(walker.frame()[5], 0, "the cursor is inside a statement");
    walker.tap(F2);
    walker.list.run.run(300_000);
    walker.list.run.m().press_key(CLEAR);
    walker.trace("theorem_enter", mask, speed);
    let t = walker.trace("theorem_draw", mask, speed);
    walker.list.run.m().release_key(CLEAR);
    put("draw with the cursor's rectangles", &[t]);
    let mut walker = at(plain);
    walker.list.run.m().press_key(DOWN);
    let t = walker.trace("theorem_key", mask, speed);
    walker.list.run.m().release_key(DOWN);
    put("key at the root of a short statement", &[t]);
    found
}

/// The stack each routine writes, counting the caller's return address, as the header states them; the
/// enter's is 24 on the stub OS and 34 on the reference OS, whose lookup of the status table is deeper.
pub fn depths(enter: u16, draw: u16) -> BTreeMap<String, u16> {
    [
        ("draw", draw),
        ("draw with the cursor's rectangles", draw),
        ("enter on a push", enter),
        ("enter on an uncovering", enter),
        ("key at the root of a short statement", 14),
        ("key clear", 2),
        ("key f2", 6),
        ("key f3", 6),
        ("key f4", 6),
        ("key f5", 4),
        ("key letter", 4),
    ]
    .map(|(name, depth)| (name.to_string(), depth))
    .into()
}

pub fn depths_on_the_reference_os() -> BTreeMap<String, u16> {
    depths(34, 18)
}

#[test]
fn the_routines_return_with_interrupts_and_speed_as_they_were_and_the_stack_balanced() {
    for speed in [0, 1] {
        figures(speed, false);
        let mut got = figures(speed, true);
        assert_eq!(
            got.remove("list_key enter"),
            Some(8),
            "speed {speed}: the list's push"
        );
        assert_eq!(got, depths(24, 18), "speed {speed}");
    }
}
