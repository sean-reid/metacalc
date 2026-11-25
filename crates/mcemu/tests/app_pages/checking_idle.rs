//! The interrupt state, speed and stack of every idle call of the checking screen, and what a frame
//! of it costs.

use crate::app::*;
use crate::checking_errors::damaged;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Machine;

/// What one idle call returned: A, the carry, and the bytes it wrote below the stack pointer it was
/// called with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub action: u8,
    pub carry: bool,
    pub depth: u16,
}

/// Runs the shell's idle calls one at a time: at the call of scr_idle fills the 256 bytes below the
/// stack pointer, and at its return checks that interrupts are on and the speed port is `speed`,
/// and measures how deep the call wrote. `each` sees every sample and may change the machine for
/// the next call; it answers true to stop.
pub fn idle_calls(
    what: &str,
    run: &mut Run,
    speed: u8,
    mut before: impl FnMut(&mut Run, usize),
    mut each: impl FnMut(&mut Run, &Sample, usize) -> bool,
) -> Vec<Sample> {
    let work = run.shell.app.label("app_work");
    let call = work + 5;
    let back = work + 8;
    let page = run
        .shell
        .app
        .m
        .read_flash(BASE, usize::from(call - 0x4000), 3);
    assert_eq!(
        page[0], 0xCD,
        "app_work + 5 is the call of scr_idle: {page:02X?}"
    );
    let mut all = Vec::new();
    loop {
        run.run_to_addr(
            &format!("{what}, after {} calls: the call of scr_idle", all.len()),
            call,
            WAIT,
        );
        before(run, all.len() + 1);
        let sp = run.m().regs().sp;
        let kept = run.m().read_ram(sp - 256, 256);
        let fill: Vec<u8> = (0..256u16).map(|i| (i as u8).wrapping_mul(7) | 1).collect();
        run.m().load_ram(sp - 256, &fill);
        run.run_to_addr("the return of scr_idle", back, WAIT);
        let regs = run.m().regs();
        assert_eq!(regs.iff1, 1, "idle call {}: interrupts are on", all.len());
        assert_eq!(
            run.m().in_port(0x20) & 1,
            speed,
            "idle call {}: the speed the shell had",
            all.len()
        );
        assert_eq!(
            regs.sp,
            sp,
            "idle call {}: the stack is balanced",
            all.len()
        );
        let after = run.m().read_ram(sp - 256, 256);
        run.m().load_ram(sp - 256, &kept);
        let lowest = after
            .iter()
            .zip(&fill)
            .position(|(a, f)| a != f)
            .map_or(0, |i| 256 - i as u16);
        let sample = Sample {
            action: (regs.af >> 8) as u8,
            carry: regs.af & 1 != 0,
            depth: lowest,
        };
        all.push(sample);
        run.m().resume(1);
        if each(run, &sample, all.len()) {
            return all;
        }
        assert!(all.len() < 5_000, "the scenario never ended");
    }
}

impl Run {
    /// Runs until the instruction at `at` on the app's page is next.
    pub fn run_to_addr(&mut self, what: &str, at: u16, limit: u32) {
        let m = self.m();
        let start = m.clock();
        loop {
            match m.run_to(BASE, at, limit) {
                mcemu::Reached::Target => return,
                mcemu::Reached::Vector38 => {}
                mcemu::Reached::Limit => panic!("{what}: the app never reached {at:04X}"),
            }
            assert!(
                m.clock().wrapping_sub(start) < limit,
                "{what}: the app never reached {at:04X}"
            );
        }
    }
}

const STAY: u8 = 0;
const REDRAW: u8 = 1;

/// Every idle call of the checking screen returns with interrupts on and the speed the shell had, at
/// 6 and at 15 MHz, for every status the job answers: more, done, a chunk, the CRC run ending, ON
/// at the idle point, CK_PAUSED, a CRC that is wrong, a table lost and a reader that fails; and the
/// deepest call writes the figure the header gives.
#[test]
fn every_idle_call_of_the_checking_screen_returns_with_interrupts_on_and_the_speed_it_had() {
    let mut deepest = 0;
    for speed in [0u8, 1] {
        let boot = |built: &'static Built, table: Option<&[u8]>| {
            let mut run = Run::boot(built, table);
            run.m().out_port(0x20, speed);
            run.m().press_key(F2);
            run
        };
        // The whole of a check with a CRC run: more, a chunk, the run ending, the end.
        let built = proofs();
        let mut run = boot(built, None);
        let mut finished = None;
        let all = idle_calls(
            "full run",
            &mut run,
            speed,
            |_, _| {},
            |r, s, n| {
                let Some(at) = finished else {
                    if s.action == REDRAW
                        && !s.carry
                        && r.values().is_some_and(|v| v.iter().all(|&x| x != 0))
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
        let end = finished.expect("the check finished");
        assert_eq!(
            all.len(),
            end + 1,
            "one call after the finish, taken for a key"
        );
        assert_eq!(
            (all[end].action, all[end].carry),
            (STAY, false),
            "after the finish the idle call touches nothing"
        );
        let kinds: std::collections::BTreeSet<(u8, bool)> =
            all[..end].iter().map(|s| (s.action, s.carry)).collect();
        assert!(kinds.contains(&(STAY, true)), "a step that answers more");
        assert!(kinds.contains(&(REDRAW, true)), "a frame and more work");
        assert!(kinds.contains(&(REDRAW, false)), "the end");
        // ON at the idle point.
        let mut run = boot(built, Some(&empty_table(built)));
        let all2 = idle_calls(
            "ON at the idle point",
            &mut run,
            speed,
            |_, _| {},
            |r, _, n| {
                if n == 3 {
                    r.m().press_key(Machine::KEY_ON);
                }
                n > 3 && r.depth() == 1
            },
        );
        assert_eq!(
            all2.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "ON popped, which scr_idle answers as a redraw with the new top's work pending: {all2:?}"
        );
        // CK_PAUSED: ON down as the kernel is called.
        let mut run = boot(built, Some(&empty_table(built)));
        let all3 = idle_calls(
            "CK_PAUSED",
            &mut run,
            speed,
            |r, n| {
                if n == 4 {
                    r.m().press_key(Machine::KEY_ON);
                }
            },
            |r, _, n| n > 3 && r.depth() == 1,
        );
        assert_eq!(
            all3.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "CK_PAUSED popped: {all3:?}"
        );
        // CK_PAUSED from the CRC run: ON down as the third chunk is called.
        let mut run = boot(padded(), None);
        let all7 = idle_calls(
            "CK_PAUSED from reading",
            &mut run,
            speed,
            |r, n| {
                if n == 4 {
                    r.m().press_key(Machine::KEY_ON);
                }
            },
            |r, _, n| n > 3 && r.depth() == 1,
        );
        assert_eq!(
            all7.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, true)),
            "CK_PAUSED popped: {all7:?}"
        );
        assert_eq!(run.crc_calls(), (1, 3), "the paused call was a chunk call");
        // A table lost between steps and a reader that fails.
        for what in ["table", "reader"] {
            let mut run = boot(built, Some(&empty_table(built)));
            let length = run.shell.app.label("lr_length");
            let all4 = idle_calls(
                what,
                &mut run,
                speed,
                |_, _| {},
                |r, _, n| {
                    if n == 3 {
                        match what {
                            "table" => {
                                r.m().delete_var(crate::table::VAR);
                            }
                            _ => r.m().load_ram(length, &[0, 0, 0]),
                        }
                    }
                    n > 3 && r.top_kind() == KIND_NOTICE
                },
            );
            assert_eq!(
                all4.last().map(|s| (s.action, s.carry)),
                Some((REDRAW, false)),
                "{what}: the screen was replaced"
            );
            deepest = deepest.max(all4.iter().map(|s| s.depth).max().unwrap());
        }
        // A CRC that is wrong.
        let mut run = boot(damaged(false), None);
        let all5 = idle_calls(
            "damaged CRC",
            &mut run,
            speed,
            |_, _| {},
            |r, _, _| r.top_kind() == KIND_NOTICE,
        );
        assert_eq!(
            all5.last().map(|s| (s.action, s.carry)),
            Some((REDRAW, false))
        );
        // Many chunks, then steps, then a stop: the CRC run's chunks.
        let mut run = boot(padded(), None);
        let all6 = idle_calls(
            "padded",
            &mut run,
            speed,
            |_, _| {},
            |r, _, n| {
                n > chunks_of(padded()) + 20 && {
                    r.m().press_key(Machine::KEY_ON);
                    true
                }
            },
        );
        for sample in all
            .iter()
            .chain(&all2)
            .chain(&all3)
            .chain(&all5)
            .chain(&all6)
            .chain(&all7)
        {
            deepest = deepest.max(sample.depth);
        }
    }
    println!("the deepest idle call wrote {deepest} bytes below the stack pointer of its call");
    assert_eq!(deepest, 32, "the stack the header gives");
}

/// The most T-states scr_draw and ui_blit may take together for each frame of the checking screen,
/// cold (the strings copied to APP_STAGE first) and warm, a little above what they take.
const FRAME_READING: u32 = 132_000;
const FRAME_STEPS: u32 = 152_000;
const FRAME_FINISHED: u32 = 127_500;
const FRAME_DAMAGED: u32 = 131_000;

/// What one frame costs, printed on every run and held to a ceiling: the reading frame, a frame of
/// the steps with a label and a failure, and the finished frame, each cold and warm.
#[test]
fn a_frame_of_the_checking_screen_costs_what_its_ceiling_says() {
    use crate::shell_cost::{FRAME_BUDGET, frame};
    let stage = |run: &mut Run, name: &str, ceiling: u32| {
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
        assert!(cold.0 + cold.1 <= ceiling, "{name}: cold over {ceiling}");
        assert!(ceiling <= FRAME_BUDGET);
        assert!(warm.0 + warm.1 < cold.0 + cold.1, "{name}: warm is cheaper");
    };
    let mut run = Run::boot(padded(), None);
    run.m().press_key(F2);
    for _ in 0..4 {
        run.next_blit();
    }
    stage(&mut run, "reading", FRAME_READING);
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    for _ in 0..2 {
        run.next_blit();
    }
    stage(&mut run, "steps", FRAME_STEPS);
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    loop {
        run.next_blit();
        if run.values().is_some_and(|v| v.iter().all(|&x| x != 0)) {
            break;
        }
    }
    stage(&mut run, "finished", FRAME_FINISHED);
    let mut run = Run::boot(damaged(false), None);
    run.m().press_key(F2);
    while run.top_kind() != KIND_NOTICE {
        run.next_blit();
    }
    stage(&mut run, "damaged notice", FRAME_DAMAGED);
}
