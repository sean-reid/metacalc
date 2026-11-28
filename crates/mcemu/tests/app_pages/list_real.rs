//! The list on the reference OS with the shipped libraries, ignored in the default run: every
//! assertion of prop and of ring walked against the model, the wait and the key repeat, a key during
//! the slowest fit, what each step costs against its budget, and what a user does with a list.

use crate::app::*;
use crate::checking_real::launched;
use crate::checkrig::{Run, WAIT};
use crate::list_depth::Trace;
use crate::list_exits::{caps_built, index_of};
use crate::list_preview::{Counts, model_counts};
use crate::list_rig::*;
use crate::list_wait::{TICK, WAIT_TICKS};
use crate::realos::BASE;
use crate::shellrig::*;
use mcui::Key;

/// Prop, pred and ring are the three stages of the shipped build.
pub const PROP: usize = 0;
pub const RING: usize = 2;

impl Lister {
    /// The shipped build on the reference OS at its Libraries screen with the cursor on `stage`, and
    /// the model beside it, whose marks are `status` (none: every assertion unchecked).
    pub fn on_rom(stage: usize) -> Lister {
        Lister::on_rom_of(shipped_staged(), stage)
    }

    /// `on_rom` for the app in `built`.
    pub fn on_rom_of(built: &'static Built, stage: usize) -> Lister {
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
            log: vec![Key::Down; stage],
            seen: (0, 0),
            title_from: None,
            stale_titles: 0,
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
        self.run.shell.app.poke(count, &[0]);
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
    assert!(seen.drew > 0, "{seen:?}");
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
    assert!(seen.drew > 0, "{seen:?}");
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
    within("the wait to the fit", waited.tstates, 576_754);
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

/// One step on ring with the wait as it is, not skipped: the frame after the idle work is the model's.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_one_step_on_ring_with_the_wait_run_in_full_is_the_models_frame() {
    let mut list = Lister::on_rom(RING);
    list.open_on_rom();
    assert!(list.key(DOWN, Key::Down));
    let waited = list.wait_for_the_fit();
    assert_eq!(waited.interrupts, WAIT_TICKS, "the wait ran its 11 ticks");
    assert!(list.settle(), "the preview of the next assertion is drawn");
    list.same_position("one step on ring");
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
    let repeat = &gaps[1..];
    let (low, high) = (
        repeat.iter().cloned().fold(f64::MAX, f64::min),
        repeat.iter().cloned().fold(0.0, f64::max),
    );
    println!(
        "the repeat interval with the list open: {low:.1} to {high:.1} ms, after a first gap of {:.1} ms; a wait is 11 ticks, about 102 ms",
        gaps[0]
    );
    assert!(high < 100.0, "the repeat is faster than the wait");
    let last = *keys.last().unwrap();
    for &at in &fit_at {
        let between = keys[1..].windows(2).any(|w| w[0] < at && at < w[1]);
        println!(
            "a fit at {at:.1} ms is {}",
            if between {
                "between two repeats"
            } else if at < keys[1] {
                "in the first gap, before the key repeats"
            } else {
                "after the last key"
            }
        );
        assert!(!between, "a fit started between two repeats of a held Down");
        assert!(at < keys[1] || at > last, "{at}");
    }
    assert_eq!(
        fit_at.len(),
        1,
        "the one fit of the first key, in the first gap, and none after it while the key is held"
    );
    let fits = list.fm_calls()[2];
    list.run.run(1_200_000);
    assert_eq!(
        list.fm_calls()[2],
        fits + 1,
        "one fit once the key is let go"
    );
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
    list.run.shell.app.poke(count, &[0]);
    list.run.shell.app.poke(preview, &[PENDING]);
    let quiet = list.quiet_key();
    list.tap_quiet(quiet);
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
            assert_eq!(
                list.run.scr_keys(),
                keys,
                "no key was acted on while the fit ran"
            );
            list.run.run(600_000);
            println!(
                "ROM, Down held through the fit of df-rng: the fit took about {:.1} ms after the press, scr_key +{}",
                f64::from(fit_end) / 6000.0,
                list.run.scr_keys() - keys
            );
            list.run.m().release_key(DOWN);
            assert_eq!(
                list.run.scr_keys(),
                keys + 1,
                "the held key was acted on once, right after the fit"
            );
            let moved = list.frame_bytes();
            assert_eq!(
                u16::from_le_bytes([moved[2], moved[3]]),
                u16::from_le_bytes([cursor[2], cursor[3]]) + 1,
                "and the cursor moved one row"
            );
        } else {
            list.run.run(300_000);
            list.run.m().release_key(DOWN);
            assert_eq!(
                list.var("LV_PREVIEW"),
                PENDING,
                "the key was let go inside the fit"
            );
            while list.var("LV_PREVIEW") == PENDING {
                list.run.run(30_000);
            }
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
        app.poke(frame + 2, &[at[0], at[1], at[0], at[1]]);
        app.poke(count, &[0]);
        app.poke(preview, &[PENDING]);
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
    for (name, stage, median_measured, worst_measured, worst_at) in [
        ("prop", PROP, 68_891, 331_490, 138),
        ("ring", RING, 77_198, 1_125_910, 3_307),
    ] {
        let costs = idle_costs(stage);
        let (median, worst, at) = median_and_worst(&costs);
        println!(
            "ROM, list_idle's fit over {name}'s {} assertions: median {median}, worst {worst} (assertion {at}) T-states",
            costs.len()
        );
        within(&format!("{name}'s median"), median, median_measured);
        within(&format!("{name}'s worst"), worst, worst_measured);
        assert_eq!(at, worst_at, "{name}: the slowest assertion");
    }
}

/// The idle call at its entry with the wait over on a conclusion fm refuses, run at `speed` to its
/// return: fm_fit must answer FM_ERR_LIMIT, then the call is as `nothing_settled` has it.
fn idle_over_the_caps(list: &mut Lister, at: &str, speed: u8) {
    list.run_to_the_work();
    let blits = list.run.blits();
    list.run.m().out_port(0x20, speed);
    let mut regs = list.run.m().regs();
    regs.af = 0x77D7;
    list.run.m().set_regs(&regs);
    let sp = regs.sp;
    let before = (regs.iff1, list.run.m().in_port(0x20) & 1);
    let fit = list.run.shell.app.label("far_fm_fit");
    list.run.run_to_addr("far_fm_fit", fit, WAIT);
    let ret = list.run.m().read_ram(sp - 2, 2);
    let ret = u16::from_le_bytes([ret[0], ret[1]]);
    list.run.run_to_addr("the return of far_fm_fit", ret, WAIT);
    let limit = list.built.label("FM_ERR_LIMIT") as u8;
    assert_eq!(
        list.run.m().regs().af >> 8,
        u16::from(limit),
        "{at}: fm_fit answered FM_ERR_LIMIT"
    );
    let back = list.run.shell.app.label("scr_idle") + 5;
    list.run
        .run_to_addr(&format!("the return of list_idle, {at}"), back, WAIT);
    let regs = list.run.m().regs();
    assert_eq!(regs.sp, sp + 2, "{at}: the stack is balanced");
    let t = Trace {
        depth: 0,
        iff: (before.0, regs.iff1),
        speed: (before.1, list.run.m().in_port(0x20) & 1),
        action: (regs.af >> 8) as u8,
        carry: regs.af & 1 != 0,
    };
    list.run.m().out_port(0x20, 0);
    list.nothing_settled(at, speed, blits, &t);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_conclusion_over_the_caps_settles_with_nothing_and_the_idle_call_returns_as_it_was_entered()
 {
    let built = caps_built();
    let mut list = Lister::on_rom_of(built, 0);
    list.open_on_rom();
    let (at, over, last) = (
        index_of(built, "capat"),
        index_of(built, "capov1"),
        index_of(built, "capov2"),
    );
    assert_eq!((over, last), (at + 1, at + 2));
    for _ in 0..at {
        assert!(list.key(DOWN, Key::Down));
        list.skip_the_wait();
        list.settle();
    }
    assert_eq!(
        list.var("LV_PREVIEW"),
        ROW,
        "the conclusion at the cap is drawn"
    );
    assert!(list.key(DOWN, Key::Down));
    idle_over_the_caps(&mut list, "one over the cap, 6 MHz", 0);
    assert!(!list.settle(), "the model draws nothing either");
    assert!(list.key(DOWN, Key::Down));
    idle_over_the_caps(&mut list, "two over the cap, 15 MHz", 1);
    assert!(!list.settle());
    assert_eq!(list.pos().cursor, last);
}
