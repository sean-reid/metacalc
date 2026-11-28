//! What the checking screen costs on the reference OS with the shipped libraries, ignored in the
//! default run: the three frames of a check, whose OS lookups cost more than the stub's, and the
//! bytes each class of idle call writes below the stack pointer, with interrupts off at the call
//! and with them on.

use crate::app::*;
use crate::checking_depth::{
    Class, Fill, LEAVES, SHELL_STACK, Sample, agreed, assert_stop_path, by_class, idle_calls,
    inside_by_class, live_depths,
};
use crate::checking_real::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shell_cost::FRAME_BUDGET;
use crate::shellrig::*;
use crate::timer_spacing::REFERENCE_STACK;
use mcemu::Machine;

/// The least and the most T-states scr_draw and ui_blit may take together, cold, for each frame of
/// the checking screen of prop in ring's build, a little under and a little above what they take
/// on the reference OS.
const ROM_FRAME_READING: (u32, u32) = (133_719, 138_819);
const ROM_FRAME_STEPS: (u32, u32) = (157_719, 163_819);
const ROM_FRAME_FINISHED: (u32, u32) = (131_519, 136_619);
/// The least and greatest bytes an idle call of each class writes below the stack pointer
/// scr_idle was called with on the reference OS, interrupts off at the call, and the fewest calls
/// of the class in prop's check, exactly for the classes that leave the screen.
const ROM_DEPTHS: [(Class, (u16, u16, usize)); 8] = [
    (Class::StepStays, (42, 42, 700)),
    (Class::StepRedraws, (42, 42, 20)),
    (Class::Chunk, (14, 14, 25)),
    (Class::EndsCrcRun, (44, 44, 1)),
    (Class::Finishing, (42, 42, 1)),
    (Class::AfterFinish, (10, 10, 1)),
    (Class::StopOn, (40, 40, 1)),
    (Class::PausedRead, (40, 40, 1)),
];

/// The depth of each class when checking_idle returns, before the shell acts on its answer.
const ROM_INSIDE: [(Class, [u16; 1]); 8] = [
    (Class::StepStays, [42]),
    (Class::StepRedraws, [42]),
    (Class::Chunk, [14]),
    (Class::EndsCrcRun, [44]),
    (Class::Finishing, [42]),
    (Class::AfterFinish, [10]),
    (Class::StopOn, [16]),
    (Class::PausedRead, [16]),
];

/// Microseconds the runs wait after the launch before the first key, a quarter of the timer period
/// of 1953 apart, which moves where the timer interrupt falls in the calls that follow.
const LEADS_US: [u32; 3] = [488, 977, 1465];

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
    run.shell.app.poke(tag, &[0, 0]);
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
fn prop_calls(mask: bool, fill: Fill, lead_us: u32) -> Vec<Sample> {
    let built = shipped_staged();
    let end = built.stages[0].end;
    let mut run = launched(built, 0, &[]);
    run.step_us(lead_us);
    run.m().press_key(F2);
    let mut finished = None;
    let all = idle_calls(
        "prop on the reference OS",
        &mut run,
        0,
        mask,
        fill,
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

/// The idle calls of two stops on the reference OS: ON down at the idle point, and ON down as the
/// kernel is called, which the job answers with CK_PAUSED. Both pop the screen, whichever call they
/// land in.
fn stop_calls(mask: bool, fill: Fill, lead_us: u32) -> Vec<Sample> {
    let mut all = Vec::new();
    for paused in [false, true] {
        let built = shipped_staged();
        let mut run = launched(built, 0, &[]);
        run.step_us(lead_us);
        run.m().press_key(F2);
        let calls = idle_calls(
            "ON on the reference OS",
            &mut run,
            0,
            mask,
            fill,
            |r, n| {
                if paused && n == 4 {
                    r.m().press_key(Machine::KEY_ON);
                }
            },
            |r, _, n| {
                if n == 2 {
                    r.m().release_key(F2);
                }
                if !paused && n == 3 {
                    r.m().press_key(Machine::KEY_ON);
                }
                n > 3 && r.depth() == 1
            },
        );
        assert_eq!(
            calls.last().map(|s| (s.action, s.carry, s.stays)),
            Some((1, true, false)),
            "ON popped the screen, paused {paused}"
        );
        assert_stop_path("ON on the reference OS", &calls, !paused);
        all.extend(calls);
    }
    all
}

/// Every idle call of the checking screen on the reference OS, over the whole of prop's check and
/// one call after the finish, returns with interrupts on, the speed it had and the stack balanced.
/// Run again with interrupts off at each call, so no handler's pushes are in it, each class of call
/// (a step that stays, a step that redraws, a chunk, the call that ends the CRC run, the finishing
/// step, a call after the finish, and the two ON stops) writes the depths the header gives below the
/// stack pointer scr_idle was called with. Run with interrupts on, the greatest of each class is
/// deeper where an interrupt landed in the call.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_every_idle_call_keeps_the_interrupts_the_speed_and_the_stack_the_header_gives()
 {
    let calls = |mask, f, lead| [prop_calls(mask, f, lead), stop_calls(mask, f, lead)].concat();
    let [live_odd, live_inverse] = Fill::BOTH.map(|f| calls(false, f, 0));
    let live = agreed("interrupts on", &live_odd, &live_inverse);
    let [odd, inverse] = Fill::BOTH.map(|f| calls(true, f, 0));
    let all = agreed("interrupts off", &odd, &inverse);
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
    assert_eq!(
        classes.keys().copied().collect::<Vec<_>>(),
        ROM_DEPTHS.map(|c| c.0).to_vec(),
        "the classes of call, none unclassified"
    );
    for (class, (least, most, calls)) in ROM_DEPTHS {
        let got = classes
            .get(&class)
            .unwrap_or_else(|| panic!("no call of {class:?}"));
        assert_eq!((got.0, got.1), (least, most), "{class:?}: bytes of depth");
        if LEAVES.contains(&class) {
            assert_eq!(got.2, calls, "{class:?}: calls");
        } else {
            assert!(
                got.2 >= calls,
                "{class:?}: {} calls, at least {calls}",
                got.2
            );
        }
    }
    let inside = inside_by_class(&all);
    println!("the depth at the return of checking_idle, by class: {inside:?}");
    let expected: std::collections::BTreeMap<Class, std::collections::BTreeSet<u16>> =
        ROM_INSIDE.map(|(c, v)| (c, v.into())).into();
    assert_eq!(inside, expected, "the depth at the return of checking_idle");
    let mut swept = live.clone();
    let mut tables = vec![(0, by_class(&live))];
    for lead in LEADS_US {
        let on = calls(false, Fill::Odd, lead);
        tables.push((lead, by_class(&on)));
        swept.extend(on);
    }
    let live_most = live_depths(&all, &swept);
    for (lead, table) in &tables {
        let row: Vec<String> = table
            .iter()
            .map(|(class, (_, most, _))| format!("{class:?} {most}"))
            .collect();
        println!(
            "lead {lead} us, greatest with interrupts on: {}",
            row.join(", ")
        );
    }
    let on = by_class(&swept);
    let mut reached = Vec::new();
    for (class, most) in &live_most {
        let (least, greatest, _) = classes[class];
        let least_on = on[class].0;
        assert!(
            least_on >= least,
            "{class:?}: {least_on} bytes with interrupts on, under the least masked {least}"
        );
        assert!(
            (greatest..=greatest + REFERENCE_STACK).contains(most),
            "{class:?}: {most} bytes with interrupts on, masked {greatest}, the handler holds {REFERENCE_STACK}"
        );
        if *most == greatest + REFERENCE_STACK {
            reached.push(*class);
        }
    }
    println!("classes that reach the masked depth plus the handler: {reached:?}");
    assert!(
        !reached.is_empty(),
        "an interrupt landed at the deepest point of some class"
    );
    assert_eq!(
        live_most.len(),
        classes.len(),
        "every class was seen with interrupts on"
    );
    let shell: std::collections::BTreeSet<u16> =
        all.iter().map(|s| s.shell).filter(|&b| b > 0).collect();
    assert_eq!(
        shell,
        [SHELL_STACK].into(),
        "the shell's share of the depth"
    );
}
