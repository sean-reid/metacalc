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
/// The most the loop may take around app_key for a key no screen answers, scr_key included.
const IGNORED_KEY: u32 = 1_250;
/// The most bytes each code page may use: entry, draw and data.
const PAGES_PLAIN: [u32; 3] = [1_700, 15_600, 9_300];
const PAGES_LIBRARY: [u32; 3] = [1_950, 15_600, 9_300];

#[test]
fn a_frame_of_each_screen_fits_its_ceiling() {
    for (name, built, open, ceiling) in [
        ("start, no library", plain(), None, START_PLAIN),
        (
            "start, fixture library",
            with_library(),
            None,
            START_LIBRARY,
        ),
        ("About, no library", plain(), Some(F3), ABOUT),
        ("About, fixture library", with_library(), Some(F3), ABOUT),
        ("Notice", plain(), Some(F1), NOTICE),
    ] {
        let mut app = App::boot(built);
        if let Some(open) = open {
            tap(&mut app, open);
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
    }
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
