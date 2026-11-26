//! The stack each of the list's routines writes under each kind, the interrupts and the speed they
//! return with: every key over a whole walk, F3 from two places, list_enter on the path that lands
//! the frame again and on the one that does not, list_draw blank and with a preview, and the idle
//! calls, on the stub OS and, ignored, on the reference OS.

use crate::list_depth::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind};
use std::collections::BTreeMap;

/// Where a test gets its lists: one opened under a kind and settled, with a table.
pub struct Kit {
    pub name: &'static str,
    pub open: fn(Kind) -> Lister,
    pub rig: fn(Kind) -> Rig,
    /// The most keys traced in one direction of the walk.
    pub cap: usize,
    /// The values of the speed port the figures are taken at.
    pub speeds: &'static [u8],
}

/// The depths of one kind: the arrows' greatest over the walk, the keys of a settled list at one place,
/// F3 from the first assertion and from a cursor with matches above it, list_enter plain and landing
/// the frame again, list_draw blank and with a preview, a key that moves nothing, the idle calls.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KindFigures {
    pub keys: BTreeMap<&'static str, u16>,
    pub f3: [u16; 2],
    pub enter: [u16; 3],
    pub draw: [u16; 2],
    pub ends: u16,
    pub idle: BTreeMap<&'static str, u16>,
}

fn sync(list: &mut Lister, key: Key, action: u8) {
    list.ui.press(key);
    list.log.push(key);
    if action == 1 {
        list.run.next_blit();
        list.run.run(2_000);
        list.skip_the_wait();
    }
    list.settle();
    list.run.run(600_000);
}

/// The arrows over the walk, each call traced: the greatest depth of each key, of the keys that moved
/// nothing, and how many calls were traced.
fn walk(kit: &Kit, kind: Kind, mask: bool, speed: u8) -> (BTreeMap<&'static str, u16>, u16, usize) {
    let mut list = (kit.open)(kind);
    let len = list.pos().len;
    let mut found: BTreeMap<&'static str, Vec<Trace>> = BTreeMap::new();
    let mut ends = Vec::new();
    let pages = len / 5 + 2;
    let plan: Vec<(&'static str, u8, Key, usize)> = vec![
        ("down", DOWN, Key::Down, len + 1),
        ("up", UP, Key::Up, len + 1),
        ("right", RIGHT, Key::Right, pages),
        ("left", LEFT, Key::Left, pages),
    ];
    let mut traced = 0;
    for (name, scan, key, times) in plan {
        for _ in 0..times.min(kit.cap) {
            list.run.m().press_key(scan);
            let t = list.trace("list_key", mask, speed);
            list.run.m().release_key(scan);
            found.entry(name).or_default().push(t);
            if t.action == 0 {
                ends.push(t);
            }
            traced += 1;
            sync(&mut list, key, t.action);
        }
    }
    let ends = if ends.is_empty() {
        0
    } else {
        kept("list_key at an end", &ends, mask, false, speed)
    };
    let keys = found
        .into_iter()
        .map(|(k, calls)| {
            (
                k,
                kept(&format!("list_key {k}"), &calls, mask, false, speed),
            )
        })
        .collect();
    (keys, ends, traced)
}

/// F3 traced with the cursor `place` moves of five from the first assertion the kind shows.
fn f3_at(kit: &Kit, kind: Kind, place: usize, mask: bool, speed: u8) -> u16 {
    let mut list = (kit.open)(kind);
    for _ in 0..place * 2 {
        list.key(RIGHT, Key::Right);
        list.skip_the_wait();
        list.settle();
    }
    list.run.m().press_key(F3);
    let t = list.trace("list_key", mask, speed);
    list.run.m().release_key(F3);
    assert_eq!(t.action, 1, "F3 redraws");
    kept("list_key F3", &[t], mask, false, speed)
}

/// How the frame list_enter is given stands against what the kind shows.
#[derive(Clone, Copy)]
enum Frame {
    /// As the key left it.
    Plain,
    /// A cursor and a first row the kind does not show: the enter lands them as F3 does.
    Land,
    /// Both shown, the cursor the `n`th shown assertion after the first row: for `n` from 5 on the cursor is off
    /// the screen and the enter gives the first row four shown assertions above it.
    Below(usize),
}

/// list_enter after a notice pops, with the frame as `how` says.
fn enter_at(kit: &Kit, kind: Kind, how: Frame, mask: bool, speed: u8) -> u16 {
    let mut list = (kit.open)(kind);
    list.tap(F4);
    list.run.run(300_000);
    let stack = list.run.shell.app.label("SCR_STACK") + 16;
    match how {
        Frame::Plain => {}
        Frame::Land => {
            let end = list.built.stages[usize::from(list.run.frames()[1][1])].end;
            let shown = list.items();
            let hidden = (0..end).find(|i| !shown.contains(i)).expect("a hidden one");
            let at = (hidden as u16).to_le_bytes();
            list.run
                .m()
                .load_ram(stack + 2, &[at[0], at[1], at[0], at[1]]);
        }
        Frame::Below(n) => {
            let shown = list.items();
            assert!(
                shown.len() > n,
                "{kind:?}: {} shown, cursor at {n}",
                shown.len()
            );
            let (cursor, top) = (
                (shown[n] as u16).to_le_bytes(),
                (shown[0] as u16).to_le_bytes(),
            );
            list.run
                .m()
                .load_ram(stack + 2, &[cursor[0], cursor[1], top[0], top[1]]);
        }
    }
    let before = list.run.frames()[1].clone();
    list.run.m().press_key(CLEAR);
    let t = list.trace("list_enter", mask, speed);
    list.run.m().release_key(CLEAR);
    let after = list.run.frames()[1].clone();
    assert_eq!(
        after != before,
        !matches!(how, Frame::Plain),
        "{kind:?}: the enter changes the frame: {before:?} {after:?}"
    );
    if let Frame::Below(n) = how {
        let shown = list.items();
        let top = usize::from(u16::from_le_bytes([after[4], after[5]]));
        assert_eq!(
            top,
            shown[n - 4],
            "{kind:?}: the first row four shown above the cursor"
        );
    }
    kept("list_enter", &[t], mask, false, speed)
}

/// list_enter given a cursor off the screen, the fifth shown after the first row and the last shown.
fn enter_below(kit: &Kit, kind: Kind, mask: bool, speed: u8) -> u16 {
    let shown = (kit.open)(kind).items().len();
    assert!(shown > 6, "{kind:?}: {shown} shown");
    enter_at(kit, kind, Frame::Below(5), mask, speed).max(enter_at(
        kit,
        kind,
        Frame::Below(shown - 1),
        mask,
        speed,
    ))
}

/// list_draw after a move, with the preview blank, and again when the idle call has laid the preview out.
fn draw_at(kit: &Kit, kind: Kind, mask: bool, speed: u8) -> [u16; 2] {
    let mut list = (kit.open)(kind);
    list.run.m().press_key(DOWN);
    let blank = list.trace("list_draw", mask, speed);
    list.run.m().release_key(DOWN);
    assert_eq!(list.var("LV_PREVIEW"), PENDING);
    list.ui.press(Key::Down);
    list.log.push(Key::Down);
    list.skip_the_wait();
    let shown = list.trace("list_draw", mask, speed);
    assert_eq!(
        list.var("LV_PREVIEW"),
        ROW,
        "the second draw shows the preview"
    );
    [
        kept("list_draw blank", &[blank], mask, false, speed),
        kept("list_draw preview", &[shown], mask, false, speed),
    ]
}

pub fn figures_of(kit: &Kit, kind: Kind, mask: bool, speed: u8) -> KindFigures {
    let rig = (kit.rig)(kind);
    let mut keys = key_depths(mask, speed, &rig);
    let (arrows, ends, traced) = walk(kit, kind, mask, speed);
    assert!(traced > 8, "{traced} calls");
    keys.extend(arrows);
    let f3 = [
        f3_at(kit, kind, 0, mask, speed),
        f3_at(kit, kind, 1, mask, speed),
    ];
    keys.insert("f3", f3[0].max(f3[1]));
    let figures = KindFigures {
        keys,
        f3,
        enter: [
            enter_at(kit, kind, Frame::Plain, mask, speed),
            enter_at(kit, kind, Frame::Land, mask, speed),
            enter_below(kit, kind, mask, speed),
        ],
        draw: draw_at(kit, kind, mask, speed),
        ends,
        idle: idle_depths(mask, speed, &rig),
    };
    println!(
        "{} {kind:?}, interrupts {}, speed port {speed}: {figures:?}",
        kit.name,
        if mask { "masked" } else { "as the app runs" }
    );
    figures
}

/// The figures masked, at both speeds, are `want`, and an interrupt that lands in a call only adds.
pub fn check(kit: &Kit, kind: Kind, want: &KindFigures) {
    for &speed in kit.speeds {
        let masked = figures_of(kit, kind, true, speed);
        assert_eq!(
            &masked, want,
            "{} {kind:?}: the depths at speed {speed}",
            kit.name
        );
        let live = figures_of(kit, kind, false, speed);
        for (name, depth) in &live.keys {
            assert!(*depth >= masked.keys[name], "{kind:?} {name}");
        }
        for (name, depth) in &live.idle {
            assert!(*depth >= masked.idle[name], "{kind:?} idle {name}");
        }
        assert!((0..3).all(|i| live.enter[i] >= masked.enter[i]));
        assert!((0..2).all(|i| live.draw[i] >= masked.draw[i]));
        assert!(
            live.ends >= masked.ends && live.f3[0] >= masked.f3[0] && live.f3[1] >= masked.f3[1]
        );
    }
}

fn open_with(kind: Kind) -> Lister {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut list = Lister::open(built, Some((&data, &status)), 2);
    list.settle();
    list.go_to_kind(kind);
    list
}

fn open_down(kind: Kind) -> Lister {
    let mut list = open_with(kind);
    list.key(DOWN, Key::Down);
    list
}

fn before() -> Lister {
    Lister::at_stage(proofs_three(), None, 2)
}

fn stub_rig(kind: Kind) -> Rig {
    let open: fn() -> Lister = match kind {
        Kind::Theorems => || open_down(Kind::Theorems),
        Kind::Axioms => || open_down(Kind::Axioms),
        _ => || open_down(Kind::Unchecked),
    };
    Rig {
        name: "kind",
        open,
        before,
    }
}

pub const STUB: Kit = Kit {
    name: "stub",
    open: open_with,
    rig: stub_rig,
    cap: usize::MAX,
    speeds: &[0, 1],
};

pub fn fig(
    keys: [(&'static str, u16); 12],
    f3: [u16; 2],
    enter: [u16; 3],
    draw: [u16; 2],
    ends: u16,
    idle: [u16; 3],
) -> KindFigures {
    KindFigures {
        keys: keys.into_iter().collect(),
        f3,
        enter,
        draw,
        ends,
        idle: BTreeMap::from([("fit", idle[0]), ("settled", idle[1]), ("tick", idle[2])]),
    }
}

fn stub_want(kind: Kind) -> KindFigures {
    // (down, up, left, right, f3, enter landing, enter below the view, ends, idle fit)
    let (down, up, left, right, f3, land, below, ends, fit) = match kind {
        Kind::Theorems => (16, 16, 16, 20, 24, 24, 22, 16, 16),
        Kind::Axioms => (16, 16, 16, 20, 28, 24, 22, 16, 16),
        _ => (22, 20, 20, 24, 22, 28, 26, 22, 22),
    };
    let plain = if kind == Kind::Unchecked { 24 } else { 22 };
    fig(
        [
            ("clear", 2),
            ("down", down),
            ("enter", down),
            ("f1", down),
            ("f2", 6),
            ("f3", f3),
            ("f4", 6),
            ("f5", 2),
            ("left", left),
            ("letter", 2),
            ("right", right),
            ("up", up),
        ],
        [f3, f3],
        [plain, land, below],
        [26, 26],
        ends,
        [fit, 2, 8],
    )
}

#[test]
fn the_keys_enter_draw_and_idle_calls_under_theorems_write_these_depths() {
    check(&STUB, Kind::Theorems, &stub_want(Kind::Theorems));
}

#[test]
fn the_keys_enter_draw_and_idle_calls_under_axioms_write_these_depths() {
    check(&STUB, Kind::Axioms, &stub_want(Kind::Axioms));
}

#[test]
fn the_keys_enter_draw_and_idle_calls_under_unchecked_write_these_depths() {
    check(&STUB, Kind::Unchecked, &stub_want(Kind::Unchecked));
}

#[test]
fn f3_under_all_writes_the_depth_of_the_cursors_place_not_the_os() {
    let mut got = Vec::new();
    for place in [0, 1] {
        let mut l = Lister::open(proofs_three(), None, 2);
        l.settle();
        for _ in 0..place * 2 {
            l.key(RIGHT, Key::Right);
            l.skip_the_wait();
            l.settle();
        }
        l.run.m().press_key(F3);
        let t = l.trace("list_key", true, 0);
        l.run.m().release_key(F3);
        got.push(t.depth);
    }
    println!("F3 under All on the stub: {got:?}");
    assert_eq!(got, [24, 24]);
}
