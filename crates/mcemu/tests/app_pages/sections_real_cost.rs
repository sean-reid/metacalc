//! What the sections screen costs on the reference OS at 6 MHz, ignored in the default run: a move with
//! its frame by title and by position, the draw broken down by call, opening the screen and the jump.

use crate::app::*;
use crate::list_real::RING;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::sections_rig::KIND_SECTIONS;
use crate::shellrig::KIND_LIST;
use crate::theorem_rig::lib_of;
use mcemu::{Reached, Regs};

pub const MOVE_BUDGET: u32 = 300_000;

/// The sections screen of a stage on the reference OS, with the machine at rest and its frame's address.
pub struct Probe {
    pub list: Lister,
    pub frame: u16,
    pub len: usize,
}

impl Probe {
    /// The list of `stage` opened at its first assertion and F5 pressed.
    pub fn open(stage: usize) -> Probe {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        let at = list.open_sections();
        list.run.shell.at_rest();
        let stack = list.run.shell.app.label("SCR_STACK");
        let frame = stack + 16 * (list.run.depth() as u16 - 1);
        Probe {
            list,
            frame,
            len: at.len,
        }
    }

    pub fn app(&mut self) -> &mut App {
        &mut self.list.run.shell.app
    }

    /// Writes the cursor and the first title shown into the frame.
    pub fn put(&mut self, cursor: usize, top: usize) {
        let (c, t) = ((cursor as u16).to_le_bytes(), (top as u16).to_le_bytes());
        let frame = self.frame;
        self.app().m.load_ram(frame + 2, &[c[0], c[1], t[0], t[1]]);
    }

    /// The cursor and the first title shown in the frame.
    pub fn at(&mut self) -> (usize, usize) {
        let frame = self.frame;
        let b = self.app().m.read_ram(frame + 2, 4);
        (
            usize::from(u16::from_le_bytes([b[0], b[1]])),
            usize::from(u16::from_le_bytes([b[2], b[3]])),
        )
    }

    /// sections_key for scancode-code `code`: its status and T-states.
    pub fn key(&mut self, code: u8) -> (u8, u32) {
        let frame = self.frame;
        let app = self.app();
        let key = app.label("sections_key");
        let done = app.timed(
            BASE,
            key,
            In {
                a: code,
                hl: frame,
                ..In::default()
            },
        );
        (done.status(), done.tstates)
    }

    /// scr_draw and ui_blit of the screen on top.
    pub fn frame_cost(&mut self) -> (u32, u32) {
        let app = self.app();
        let draw = app.label("scr_draw");
        let draw = app.timed(BASE, draw, In::default()).tstates;
        let blit = app
            .timed(BASE, app.label("far_ui_blit"), In::default())
            .tstates;
        (draw, blit)
    }

    /// The order the calls of one frame are made in: the name's cut and the title bar, then for each of
    /// `rows` rows the read, the glyph conversion, the cut and the row, then the keys.
    pub fn profile(&mut self, rows: usize) -> Vec<(String, u32)> {
        let mut order = vec!["far_ui_clear", "far_ui_truncate", "far_ui_title"];
        for _ in 0..rows {
            order.extend([
                "far_lib_title",
                "ll_glyph",
                "far_ui_truncate",
                "far_ui_list_row",
            ]);
        }
        order.push("far_ui_soft_keys");
        let app = self.app();
        let stop = app.label("screen_ret");
        let draw = app.label("scr_draw");
        let [lo, hi] = draw.to_le_bytes();
        let [slo, shi] = stop.to_le_bytes();
        app.m.load_ram(app.driver, &[0xCD, lo, hi, 0xC3, slo, shi]);
        app.m.out_port(0x06, BASE);
        app.m.set_regs(&Regs {
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            ix: 0x1111,
            iy: 0x89F0,
            sp: app.stack,
            pc: app.driver,
            af2: 0x2222,
            bc2: 0x3333,
            de2: 0x4444,
            hl2: 0x5555,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        let start = app.m.clock();
        let mut out = Vec::new();
        for name in order {
            let at = app.label(name);
            assert_eq!(app.m.run_to(BASE, at, 5_000_000), Reached::Target, "{name}");
            let (c0, sp) = (app.m.clock(), app.m.regs().sp);
            if name == "far_ui_soft_keys" {
                assert_eq!(app.m.run_to(BASE, stop, 5_000_000), Reached::Target);
                out.push((name.to_string(), app.m.clock().wrapping_sub(c0)));
                break;
            }
            let ret = app.m.read_ram(sp, 2);
            let ret = u16::from_le_bytes([ret[0], ret[1]]);
            assert_eq!(
                app.m.run_to(BASE, ret, 5_000_000),
                Reached::Target,
                "{name} returns"
            );
            out.push((name.to_string(), app.m.clock().wrapping_sub(c0)));
        }
        let total = app.m.clock().wrapping_sub(start);
        out.push(("total (with the driver)".into(), total));
        app.m.out_port(0x06, BASE);
        out
    }
}

/// The text of title `n` of `stage` as the row shows it: its characters, its width in pixels, and the
/// width after the indent and the cut at 48 characters.
pub fn row_of(built: &Built, n: usize) -> (usize, usize, u8) {
    let lib = lib_of(built);
    let t = &lib.titles()[n];
    let shown: String = t.text.chars().take(48).collect();
    (
        t.text.chars().count(),
        mcui::canvas::text_width(&shown),
        t.depth,
    )
}

const SCR_REDRAW: u8 = 1;
const UP_CODE: u8 = 1;
const DOWN_CODE: u8 = 2;
const LEFT_CODE: u8 = 3;
const RIGHT_CODE: u8 = 4;
const OPEN_BUDGET: u32 = 1_800_000;

/// Where a key leaves the cursor and the first title shown: one or six titles, clamped, the first title
/// moving as little as it must.
fn after(cursor: usize, top: usize, code: u8, len: usize) -> (usize, usize) {
    let c = match code {
        UP_CODE => cursor.saturating_sub(1),
        LEFT_CODE => cursor.saturating_sub(6),
        DOWN_CODE => (cursor + 1).min(len - 1),
        _ => (cursor + 6).min(len - 1),
    };
    let t = if c < top {
        c
    } else if c >= top + 6 {
        c - 5
    } else {
        top
    };
    (c, t)
}

/// Pinned costs: asserted within 1% above and 2% below, and printed as source lines when one is missing.
fn pinned(pins: &[(&str, u32)], measured: &[(String, u32)]) {
    let mut missing = Vec::new();
    for (name, got) in measured {
        println!("ROM {name}: {got} T-states");
        match pins.iter().find(|(n, _)| n == name) {
            Some(&(_, want)) => within(name, *got, want),
            None => missing.push(format!("    (\"{name}\", {got}),")),
        }
    }
    assert!(missing.is_empty(), "no pin for:\n{}", missing.join("\n"));
}

impl Probe {
    /// A move from `cursor` with the first title at `top`: the key was taken, the cursor and first title are
    /// the model's rule, one frame is blitted. Returns the key, the draw and the blit.
    pub fn moved(&mut self, cursor: usize, top: usize, code: u8) -> (u32, u32, u32) {
        self.put(cursor, top);
        let blits = self.list.run.blits();
        let (status, key) = self.key(code);
        assert_eq!(status, SCR_REDRAW, "the key was taken and moves");
        let want = after(cursor, top, code, self.len);
        assert_ne!(want.0, cursor, "the cursor moves");
        assert_eq!(self.at(), want, "the cursor and first title after");
        let (draw, blit) = self.frame_cost();
        assert_eq!(self.list.run.blits(), blits + 1, "one frame blitted");
        (key, draw, blit)
    }
}

/// A move with its frame at the first, middle and last title of each stage, in T-states, as measured.
const MOVE_PINS: &[(&str, u32)] = &[
    ("stage 0 first row on", 311020),
    ("stage 0 first page on", 308296),
    ("stage 0 middle row back", 309070),
    ("stage 0 middle page back", 298542),
    ("stage 0 middle row on", 310142),
    ("stage 0 middle page on", 314235),
    ("stage 0 last row back", 320809),
    ("stage 0 last page back", 314841),
    ("stage 1 first row on", 310746),
    ("stage 1 first page on", 308022),
    ("stage 1 middle row back", 330211),
    ("stage 1 middle page back", 323199),
    ("stage 1 middle row on", 330293),
    ("stage 1 middle page on", 322069),
    ("stage 1 last row back", 325374),
    ("stage 1 last page back", 327129),
    ("stage 2 first row on", 311497),
    ("stage 2 first page on", 308773),
    ("stage 2 middle row back", 324214),
    ("stage 2 middle page back", 312268),
    ("stage 2 middle row on", 324291),
    ("stage 2 middle page on", 322794),
    ("stage 2 last row back", 278812),
    ("stage 2 last page back", 292083),
];

/// Opening the screen from the stage's last assertion: scr_key (F5, the push and the enter), the first
/// draw and the blit.
const OPEN_PINS: &[(&str, u32)] = &[
    ("opening stage 0", 390849),
    ("opening stage 1", 395418),
    ("opening stage 2", 348868),
];

/// ENTER on the sections screen with the list's first frame, under each kind, from the first, middle and
/// last title of ring's stage.
const JUMP_PINS: &[(&str, u32)] = &[
    ("jump all first", 227636),
    ("jump all middle", 246638),
    ("jump all last", 251104),
    ("jump theorems first", 285213),
    ("jump theorems middle", 322760),
    ("jump theorems last", 330489),
    ("jump axioms first", 287677),
    ("jump axioms middle", 340073),
    ("jump axioms last", 331390),
    ("jump unchecked first", 583103),
    ("jump unchecked middle", 657648),
    ("jump unchecked last", 669504),
];

/// The slowest move of ring's stage: Down from the cursor four rows into a view whose first title is
/// the one that makes the six rows costliest, 337,773 T-states for the frame and 457 for the key. It is
/// over MOVE_BUDGET, which a move should fit, so it is pinned at what it measures.
const WORST_MOVE_OVER_THE_BUDGET: u32 = 338_230;
/// The first title of that view.
const WORST_MOVE_FIRST_TITLE: usize = 123;

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_move_with_its_frame_costs_this_at_the_first_middle_and_last_title_of_each_stage()
 {
    let mut measured = Vec::new();
    for stage in 0..3 {
        let mut p = Probe::open(stage);
        let n = p.len;
        for (place, cursor, codes) in [
            ("first", 0, [DOWN_CODE, RIGHT_CODE]),
            ("middle", n / 2, [UP_CODE, LEFT_CODE]),
            ("middle", n / 2, [DOWN_CODE, RIGHT_CODE]),
            ("last", n - 1, [UP_CODE, LEFT_CODE]),
        ] {
            let top = cursor.saturating_sub(2).min(n - 6);
            for code in codes {
                let (key, draw, blit) = p.moved(cursor, top, code);
                let by = if code == UP_CODE || code == DOWN_CODE {
                    "row"
                } else {
                    "page"
                };
                let dir = if code == UP_CODE || code == LEFT_CODE {
                    "back"
                } else {
                    "on"
                };
                println!(
                    "ROM stage {stage} {place} {by} {dir}: key {key}, draw {draw}, blit {blit}"
                );
                measured.push((
                    format!("stage {stage} {place} {by} {dir}"),
                    key + draw + blit,
                ));
            }
        }
    }
    pinned(MOVE_PINS, &measured);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_slowest_move_on_ring_is_over_the_budget_and_costs_this() {
    let mut p = Probe::open(RING);
    let n = p.len;
    let mut costs = Vec::new();
    for top in 0..=n - 6 {
        let (key, draw, blit) = p.moved(top + 4, top, DOWN_CODE);
        costs.push((key + draw + blit, top));
    }
    let (worst, at) = *costs.iter().max().unwrap();
    let best = costs.iter().map(|c| c.0).min().unwrap();
    println!(
        "ROM ring, a row move with the view's first title anywhere: {best} to {worst} (first title {at}) against {MOVE_BUDGET}"
    );
    assert_eq!(at, WORST_MOVE_FIRST_TITLE);
    assert!(worst > MOVE_BUDGET, "the slowest move is over the budget");
    within(
        "the slowest move on ring",
        worst,
        WORST_MOVE_OVER_THE_BUDGET,
    );
    let over = costs.iter().filter(|c| c.0 > MOVE_BUDGET).count();
    println!(
        "ROM ring: {over} of {} first titles are over the budget",
        costs.len()
    );
    let built = p.list.built;
    for i in at..at + 6 {
        let (chars, px, depth) = row_of(built, i);
        println!(
            "ROM worst frame, title {i}: depth {depth}, {chars} characters, {px} pixels of the first 48"
        );
    }
    p.put(at + 4, at);
    for (name, t) in p.profile(6) {
        println!("ROM profile {name}: {t}");
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_the_screen_from_the_last_assertion_of_each_stage_fits_its_budget() {
    let mut measured = Vec::new();
    for stage in 0..3 {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        list.jump_to_the_end();
        list.run.shell.at_rest();
        let frame = list.frame_bytes();
        let last = u16::from_le_bytes([frame[2], frame[3]]);
        assert_eq!(
            usize::from(last) + 1,
            list.built.stages[stage].end,
            "the cursor is on the stage's last assertion"
        );
        let app = &mut list.run.shell.app;
        let titles_for = app.label("far_lib_title_for");
        let push = app.timed(
            BASE,
            titles_for,
            In {
                hl: last,
                ..In::default()
            },
        );
        assert_eq!(push.status(), 0);
        let n = usize::from(push.regs.hl) + 1;
        let key = app.label("scr_key");
        let opened = app.timed(
            BASE,
            key,
            In {
                a: 11,
                ..In::default()
            },
        );
        assert_eq!(opened.status(), 1, "F5 pushed and the screen is entered");
        assert_eq!(list.run.depth(), 3);
        assert_eq!(list.run.top_kind(), KIND_SECTIONS);
        let mut p = Probe {
            list,
            frame: 0,
            len: n,
        };
        let blits = p.list.run.blits();
        let (draw, blit) = p.frame_cost();
        assert_eq!(p.list.run.blits(), blits + 1, "one frame blitted");
        let total = opened.tstates + draw + blit;
        println!(
            "ROM stage {stage}: {n} titles, lib_title_for of the push {} T-states, scr_key {}, draw {draw}, blit {blit}: {total} against {OPEN_BUDGET}",
            push.tstates, opened.tstates
        );
        assert!(total < OPEN_BUDGET);
        measured.push((format!("opening stage {stage}"), total));
    }
    pinned(OPEN_PINS, &measured);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_jump_with_the_lists_frame_fits_its_budget_under_each_kind() {
    use crate::sections_depth::{ROM, Table};
    let mut measured = Vec::new();
    for (name, kind, table) in [
        ("all", mcui::Kind::All, Table::None),
        ("theorems", mcui::Kind::Theorems, Table::None),
        ("axioms", mcui::Kind::Axioms, Table::None),
        ("unchecked", mcui::Kind::Unchecked, Table::Marks),
    ] {
        for place in ["first", "middle", "last"] {
            let mut list = (ROM.list)(kind, table);
            list.open_sections();
            let n = list.sections().len;
            let t = match place {
                "first" => 0,
                "middle" => n / 2,
                _ => n - 1,
            };
            list.goto_title(t);
            list.run.shell.at_rest();
            let app = &mut list.run.shell.app;
            let key = app.label("scr_key");
            let jumped = app.timed(
                BASE,
                key,
                In {
                    a: 5,
                    ..In::default()
                },
            );
            assert_eq!(jumped.status(), 1, "ENTER pops to the list");
            assert_eq!(list.run.depth(), 2);
            assert_eq!(list.run.top_kind(), KIND_LIST);
            let mut p = Probe {
                list,
                frame: 0,
                len: n,
            };
            let blits = p.list.run.blits();
            let (draw, blit) = p.frame_cost();
            assert_eq!(p.list.run.blits(), blits + 1, "one frame blitted");
            let total = jumped.tstates + draw + blit;
            println!(
                "ROM jump under {name} from the {place} title: scr_key {}, draw {draw}, blit {blit}: {total} against {OPEN_BUDGET}",
                jumped.tstates
            );
            assert!(total < OPEN_BUDGET);
            measured.push((format!("jump {name} {place}"), total));
        }
    }
    pinned(JUMP_PINS, &measured);
}
