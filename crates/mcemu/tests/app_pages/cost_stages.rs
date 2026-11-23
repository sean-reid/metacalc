//! What the staged builds cost beyond shell_cost.rs's fixtures: the size of their code pages, and a
//! cursor move taken with the frame it redraws. Figures are printed on every passing run.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shell_cost::*;

/// The most bytes the three code pages (entry, draw and data) may use in the builds around the
/// fixture library as three and as six stages, and around ring as its shipped three.
const PAGES_THREE: [u32; 3] = [4_215, 15_000, 9_250];
const PAGES_SIX: [u32; 3] = [4_290, 15_000, 9_250];
const PAGES_RING: [u32; 3] = [4_240, 15_000, 9_250];

fn fits(name: &str, built: &Built, ceilings: [u32; 3]) {
    println!("{name}: code pages {:?} of 16384 bytes", built.used);
    assert_eq!(built.used.len(), 3);
    for (page, (used, ceiling)) in built.used.iter().zip(ceilings).enumerate() {
        assert!(
            *used <= ceiling,
            "{name}: page {page} uses {used}, over {ceiling}"
        );
    }
}

#[test]
fn the_code_pages_of_the_three_and_six_stage_builds_stay_within_their_ceilings() {
    fits("three stages", three_stages(), PAGES_THREE);
    fits("six stages", six_stages(), PAGES_SIX);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_code_pages_of_ring_in_three_stages_stay_within_their_ceilings() {
    fits("ring in three stages", shipped_staged(), PAGES_RING);
}

/// The most T-states a cursor move may take with the frame it redraws, on six stages with every
/// stage under way: libs_key, then scr_draw and ui_blit, for the move that scrolls the rows and
/// for those that do not.
const MOVE_AND_REDRAW: u32 = 161_900;

#[test]
fn a_cursor_move_and_the_frame_it_redraws_fit_their_ceiling() {
    let built = six_stages();
    let data = alternating(built);
    let mut app = App::boot_with(built, |m| crate::table::install(m, &data));
    let driver = driver_cost(&mut app);
    let (key, stack) = (app.label("libs_key"), app.label("SCR_STACK"));
    let mut worst = 0;
    for step in 1..=6 {
        let key_cost = app
            .call(
                BASE,
                key,
                In {
                    a: 2,
                    hl: stack,
                    ..In::default()
                },
            )
            .tstates
            - driver;
        let (draw, blit) = frame(&mut app);
        let total = key_cost + draw + blit;
        println!(
            "cursor move {step}: libs_key {key_cost} and the redraw {}: {total} T-states",
            draw + blit
        );
        assert!(total < FRAME_BUDGET, "move {step}: over the frame budget");
        worst = worst.max(total);
    }
    assert!(worst <= MOVE_AND_REDRAW, "{worst} over {MOVE_AND_REDRAW}");
}
