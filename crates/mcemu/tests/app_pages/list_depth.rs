//! The list's four routines traced one call at a time on the app's own path: the stack each writes,
//! counting the caller's return address, whether interrupts and the speed are as they were at the
//! call, and that the stack comes back balanced. Each call is traced twice, once with interrupts
//! masked for its length so that no handler's pushes are in the figure and once as the app runs.

use crate::app::*;
use crate::checkrig::{Run, WAIT};
use crate::list_rig::*;
use crate::shellrig::*;
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
    pub carry: bool,
}

fn set_interrupts(run: &mut Run, on: u8) {
    let mut regs = run.m().regs();
    regs.iff1 = on;
    regs.iff2 = on;
    run.m().set_regs(&regs);
}

impl Lister {
    /// Runs to the entry of the routine at `label`, runs it to its return and reports it. With
    /// `mask` interrupts are off for the call and on again after it. The machine is left at the
    /// instruction after the return.
    pub fn trace(&mut self, label: &str, mask: bool) -> Trace {
        let at = self.run.shell.app.label(label);
        self.run.run_to_addr(label, at, WAIT);
        self.trace_here(label, mask)
    }

    /// `trace` for a machine that is at the entry of `label` already.
    pub fn trace_here(&mut self, label: &str, mask: bool) -> Trace {
        let sp = self.run.m().regs().sp;
        let ret = {
            let b = self.run.m().read_ram(sp, 2);
            u16::from_le_bytes([b[0], b[1]])
        };
        if mask {
            set_interrupts(&mut self.run, 0);
        }
        let kept = self.run.m().read_ram(sp - 256, 256);
        let fill: Vec<u8> = (0..=255u8).map(|i| i.wrapping_mul(7) | 1).collect();
        self.run.m().load_ram(sp - 256, &fill);
        let before = (self.run.m().regs().iff1, self.run.m().in_port(0x20) & 1);
        let back = match label {
            "list_key" => self.run.shell.app.label("scr_key") + 5,
            "list_idle" => self.run.shell.app.label("scr_idle") + 5,
            _ => ret,
        };
        self.run
            .run_to_addr(&format!("the return of {label}"), back, WAIT);
        let regs = self.run.m().regs();
        assert_eq!(regs.sp, sp + 2, "{label}: the stack is balanced");
        let after = self.run.m().read_ram(sp - 256, 256);
        self.run.m().load_ram(sp - 256, &kept);
        let lowest = after
            .iter()
            .zip(&fill)
            .position(|(a, f)| a != f)
            .map_or(0, |i| 256 - i as u16);
        let trace = Trace {
            depth: lowest + 2,
            iff: (before.0, regs.iff1),
            speed: (before.1, self.run.m().in_port(0x20) & 1),
            action: (regs.af >> 8) as u8,
            carry: regs.af & 1 != 0,
        };
        if mask {
            set_interrupts(&mut self.run, 1);
        }
        trace
    }
}

/// The greatest depth over `calls`, asserting that every call kept interrupts, as they were at the
/// call or on when `on_exit`, and kept the speed.
fn kept(what: &str, calls: &[Trace], masked: bool, on_exit: bool) -> u16 {
    assert!(!calls.is_empty(), "{what}: no call was traced");
    for (n, t) in calls.iter().enumerate() {
        let want = if on_exit { 1 } else { t.iff.0 };
        assert_eq!(t.iff.1, want, "{what}, call {n}: interrupts at the return");
        assert_eq!(
            t.iff.0,
            u8::from(!masked),
            "{what}, call {n}: the state at the call"
        );
        assert_eq!(t.speed.0, t.speed.1, "{what}, call {n}: the speed");
    }
    calls.iter().map(|t| t.depth).max().unwrap()
}

/// The list of the fixture on stage 1, opened by ENTER whose enter is traced, settled, and every
/// key traced at the cursor of a list long enough for each move.
pub fn key_depths(mask: bool, rig: &Rig) -> BTreeMap<&'static str, u16> {
    let mut found = BTreeMap::new();
    let keys: [(&str, u8, Key); 12] = [
        ("up", UP, Key::Up),
        ("down", DOWN, Key::Down),
        ("left", LEFT, Key::Left),
        ("right", RIGHT, Key::Right),
        ("enter", ENTER, Key::Enter),
        ("f1", F1, Key::F1),
        ("f2", F2, Key::F2),
        ("f3", F3, Key::F3),
        ("f4", F4, Key::F4),
        ("f5", F5, Key::F5),
        ("clear", CLEAR, Key::Clear),
        ("letter", LETTER, Key::Letter('A')),
    ];
    for (name, scan, _) in keys {
        let mut list = (rig.open)();
        list.settle();
        list.key(RIGHT, Key::Right);
        list.settle();
        list.key(DOWN, Key::Down);
        list.settle();
        list.run.m().press_key(scan);
        let t = list.trace("list_key", mask);
        list.run.m().release_key(scan);
        found.insert(name, kept(&format!("list_key {name}"), &[t], mask, false));
    }
    found
}

impl Lister {
    /// Puts the open list on its last assertion by writing the cursor and the top into the frame as
    /// the model has them after paging to the end, and lets its preview settle.
    pub fn jump_to_the_end(&mut self) {
        while self.pos().cursor + 1 < self.pos().len {
            self.ui.press(Key::Right);
            self.log.push(Key::Right);
        }
        let pos = self.pos();
        let frame = self.run.shell.app.label("SCR_STACK") + 16;
        let (cursor, top) = (
            (pos.cursor as u16).to_le_bytes(),
            (pos.top as u16).to_le_bytes(),
        );
        self.run
            .m()
            .load_ram(frame + 2, &[cursor[0], cursor[1], top[0], top[1]]);
        let (count, preview) = (
            self.run.shell.app.label("LV_COUNT"),
            self.run.shell.app.label("LV_PREVIEW"),
        );
        self.run.m().load_ram(count, &[0]);
        self.run.m().load_ram(preview, &[PENDING]);
        self.tap(LETTER);
        self.settle();
    }
}

/// A move that keeps the cursor where it is, at each end, traced too.
pub fn end_depths(mask: bool, rig: &Rig) -> Vec<u16> {
    let mut all = Vec::new();
    for (scan, ends) in [(UP, true), (LEFT, true), (DOWN, false), (RIGHT, false)] {
        let mut list = (rig.open)();
        list.settle();
        if !ends {
            list.jump_to_the_end();
        }
        list.run.m().press_key(scan);
        let t = list.trace("list_key", mask);
        list.run.m().release_key(scan);
        assert_eq!(t.action, 0, "{scan:02X} at an end: SCR_STAY");
        all.push(t);
    }
    vec![kept("list_key at an end", &all, mask, false)]
}

pub fn enter_depths(mask: bool, rig: &Rig) -> u16 {
    let mut all = Vec::new();
    let mut list = (rig.before)();
    list.run.m().press_key(ENTER);
    all.push(list.trace("list_enter", mask));
    list.run.m().release_key(ENTER);
    list.run.run(1_000_000);
    list.tap(F2);
    list.run.run(300_000);
    list.run.m().press_key(CLEAR);
    all.push(list.trace("list_enter", mask));
    list.run.m().release_key(CLEAR);
    kept("list_enter", &all, mask, false)
}

/// Every kind of idle call: a tick, the fit, and a settled list, which the shell asks once after a
/// key that redraws nothing.
pub fn idle_depths(mask: bool, rig: &Rig) -> BTreeMap<&'static str, u16> {
    let mut list = (rig.open)();
    let mut classes: BTreeMap<&'static str, Vec<Trace>> = BTreeMap::new();
    let mut trace_one = |list: &mut Lister, classes: &mut BTreeMap<&'static str, Vec<Trace>>| {
        let at = list.run.shell.app.label("list_idle");
        list.run.run_to_addr("list_idle", at, WAIT);
        let (count, preview) = (list.var("LV_COUNT"), list.var("LV_PREVIEW"));
        let class = match (count, preview) {
            (c, PENDING) if c > 0 => "tick",
            (0, PENDING) => "fit",
            (0, _) => "settled",
            other => panic!("{other:?}"),
        };
        let t = list.trace_here("list_idle", mask);
        classes.entry(class).or_default().push(t);
        list.run.m().resume(1);
        class
    };
    while trace_one(&mut list, &mut classes) != "fit" {}
    list.run.run(300_000);
    list.run.m().press_key(LETTER);
    assert_eq!(trace_one(&mut list, &mut classes), "settled");
    list.run.m().release_key(LETTER);
    classes
        .into_iter()
        .map(|(c, calls)| {
            (
                c,
                kept(&format!("list_idle {c}"), &calls, mask, c == "tick"),
            )
        })
        .collect()
}

pub fn draw_depths(mask: bool, rig: &Rig) -> u16 {
    let mut list = (rig.before)();
    let mut all = Vec::new();
    for (scan, key) in [(ENTER, Key::Enter), (DOWN, Key::Down)] {
        list.run.m().press_key(scan);
        all.push(list.trace("list_draw", mask));
        list.run.m().release_key(scan);
        list.ui.press(key);
        all.push(list.trace("list_draw", mask));
        assert_eq!(
            list.var("LV_PREVIEW"),
            ROW,
            "the second draw shows the preview"
        );
        list.settle();
    }
    kept("list_draw", &all, mask, false)
}

/// How a test gets the list it traces: one just opened by ENTER, and the Libraries screen with the
/// cursor on the stage to open.
pub struct Rig {
    pub name: &'static str,
    pub open: fn() -> Lister,
    pub before: fn() -> Lister,
}

/// The greatest depth each routine's calls reached, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Figures {
    pub keys: BTreeMap<&'static str, u16>,
    pub ends: u16,
    pub enter: u16,
    pub idle: BTreeMap<&'static str, u16>,
    pub draw: u16,
}

/// The figures of one pass over every routine, with interrupts masked for each call or as the app
/// runs them.
pub fn figures(rig: &Rig, mask: bool) -> Figures {
    let figures = Figures {
        keys: key_depths(mask, rig),
        ends: end_depths(mask, rig)[0],
        enter: enter_depths(mask, rig),
        idle: idle_depths(mask, rig),
        draw: draw_depths(mask, rig),
    };
    println!(
        "{}, interrupts {}: {figures:?}",
        rig.name,
        if mask { "masked" } else { "as the app runs" }
    );
    assert_eq!(
        figures.idle.len(),
        3,
        "a tick, a fit and a settled call were all traced: {:?}",
        figures.idle
    );
    figures
}

/// Asserts the masked figures are `want` and that an interrupt that lands in a call only adds to them.
pub fn traced(rig: &Rig, want: &Figures) {
    let masked = figures(rig, true);
    assert_eq!(&masked, want, "{}: the depths", rig.name);
    let live = figures(rig, false);
    for (name, depth) in &live.keys {
        assert!(*depth >= masked.keys[name], "{name}");
    }
    for (name, depth) in &live.idle {
        assert!(*depth >= masked.idle[name], "{name}");
    }
    assert!(live.enter >= masked.enter && live.draw >= masked.draw && live.ends >= masked.ends);
}

/// The stub OS's figures: the stack each routine writes, the caller's return address counted.
pub fn stub_figures() -> Figures {
    Figures {
        keys: BTreeMap::from([
            ("clear", 2),
            ("down", 4),
            ("enter", 6),
            ("f1", 6),
            ("f2", 6),
            ("f3", 6),
            ("f4", 6),
            ("f5", 2),
            ("left", 4),
            ("letter", 2),
            ("right", 4),
            ("up", 4),
        ]),
        ends: 4,
        enter: 22,
        idle: BTreeMap::from([("fit", 8), ("settled", 2), ("tick", 8)]),
        draw: 26,
    }
}

/// The fixture's status table with every other assertion checked, and the model's marks for it, so
/// that every frame reads the table.
pub fn alternating_marks(built: &Built) -> (Vec<u8>, Vec<mcui::Status>) {
    let n = usize::from(crate::table::header_of(built.library.as_ref().unwrap()).1);
    let status = (0..n)
        .map(|i| {
            if i % 2 == 1 {
                mcui::Status::Checked
            } else {
                mcui::Status::Unchecked
            }
        })
        .collect();
    (crate::shell_cost::alternating(built), status)
}

#[test]
fn the_lists_routines_keep_interrupts_and_speed_and_the_stack_balanced_and_write_these_depths() {
    traced(
        &Rig {
            name: "stub",
            open: || {
                let built = with_library();
                let (data, status) = alternating_marks(built);
                Lister::open(built, Some((&data, &status)), 0)
            },
            before: || {
                let built = crate::builds::three_stages();
                let (data, status) = alternating_marks(built);
                Lister::at_stage(built, Some((&data, &status)), 1)
            },
        },
        &stub_figures(),
    );
}
