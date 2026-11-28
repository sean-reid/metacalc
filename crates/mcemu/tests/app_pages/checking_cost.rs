//! What a frame of the checking screen costs on the stub OS: the cold cost between a floor and a
//! ceiling and the exact saving of a warm frame.

use crate::checking_errors::damaged;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;

/// The least and the most T-states scr_draw and ui_blit may take together for each frame of the
/// checking screen, cold (the strings copied to APP_STAGE first), a little under and a little above
/// what they take, and what the warm frame saves.
const FRAME_READING: (u32, u32) = (128_784, 133_184);
const FRAME_STEPS: (u32, u32) = (147_798, 153_298);
const FRAME_FINISHED: (u32, u32) = (123_834, 128_684);
const FRAME_DAMAGED: (u32, u32) = (127_090, 132_140);
const SAVED_WARM: u32 = 1_921;
const SAVED_WARM_DAMAGED: u32 = 2_641;

/// What one frame costs, printed on every run and held to a floor and a ceiling: the reading frame,
/// a frame of the steps with a label and a failure, the finished frame and the damaged notice, each
/// cold and warm.
#[test]
fn a_frame_of_the_checking_screen_costs_what_its_floor_and_ceiling_say() {
    use crate::shell_cost::{FRAME_BUDGET, frame};
    let stage = |run: &mut Run, name: &str, (floor, ceiling): (u32, u32), saved: u32| {
        let tag = run.shell.app.label("APP_STAGE");
        run.m().load_ram(tag, &[0, 0]);
        let cold = frame(&mut run.shell.app);
        let warm = frame(&mut run.shell.app);
        println!(
            "{name}: cold {} + {} = {}, warm {} + {} = {} T-states",
            cold.0,
            cold.1,
            cold.0 + cold.1,
            warm.0,
            warm.1,
            warm.0 + warm.1
        );
        let (cold, warm) = (cold.0 + cold.1, warm.0 + warm.1);
        assert!(cold <= ceiling, "{name}: cold over {ceiling}");
        assert!(cold >= floor, "{name}: cold under {floor}");
        assert!(ceiling <= FRAME_BUDGET);
        assert_eq!(cold - warm, saved, "{name}: what the warm frame saves");
    };
    let mut run = Run::boot(padded(), None);
    run.m().press_key(F2);
    for _ in 0..4 {
        run.next_blit();
    }
    stage(&mut run, "reading", FRAME_READING, SAVED_WARM);
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    for _ in 0..2 {
        run.next_blit();
    }
    stage(&mut run, "steps", FRAME_STEPS, SAVED_WARM);
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    loop {
        run.next_blit();
        if run.values_here().is_some_and(|v| v.iter().all(|&x| x != 0)) {
            break;
        }
    }
    stage(&mut run, "finished", FRAME_FINISHED, SAVED_WARM);
    let mut run = Run::boot(damaged(false), None);
    run.m().press_key(F2);
    while run.top_kind() != KIND_NOTICE {
        run.next_blit();
    }
    stage(
        &mut run,
        "damaged notice",
        FRAME_DAMAGED,
        SAVED_WARM_DAMAGED,
    );
}
