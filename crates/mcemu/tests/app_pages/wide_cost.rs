//! What the 12,012 assertion library costs the Libraries screen: its enter, which makes one count
//! pass over a 3,013 byte status table, and its widest frames, whose rows read "checked/total" in
//! eleven characters. Figures are printed on every passing run. The enter is not a frame and is not
//! held to FRAME_BUDGET; the frames are.

use crate::app::*;
use crate::numbers::{one_short, wide, wide_stages};
use crate::realos::BASE;
use crate::shell_cost::*;
use crate::table;
use mcemu::Stop;

const TAP: u32 = 120_000;
/// The most T-states libs_enter may take with the status table installed.
const ENTER_WIDE: u32 = 731_000;
/// The most T-states scr_draw and ui_blit may take on the Libraries frame, warm and cold, and on a
/// stage's About frame, warm and cold.
const LIBRARIES_WIDE: u32 = 165_340;
const LIBRARIES_WIDE_COLD: u32 = 166_740;
const STAGE_ABOUT_WIDE: u32 = 149_597;
const STAGE_ABOUT_WIDE_COLD: u32 = 149_597;

fn data() -> Vec<u8> {
    let built = wide_stages();
    table::of(wide(), &table::values(built, &one_short()))
}

/// The T-states of the routine at `label` called with `hl` on the top frame, the driver's own
/// taken off.
fn cost_of(app: &mut App, label: &str, hl: u16) -> u32 {
    let driver = driver_cost(app);
    let at = app.label(label);
    let done = app.call(
        BASE,
        at,
        In {
            hl,
            ..In::default()
        },
    );
    assert_eq!(done.status(), 0, "{label} succeeds");
    done.tstates - driver
}

#[test]
fn the_libraries_enter_on_the_wide_library_fits_its_ceiling() {
    let (built, data) = (wide_stages(), data());
    let mut app = App::boot_with(built, |m| table::install(m, &data));
    let frame = app.label("SCR_STACK");
    let enter = cost_of(&mut app, "libs_enter", frame);
    let counts = cost_of(&mut app, "libs_counts", 0);
    let mut bare = App::boot(built);
    let untabled = cost_of(&mut bare, "libs_counts", 0);
    println!(
        "Libraries enter, 12,012 assertions in three stages, a table: {enter} T-states, \
         {:.1} ms at 6 MHz, {:.1} ms at 15 MHz; libs_counts {counts} of them, {untabled} with no table \
         so the count pass is {}, the rest of the enter {}",
        f64::from(enter) / 6_000.0,
        f64::from(enter) / 15_000.0,
        counts - untabled,
        enter - counts
    );
    assert!(enter <= ENTER_WIDE, "{enter} over {ENTER_WIDE}");
}

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(40 * TAP).stop, Stop::Limit);
}

/// The warm and the cold frame of the screen `open` reaches, each as scr_draw and ui_blit.
fn frames(open: &[u8], depth: u8) -> (u32, u32) {
    let (built, data) = (wide_stages(), data());
    let mut app = App::boot_with(built, |m| table::install(m, &data));
    for &key in open {
        tap(&mut app, key);
    }
    let at = app.label("scr_count");
    assert_eq!(app.peek(at, 1), [depth], "the screen opened");
    let (draw, blit) = frame(&mut app);
    go_cold(&mut app);
    let (cold_draw, cold_blit) = frame(&mut app);
    (draw + blit, cold_draw + cold_blit)
}

#[test]
fn the_widest_frames_fit_the_budget_and_their_ceilings() {
    let down = 0x01;
    let f3 = 0x33;
    let (libs, libs_cold) = frames(&[], 1);
    let (about, about_cold) = frames(&[down, down, f3], 2);
    for (name, warm, cold, ceiling, cold_ceiling) in [
        (
            "Libraries, 12,012 assertions, rows of eleven characters",
            libs,
            libs_cold,
            LIBRARIES_WIDE,
            LIBRARIES_WIDE_COLD,
        ),
        (
            "Stage About, third stage, a table",
            about,
            about_cold,
            STAGE_ABOUT_WIDE,
            STAGE_ABOUT_WIDE_COLD,
        ),
    ] {
        println!("{name}: {warm} T-states warm, {cold} cold");
        assert!(warm <= ceiling, "{name}: {warm} over {ceiling}");
        assert!(
            cold <= cold_ceiling,
            "{name}: cold {cold} over {cold_ceiling}"
        );
        assert!(warm < FRAME_BUDGET, "{name}: over the frame budget");
        assert!(cold < FRAME_BUDGET, "{name}: cold, over the frame budget");
    }
}
