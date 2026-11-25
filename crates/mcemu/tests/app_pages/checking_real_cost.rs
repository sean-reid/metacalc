//! What the checking screen costs on the reference OS with the shipped libraries, ignored in the
//! default run: the three frames of a check, whose OS lookups cost more than the stub's, and the
//! stack of the deepest idle call.

use crate::app::*;
use crate::checking_idle::{CLASSES, Class, SHELL_STACK, Sample, by_class, idle_calls};
use crate::checking_real::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shell_cost::FRAME_BUDGET;
use crate::shellrig::*;

/// The least and the most T-states scr_draw and ui_blit may take together, cold, for each frame of
/// the checking screen of prop in ring's build, a little under and a little above what they take
/// on the reference OS.
const ROM_FRAME_READING: (u32, u32) = (128_800, 133_900);
const ROM_FRAME_STEPS: (u32, u32) = (152_800, 158_900);
const ROM_FRAME_FINISHED: (u32, u32) = (126_600, 131_700);
/// The least and greatest bytes an idle call of each class writes below the stack pointer
/// scr_idle was called with on the reference OS, interrupts off at the call, and the fewest calls
/// of the class in prop's check.
const ROM_DEPTHS: [(Class, (u16, u16, usize)); 6] = [
    (Class::StepStays, (42, 42, 700)),
    (Class::StepRedraws, (42, 42, 20)),
    (Class::Chunk, (14, 14, 25)),
    (Class::EndsCrcRun, (44, 44, 1)),
    (Class::Finishing, (42, 42, 1)),
    (Class::AfterFinish, (10, 10, 1)),
];

/// The T-states of scr_draw and then ui_blit for the screen on top, the driver's own taken off.
fn frame_on_the_rom(app: &mut App) -> (u32, u32) {
    let draw = app.label("scr_draw");
    let draw = app.timed(BASE, draw, In::default()).tstates;
    let blit = app.label("far_ui_blit");
    let blit = app.timed(BASE, blit, In::default()).tstates;
    (draw, blit)
}

fn cost(run: &mut Run, name: &str, (floor, ceiling): (u32, u32)) {
    let tag = run.shell.app.label("APP_STAGE");
    run.m().load_ram(tag, &[0, 0]);
    let cold = frame_on_the_rom(&mut run.shell.app);
    let warm = frame_on_the_rom(&mut run.shell.app);
    println!(
        "{name}: cold {} + {} = {}, warm {} + {} = {} T-states",
        cold.0,
        cold.1,
        cold.0 + cold.1,
        warm.0,
        warm.1,
        warm.0 + warm.1
    );
    assert!(cold.0 + cold.1 <= ceiling, "{name}: cold over {ceiling}");
    assert!(cold.0 + cold.1 >= floor, "{name}: cold under {floor}");
    assert!(ceiling <= FRAME_BUDGET);
    assert!(warm.0 + warm.1 < cold.0 + cold.1, "{name}: warm is cheaper");
}

/// A run of prop with F2 held, stopped after `frames` blits of the screen.
fn after_frames(frames: usize) -> Run {
    let built = shipped_staged();
    let mut run = launched(built, 0, &[]);
    run.m().press_key(F2);
    for _ in 0..frames {
        run.next_blit();
    }
    run
}

/// What a frame costs on the reference OS: the reading frame with chunks read, a frame of the
/// steps with a label and a failure count, and the finished frame, each cold and warm.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_frame_of_the_checking_screen_costs_what_its_ceiling_says() {
    let mut run = after_frames(4);
    assert!(run.values().is_none(), "still reading");
    cost(&mut run, "reading", ROM_FRAME_READING);
    let mut run = after_frames(1);
    while run.values().is_none() {
        run.next_blit();
    }
    run.next_blit();
    run.next_blit();
    assert!(run.settled() >= EVERY, "in the steps");
    cost(&mut run, "steps", ROM_FRAME_STEPS);
    let built = shipped_staged();
    let end = built.stages[0].end;
    let mut run = launched(built, 0, &[]);
    run_stage(&mut run, end);
    cost(&mut run, "finished", ROM_FRAME_FINISHED);
}

/// Prop's whole check on the reference OS and one call after the finish, every idle call sampled
/// with interrupts on at the call, or off when `mask` is set.
fn prop_calls(mask: bool) -> Vec<Sample> {
    let built = shipped_staged();
    let end = built.stages[0].end;
    let mut run = launched(built, 0, &[]);
    run.m().press_key(F2);
    let mut finished = None;
    let all = idle_calls(
        "prop on the reference OS",
        &mut run,
        0,
        mask,
        |_, _| {},
        |r, s, n| {
            if n == 2 {
                r.m().release_key(F2);
            }
            let Some(at) = finished else {
                if s.action == 1
                    && !s.carry
                    && r.values().is_some_and(|v| v[..end].iter().all(|&x| x != 0))
                {
                    finished = Some(n);
                    r.m().press_key(F4);
                }
                return false;
            };
            if n > at {
                r.m().release_key(F4);
            }
            n > at
        },
    );
    let at = finished.expect("the check finished");
    assert_eq!(all.len(), at + 1, "one call after the finish");
    all
}

/// Every idle call of the checking screen on the reference OS, over the whole of prop's check and
/// one call after the finish, returns with interrupts on, the speed it had and the stack balanced.
/// Run again with interrupts off at each call, so no handler's pushes are in it, each class of call
/// (a step that stays, a step that redraws, a chunk, the call that ends the CRC run, the finishing
/// step, a call after the finish) writes the depths the header gives below the stack pointer
/// scr_idle was called with.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_idle_call_keeps_the_interrupts_the_speed_and_the_stack_the_header_gives()
 {
    let with_interrupts = prop_calls(false);
    println!("{} idle calls with interrupts on", with_interrupts.len());
    let all = prop_calls(true);
    let deepest = all.iter().map(|s| s.depth).max().unwrap();
    println!(
        "{} idle calls, the deepest wrote {deepest} bytes below its stack pointer",
        all.len()
    );
    let classes = by_class(&all);
    for (class, (least, most, calls)) in &classes {
        println!("{class:?}: {calls} calls, {least} to {most} bytes");
    }
    assert_eq!(
        deepest, 44,
        "the deepest call is the one that ends the CRC run"
    );
    assert_eq!(ROM_DEPTHS.map(|c| c.0), CLASSES);
    for (class, (least, most, calls)) in ROM_DEPTHS {
        let got = classes
            .get(&class)
            .unwrap_or_else(|| panic!("no call of {class:?}"));
        assert_eq!((got.0, got.1), (least, most), "{class:?}: bytes of depth");
        assert!(
            got.2 >= calls,
            "{class:?}: {} calls, at least {calls}",
            got.2
        );
    }
    let shell: std::collections::BTreeSet<u16> =
        all.iter().map(|s| s.shell).filter(|&b| b > 0).collect();
    assert_eq!(
        shell,
        [SHELL_STACK].into(),
        "the shell's share of the depth"
    );
}
