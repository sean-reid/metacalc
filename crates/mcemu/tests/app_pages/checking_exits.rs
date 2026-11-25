//! How a check ends other than by running out: ON as a tap between two steps and as a press held
//! through a kernel call, 2nd+QUIT and the exits that skip check_end, a power down after the end, a
//! key the screen ignores, and the interrupt state and speed every idle call leaves.

use crate::app::*;
use crate::builds::*;
use crate::checking_errors::damaged;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcemu::{Machine, Stop};

/// A run of the proofs library with its table, stopped at the blit of the third frame, 32 steps in.
fn mid_check() -> Run {
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    for _ in 0..3 {
        run.next_blit();
    }
    run.m().release_key(F2);
    run
}

fn settled(run: &mut Run) -> usize {
    run.values().unwrap().iter().filter(|&&v| v != 0).count()
}

/// ON pressed at the idle point, a tap between two steps: the shell sets APP_ON, the idle call
/// reads and clears it and pops before the next step, so no kernel call is made. ON pressed just
/// as the kernel starts a verify, a press held through a kernel call: the call answers
/// CK_PAUSED with nothing written for its assertion and the idle call pops. Both leave "next" at
/// the first assertion not verified.
#[test]
fn on_between_two_steps_and_on_held_through_a_kernel_call_both_stop_the_check() {
    // Between two steps: the blit is followed by the idle point.
    let mut run = mid_check();
    let (done, verifies) = (settled(&mut run), run.verifies());
    assert_eq!(done, 32);
    run.m().press_key(Machine::KEY_ON);
    run.next_blit();
    assert_eq!(run.depth(), 1, "popped");
    assert_eq!(run.verifies(), verifies, "no kernel call was made");
    assert_eq!(settled(&mut run), done);
    assert_eq!(usize::from(run.next_number()), done);
    assert_eq!(run.byte("APP_ON"), 0, "the idle call cleared it");
    run.m().release_key(Machine::KEY_ON);
    // Held through a kernel call: the verify is entered and answers CK_PAUSED.
    let mut run = mid_check();
    run.run_to("far_kernel_verify", WAIT);
    run.m().press_key(Machine::KEY_ON);
    run.next_blit();
    assert_eq!(run.depth(), 1, "popped");
    assert_eq!(
        run.verifies(),
        verifies + 1,
        "the verify was called and stopped"
    );
    assert_eq!(
        settled(&mut run),
        done,
        "nothing was written for its assertion"
    );
    assert_eq!(
        usize::from(run.next_number()),
        done,
        "next is the assertion in hand"
    );
    assert_eq!(run.byte("APP_ON"), 0, "CK_PAUSED, not APP_ON, stopped it");
    run.m().release_key(Machine::KEY_ON);
    run.run(2_000_000);
    assert_eq!(run.depth(), 1, "the app did not leave");
}

/// A key the screen ignores in the middle of a check costs one trip through app_key and loses no
/// step: every idle call between the frames did one verify and every assertion was verified once.
#[test]
fn a_key_the_screen_ignores_loses_no_step() {
    let mut run = mid_check();
    let built = run.built;
    let (idles, waits, verifies, keys) = (
        run.shell.idle_calls(),
        run.shell.key_waiting_calls(),
        run.verifies(),
        run.shell.scr_keys(),
    );
    run.tap(F4);
    let mut last = run.next_frame();
    assert_eq!(run.depth(), 2, "the screen is still open");
    while last.1.as_ref().unwrap().iter().any(|&v| v == 0) {
        last = run.next_frame();
    }
    let n = last.1.unwrap().len();
    assert_eq!(
        run.shell.scr_keys(),
        keys + 1,
        "the key reached the screen once"
    );
    let verified = run.verifies() - verifies;
    let idled = run.shell.idle_calls() - idles;
    let waited = run.shell.key_waiting_calls() - waits;
    println!("{verified} verifies, {idled} idle calls, {waited} key checks after the key");
    assert_eq!(idled, verified, "every idle call did one step");
    assert_eq!(
        waited,
        idled + 1,
        "the key cost one more trip through the idle point"
    );
    assert_eq!(
        usize::from(run.verifies()),
        n,
        "no assertion was verified twice or skipped"
    );
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    assert_eq!(run.values().unwrap(), expected_values(built, &verdicts, n));
}

/// How the app is left in the middle of a check without check_end.
#[derive(Clone, Copy, Debug)]
enum Leave {
    Quit,
    Off,
    Error,
}

/// Leaves as `how` does: 2nd+QUIT is two keys; the OS runs cxPPutAway, which is app_putaway, at
/// 2nd+OFF and when an OS call raises an error, and never returns to the app, which the call does
/// by running app_putaway on the machine and abandoning the app where it stood.
fn leave(run: &mut Run, how: Leave) {
    match how {
        Leave::Quit => {
            run.m().press_key(SECOND);
            run.run(200_000);
            run.m().release_key(SECOND);
            run.run(300_000);
            run.m().press_key(MODE);
            let stop = run.m().run_app(3_000_000).stop;
            assert_eq!(stop, Stop::Exited, "the app left");
            run.m().release_key(MODE);
        }
        Leave::Off | Leave::Error => {
            let at = run.shell.app.label("app_putaway");
            run.shell.app.call(BASE, at, In::default());
        }
    }
}

/// The ways out that skip check_end leave the job where it was: "next" is not written, the table
/// holds what the steps wrote, and after a relaunch a Run carries on and finishes with every
/// assertion verified once.
#[test]
fn leaving_without_check_end_leaves_the_job_where_it_was_and_a_relaunch_carries_on() {
    for how in [Leave::Quit, Leave::Off, Leave::Error] {
        let name = format!("{how:?}");
        let mut run = mid_check();
        let built = run.built;
        let n = usize::from(crate::table::header_of(run.library()).1);
        assert_eq!(run.next_number(), 0, "{name}: no check_end has run");
        leave(&mut run, how);
        let held = settled(&mut run);
        assert!(held >= 32 && held < n, "{name}: part way, {held} of {n}");
        assert_eq!(run.next_number(), 0, "{name}: leaving wrote no next");
        run.m().launch(BASE).expect("the app launches again");
        assert_eq!(run.m().run_app(3_000_000).stop, Stop::Limit);
        assert_eq!(run.shell.top_kind(), KIND_LIBRARIES, "{name}");
        let values = run.values().unwrap();
        assert_eq!(
            values.iter().filter(|&&v| v != 0).count(),
            held,
            "{name}: the table kept"
        );
        let frame = run.shell.app.frame();
        same_frame(
            &name,
            &frame,
            &model::frame(&model::ui(built, &status_of(&values))),
        );
        let before = run.verifies();
        run.m().press_key(F2);
        let (last, values) = loop {
            let (frame, values) = run.next_frame();
            if let Some(values) = values
                && values.iter().all(|&v| v != 0)
            {
                break (frame, values);
            }
        };
        run.m().release_key(F2);
        let verdicts = host_verdicts(run.library());
        assert_eq!(values, expected_values(built, &verdicts, n), "{name}");
        assert_eq!(
            usize::from(run.verifies()),
            n,
            "{name}: none verified twice, {} before the exit and {} after",
            before,
            run.verifies() - before
        );
        shows(&name, &last, &checking_model(built, &values, 0));
    }
}

/// When the check has ended the shell blocks and the OS powers down as on any screen; the refresh on
/// wake runs the enter again, which finds the table, counts it and shows "All checked", with no
/// CRC run and no step.
#[test]
fn a_power_down_after_the_end_is_refreshed_from_the_table_with_no_crc_run() {
    let built = proofs();
    let mut run = Run::boot(built, None);
    run.m().press_key(F2);
    let (finished, values) = loop {
        let (frame, values) = run.next_frame();
        if let Some(values) = values
            && values.iter().all(|&v| v != 0)
        {
            break (frame, values);
        }
    };
    run.m().release_key(F2);
    let (crc, verifies) = (run.crc_calls(), run.verifies());
    assert_eq!(crc.0, 1, "the CRC run began once");
    assert!(crc.1 >= 1);
    let want = checking_model(built, &values, 0);
    shows("finished", &finished, &want);
    run.run(3_000_000);
    run.shell.power_down();
    run.tap(F4);
    let frame = run.next_blit();
    shows("after the power down", &frame, &want);
    assert_eq!(run.crc_calls(), crc, "no CRC run");
    assert_eq!(run.verifies(), verifies, "no step");
    assert_eq!((run.depth(), run.shell.top_kind()), (2, KIND_CHECKING));
    assert_eq!(run.values().unwrap(), values, "the table is as it was");
    run.run(2_000_000);
    run.tap(CLEAR);
    run.next_blit();
    assert_eq!(run.depth(), 1);
    assert_eq!(usize::from(run.next_number()), values.len());
}

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
        let all = idle_calls(
            "full run",
            &mut run,
            speed,
            |_, _| {},
            |r, s, _| {
                s.action == REDRAW
                    && !s.carry
                    && r.values().is_some_and(|v| v.iter().all(|&x| x != 0))
            },
        );
        let kinds: std::collections::BTreeSet<(u8, bool)> =
            all.iter().map(|s| (s.action, s.carry)).collect();
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
                    n > 3 && r.shell.top_kind() == KIND_NOTICE
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
            |r, _, _| r.shell.top_kind() == KIND_NOTICE,
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
        {
            deepest = deepest.max(sample.depth);
        }
    }
    println!("the deepest idle call wrote {deepest} bytes below the stack pointer of its call");
}

/// The most T-states scr_draw and ui_blit may take together for each frame of the checking screen,
/// cold (the strings copied to APP_STAGE first) and warm, a little above what they take.
const FRAME_READING: u32 = 300_000;
const FRAME_STEPS: u32 = 300_000;
const FRAME_FINISHED: u32 = 300_000;

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
    for _ in 0..3 {
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
}
