//! The Dist screen on the reference OS, ignored in the default run: what opening it costs on ring's
//! largest set of pairs, what a scroll costs on a library packed for it, what the way back to the
//! theorem screen costs, and the frames through the real shell.

use crate::app::*;
use crate::dist_lib::{self, SCROLL};
use crate::dist_rig::*;
use crate::list_real::RING;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_real::{Rom, measure, timed_frame_split};
use crate::theorem_rig::*;
use mcui::Key;

/// The budgets at 6 MHz: opening in 300 ms and a scroll with its frame in 50 ms.
pub const OPEN_BUDGET: u32 = 1_800_000;
pub const SCROLL_BUDGET: u32 = 300_000;

const KEY_UP: u8 = 1;
const KEY_DOWN: u8 = 2;
const KEY_CLEAR: u8 = 6;
const KEY_F2: u8 = 8;

/// What opening or scrolling cost, in T-states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    pub key: u32,
    pub enter: u32,
    pub draw: u32,
    pub blit: u32,
}

impl Cost {
    pub fn total(&self) -> u32 {
        self.key + self.enter + self.draw + self.blit
    }
}

/// What a scroll costs by part: the key, the walk to the first row shown, the six rows, the title, the
/// dist_open, the rest of the draw (the clear, the strings and the keys) and the blit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Split {
    pub key: u32,
    pub to_first_row: u32,
    pub six_rows: u32,
    pub title: u32,
    pub open: u32,
    pub rest: u32,
    pub blit: u32,
}

/// The Dist screen on the reference OS: the theorem screen of an assertion opened by `Rom`, then F2.
pub struct DistRom {
    pub rom: Rom,
    lib: mcui::data::Lib,
    assertion: usize,
}

fn call(rom: &mut Rom, label: &str, input: In) -> Done {
    let app = &mut rom.list.run.shell.app;
    let at = app.label(label);
    measure(app, at, input)
}

impl DistRom {
    pub fn new(built: &'static Built, stage: usize) -> DistRom {
        DistRom {
            rom: Rom::open_of(built, stage),
            lib: lib_of(built),
            assertion: 0,
        }
    }

    pub fn lib(&self) -> &mcui::data::Lib {
        &self.lib
    }

    /// The theorem screen of assertion `n` opened and F2 pressed: the key, the enter and the first
    /// frame.
    pub fn open(&mut self, n: usize) -> Cost {
        self.rom.open_at(n);
        self.assertion = n;
        let theorem = self.rom.stack + 32;
        let pushed = call(
            &mut self.rom,
            "theorem_key",
            In {
                a: KEY_F2,
                hl: theorem,
                ..In::default()
            },
        );
        assert_eq!(pushed.status(), 2, "assertion {n}: F2 pushes");
        let new = self.rom.new;
        let entered = call(
            &mut self.rom,
            "dist_enter",
            In {
                hl: new,
                ..In::default()
            },
        );
        assert_eq!(entered.status(), 0, "assertion {n}: the enter");
        let (stack, count, tag) = (self.rom.stack, self.rom.count, self.rom.tag);
        let app = &mut self.rom.list.run.shell.app;
        let frame = app.peek(new, 16);
        assert_eq!(frame[0], KIND_DIST);
        app.poke(stack + 48, &frame);
        app.poke(count, &[4]);
        app.poke(tag, &[0, 0]);
        let (_, draw, blit) = timed_frame_split(app);
        Cost {
            key: pushed.tstates,
            enter: entered.tstates,
            draw,
            blit,
        }
    }

    /// The first row shown, written into the frame.
    pub fn show_row(&mut self, top: usize) {
        let at = self.rom.stack + 48 + 4;
        let app = &mut self.rom.list.run.shell.app;
        app.poke(at, &(top as u16).to_le_bytes());
    }

    /// A key on the screen, timed with its frame when it draws: the answer and the cost.
    pub fn press(&mut self, code: u8) -> (u8, Cost) {
        let hl = self.rom.stack + 48;
        let done = call(
            &mut self.rom,
            "dist_key",
            In {
                a: code,
                hl,
                ..In::default()
            },
        );
        let answer = done.status();
        let mut cost = Cost {
            key: done.tstates,
            ..Cost::default()
        };
        if answer == 1 {
            let app = &mut self.rom.list.run.shell.app;
            let (_, draw, blit) = timed_frame_split(app);
            cost.draw = draw;
            cost.blit = blit;
        }
        (answer, cost)
    }

    /// The same cost split into its parts, after a draw of the screen at its frame's first row: the
    /// walk to that row is the walk over the pairs above it with nothing drawn.
    pub fn split(&mut self, cost: Cost, top: usize) -> Split {
        let above: usize = rows_of(&self.lib, self.assertion)[..top]
            .iter()
            .map(Vec::len)
            .sum();
        let open = call(&mut self.rom, "dist_open", In::default()).tstates;
        let walk = call(&mut self.rom, "dist_walk", In::default()).tstates;
        let npairs = self.rom.list.run.shell.app.label("DV_NPAIRS");
        call(&mut self.rom, "dist_open", In::default());
        let app = &mut self.rom.list.run.shell.app;
        app.poke(npairs, &(above as u16).to_le_bytes());
        let to_first_row = call(&mut self.rom, "dist_walk", In::default()).tstates;
        call(&mut self.rom, "dist_open", In::default());
        let title = call(&mut self.rom, "dist_title", In::default()).tstates;
        Split {
            key: cost.key,
            to_first_row,
            six_rows: walk.saturating_sub(to_first_row),
            title,
            open,
            rest: cost.draw.saturating_sub(open + walk + title),
            blit: cost.blit,
        }
    }

    /// CLEAR on the screen: the key, the theorem screen's enter on its frame and the frame.
    pub fn back(&mut self) -> Cost {
        let (answer, key) = self.press(KEY_CLEAR);
        assert_eq!(answer, 3, "CLEAR pops");
        let theorem = self.rom.stack + 32;
        let entered = call(
            &mut self.rom,
            "theorem_enter",
            In {
                hl: theorem,
                ..In::default()
            },
        );
        assert_eq!(entered.status(), 0, "the theorem's enter");
        let (count, tag) = (self.rom.count, self.rom.tag);
        let app = &mut self.rom.list.run.shell.app;
        app.poke(count, &[3]);
        app.poke(tag, &[0, 0]);
        let (_, draw, blit) = timed_frame_split(app);
        Cost {
            key: key.key,
            enter: entered.tstates,
            draw,
            blit,
        }
    }
}

fn label_index(lib: &mcui::data::Lib, label: &str) -> usize {
    index_of(lib, label)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_frrlem1_and_pressing_down_cost_what_the_header_says() {
    let mut dist = DistRom::new(shipped_staged(), RING);
    let n = label_index(dist.lib(), "frrlem1");
    let cost = dist.open(n);
    let (pairs, rows) = (
        dist.rom.list.run.shell.app.label("DV_NPAIRS"),
        dist.rom.list.run.shell.app.label("DV_ROWS"),
    );
    let app = &mut dist.rom.list.run.shell.app;
    let read = |app: &mut App, at: u16| {
        let b = app.peek(at, 2);
        usize::from(u16::from_le_bytes([b[0], b[1]]))
    };
    assert_eq!((read(app, pairs), read(app, rows)), (33, 6), "frrlem1");
    println!(
        "ROM, opening frrlem1: the key {}, the enter {}, the draw {} and the blit {}: {} T-states ({:.1} ms)",
        cost.key,
        cost.enter,
        cost.draw,
        cost.blit,
        cost.total(),
        f64::from(cost.total()) / 6000.0
    );
    assert!(cost.total() <= OPEN_BUDGET, "inside 300 ms");
    let ida = dist.rom.list.run.shell.app.label("DV_IDA");
    let id = dist.rom.list.run.shell.app.peek(ida, 1)[0];
    let token = call(
        &mut dist.rom,
        "dist_token",
        In {
            a: id,
            ..In::default()
        },
    )
    .tstates;
    let ink = call(&mut dist.rom, "dist_ink", In::default()).tstates;
    println!(
        "ROM, one token (far_lib_glyph and far_ui_text_width): {token} T-states, so {} to place a pair; drawing one pair (two more tokens and three far_ui_text): {ink}",
        2 * token
    );
    within("one token", token, TOKEN_COST);
    within("drawing one pair", ink, INK_COST);
    let mut stays = Vec::new();
    for (name, code) in [("Down", KEY_DOWN), ("Up", KEY_UP)] {
        let (answer, stay) = dist.press(code);
        println!("ROM, {name} on frrlem1 (a stay): {} T-states", stay.key);
        assert_eq!(answer, 0, "{name} stays");
        assert_eq!(stay.draw + stay.blit, 0);
        stays.push(stay.key);
    }
    within("opening frrlem1", cost.total(), OPEN_COST);
    within("Down on frrlem1", stays[0], STAY_COST);
    within("Up on frrlem1", stays[1], STAY_UP_COST);
}

/// The measured total of the open of frrlem1 and of a key that stays.
const OPEN_COST: u32 = 1_588_121;
const STAY_COST: u32 = 218;
const STAY_UP_COST: u32 = 146;
const TOKEN_COST: u32 = 6_311;
const INK_COST: u32 = 17_896;
const BACK_COST: u32 = 1_763_313;

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_scroll_on_the_packed_library_is_over_50_ms_and_no_shipped_library_has_one_to_scroll()
 {
    let built = dist_lib::built();
    let mut dist = DistRom::new(built, 0);
    let n = label_index(dist.lib(), SCROLL);
    let rows = rows_of(dist.lib(), n).len();
    let last = last_of(rows);
    assert!(rows >= 14, "{rows} rows");
    dist.open(n);
    let mut worst = (0, Split::default(), String::new());
    let mut scrolls = 0;
    for (what, top) in [("first", 0), ("middle", last / 2), ("last", last - 1)] {
        for (name, code, from) in [("Down", KEY_DOWN, top), ("Up", KEY_UP, top + 1)] {
            dist.show_row(from);
            let (answer, cost) = dist.press(code);
            assert_eq!(answer, 1, "{what} {name}: scrolls");
            let to = if code == KEY_DOWN { from + 1 } else { from - 1 };
            let split = dist.split(cost, to);
            let total = cost.key + cost.draw + cost.blit;
            println!(
                "ROM, {name} from row {from} of {rows} ({what}): {total} T-states ({:.1} ms), split {split:?}",
                f64::from(total) / 6000.0
            );
            assert!(
                total > SCROLL_BUDGET,
                "{name} from row {from}: {total} is over the 50 ms the shipped libraries stay inside"
            );
            within(
                &format!("{name} from row {from}"),
                total,
                scroll_cost(code, from),
            );
            scrolls += 1;
            if total > worst.0 {
                worst = (total, split, format!("{name} from row {from}"));
            }
        }
    }
    assert_eq!(scrolls, 6);
    let (total, split, what) = worst;
    println!(
        "ROM, the worst scroll, {what}: {total} T-states ({:.1} ms) against {SCROLL_BUDGET}: the key {}, the walk to the first row shown {}, the six rows {}, the title {}, dist_open {}, the rest of the draw {}, the blit {}",
        f64::from(total) / 6000.0,
        split.key,
        split.to_first_row,
        split.six_rows,
        split.title,
        split.open,
        split.rest,
        split.blit
    );
    assert_eq!(what, "Down from row 9", "the worst scroll");
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_way_back_to_the_theorem_screen_costs_what_the_theorem_enter_does() {
    let mut dist = DistRom::new(shipped_staged(), RING);
    let lib = lib_of(shipped_staged());
    let mut worst = (0, 0);
    for n in 0..lib.len() {
        if lib.dist(n).is_empty() {
            continue;
        }
        let open = dist.rom.open_at(n);
        let cost = open.0 + open.1 + open.2;
        if cost > worst.0 {
            worst = (cost, n);
        }
    }
    println!(
        "ROM, the worst theorem open among ring's assertions with pairs: {} ({}) {}",
        lib.label(worst.1),
        worst.1,
        worst.0
    );
    let n = label_index(&lib, "rglcom4d");
    assert_eq!(
        worst.1, n,
        "rglcom4d is the costliest open among ring's assertions with pairs"
    );
    assert!(!lib.dist(n).is_empty(), "rglcom4d has pairs");
    dist.open(n);
    let back = dist.back();
    println!(
        "ROM, back to the theorem from rglcom4d: the key {}, the enter {}, the draw {} and the blit {}: {} T-states",
        back.key,
        back.enter,
        back.draw,
        back.blit,
        back.total()
    );
    within("the way back from rglcom4d", back.total(), BACK_COST);
}

/// The shipped build's list of `stage`, open on the reference OS, with the model beside it.
fn walker(stage: usize) -> Walker {
    let mut list = Lister::on_rom(stage);
    list.open_on_rom();
    let mut walker = Walker::new(list, stage, shipped_staged());
    walker.gap = 600_000;
    walker.mirrored = false;
    walker
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn through_the_real_shell_the_dist_screen_of_ring_is_the_models_and_the_theorem_after_it_too() {
    let built = shipped_staged();
    let lib = lib_of(built);
    let with = |a: usize, b: usize, few: usize| -> usize {
        (a..b)
            .find(|&n| lib.dist(n).len() == few)
            .expect("an assertion with that many pairs")
    };
    let none = (0..779)
        .find(|&n| lib.dist(n).is_empty() && !facts(&lib, n).large)
        .expect("one with none");
    let few = with(779, lib.len(), 3);
    let most = label_index(&lib, "frrlem1");
    assert_eq!(lib.dist(most).len(), 33);
    let mut w = walker(RING);
    for (what, n) in [("none", none), ("a few", few), ("the most", most)] {
        println!("ROM shell: {what}, assertion {n}: goto");
        w.goto(n);
        w.hold_preview();
        println!("ROM shell: {what}: open");
        w.open();
        println!("ROM shell: {what}: F2");
        open_dist(&mut w);
        let rows = rows_of(&lib, n).len();
        assert_eq!(app_rows(&mut w), rows, "{what}");
        for (scan, key) in [(DOWN, Key::Down), (UP, Key::Up)] {
            println!("ROM shell: {what}: key {scan:02X}");
            assert!(!press(&mut w, scan, key), "{what}: {scan:02X} stays");
        }
        leave(&mut w);
        w.mirrored = true;
        assert_eq!(
            w.var("TV_LARGE"),
            0,
            "{what}: the theorem screen is not the too large one"
        );
        let mut drawn = 0;
        for (scan, key) in [
            (LEFT, Key::Left),
            (RIGHT, Key::Right),
            (ENTER, Key::Enter),
            (F1, Key::F1),
            (DOWN, Key::Down),
        ] {
            println!("ROM shell: {what}: theorem key {scan:02X}");
            drawn += usize::from(w.press(scan, key));
        }
        println!("ROM shell: {what}: {drawn} of the theorem's keys redrew");
        assert!(
            drawn > 0,
            "{what}: a move or a fold on the theorem after CLEAR drew"
        );
        w.mirrored = false;
        w.back();
        w.hold_preview();
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_clear_on_the_dist_screen_and_clear_on_the_theorem_screen_follow_with_no_gap()
{
    let lib = lib_of(shipped_staged());
    let n = label_index(&lib, "frrlem1");
    let mut w = walker(RING);
    w.goto(n);
    w.hold_preview();
    w.open();
    open_dist(&mut w);
    leave(&mut w);
    w.mirrored = false;
    let since = w.list.run.scans_since_release();
    println!("ROM shell: {since} scans since the release of the first CLEAR");
    assert!(since < 5, "the second CLEAR follows the first");
    w.back();
    assert_eq!(w.list.run.top_kind(), KIND_LIST);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_every_routine_of_the_dist_screen_keeps_interrupts_and_speed_and_writes_these_depths()
 {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let on_theorem = |label: &str| {
        let mut list = Lister::on_rom_of(built, 0);
        list.open_on_rom();
        let mut walker = Walker::new(list, 0, built);
        walker.gap = 600_000;
        walker.mirrored = false;
        walker.goto(index_of(&lib, label));
        walker.hold_preview();
        walker.open();
        walker
    };
    let large = crate::dist_large::built();
    let large_lib = lib_of(large);
    let on_large = || {
        let mut list = Lister::on_rom_of(large, 0);
        list.open_on_rom();
        let mut walker = Walker::new(list, 0, large);
        walker.gap = 600_000;
        walker.mirrored = false;
        walker.goto(index_of(&large_lib, crate::dist_large::LARGE_SEVEN));
        walker.hold_preview();
        walker.open();
        walker
    };
    let mut seen = Vec::new();
    for mask in [true, false] {
        for speed in [0, 1] {
            let got =
                crate::dist_depth::figures_on(&on_theorem, &on_large, dist_lib::SEVEN, speed, mask);
            println!("ROM, speed {speed}, masked {mask}: {got:?}");
            seen.push((mask, speed, got));
        }
    }
    for (mask, speed, got) in seen {
        crate::dist_depth::same_figures(
            &got,
            mask,
            true,
            &format!("ROM, masked {mask}, speed {speed}"),
        );
    }
}

/// What a scroll with its frame costs on the 16 rows of 6 pairs, by key and first row it starts from.
fn scroll_cost(code: u8, from: usize) -> u32 {
    match (code, from) {
        (KEY_DOWN, 0) => 1_326_515,
        (KEY_UP, 1) => 1_248_899,
        (KEY_DOWN, 5) => 1_724_749,
        (KEY_UP, 6) => 1_645_827,
        (KEY_DOWN, 9) => 2_035_656,
        (KEY_UP, 10) => 1_967_792,
        _ => panic!("no figure for key {code} from row {from}"),
    }
}
