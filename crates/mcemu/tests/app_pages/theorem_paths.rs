//! The stack theorem_key and theorem_draw write on each path the header names, on states built for the
//! path and asserted from the model first: the same states on the stub OS and on the reference OS.

use crate::app::*;
use crate::list_rig::Lister;
use crate::shellrig::*;
use crate::theorem_depth::{Trace, kept};
use crate::theorem_keys::{onto_a_hidden_node, scroll};
use crate::theorem_return::next_node;
use crate::theorem_rig::*;
use crate::theorem_stay::DEL;
use crate::ui_support::same_frame;
use mcui::data::Lib;
use mcui::{Key, State};
use std::collections::BTreeMap;

/// Where the cursor is, from the model and the mirror of its folds.
pub struct Probe {
    pub s: usize,
    pub node: usize,
    /// Under a fold, so not drawn.
    pub hidden: bool,
    /// The cursor's own fold is set.
    pub folded: bool,
    pub nodes: usize,
    pub children: usize,
    pub sibling_before: bool,
    pub sibling_after: bool,
}

pub fn probe(walker: &mut Walker, lib: &Lib, n: usize) -> Probe {
    let State::Theorem {
        statement: s, node, ..
    } = walker.list.ui.state()
    else {
        panic!("the model is not on a theorem");
    };
    let statement = lib.statement(n);
    let expr = statement.at(s);
    let mirror = walker.mirror();
    Probe {
        s,
        node,
        hidden: mirror.hides(s, node),
        folded: mirror.is_folded(s, node),
        nodes: expr.len(),
        children: expr.children(node).len(),
        sibling_before: expr.sibling(node, false).is_some(),
        sibling_after: expr.sibling(node, true).is_some(),
    }
}

/// A key traced, then let finish, with the model told: the frame the app draws is the model's.
pub fn traced(walker: &mut Walker, scan: u8, key: Key, speed: u8) -> Trace {
    walker.list.run.m().press_key(scan);
    let t = walker.trace("theorem_key", true, speed);
    walker.list.run.m().release_key(scan);
    walker.list.run.run(2_000_000);
    walker.list.ui.press(key);
    walker.list.log.push(key);
    same_frame(
        &format!("after the traced {scan:02X}"),
        &walker.list.run.frame(),
        &walker.list.shown(),
    );
    t
}

/// Takes the cursor through the nodes, statement by statement toward the first, to a node `want` holds
/// of, and returns what the probe there says.
pub fn onto(walker: &mut Walker, lib: &Lib, n: usize, want: &dyn Fn(&Probe) -> bool) -> Probe {
    for _ in 0..800 {
        let p = probe(walker, lib, n);
        if want(&p) {
            return p;
        }
        if !next_node(walker) {
            assert!(
                walker.press(LEFT, Key::Left),
                "a statement is left to go to"
            );
        }
    }
    panic!("the cursor never reached a node of the kind wanted");
}

/// Where the cursor must be for a path.
type Want = fn(&Probe) -> bool;

/// What a path of the cursor needs: where the cursor is, and the key pressed there.
const MOVES: [(&str, u8, Key, Want); 13] = [
    ("left to a sibling", LEFT, Key::Left, |p| {
        !p.hidden && p.sibling_before
    }),
    ("right to a sibling", RIGHT, Key::Right, |p| {
        !p.hidden && p.sibling_after
    }),
    ("up to the parent", UP, Key::Up, |p| {
        !p.hidden && p.node != 0
    }),
    ("down to the first child", DOWN, Key::Down, |p| {
        !p.hidden && !p.folded && p.children > 0
    }),
    ("down at a leaf", DOWN, Key::Down, |p| {
        !p.hidden && p.children == 0 && p.nodes > 1
    }),
    ("down at a folded node", DOWN, Key::Down, |p| {
        !p.hidden && p.folded && p.nodes > 1 && p.node != 0
    }),
    (
        "up at the root of a statement of several nodes",
        UP,
        Key::Up,
        |p| p.node == 0 && p.nodes > 1,
    ),
    ("enter on a leaf", ENTER, Key::Enter, |p| {
        !p.hidden && p.children == 0 && p.nodes > 1
    }),
    (
        "enter on a node that is not folded",
        ENTER,
        Key::Enter,
        |p| !p.hidden && !p.folded && p.children > 0 && p.node != 0,
    ),
    ("enter on a folded node", ENTER, Key::Enter, |p| {
        !p.hidden && p.folded
    }),
    ("f1 on a node that is not folded", F1, Key::F1, |p| {
        !p.hidden && !p.folded && p.children > 0 && p.node != 0
    }),
    ("f1 on a folded node", F1, Key::F1, |p| {
        !p.hidden && p.folded
    }),
    ("f1 on a leaf", F1, Key::F1, |p| {
        !p.hidden && p.children == 0 && p.nodes > 1
    }),
];

/// What a path is built on: the library, a way to its list with the cursor on assertion `n` opened, an
/// assertion of several statements, one with a hypothesis of one node and rows to scroll to, and one
/// whose first statements lie above the window.
pub struct Site<'a> {
    pub lib: &'a Lib,
    pub open: &'a dyn Fn(usize) -> Walker,
    pub many: usize,
    pub scroll: usize,
    /// An assertion whose rows put a statement above the window when it opens.
    pub skip: usize,
}

/// The depth of each named path of the keys that move the cursor, the greatest over the traced calls,
/// which are asserted to keep interrupts and the speed.
pub fn moves(site: &Site, speed: u8) -> BTreeMap<String, u16> {
    let lib = site.lib;
    let mut found = BTreeMap::new();
    let mut put = |name: &str, t: Trace| {
        let depth = kept(name, &[t], speed, true);
        println!("path {name}: {depth}");
        assert!(found.insert(name.to_string(), depth).is_none(), "{name}");
    };
    for (name, scan, key, want) in MOVES {
        let mut walker = (site.open)(site.many);
        let p = onto(&mut walker, lib, site.many, &want);
        assert!(want(&p), "{name}: the state");
        put(name, traced(&mut walker, scan, key, speed));
    }
    let hyps = lib.statement(site.many).hyps.len();
    let mut walker = (site.open)(site.many);
    assert_eq!(probe(&mut walker, lib, site.many).s, hyps);
    put(
        "left across statements",
        traced(&mut walker, LEFT, Key::Left, speed),
    );
    assert_eq!(probe(&mut walker, lib, site.many).s, hyps - 1);
    put(
        "right across statements",
        traced(&mut walker, RIGHT, Key::Right, speed),
    );
    assert_eq!(probe(&mut walker, lib, site.many).s, hyps);
    put(
        "right at the last statement",
        traced(&mut walker, RIGHT, Key::Right, speed),
    );
    for _ in 0..hyps {
        assert!(walker.press(LEFT, Key::Left));
    }
    let p = probe(&mut walker, lib, site.many);
    assert_eq!((p.s, p.node), (0, 0));
    put(
        "left at the first statement",
        traced(&mut walker, LEFT, Key::Left, speed),
    );
    for (name, scan, key) in [
        ("left", LEFT, Key::Left),
        ("right", RIGHT, Key::Right),
        ("up", UP, Key::Up),
        ("down", DOWN, Key::Down),
        ("enter", ENTER, Key::Enter),
        ("f1", F1, Key::F1),
    ] {
        let mut walker = (site.open)(site.many);
        onto_a_hidden_node(&mut walker, 0);
        let p = probe(&mut walker, lib, site.many);
        assert!(p.hidden, "{name}: the cursor is on a hidden node");
        put(
            &format!("{name} on a hidden node"),
            traced(&mut walker, scan, key, speed),
        );
    }
    found
}

/// The scroll case: a statement of one node, Down and Up with a row to scroll to and with none, with the
/// cursor inside the window (byte 8 at 1) and outside it (byte 8 at 0), and the keys that do nothing
/// with byte 8 at 0. Returns the figures of the states the assertion has, and the names of those it
/// has no statement for.
pub fn scrolls(site: &Site, speed: u8) -> (BTreeMap<String, u16>, Vec<String>) {
    let (mut found, mut missing) = (BTreeMap::new(), Vec::new());
    let lib = site.lib;
    let n = site.scroll;
    let statement = lib.statement(n);
    let hyps = statement.hyps.len();
    let ones: Vec<usize> = (0..=hyps).filter(|&s| statement.at(s).len() == 1).collect();
    assert!(!ones.is_empty(), "a statement of one node");
    let (starts, total) = (site.open)(n).mirror().starts();
    assert!(total > 6, "rows to scroll to");
    let bottom = total - 6;
    let reach = |s: usize, scroll_to: usize| -> Walker {
        let mut walker = (site.open)(n);
        for _ in 0..hyps - s {
            assert!(walker.press(LEFT, Key::Left));
        }
        let p = probe(&mut walker, lib, n);
        assert_eq!((p.s, p.node, p.nodes), (s, 0, 1));
        while scroll(&walker) > scroll_to {
            assert!(walker.press(UP, Key::Up));
        }
        while scroll(&walker) < scroll_to {
            assert!(walker.press(DOWN, Key::Down));
        }
        let inside = starts[s] >= scroll_to && starts[s] < scroll_to + 6;
        assert_eq!(
            walker.frame()[8],
            u8::from(inside),
            "byte 8 at statement {s}, scroll {scroll_to}"
        );
        walker
    };
    let mut put = |name: String, t: Trace| {
        let depth = kept(&name, &[t], speed, true);
        println!("path {name}: {depth}");
        assert!(found.insert(name, depth).is_none());
    };
    let first = ones[0];
    let mut walker = reach(first, 0);
    put(
        "down in the scroll case with a row to scroll to".into(),
        traced(&mut walker, DOWN, Key::Down, speed),
    );
    assert_eq!(scroll(&walker), 1, "Down scrolled one row");
    let mut walker = reach(first, bottom);
    put(
        "up in the scroll case with a row to scroll to".into(),
        traced(&mut walker, UP, Key::Up, speed),
    );
    assert_eq!(scroll(&walker), bottom - 1, "Up scrolled one row");
    for (name, scan, key, scroll_to) in [("down", DOWN, Key::Down, bottom), ("up", UP, Key::Up, 0)]
    {
        for inside in [true, false] {
            let label = format!(
                "{name} in the scroll case with no row to scroll to, byte 8 at {}",
                u8::from(inside)
            );
            let at = ones
                .iter()
                .copied()
                .find(|&s| (starts[s] >= scroll_to && starts[s] < scroll_to + 6) == inside);
            let Some(s) = at else {
                missing.push(label);
                continue;
            };
            let mut walker = reach(s, scroll_to);
            put(label, traced(&mut walker, scan, key, speed));
            assert_eq!(scroll(&walker), scroll_to, "no row was left to scroll to");
        }
    }
    for (scan, key, name) in [
        (F5, Key::F5, "f5 with the view scrolled away"),
        (
            LETTER,
            Key::Letter('A'),
            "a letter with the view scrolled away",
        ),
        (DEL, Key::Letter('A'), "no key with the view scrolled away"),
    ] {
        let Some(&s) = ones.iter().find(|&&s| starts[s] >= 6) else {
            missing.push(name.to_string());
            continue;
        };
        let mut walker = reach(s, 0);
        assert_eq!(walker.frame()[8], 0, "{name}: byte 8 is 0");
        put(name.to_string(), traced(&mut walker, scan, key, speed));
    }
    (found, missing)
}

/// The depth of theorem_draw for a frame where `visible` of the statements have a row in the window, which
/// is asserted to be fewer than the statements, from the frame the key left and the mirror's rows.
pub fn draw_with_a_skipped_statement(site: &Site, speed: u8) -> u16 {
    let mut walker = (site.open)(site.skip);
    let hyps = site.lib.statement(site.skip).hyps.len();
    walker.list.run.m().press_key(DOWN);
    let run = &mut walker.list.run;
    let at = run.shell.app.label("theorem_draw");
    run.run_to_addr("theorem_draw", at, crate::checkrig::WAIT);
    let frame = walker.frame();
    let scroll = usize::from(u16::from_le_bytes([frame[6], frame[7]]));
    let (starts, total) = walker.mirror().starts();
    let visible = (0..=hyps)
        .filter(|&s| {
            let end = starts.get(s + 1).copied().unwrap_or(total);
            starts[s] < scroll + 6 && end > scroll
        })
        .count();
    println!(
        "draw: {visible} of {} statements have a row in the window",
        hyps + 1
    );
    assert!(visible < hyps + 1, "a statement is skipped");
    assert!(visible >= 1, "a statement is drawn");
    let t = crate::theorem_depth::trace_here(&mut walker.list.run, "theorem_draw", true, speed);
    walker.list.run.m().release_key(DOWN);
    kept("the draw with a skipped statement", &[t], speed, true)
}

/// The stack, in bytes, each path writes with the caller's return address counted, as the header gives
/// them: the same on both OSes, because theorem_key's calls do not depend on the OS.
pub fn figures() -> BTreeMap<String, u16> {
    [
        ("left to a sibling", 14),
        ("right to a sibling", 14),
        ("left across statements", 12),
        ("right across statements", 12),
        ("left at the first statement", 10),
        ("right at the last statement", 10),
        ("up to the parent", 12),
        ("up at the root of a statement of several nodes", 10),
        ("down to the first child", 14),
        ("down at a leaf", 10),
        ("down at a folded node", 10),
        ("enter on a leaf", 10),
        ("enter on a node that is not folded", 14),
        ("enter on a folded node", 14),
        ("f1 on a leaf", 10),
        ("f1 on a node that is not folded", 14),
        ("f1 on a folded node", 14),
        ("left on a hidden node", 10),
        ("right on a hidden node", 14),
        ("up on a hidden node", 14),
        ("down on a hidden node", 10),
        ("enter on a hidden node", 10),
        ("f1 on a hidden node", 10),
        ("down in the scroll case with a row to scroll to", 12),
        ("up in the scroll case with a row to scroll to", 12),
        (
            "down in the scroll case with no row to scroll to, byte 8 at 1",
            10,
        ),
        (
            "down in the scroll case with no row to scroll to, byte 8 at 0",
            12,
        ),
        (
            "up in the scroll case with no row to scroll to, byte 8 at 1",
            10,
        ),
        (
            "up in the scroll case with no row to scroll to, byte 8 at 0",
            12,
        ),
        ("f5 with the view scrolled away", 12),
        ("a letter with the view scrolled away", 12),
        ("no key with the view scrolled away", 12),
    ]
    .map(|(name, depth): (&str, u16)| (name.to_string(), depth))
    .into()
}

pub const DRAW: u16 = 18;

#[test]
fn every_path_of_the_key_and_the_draw_writes_the_headers_figure_on_the_stub_os() {
    let built = with_library();
    let lib = lib_of(built);
    let (many, _) = crate::theorem_frames::find(&lib, |f| f.hyps >= 6 && f.rows > 6 && !f.large);
    let open = |n: usize| Walker::at(built, None, 0, n);
    let site = Site {
        lib: &lib,
        open: &open,
        many,
        scroll: many,
        skip: many,
    };
    let rows =
        crate::theorem_keys::built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    let rows_lib = lib_of(rows);
    let rows8 = crate::theorem_keys::index_of(rows, "rows8");
    let open_rows = |n: usize| Walker::at(rows, None, 0, n);
    let in_rows = Site {
        lib: &rows_lib,
        open: &open_rows,
        many: rows8,
        scroll: rows8,
        skip: rows8,
    };
    for speed in [0, 1] {
        let mut got = moves(&site, speed);
        let (more, missing) = scrolls(&in_rows, speed);
        got.extend(more);
        assert_eq!(
            missing,
            Vec::<String>::new(),
            "speed {speed}: every state is built"
        );
        assert_eq!(got, figures(), "speed {speed}");
        assert_eq!(draw_with_a_skipped_statement(&site, speed), DRAW);
    }
}

/// The paths on the reference OS: the keys that move the cursor on an assertion of prop, the scroll case on
/// the assertions of one-node statements the stub OS's test builds (no shipped library has a one-node
/// hypothesis in an assertion of more than six rows), and the draw on prop's one assertion of more than six
/// rows.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_path_writes_the_figure_the_stub_os_does() {
    use crate::list_real::PROP;
    use crate::theorem_frames::find;
    use crate::theorem_keys::{built_of, index_of};
    let built = shipped_staged();
    let lib = lib_of(built);
    let (many, _) = find(&lib, |f| f.hyps >= 3 && f.rows >= 6 && !f.large);
    let (skip, _) = find(&lib, |f| f.hyps >= 3 && f.rows > 6 && !f.large);
    assert!(
        many < 779 && skip < 779,
        "both are in prop, which is stage 0"
    );
    let rows = built_of("theorem-rows", &[("rows6", 5), ("rows7", 6), ("rows8", 7)]);
    let rows_lib = lib_of(rows);
    let rows8 = index_of(rows, "rows8");
    let opener = |built: &'static Built| {
        move |n: usize| {
            let mut list = Lister::on_rom_of(built, 0);
            list.open_on_rom();
            let mut walker = Walker::new(list, PROP, built);
            walker.gap = 600_000;
            walker.goto(n);
            walker.open();
            walker
        }
    };
    let (in_prop, in_rows) = (opener(built), opener(rows));
    let prop = Site {
        lib: &lib,
        open: &in_prop,
        many,
        scroll: many,
        skip,
    };
    let tiny = Site {
        lib: &rows_lib,
        open: &in_rows,
        many: rows8,
        scroll: rows8,
        skip: rows8,
    };
    let mut got = moves(&prop, 0);
    let (more, missing) = scrolls(&tiny, 0);
    assert_eq!(missing, Vec::<String>::new(), "every state is built");
    got.extend(more);
    assert_eq!(got, figures(), "every path, on the reference OS");
    assert_eq!(draw_with_a_skipped_statement(&prop, 0), DRAW);
    println!(
        "ROM: {} paths and the draw have the stub's figures",
        got.len()
    );
}
