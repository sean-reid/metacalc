//! The theorem screen on the reference OS with the shipped libraries, ignored in the default run: the
//! budgets of opening an assertion, a cursor move and a fold with their frames, over every assertion
//! of prop and of ring, and the longest stretch with interrupts off.

use crate::app::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcemu::Regs;
use mcui::{Key, State};
use std::cell::Cell;

/// The budgets at 6 MHz: opening an assertion inside 300 ms and a cursor move with its frame inside
/// 50 ms.
pub const OPEN_BUDGET: u32 = 1_800_000;
pub const MOVE_BUDGET: u32 = 300_000;

const KEY_CODES: [(Key, u8); 6] = [
    (Key::Up, 1),
    (Key::Down, 2),
    (Key::Left, 3),
    (Key::Right, 4),
    (Key::Enter, 5),
    (Key::F1, 7),
];

fn code_of_key(key: Key) -> u8 {
    KEY_CODES.iter().find(|(k, _)| *k == key).unwrap().1
}

thread_local! {
    /// Whether `measure` reports the longest stretch with interrupts off instead of the T-states.
    static STRETCH: Cell<bool> = const { Cell::new(false) };
}

/// A call on the entry page, timed, or with `STRETCH` the longest run of T-states in it with
/// interrupts off, the call starting with them on. The result is in `tstates`.
pub fn measure(app: &mut App, target: u16, input: In) -> Done {
    if !STRETCH.get() {
        return app.timed(BASE, target, input);
    }
    let stop = app.label("screen_ret");
    let [lo, hi] = target.to_le_bytes();
    let [slo, shi] = stop.to_le_bytes();
    app.poke(app.driver, &[0xCD, lo, hi, 0xC3, slo, shi]);
    app.m.out_port(0x06, BASE);
    app.m.set_regs(&Regs {
        af: u16::from(input.a) << 8 | u16::from(input.f),
        bc: input.bc,
        de: input.de,
        hl: input.hl,
        ix: 0x1111,
        iy: 0x89F0,
        sp: app.stack,
        pc: app.driver,
        af2: 0x2222,
        bc2: 0x3333,
        de2: 0x4444,
        hl2: 0x5555,
        i: 0xFF,
        iff1: 1,
        iff2: 1,
        im: 1,
    });
    let (mut run, mut best, mut last, mut off, mut handler) =
        (0u32, 0u32, app.m.clock(), false, false);
    for _ in 0..50_000_000 {
        app.m.resume(1);
        let now = app.m.clock();
        let spent = now.wrapping_sub(last);
        last = now;
        let regs = app.m.regs();
        if regs.pc == 0x0038 {
            best = best.max(run);
            run = 0;
            handler = true;
            continue;
        }
        if handler {
            handler = regs.iff1 == 0;
            off = regs.iff1 == 0;
            continue;
        }
        if off {
            run += spent;
        } else {
            best = best.max(run);
            run = 0;
        }
        off = regs.iff1 == 0;
        if regs.pc == stop && app.m.in_port(0x06) == BASE {
            best = best.max(run);
            let done = Done {
                regs,
                tstates: best,
                page: BASE,
            };
            return done;
        }
    }
    panic!("the call to {target:04X} never returned");
}

/// scr_draw then ui_blit for the screen on top, timed, and each of the two.
pub fn timed_frame_split(app: &mut App) -> (u32, u32, u32) {
    let draw = app.label("scr_draw");
    let draw = measure(app, draw, In::default()).tstates;
    let blit = app.label("far_ui_blit");
    let blit = measure(app, blit, In::default()).tstates;
    let both = if STRETCH.get() {
        draw.max(blit)
    } else {
        draw + blit
    };
    (both, draw, blit)
}

pub fn timed_frame(app: &mut App) -> u32 {
    timed_frame_split(app).0
}

/// The app on the reference OS with the list of `stage` open, and the labels the cost walks use.
pub struct Rom {
    pub list: Lister,
    pub stack: u16,
    pub count: u16,
    pub new: u16,
    pub tag: u16,
    /// The key, the draw and the blit of the last `press`, in T-states.
    pub last: (u32, u32, u32),
}

impl Rom {
    pub fn open(stage: usize) -> Rom {
        Rom::open_of(shipped_staged(), stage)
    }

    /// `open` for the app in `built`.
    pub fn open_of(built: &'static Built, stage: usize) -> Rom {
        let mut list = Lister::on_rom_of(built, stage);
        list.open_on_rom();
        let app = &mut list.run.shell.app;
        Rom {
            stack: app.label("SCR_STACK"),
            count: app.label("SCR_COUNT"),
            new: app.label("SCR_NEW"),
            tag: app.label("APP_STAGE_TAG"),
            last: (0, 0, 0),
            list,
        }
    }

    /// The list's frame written to have its cursor on assertion `n`.
    fn aim(&mut self, n: usize) {
        let frame = self.stack + 16;
        let stage = self.list.stage_of_frame();
        let at = (n as u16).to_le_bytes();
        let bytes = [KIND_LIST, stage, at[0], at[1], at[0], at[1]];
        let app = &mut self.list.run.shell.app;
        app.poke(frame, &bytes);
        app.poke(self.count, &[2]);
    }

    /// ENTER on the list with the cursor on `n`: the key, the enter and the first frame, timed, with
    /// the theorem screen left on top of the stack. Returns the three costs.
    pub fn open_at(&mut self, n: usize) -> (u32, u32, u32) {
        self.aim(n);
        let (key, enter, new) = {
            let app = &mut self.list.run.shell.app;
            (app.label("list_key"), app.label("theorem_enter"), self.new)
        };
        let frame = self.stack + 16;
        let app = &mut self.list.run.shell.app;
        let pushed = measure(
            app,
            key,
            In {
                a: 5,
                hl: frame,
                ..In::default()
            },
        );
        assert_eq!(pushed.status(), 2, "assertion {n}: the key pushes");
        let entered = measure(
            app,
            enter,
            In {
                hl: new,
                ..In::default()
            },
        );
        assert_eq!(entered.status(), 0, "assertion {n}: the enter");
        let bytes = app.peek(new, 16);
        app.poke(self.stack + 32, &bytes);
        app.poke(self.count, &[3]);
        app.poke(self.tag, &[0, 0]);
        let drawn = timed_frame(app);
        (pushed.tstates, entered.tstates, drawn)
    }

    /// The theorem screen's frame at the top of the stack.
    pub fn frame(&mut self) -> [u8; FRAME] {
        let app = &mut self.list.run.shell.app;
        app.peek(self.stack + 32, FRAME).try_into().unwrap()
    }

    /// `key` on the open theorem screen, timed with its frame when it draws: the answer and the
    /// T-states.
    pub fn press(&mut self, key: Key) -> (u8, u32) {
        let at = self.list.run.shell.app.label("theorem_key");
        let hl = self.stack + 32;
        let app = &mut self.list.run.shell.app;
        let done = measure(
            app,
            at,
            In {
                a: code_of_key(key),
                hl,
                ..In::default()
            },
        );
        let answer = done.status();
        let mut spent = done.tstates;
        let mut split = (done.tstates, 0, 0);
        if answer == 1 {
            let (frame, draw, blit) = timed_frame_split(app);
            split = (done.tstates, draw, blit);
            spent = if STRETCH.get() {
                spent.max(frame)
            } else {
                spent + frame
            };
        }
        self.last = split;
        (answer, spent)
    }
}

impl Lister {
    /// The stage the open list is of, from its frame.
    pub fn stage_of_frame(&mut self) -> u8 {
        let stack = self.run.shell.app.label("SCR_STACK");
        self.run.shell.app.peek(stack + 17, 1)[0]
    }
}

/// The cursor's node and whether it is folded, from the model.
fn at(ui: &mcui::Ui) -> (usize, usize, bool) {
    match ui.state() {
        State::Theorem {
            statement,
            node,
            hidden,
            ..
        } => (statement, node, hidden),
        other => panic!("{other:?}"),
    }
}

/// Visits every node of every statement of assertion `n`, from the conclusion back to the first
/// hypothesis, in preorder: Down to the first child, else Right to a sibling, else Up and the same
/// again, with ENTER to unfold a node the fit folded.
fn visit(ui: &mut mcui::Ui, rom: &mut Rom, stats: &mut Stats) {
    loop {
        let (statement, node, folded) = at(ui);
        if folded {
            step(ui, rom, Key::Enter, stats);
        }
        step(ui, rom, Key::Down, stats);
        if at(ui).1 != node {
            continue;
        }
        let ended = loop {
            let (_, here, _) = at(ui);
            if here == 0 {
                break true;
            }
            step(ui, rom, Key::Right, stats);
            if at(ui).1 != here {
                break false;
            }
            step(ui, rom, Key::Up, stats);
        };
        if ended {
            if statement == 0 {
                return;
            }
            step(ui, rom, Key::Left, stats);
        }
    }
}

#[derive(Default)]
pub struct Stats {
    pub moves: Vec<(u32, String)>,
    pub folds: Vec<(u32, String)>,
    /// The key, the draw and the blit of each entry of `moves` and of `folds`.
    pub move_splits: Vec<(u32, u32, u32)>,
    pub fold_splits: Vec<(u32, u32, u32)>,
    /// The assertion the walk is on.
    pub label: String,
}

/// Presses `key` on the model and on the app and compares where they are.
fn step(ui: &mut mcui::Ui, rom: &mut Rom, key: Key, stats: &mut Stats) {
    let before = crate::shellrig::model::frame(ui);
    ui.press(key);
    let after = ui.state();
    let (answer, spent) = rom.press(key);
    let State::Theorem {
        statement,
        node,
        scroll,
        ..
    } = after.clone()
    else {
        panic!("{after:?}")
    };
    let frame = rom.frame();
    assert_eq!(
        (
            usize::from(frame[4]),
            usize::from(frame[5]),
            usize::from(u16::from_le_bytes([frame[6], frame[7]]))
        ),
        (statement, node, scroll),
        "after {key:?}: {after:?}"
    );
    assert_eq!(
        answer == 1,
        before != crate::shellrig::model::frame(ui),
        "after {key:?}: the answer"
    );
    if answer == 1 {
        match key {
            Key::Enter | Key::F1 => {
                stats
                    .folds
                    .push((spent, format!("{} {key:?} at node {node}", stats.label)));
                stats.fold_splits.push(rom.last);
            }
            _ => {
                stats
                    .moves
                    .push((spent, format!("{} {key:?} to node {node}", stats.label)));
                stats.move_splits.push(rom.last);
            }
        }
    }
}

/// What a set of costs came to: the median, the worst, how many are over `budget` and how many over
/// 200 ms (1,200,000 T-states).
#[derive(Debug, PartialEq, Eq)]
pub struct Summary {
    pub runs: usize,
    pub median: u32,
    pub worst: u32,
    pub over_200_ms: usize,
    pub over_budget: usize,
}

fn summary(what: &str, all: &[u32], budget: u32) -> Summary {
    let mut v = all.to_vec();
    v.sort_unstable();
    let got = Summary {
        runs: v.len(),
        median: v[v.len() / 2],
        worst: *v.last().unwrap(),
        over_200_ms: v.iter().filter(|&&t| t > 1_200_000).count(),
        over_budget: v.iter().filter(|&&t| t > budget).count(),
    };
    println!(
        "ROM, {what}: {} runs, median {} ({:.1} ms), worst {} ({:.1} ms), {} over 200 ms, {} over the budget of {budget}",
        got.runs,
        got.median,
        f64::from(got.median) / 6000.0,
        got.worst,
        f64::from(got.worst) / 6000.0,
        got.over_200_ms,
        got.over_budget
    );
    got
}

/// What opening each assertion of a stage cost: the key, the enter and the first frame.
fn open_costs(stage: usize) -> Vec<(u32, String)> {
    let mut rom = Rom::open(stage);
    let lib = lib_of(rom.list.built);
    let end = rom.list.pos().len;
    let mut costs = Vec::new();
    for n in 0..end {
        let (key, enter, draw) = rom.open_at(n);
        if key + enter + draw > 1_500_000 {
            println!(
                "ROM, opening {}: the key {key}, the enter {enter}, the first frame {draw}",
                lib.label(n)
            );
        }
        costs.push((key + enter + draw, lib.label(n).to_string()));
    }
    costs
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_every_assertion_of_prop_and_ring_costs_what_the_header_says() {
    for (name, stage, want, labels) in [
        (
            "prop",
            PROP,
            Summary {
                runs: 779,
                median: 326_062,
                worst: 762_336,
                over_200_ms: 0,
                over_budget: 0,
            },
            [
                "syl221anc",
                "syl211anc",
                "syl13anc",
                "3orbi123d",
                "3anbi123d",
            ],
        ),
        (
            "ring",
            RING,
            Summary {
                runs: 3326,
                median: 295_443,
                worst: 1_781_407,
                over_200_ms: 5,
                over_budget: 0,
            },
            ["rglcom4d", "grpinva", "ovmpodxf", "grpinvalem", "isrng"],
        ),
    ] {
        let costs = open_costs(stage);
        let all: Vec<u32> = costs.iter().map(|c| c.0).collect();
        let got = summary(&format!("opening {name}'s assertions"), &all, OPEN_BUDGET);
        let mut worst = costs.clone();
        worst.sort();
        worst.reverse();
        let five: Vec<&str> = worst[..5].iter().map(|c| c.1.as_str()).collect();
        println!("ROM, {name}: the five worst: {:?}", &worst[..5]);
        assert_eq!(five, labels, "{name}: the five worst");
        assert_eq!(
            (got.runs, got.over_200_ms, got.over_budget),
            (want.runs, want.over_200_ms, want.over_budget),
            "{name}"
        );
        within(&format!("{name}: the median open"), got.median, want.median);
        within(&format!("{name}: the worst open"), got.worst, want.worst);
    }
}

/// The 50 largest assertions of the stage by nodes, then every 97th of the rest.
fn picks(lib: &mcui::data::Lib, end: usize) -> Vec<usize> {
    let mut by_size: Vec<(usize, usize)> = (0..end)
        .map(|n| (lib.statement(n).all().iter().map(|e| e.len()).sum(), n))
        .collect();
    by_size.sort_unstable_by(|a, b| b.cmp(a));
    let mut picks: Vec<usize> = by_size.iter().take(50).map(|p| p.1).collect();
    picks.extend((0..end).step_by(97));
    picks
}

/// The model on the theorem screen of assertion `n` of the stage.
fn model_at(rom: &Rom, stage: usize, n: usize) -> mcui::Ui {
    let mut ui = crate::shellrig::model::ui(rom.list.built, &[]);
    for _ in 0..stage {
        ui.press(Key::Down);
    }
    ui.press(Key::Enter);
    for _ in 0..n / 5 {
        ui.press(Key::Right);
    }
    for _ in 0..n % 5 {
        ui.press(Key::Down);
    }
    ui.press(Key::Enter);
    assert!(matches!(ui.state(), State::Theorem { .. }));
    ui
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_cursor_move_and_a_fold_with_their_frames_cost_what_the_header_says() {
    let mut rom = Rom::open(RING);
    let lib = lib_of(rom.list.built);
    let end = rom.list.pos().len;
    let mut stats = Stats::default();
    for n in picks(&lib, end) {
        let mut ui = model_at(&rom, RING, n);
        rom.open_at(n);
        stats.label = lib.label(n).to_string();
        visit(&mut ui, &mut rom, &mut stats);
    }
    let moves: Vec<u32> = stats.moves.iter().map(|m| m.0).collect();
    let folds: Vec<u32> = stats.folds.iter().map(|m| m.0).collect();
    let got = summary("a cursor move with its frame", &moves, MOVE_BUDGET);
    let fold = summary("a fold with its frame", &folds, MOVE_BUDGET);
    println!(
        "ROM, the worst move: {:?}",
        stats.moves.iter().max().unwrap()
    );
    println!(
        "ROM, the worst fold: {:?}",
        stats.folds.iter().max().unwrap()
    );
    let worst_move = (0..stats.moves.len())
        .max_by_key(|&i| stats.moves[i].0)
        .unwrap();
    let worst_fold = (0..stats.folds.len())
        .max_by_key(|&i| stats.folds[i].0)
        .unwrap();
    println!(
        "ROM, the worst move split into key, draw and blit: {:?}; the worst fold: {:?}",
        stats.move_splits[worst_move], stats.fold_splits[worst_fold]
    );
    assert_eq!((got.runs, got.over_budget), (4872, 228));
    within("the median move", got.median, 266_266);
    within("the worst move", got.worst, 337_236);
    assert_eq!((fold.runs, fold.over_budget), (322, 113));
    within("the median fold", fold.median, 293_499);
    within("the worst fold", fold.worst, 390_368);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_longest_stretch_with_interrupts_off_in_an_open_and_in_a_key() {
    STRETCH.set(true);
    let mut rom = Rom::open(RING);
    let lib = lib_of(rom.list.built);
    let end = rom.list.pos().len;
    let worst = (0..end)
        .find(|&n| lib.label(n) == "rglcom4d")
        .expect("ring has the assertion that costs most to open");
    let (key, enter, draw) = rom.open_at(worst);
    let open = key.max(enter).max(draw);
    println!(
        "ROM, opening rglcom4d: interrupts off for at most {key} in the key, {enter} in the enter, {draw} in the frame: {open} T-states ({:.1} ms)",
        f64::from(open) / 6000.0
    );
    let mut stats = Stats::default();
    let mut largest = picks(&lib, end);
    largest.truncate(3);
    let mut most_open = open;
    for n in largest {
        let mut ui = model_at(&rom, RING, n);
        let (key, enter, draw) = rom.open_at(n);
        most_open = most_open.max(key).max(enter).max(draw);
        stats.label = lib.label(n).to_string();
        visit(&mut ui, &mut rom, &mut stats);
    }
    let moves = stats.moves.iter().max().unwrap();
    let folds = stats.folds.iter().max().unwrap();
    within("the longest stretch in an open", most_open, 1_282_260);
    within("the longest stretch in a move", moves.0, 78_040);
    within("the longest stretch in a fold", folds.0, 73_249);
    println!(
        "ROM, over the three largest assertions: the longest stretch in a move is {moves:?}, in a fold {folds:?}; in an open {most_open}"
    );
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_routine_of_the_theorem_screen_keeps_interrupts_and_speed_and_writes_these_depths()
 {
    use crate::theorem_frames::find;
    let built = shipped_staged();
    let lib = lib_of(built);
    let (n, _) = find(&lib, |f| f.hyps >= 3 && f.rows >= 6 && !f.large);
    let (plain, _) = find(&lib, |f| f.hyps == 0 && !f.large);
    assert!(n < 779 && plain < 779, "both are in prop, which is stage 0");
    let before = |n: usize| {
        let mut list = Lister::on_rom(PROP);
        list.open_on_rom();
        let mut walker = Walker::new(list, PROP, built);
        walker.goto(n);
        walker
    };
    let with_table = |n: usize| {
        let mut list = Lister::on_rom(PROP);
        assert!(list.partial_check(20) >= 20, "the table has values");
        list.open_on_rom();
        let mut walker = Walker::new(list, PROP, built);
        walker.goto(n);
        walker
    };
    for (table, want) in [(false, 0), (true, 1)] {
        let mut walker = if table { with_table(n) } else { before(n) };
        walker.open();
        assert_eq!(
            walker.var("TV_TABLE"),
            want,
            "the table is installed: {table}"
        );
    }
    for speed in [0, 1] {
        for (what, table) in [("no table", false), ("a table", true)] {
            let masked = if table {
                crate::theorem_depth::figures_of(&with_table, (n, plain), speed, true)
            } else {
                crate::theorem_depth::figures_of(&before, (n, plain), speed, true)
            };
            println!("ROM, speed {speed}, {what}, masked: {masked:?}");
            let mut masked = masked;
            assert_eq!(masked.remove("list_key enter"), Some(8), "speed {speed}");
            assert_eq!(
                masked,
                crate::theorem_depth::depths_on_the_reference_os(),
                "speed {speed}, {what}"
            );
        }
        crate::theorem_depth::figures_of(&before, (n, plain), speed, false);
    }
}
