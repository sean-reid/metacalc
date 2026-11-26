//! The list's budgets and the flows a user tries on the reference OS, ignored in the default run: opening
//! it and a cursor move with their frames against the 300 ms and 50 ms budgets, the marks after a
//! partial check, and the stack, interrupts and speed of each routine.

use crate::app::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcui::Key;

impl Lister {
    /// Runs the app's own check on the stage the cursor is on until `n` assertions have a value, stops
    /// it, and puts the table's marks beside the model, which is on Libraries again with the cursor
    /// where it was. Returns how many assertions have a value.
    pub fn partial_check(&mut self, n: usize) -> usize {
        self.run.m().press_key(F2);
        let mut spent = 0;
        while self
            .run
            .values()
            .is_none_or(|v| v.iter().filter(|&&x| x != 0).count() < n)
        {
            self.run.run(60_000);
            spent += 60_000;
            assert!(spent < 400_000_000, "the check never got {n} assertions in");
        }
        self.run.m().release_key(F2);
        self.tap(F1);
        self.run.run(1_200_000);
        assert_eq!(
            self.run.top_kind(),
            KIND_LIBRARIES,
            "Stop is back on Libraries"
        );
        let values = self.run.values().expect("a table");
        self.status = crate::checkrig::status_of(&values);
        let mut ui = crate::shellrig::model::ui(self.built, &self.status);
        let stage = usize::from(self.run.frames()[0][1]);
        for _ in 0..stage {
            ui.press(Key::Down);
        }
        self.ui = ui;
        values.iter().filter(|&&v| v != 0).count()
    }
}

/// scr_draw then ui_blit for the screen on top, timed.
fn timed_frame(app: &mut App) -> (u32, u32) {
    let draw = app.label("scr_draw");
    let draw = app.timed(BASE, draw, In::default()).tstates;
    let blit = app
        .timed(BASE, app.label("far_ui_blit"), In::default())
        .tstates;
    (draw, blit)
}

/// The budgets on the reference OS at 6 MHz: a cursor move with its frame inside
/// 50 ms, 300,000 T-states, and opening the list inside 300 ms, 1,800,000.
const MOVE_BUDGET: u32 = 300_000;
const OPEN_BUDGET: u32 = 1_800_000;

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_the_list_and_a_move_with_its_frame_fit_their_budgets() {
    let mut list = Lister::on_rom(RING);
    let done = list.partial_check(40);
    println!("ROM, the table has {done} assertions settled by the app's own check");
    list.open_on_rom();
    let stack = list.run.shell.app.label("SCR_STACK");
    let libs_key = list.run.shell.app.label("libs_key");
    let enter_key = list
        .run
        .shell
        .app
        .timed(
            BASE,
            libs_key,
            In {
                a: 5,
                hl: stack,
                ..In::default()
            },
        )
        .tstates;
    let app = &mut list.run.shell.app;
    let frame = stack + 16;
    let (enter, tag) = (app.label("list_enter"), app.label("APP_STAGE_TAG"));
    let entered = app.timed(
        BASE,
        enter,
        In {
            hl: frame,
            ..In::default()
        },
    );
    assert_eq!(entered.status(), 0);
    app.m.load_ram(tag, &[0, 0]);
    let (cold_draw, cold_blit) = timed_frame(app);
    let open = enter_key + entered.tstates + cold_draw + cold_blit;
    println!(
        "ROM, opening ring's list from Libraries: libs_key {enter_key}, list_enter {}, cold scr_draw {cold_draw}, ui_blit {cold_blit}: {open} T-states ({:.1} ms) against {OPEN_BUDGET}",
        entered.tstates,
        f64::from(open) / 6000.0
    );
    assert!(open < OPEN_BUDGET);
    within("opening the list", open, 461_598);
    let key = app.label("list_key");
    let (idle, count, preview) = (
        app.label("list_idle"),
        app.label("LV_COUNT"),
        app.label("LV_PREVIEW"),
    );
    for (name, code) in [("row", 2u8), ("page", 4)] {
        let moved = app.timed(
            BASE,
            key,
            In {
                a: code,
                hl: frame,
                ..In::default()
            },
        );
        assert_eq!(moved.status(), 1);
        let (draw, blit) = timed_frame(app);
        let total = moved.tstates + draw + blit;
        println!(
            "ROM, a cursor move by {name}: list_key {}, scr_draw {draw}, ui_blit {blit}: {total} T-states ({:.1} ms) against {MOVE_BUDGET}",
            moved.tstates,
            f64::from(total) / 6000.0
        );
        assert!(total < MOVE_BUDGET);
        within(
            &format!("a move by {name}"),
            total,
            if code == 2 { 223_339 } else { 227_921 },
        );
        app.m.load_ram(count, &[0]);
        assert_eq!(app.m.read_ram(preview, 1), [PENDING]);
        let fit = app.timed(
            BASE,
            idle,
            In {
                hl: frame,
                ..In::default()
            },
        );
        assert_eq!(fit.status(), 1);
        let (draw, blit) = timed_frame(app);
        println!(
            "ROM, the preview after the move by {name}: list_idle {}, the settled scr_draw {draw}, ui_blit {blit}: {} T-states for the frame",
            fit.tstates,
            draw + blit
        );
        assert!(draw + blit < MOVE_BUDGET);
        within(
            &format!("the settled frame after a move by {name}"),
            draw + blit,
            if code == 2 { 236_390 } else { 231_957 },
        );
        within(
            &format!("the fit after a move by {name}"),
            fit.tstates,
            if code == 2 { 50_951 } else { 52_421 },
        );
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_marks_after_a_partial_check_are_the_models() {
    let mut list = Lister::on_rom(PROP);
    let done = list.partial_check(60);
    assert!(
        (60..779).contains(&done),
        "the check is partial: {done} of 779"
    );
    list.open_on_rom();
    let end = list.pos().len;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..=end / 5 {
        for (_, right) in rows_of_ui(&list.ui) {
            seen.insert(right);
        }
        if list.pos().cursor + 5 < end {
            list.key(RIGHT, Key::Right);
            list.skip_the_wait();
            list.settle();
        }
    }
    println!("ROM, after a partial check of prop ({done} settled): marks drawn {seen:?}");
    assert!(seen.contains("?") && seen.contains(&mcglyph::CHECK.to_string()));
}

fn rows_of_ui(ui: &mcui::Ui) -> Vec<(String, String)> {
    ui.frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            mcui::canvas::Call::ListRow { left, right, .. } => Some((left, right)),
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_routine_of_the_list_keeps_interrupts_and_speed_and_writes_these_depths()
 {
    let rig = crate::list_depth::Rig {
        name: "reference",
        open: || {
            let mut list = Lister::on_rom(PROP);
            list.partial_check(40);
            assert!(list.key(ENTER, Key::Enter));
            list
        },
        before: || {
            let mut list = Lister::on_rom(PROP + 1);
            list.partial_check(40);
            list
        },
    };
    let want = crate::list_depth::Figures {
        keys: std::collections::BTreeMap::from([
            ("clear", 2),
            ("down", 4),
            ("enter", 8),
            ("f1", 8),
            ("f2", 6),
            ("f3", 24),
            ("f4", 6),
            ("f5", 6),
            ("left", 4),
            ("letter", 2),
            ("right", 4),
            ("up", 4),
        ]),
        ends: 4,
        enter: 36,
        idle: std::collections::BTreeMap::from([("fit", 8), ("settled", 2), ("tick", 14)]),
        draw: 34,
    };
    crate::list_depth::traced(&rig, &want);
}

/// What a user does with the list, on each stage of the shipped build: open the list, move
/// by row and by page to both ends, every soft key's notice and the way back by each of the keys
/// that pop it, and CLEAR to Libraries
/// with the stage's row where it was.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_each_stage_opens_moves_to_both_ends_opens_each_notice_and_clears_back() {
    for stage in 0..3 {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        let at = format!("stage {stage}");
        for (scan, key) in [
            (DOWN, Key::Down),
            (DOWN, Key::Down),
            (RIGHT, Key::Right),
            (RIGHT, Key::Right),
            (UP, Key::Up),
            (LEFT, Key::Left),
        ] {
            if list.key(scan, key) {
                list.same_position(&format!("{at}, after {scan:02X}"));
                list.skip_the_wait();
                list.settle();
            }
        }
        list.jump_to_the_end();
        for (scan, key) in [(DOWN, Key::Down), (RIGHT, Key::Right)] {
            assert!(
                !list.key(scan, key),
                "{at}: nothing past the last assertion"
            );
        }
        assert!(list.key(LEFT, Key::Left), "{at}");
        list.skip_the_wait();
        list.settle();
        for _ in 0..3 {
            assert!(list.key(UP, Key::Up), "{at}");
            list.skip_the_wait();
            list.settle();
        }
        let sections = list.var("LV_SECT") == 1;
        for (n, scan) in [F2, F4, F5].into_iter().enumerate() {
            if scan == F5 && !sections {
                let before = list.frame_bytes();
                list.tap(scan);
                list.run.run(600_000);
                assert_eq!(list.run.depth(), 2, "{at}: F5 with no sections");
                assert_eq!(list.frame_bytes(), before, "{at}: F5 with no sections");
                continue;
            }
            for pop in [CLEAR, ENTER, F1] {
                let before = list.frame_bytes();
                list.tap(scan);
                let frame = list.run.next_blit();
                crate::ui_support::same_frame(
                    &format!("{at}, key {scan:02X} (number {n})"),
                    &frame,
                    &crate::shellrig::model::unavailable(list.built.stage_name(stage)),
                );
                list.run.run(600_000);
                assert_eq!(list.run.depth(), 3, "{at}");
                list.tap(pop);
                list.run.next_blit();
                list.run.run(2_000);
                assert_eq!(
                    list.frame_bytes(),
                    before,
                    "{at}: the list is where it was after key {scan:02X} and a pop by {pop:02X}"
                );
                list.ui = list.replayed();
                list.same_position(&format!("{at}, back from key {scan:02X} by {pop:02X}"));
                list.settle();
            }
        }
        assert!(list.key(CLEAR, Key::Clear), "{at}");
        assert_eq!(
            list.run.frames()[0][1],
            stage as u8,
            "{at}: Libraries on the stage's row"
        );
    }
}
