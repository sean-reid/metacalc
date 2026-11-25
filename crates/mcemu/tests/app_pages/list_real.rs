//! The list on the reference OS with the shipped libraries, ignored in the default run: every
//! assertion of prop and of ring walked against the model, the wait and the key repeat, a key during
//! the slowest fit, what each step costs against its budget, and the owner's flows.

use crate::app::*;
use crate::checking_real::launched;
use crate::checkrig::Run;
use crate::list_preview::{Counts, model_counts};
use crate::list_rig::*;
use crate::list_wait::{TICK, WAIT_TICKS};
use crate::realos::BASE;
use crate::shellrig::*;
use mcui::Key;

const NEEDS: &str = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>";

/// Prop, pred and ring are the three stages of the shipped build.
pub const PROP: usize = 0;
pub const RING: usize = 2;

impl Lister {
    /// The shipped build on the reference OS at its Libraries screen with the cursor on `stage`, and
    /// the model beside it, whose marks are `status` (none: every assertion unchecked).
    pub fn on_rom(stage: usize) -> Lister {
        let built = shipped_staged();
        let run: Run = launched(built, stage, &[]);
        let mut ui = crate::shellrig::model::ui(built, &[]);
        for _ in 0..stage {
            ui.press(Key::Down);
        }
        Lister {
            run,
            ui,
            built,
            status: Vec::new(),
            log: Vec::new(),
            seen: (0, 0),
        }
    }

    /// Opens the list of the stage the cursor is on and lets its first preview settle.
    pub fn open_on_rom(&mut self) {
        assert!(self.key(ENTER, Key::Enter));
        assert_eq!(self.run.top_kind(), KIND_LIST);
        self.settle();
    }

    /// Takes the wait away from the preview the next idle call lays out.
    pub fn skip_the_wait(&mut self) {
        let count = self.run.shell.app.label("LV_COUNT");
        self.run.m().load_ram(count, &[0]);
    }
}

/// Walks the stage the cursor is on from its first assertion to its last with the wait skipped
/// after each move, every frame the model's, and returns what the previews were.
fn walk_stage(stage: usize) -> (Counts, Counts) {
    let mut list = Lister::on_rom(stage);
    list.open_on_rom();
    let end = list.pos().len;
    let mut counts = Counts::default();
    let first =
        |list: &mut Lister, counts: &mut Counts| match (list.var("LV_PREVIEW"), list.clipped()) {
            (ROW, cut) => {
                counts.drew += 1;
                counts.cut += usize::from(cut);
            }
            (NOTHING, _) => counts.nothing += 1,
            other => panic!("{other:?}"),
        };
    first(&mut list, &mut counts);
    for n in 1..end {
        list.key(DOWN, Key::Down);
        list.skip_the_wait();
        list.settle();
        list.same_position(&format!("assertion {n}"));
        first(&mut list, &mut counts);
    }
    (counts, model_counts(list.built, end))
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_assertion_of_prop_is_the_models_with_its_preview() {
    let (seen, want) = walk_stage(PROP);
    println!(
        "prop: previews that drew {}, were cut {}, settled with nothing {}; the model's {want:?}",
        seen.drew, seen.cut, seen.nothing
    );
    assert_eq!(seen, want);
    assert!(seen.drew > 0 && seen.cut > 0, "{seen:?}");
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_assertion_of_ring_is_the_models_with_its_preview() {
    let (seen, want) = walk_stage(RING);
    println!(
        "ring: previews that drew {}, were cut {}, settled with nothing {}; the model's {want:?}",
        seen.drew, seen.cut, seen.nothing
    );
    assert_eq!(seen, want);
    assert!(seen.drew > 0 && seen.cut > 0, "{seen:?}");
}

/// The first idle call of the open list to the start of its fit on the ROM: 11 ticks.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_wait_is_11_ticks_and_a_key_inside_it_starts_it_again() {
    let mut list = Lister::on_rom(PROP);
    list.open_on_rom();
    list.count_fm_calls();
    assert!(list.key(DOWN, Key::Down));
    let fits = list.fm_calls()[2];
    let waited = list.wait_for_the_fit();
    println!(
        "ROM: from the first idle call to the fit {} interrupts, {} T-states ({:.1} ms)",
        waited.interrupts,
        waited.tstates,
        f64::from(waited.tstates) / 6000.0
    );
    assert_eq!(waited.interrupts, WAIT_TICKS);
    assert_eq!(waited.fits, fits, "no fit before the wait is over");
    assert!(waited.tstates > (WAIT_TICKS - 1) * TICK && waited.tstates <= WAIT_TICKS * TICK + 100);
    list.settle();
    assert!(list.key(DOWN, Key::Down));
    list.run.run(5 * TICK);
    let fits = list.fm_calls()[2];
    assert!(list.key(DOWN, Key::Down));
    assert_eq!(list.fm_calls()[2], fits, "no fit during the wait");
    let waited = list.wait_for_the_fit();
    assert_eq!(
        waited.interrupts, WAIT_TICKS,
        "the wait began again at the second key"
    );
}

/// A held Down on the ROM: when scr_key is called and when a fit starts, in milliseconds from the
/// press.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_held_down_repeats_and_no_fit_starts_between_repeats() {
    let mut list = Lister::on_rom(PROP);
    list.open_on_rom();
    list.count_fm_calls();
    let (fits, keys0) = (list.fm_calls()[2], list.run.scr_keys());
    let start = list.run.m().clock();
    list.run.m().press_key(DOWN);
    let (mut keys, mut fit_at) = (Vec::new(), Vec::new());
    let (mut seen_keys, mut seen_fits) = (keys0, fits);
    for _ in 0..4_000 {
        list.run.run(3_000);
        let now = (f64::from(list.run.m().clock().wrapping_sub(start))) / 6000.0;
        let k = list.run.scr_keys();
        while seen_keys < k {
            keys.push(now);
            seen_keys += 1;
        }
        let f = list.fm_calls()[2];
        while seen_fits < f {
            fit_at.push(now);
            seen_fits += 1;
        }
        if now > 1_200.0 {
            break;
        }
    }
    list.run.m().release_key(DOWN);
    let gaps: Vec<f64> = keys.windows(2).map(|w| w[1] - w[0]).collect();
    println!(
        "ROM, Down held with the list open: scr_key at {keys:.1?} ms, gaps {gaps:.1?}, fits started at {fit_at:.1?} ms"
    );
    assert!(keys.len() >= 8, "the key repeated: {keys:?}");
    let repeat = &gaps[2..];
    let (low, high) = (
        repeat.iter().cloned().fold(f64::MAX, f64::min),
        repeat.iter().cloned().fold(0.0, f64::max),
    );
    println!(
        "the repeat interval with the list open: {low:.1} to {high:.1} ms against a wait of about 102 ms"
    );
    let last = *keys.last().unwrap();
    for &at in &fit_at {
        let between = keys.windows(2).any(|w| w[0] < at && at < w[1]);
        println!(
            "a fit at {at:.1} ms is {}",
            if between {
                "between two repeats"
            } else {
                "after the last key or before the first"
            }
        );
        assert!(
            !between || high >= 102.0,
            "a fit started between two repeats of a held Down"
        );
        assert!(at > last - 1.0 || at < keys[0], "{at}");
    }
}

/// The slowest conclusion of ring, df-rng, fitted with interrupts off: a tap inside the fit is lost
/// and a key still down at its end is acted on.
fn at_df_rng() -> (Lister, usize) {
    let mut list = Lister::on_rom(RING);
    list.open_on_rom();
    let ring = crate::support::decoded(list.built.library.as_ref().unwrap());
    let n = ring
        .assertions
        .iter()
        .position(|a| a.label == "df-rng")
        .expect("df-rng");
    list.count_fm_calls();
    let frame = list.run.shell.app.label("SCR_STACK") + 16;
    let at = (n as u16).to_le_bytes();
    list.run
        .m()
        .load_ram(frame + 2, &[at[0], at[1], at[0], at[1]]);
    let (count, preview) = (
        list.run.shell.app.label("LV_COUNT"),
        list.run.shell.app.label("LV_PREVIEW"),
    );
    list.run.m().load_ram(count, &[0]);
    list.run.m().load_ram(preview, &[PENDING]);
    (list, n)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_tap_during_the_slowest_fit_is_lost_and_a_held_key_is_acted_on_after_it() {
    for held in [false, true] {
        let (mut list, _) = at_df_rng();
        let fits = list.fm_calls()[2];
        let mut spent = 0;
        while list.fm_calls()[2] == fits {
            list.run.run(600);
            spent += 600;
            assert!(spent < 3_000_000, "the fit never started");
        }
        let (keys, cursor) = (list.run.scr_keys(), list.frame_bytes());
        let start = list.run.m().clock();
        list.run.m().press_key(DOWN);
        if held {
            while list.var("LV_PREVIEW") == PENDING {
                list.run.run(3_000);
            }
            let fit_end = list.run.m().clock().wrapping_sub(start);
            list.run.run(600_000);
            println!(
                "ROM, Down held through the fit of df-rng: the fit took about {:.1} ms after the press, scr_key +{}",
                f64::from(fit_end) / 6000.0,
                list.run.scr_keys() - keys
            );
            list.run.m().release_key(DOWN);
            assert!(
                list.run.scr_keys() > keys,
                "the held key was acted on after the fit"
            );
        } else {
            list.run.run(300_000);
            list.run.m().release_key(DOWN);
            assert_eq!(list.var("LV_PREVIEW"), ROW);
            list.run.run(1_200_000);
            println!(
                "ROM, a tap inside the fit of df-rng: scr_key +{}",
                list.run.scr_keys() - keys
            );
            assert_eq!(list.run.scr_keys(), keys, "a tap inside the fit is lost");
            assert_eq!(list.frame_bytes(), cursor, "and the cursor did not move");
        }
    }
}

/// The T-states of the preview's idle call on every assertion of the stage the cursor is on.
fn idle_costs(stage: usize) -> Vec<u32> {
    let mut list = Lister::on_rom(stage);
    list.open_on_rom();
    let end = list.pos().len;
    let app = &mut list.run.shell.app;
    let (frame, count, preview, idle) = (
        app.label("SCR_STACK") + 16,
        app.label("LV_COUNT"),
        app.label("LV_PREVIEW"),
        app.label("list_idle"),
    );
    let mut costs = Vec::with_capacity(end);
    for n in 0..end {
        let at = (n as u16).to_le_bytes();
        app.m.load_ram(frame + 2, &[at[0], at[1], at[0], at[1]]);
        app.m.load_ram(count, &[0]);
        app.m.load_ram(preview, &[PENDING]);
        let done = app.timed(
            BASE,
            idle,
            In {
                hl: frame,
                ..In::default()
            },
        );
        assert!(done.status() <= 1);
        costs.push(done.tstates);
    }
    costs
}

fn median_and_worst(costs: &[u32]) -> (u32, u32, usize) {
    let mut sorted = costs.to_vec();
    sorted.sort_unstable();
    let worst = *sorted.last().unwrap();
    let at = costs.iter().position(|&c| c == worst).unwrap();
    (sorted[sorted.len() / 2], worst, at)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_preview_idle_call_costs_this_over_every_assertion_of_prop_and_ring() {
    for (name, stage, median_ceiling, worst_ceiling) in [
        ("prop", PROP, NO_CEILING, NO_CEILING),
        ("ring", RING, NO_CEILING, NO_CEILING),
    ] {
        let costs = idle_costs(stage);
        let (median, worst, at) = median_and_worst(&costs);
        println!(
            "ROM, list_idle's fit over {name}'s {} assertions: median {median}, worst {worst} (assertion {at}) T-states",
            costs.len()
        );
        assert!(median <= median_ceiling && worst <= worst_ceiling);
    }
}

pub const NO_CEILING: u32 = u32::MAX;

impl Lister {
    /// Creates the status table of the library through the app's own store, with `marks` as the
    /// values of the first assertions, and puts the model's marks beside it. The cursor is on `stage`
    /// of the Libraries screen.
    pub fn with_marks(&mut self, stage: usize, marks: &[u8]) {
        let (crc, count) = crate::table::header_of(self.built.library.as_ref().unwrap());
        let crc = u32::from_le_bytes(crc);
        let app = &mut self.run.shell.app;
        let open = In {
            hl: crc as u16,
            de: (crc >> 16) as u16,
            bc: count,
            ..In::default()
        };
        assert_eq!(app.far("store_open", open).status(), 0, "the table is made");
        let mut status = vec![mcui::Status::Unchecked; usize::from(count)];
        for (n, &value) in marks.iter().enumerate() {
            let set = In {
                hl: n as u16,
                bc: u16::from(value) << 8,
                ..In::default()
            };
            assert_eq!(app.far("store_set", set).status(), 0);
            status[n] = match value {
                1 => mcui::Status::Checked,
                2 => mcui::Status::Failed,
                _ => mcui::Status::Unchecked,
            };
        }
        let mut ui = crate::shellrig::model::ui(self.built, &status);
        for _ in 0..stage {
            ui.press(Key::Down);
        }
        self.ui = ui;
        self.status = status;
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

/// The budgets of the brief on the reference OS at 6 MHz: a cursor move with its frame inside
/// 50 ms, 300,000 T-states, and opening the list inside 300 ms, 1,800,000.
const MOVE_BUDGET: u32 = 300_000;
const OPEN_BUDGET: u32 = 1_800_000;

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_opening_the_list_and_a_move_with_its_frame_fit_their_budgets() {
    let mut list = Lister::on_rom(RING);
    let marks: Vec<u8> = (0..30).map(|i| [1, 1, 0, 2, 1][i % 5]).collect();
    list.with_marks(RING, &marks);
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
    list.open_on_rom();
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
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_marks_after_a_partial_check_are_the_models() {
    let mut list = Lister::on_rom(PROP);
    list.run.m().press_key(F2);
    let mut spent = 0;
    while list
        .run
        .values()
        .is_none_or(|v| v.iter().filter(|&&x| x != 0).count() < 60)
    {
        list.run.run(60_000);
        spent += 60_000;
        assert!(spent < 120_000_000, "the check never got 60 assertions in");
    }
    list.run.m().release_key(F2);
    list.tap(F1);
    list.run.run(1_200_000);
    assert_eq!(
        list.run.top_kind(),
        KIND_LIBRARIES,
        "Stop is back on Libraries"
    );
    let values = list.run.values().expect("a table");
    let done = values.iter().filter(|&&v| v != 0).count();
    assert!(
        (60..779).contains(&done),
        "the check is partial: {done} of {}",
        values.len()
    );
    list.status = crate::checkrig::status_of(&values);
    list.ui = crate::shellrig::model::ui(list.built, &list.status);
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
