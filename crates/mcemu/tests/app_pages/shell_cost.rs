//! What the shell's frames and keys cost on the real app, printed on every passing run, and the
//! sizes of the three code pages. Every call is made by a RAM driver standing on the entry page
//! after the app has reached its key wait (see `App::call`).

use crate::app::*;
use crate::realos::BASE;
use crate::shellrig::*;

/// The key code of KEY_DOWN, which no screen answers.
const KEY_DOWN: u8 = 2;

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

/// The most T-states scr_draw and ui_blit may take together for each screen, a little above what
/// they take on the fixture library.
const START_PLAIN: u32 = 100_000;
const START_LIBRARY: u32 = 113_000;
const ABOUT: u32 = 141_000;
const NOTICE: u32 = 113_000;
/// The most scr_key may take for a key no screen answers, over the driver's own call.
const IGNORED_KEY: u32 = 700;

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
        let mut shell = Shell::boot(built, Enter::Real);
        if let Some(open) = open {
            shell.tap(open);
        }
        let (draw, blit) = frame(&mut shell.app);
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

#[test]
fn a_key_no_screen_answers_costs_little_in_the_stack() {
    for (name, built) in [("no library", plain()), ("fixture library", with_library())] {
        let mut shell = Shell::boot(built, Enter::Real);
        let driver = driver_cost(&mut shell.app);
        let key = shell.app.label("scr_key");
        let cost = shell
            .app
            .call(
                BASE,
                key,
                In {
                    a: KEY_DOWN,
                    ..In::default()
                },
            )
            .tstates
            - driver;
        println!(
            "{name}: scr_key for a key no screen answers: {cost} T-states over the driver's call"
        );
        assert!(cost <= IGNORED_KEY, "{name}: {cost} over {IGNORED_KEY}");
    }
}

#[test]
fn the_code_pages_stay_within_their_bytes() {
    for (name, built) in [("no library", plain()), ("fixture library", with_library())] {
        println!("{name}: code pages {:?} of 16384 bytes", built.used);
        assert_eq!(built.used.len(), 3);
        assert!(built.used.iter().all(|&n| n <= 16384));
    }
}
