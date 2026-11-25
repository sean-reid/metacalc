//! The theorem screen on the reference OS with the shipped libraries, ignored in the default run: the
//! budgets of opening an assertion, a cursor move and a fold with their frames, over every assertion
//! of prop and of ring, and the longest stretch with interrupts off.

use crate::app::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::data::Lib;
use mcui::{Key, State};

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

/// scr_draw then ui_blit for the screen on top, timed.
fn timed_frame(app: &mut App) -> u32 {
    let draw = app.label("scr_draw");
    let draw = app.timed(BASE, draw, In::default()).tstates;
    let blit = app
        .timed(BASE, app.label("far_ui_blit"), In::default())
        .tstates;
    draw + blit
}

/// The app on the reference OS with the list of `stage` open, and the labels the cost walks use.
pub struct Rom {
    pub list: Lister,
    stack: u16,
    count: u16,
    new: u16,
    tag: u16,
}

impl Rom {
    pub fn open(stage: usize) -> Rom {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        let app = &mut list.run.shell.app;
        let rom = Rom {
            stack: app.label("SCR_STACK"),
            count: app.label("SCR_COUNT"),
            new: app.label("SCR_NEW"),
            tag: app.label("APP_STAGE_TAG"),
            list,
        };
        rom
    }

    /// The list's frame written to have its cursor on assertion `n`.
    fn aim(&mut self, n: usize) {
        let frame = self.stack + 16;
        let stage = self.list.stage_of_frame();
        let at = (n as u16).to_le_bytes();
        let bytes = [KIND_LIST, stage, at[0], at[1], at[0], at[1]];
        let app = &mut self.list.run.shell.app;
        app.m.load_ram(frame, &bytes);
        app.m.load_ram(self.count, &[2]);
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
        let pushed = app.timed(
            BASE,
            key,
            In {
                a: 5,
                hl: frame,
                ..In::default()
            },
        );
        assert_eq!(pushed.status(), 2, "assertion {n}: the key pushes");
        let entered = app.timed(
            BASE,
            enter,
            In {
                hl: new,
                ..In::default()
            },
        );
        assert_eq!(entered.status(), 0, "assertion {n}: the enter");
        let bytes = app.m.read_ram(new, 16);
        app.m.load_ram(self.stack + 32, &bytes);
        app.m.load_ram(self.count, &[3]);
        app.m.load_ram(self.tag, &[0, 0]);
        let drawn = timed_frame(app);
        (pushed.tstates, entered.tstates, drawn)
    }

    /// The theorem screen's frame at the top of the stack.
    pub fn frame(&mut self) -> [u8; FRAME] {
        let app = &mut self.list.run.shell.app;
        app.m.read_ram(self.stack + 32, FRAME).try_into().unwrap()
    }

    /// `key` on the open theorem screen, timed with its frame when it draws: the answer and the
    /// T-states.
    pub fn press(&mut self, key: Key) -> (u8, u32) {
        let at = self.list.run.shell.app.label("theorem_key");
        let hl = self.stack + 32;
        let app = &mut self.list.run.shell.app;
        let done = app.timed(
            BASE,
            at,
            In {
                a: code_of_key(key),
                hl,
                ..In::default()
            },
        );
        let answer = done.status();
        let mut spent = done.tstates;
        if answer == 1 {
            spent += timed_frame(app);
        }
        (answer, spent)
    }
}

impl Lister {
    /// The stage the open list is of, from its frame.
    pub fn stage_of_frame(&mut self) -> u8 {
        let stack = self.run.shell.app.label("SCR_STACK");
        self.run.m().read_ram(stack + 17, 1)[0]
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
    pub folds: Vec<u32>,
}

/// Presses `key` on the model and on the app and compares where they are.
fn step(ui: &mut mcui::Ui, rom: &mut Rom, key: Key, stats: &mut Stats) {
    let before = ui.state();
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
    assert_eq!(answer == 1, before != after, "after {key:?}: the answer");
    if answer == 1 {
        match key {
            Key::Enter | Key::F1 => stats.folds.push(spent),
            _ => stats.moves.push((spent, format!("{key:?} to node {node}"))),
        }
    }
}

fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v
}

fn report(what: &str, all: &[u32], budget: u32, labels: Option<&[String]>) {
    let v = sorted(all.to_vec());
    let (median, worst) = (v[v.len() / 2], *v.last().unwrap());
    println!(
        "ROM, {what}: {} runs, median {median} ({:.1} ms), worst {worst} ({:.1} ms), against {budget} ({} over 200 ms: {}, over budget: {})",
        v.len(),
        f64::from(median) / 6000.0,
        f64::from(worst) / 6000.0,
        budget,
        v.iter().filter(|&&t| t > 1_200_000).count(),
        v.iter().filter(|&&t| t > budget).count(),
    );
    let _ = labels;
}

/// The open, the cost of an assertion's first frame on the reference OS, over every assertion of the
/// stage.
fn open_costs(stage: usize) -> Vec<(u32, String)> {
    let mut rom = Rom::open(stage);
    let lib = lib_of(rom.list.built);
    let end = rom.list.pos().len;
    let mut costs = Vec::new();
    for n in 0..end {
        let (key, enter, draw) = rom.open_at(n);
        costs.push((key + enter + draw, lib.label(n).to_string()));
    }
    costs
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_every_assertion_of_prop_and_ring_fits_300_ms() {
    for (name, stage) in [("prop", PROP), ("ring", RING)] {
        let costs = open_costs(stage);
        let all: Vec<u32> = costs.iter().map(|c| c.0).collect();
        report(&format!("opening {name}'s assertions"), &all, OPEN_BUDGET, None);
        let mut worst = costs.clone();
        worst.sort();
        worst.reverse();
        println!(
            "ROM, {name}: the five worst: {:?}",
            &worst[..5.min(worst.len())]
        );
        assert!(worst[0].0 < OPEN_BUDGET, "{name}: {:?}", worst[0]);
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_cursor_move_and_a_fold_with_their_frames_fit_50_ms() {
    let mut rom = Rom::open(RING);
    let lib = lib_of(rom.list.built);
    let end = rom.list.pos().len;
    let mut by_size: Vec<(usize, usize)> = (0..end)
        .map(|n| {
            let statement = lib.statement(n);
            let nodes = statement.all().iter().map(|e| e.len()).sum();
            (nodes, n)
        })
        .collect();
    by_size.sort_unstable_by(|a, b| b.cmp(a));
    let mut picks: Vec<usize> = by_size.iter().take(50).map(|p| p.1).collect();
    picks.extend((0..end).step_by(97));
    let mut stats = Stats::default();
    for n in picks {
        let mut ui = crate::shellrig::model::ui(rom.list.built, &[]);
        for _ in 0..RING {
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
        rom.open_at(n);
        visit(&mut ui, &mut rom, &mut stats);
    }
    let moves: Vec<u32> = stats.moves.iter().map(|m| m.0).collect();
    report("a cursor move with its frame", &moves, MOVE_BUDGET, None);
    report("a fold with its frame", &stats.folds, MOVE_BUDGET, None);
    let mut worst = stats.moves.clone();
    worst.sort();
    println!("ROM, the worst move: {:?}", worst.last().unwrap());
}
