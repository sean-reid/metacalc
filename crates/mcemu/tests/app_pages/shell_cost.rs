//! What the shell's frames and keys cost on the real app, printed on every passing run, and the
//! sizes of the three code pages. The frames are timed by a RAM driver standing on the entry page
//! after the app has reached its key wait (see `App::call`), on an app with no hook in it. The
//! loop's own cost for a key is timed with the OS's key wait replaced by a routine that returns a
//! key at once.

use crate::app::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Stop;

const TAP: u32 = 120_000;
const HOOK: u16 = 0xA640;
const COUNT: u16 = 0xA660;
/// The OS's code for the left arrow, which every screen ignores, and scr_key's own for KEY_LEFT.
const LEFT_CODE: u8 = 0x02;

fn driver_cost(app: &mut App) -> u32 {
    app.m.load_ram(SCRATCH - 16, &[0xC9]);
    app.call(BASE, SCRATCH - 16, In::default()).tstates
}

/// The T-states of scr_draw and then ui_blit for the screen on top, the driver's own taken off.
fn frame(app: &mut App) -> (u32, u32) {
    let driver = driver_cost(app);
    let draw = app.label("scr_draw");
    let draw = app.call(BASE, draw, In::default()).tstates - driver;
    let blit = app.far("far_ui_blit", In::default()).tstates - driver;
    (draw, blit)
}

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(4 * TAP).stop, Stop::Limit);
}

/// The most T-states scr_draw and ui_blit may take together for each screen, a little above what
/// they take on the fixture library.
const START_PLAIN: u32 = 100_000;
const START_LIBRARY: u32 = 113_000;
const ABOUT: u32 = 141_000;
const NOTICE: u32 = 113_000;
/// The same for the cold frame, the first one after a screen opens or after a power down, which
/// copies the screen's strings into APP_STAGE first. That is the frame a user sees first.
const START_PLAIN_COLD: u32 = 100_000;
const START_LIBRARY_COLD: u32 = 113_200;
const ABOUT_COLD: u32 = 141_900;
const NOTICE_COLD: u32 = 114_100;
/// The most the loop may take around app_key for a key no screen answers, scr_key included.
const IGNORED_KEY: u32 = 1_250;
/// The most app_stage_block may take for a block the area already holds.
const WARM_STAGE: u32 = 120;
/// The most bytes each code page may use: entry, draw and data.
const PAGES_PLAIN: [u32; 3] = [1_925, 15_000, 9_250];
const PAGES_LIBRARY: [u32; 3] = [2_190, 15_000, 9_250];

/// Clears the staging tag, so that the next draw copies its screen's strings.
fn go_cold(app: &mut App) {
    let tag = app.label("APP_STAGE_TAG");
    app.m.load_ram(tag, &[0, 0]);
}

/// The frame of each screen when its strings are staged (warm) and when the staging tag is cleared
/// first (cold). Opening a screen, or the refresh after a power down, draws it cold; every redraw
/// after that is warm.
#[test]
fn a_frame_of_each_screen_fits_its_warm_and_its_cold_ceiling() {
    for (name, built, open, ceiling, cold_ceiling) in [
        (
            "start, no library",
            plain(),
            None,
            START_PLAIN,
            START_PLAIN_COLD,
        ),
        (
            "start, fixture library",
            with_library(),
            None,
            START_LIBRARY,
            START_LIBRARY_COLD,
        ),
        ("About, no library", plain(), Some(F3), ABOUT, ABOUT_COLD),
        (
            "About, fixture library",
            with_library(),
            Some(F3),
            ABOUT,
            ABOUT_COLD,
        ),
        ("Notice", plain(), Some(F1), NOTICE, NOTICE_COLD),
        ("Notice, Verify", plain(), Some(F2), NOTICE, NOTICE_COLD),
    ] {
        let mut app = App::boot(built);
        if let Some(open) = open {
            tap(&mut app, open);
            let depth = app.label("scr_count");
            assert_eq!(app.m.read_ram(depth, 1), [2], "{name}: the screen opened");
        }
        let (draw, blit) = frame(&mut app);
        println!(
            "{name}: scr_draw {draw} and ui_blit {blit}: {} T-states",
            draw + blit
        );
        assert!(
            draw + blit <= ceiling,
            "{name}: {} over {ceiling}",
            draw + blit
        );
        go_cold(&mut app);
        let (cold_draw, cold_blit) = frame(&mut app);
        println!(
            "{name}: cold scr_draw {cold_draw} and ui_blit {cold_blit}: {} T-states, {} over the warm frame",
            cold_draw + cold_blit,
            cold_draw + cold_blit - draw - blit
        );
        assert!(
            cold_draw + cold_blit <= cold_ceiling,
            "{name}: cold {} over {cold_ceiling}",
            cold_draw + cold_blit
        );
    }
}

/// What staging a block that is already there costs, the driver's call and halt taken off.
#[test]
fn staging_a_block_the_area_holds_costs_little() {
    let mut app = App::boot(plain());
    let at = app.label("app_stage_block");
    let input = In {
        hl: app.label("start_block"),
        bc: app.label("START_BLOCK_SIZE"),
        ..In::default()
    };
    let driver = driver_cost(&mut app);
    let warm = app.call(BASE, at, input).tstates - driver;
    go_cold(&mut app);
    let cold = app.call(BASE, at, input).tstates - driver;
    println!("app_stage_block: {warm} T-states when the block is staged, {cold} when it is copied");
    assert!(warm <= WARM_STAGE, "{warm} over {WARM_STAGE}");
}

/// `ei`, then `bcall(_GetKey)`: the three bytes after app_key's `ei` become a call to a routine
/// that counts, turns interrupts off and returns the left arrow's code.
fn spinning(built: &'static Built) -> App {
    App::boot_with(built, |m| {
        let [c0, c1] = COUNT.to_le_bytes();
        let hook = [
            0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0xF3, 0x3E, LEFT_CODE, 0xC9,
        ];
        m.load_ram(HOOK, &hook);
        m.load_ram(COUNT, &[0, 0]);
        let [h0, h1] = HOOK.to_le_bytes();
        let at = usize::from(built.label("ak_wait") - 0x4000) + 1;
        m.load_flash(BASE, at, &[0xCD, h0, h1]);
    })
}

#[test]
fn a_frame_of_the_error_notice_fits_its_ceiling() {
    let mut app = App::boot(plain());
    tap(&mut app, F1);
    let at = app.label("SCR_STACK") + 16;
    app.m.load_ram(at + 1, &[2, 0x5A]);
    let enter = app.label("notice_enter");
    app.call(
        BASE,
        enter,
        In {
            hl: at,
            ..In::default()
        },
    );
    let (draw, blit) = frame(&mut app);
    println!(
        "Notice, error: scr_draw {draw} and ui_blit {blit}: {} T-states",
        draw + blit
    );
    assert!(draw + blit <= NOTICE, "{} over {NOTICE}", draw + blit);
    go_cold(&mut app);
    let (draw, blit) = frame(&mut app);
    println!(
        "Notice, error: cold scr_draw {draw} and ui_blit {blit}: {} T-states",
        draw + blit
    );
    assert!(
        draw + blit <= NOTICE_COLD,
        "cold {} over {NOTICE_COLD}",
        draw + blit
    );
}

#[test]
fn the_loop_around_a_key_no_screen_answers_costs_little() {
    for (name, built) in [("no library", plain()), ("fixture library", with_library())] {
        let mut app = spinning(built);
        let blits_before = app.frame();
        app.m.load_ram(COUNT, &[0, 0]);
        let window = 2_000_000;
        let run = app.m.run_app(window);
        assert_eq!(run.stop, Stop::Limit);
        let count = u32::from(u16::from_le_bytes(
            app.m.read_ram(COUNT, 2).try_into().unwrap(),
        ));
        assert!(count > 100, "{name}: the loop ran {count} times");
        let per_key = run.tstates / count;
        assert_eq!(app.frame(), blits_before, "ignored keys draw nothing");
        let driver = driver_cost(&mut app);
        let key = app.label("app_key");
        let wait = app.call(BASE, key, In::default()).tstates - driver;
        let overhead = per_key - wait;
        println!(
            "{name}: an ignored key takes {per_key} T-states from one app_key to the next, \
             {wait} of them in app_key and its stand-in for the OS, {overhead} in the loop and scr_key"
        );
        assert!(
            overhead <= IGNORED_KEY,
            "{name}: {overhead} over {IGNORED_KEY}"
        );
    }
}

#[test]
fn the_code_pages_stay_within_their_ceilings() {
    for (name, built, ceilings) in [
        ("no library", plain(), PAGES_PLAIN),
        ("fixture library", with_library(), PAGES_LIBRARY),
    ] {
        println!("{name}: code pages {:?} of 16384 bytes", built.used);
        assert_eq!(built.used.len(), 3);
        for (page, (used, ceiling)) in built.used.iter().zip(ceilings).enumerate() {
            assert!(
                *used <= ceiling,
                "{name}: page {page} uses {used}, over {ceiling}"
            );
        }
    }
}
