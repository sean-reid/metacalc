//! The list's four routines traced one call at a time on the app's own path: the stack each writes,
//! counting the caller's return address, whether interrupts and the speed are as they were at the
//! call, and that the stack comes back balanced. Each call is traced twice, once with interrupts
//! masked for its length so that no handler's pushes are in the figure and once as the app runs.

use crate::app::*;
use crate::checkrig::{Run, WAIT};
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Status};
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
pub fn key_depths(mask: bool) -> BTreeMap<&'static str, u16> {
    let mut found = BTreeMap::new();
    let built = with_library();
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
        let mut list = Lister::open(built, None, 0);
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

/// A move that keeps the cursor where it is, at each end, traced too.
pub fn end_depths(mask: bool) -> Vec<u16> {
    let built = with_library();
    let mut all = Vec::new();
    for (scan, ends) in [(UP, true), (LEFT, true), (DOWN, false), (RIGHT, false)] {
        let mut list = Lister::open(built, None, 0);
        list.settle();
        if !ends {
            for _ in 0..8 {
                list.key(RIGHT, Key::Right);
                list.settle();
            }
        }
        list.run.m().press_key(scan);
        let t = list.trace("list_key", mask);
        list.run.m().release_key(scan);
        assert_eq!(t.action, 0, "{scan:02X} at an end: SCR_STAY");
        all.push(t);
    }
    vec![kept("list_key at an end", &all, mask, false)]
}

pub fn enter_depths(mask: bool) -> u16 {
    let built = crate::builds::three_stages();
    let mut all = Vec::new();
    let mut list = Lister::at_stage(built, None, 1);
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
pub fn idle_depths(mask: bool) -> BTreeMap<&'static str, u16> {
    let built = with_library();
    let mut list = Lister::open(built, None, 0);
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

pub fn draw_depths(mask: bool, built: &'static Built, table: Option<(&[u8], &[Status])>) -> u16 {
    let mut list = Lister::at_stage(built, table, 0);
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

#[test]
fn the_lists_routines_keep_interrupts_and_speed_and_the_stack_balanced_and_write_these_depths() {
    for mask in [true, false] {
        let keys = key_depths(mask);
        let ends = end_depths(mask);
        let enter = enter_depths(mask);
        let idle = idle_depths(mask);
        let draw = draw_depths(mask, with_library(), None);
        println!(
            "stub, interrupts {}: list_key {keys:?} at the ends {ends:?}, list_enter {enter}, list_idle {idle:?}, list_draw {draw}",
            if mask { "masked" } else { "as the app runs" }
        );
        assert_eq!(
            idle.len(),
            3,
            "a tick, a fit and a settled call were all traced: {idle:?}"
        );
    }
}
