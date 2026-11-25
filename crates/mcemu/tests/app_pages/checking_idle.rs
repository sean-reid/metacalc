//! The interrupt state, speed and stack of every idle call of the checking screen.

use crate::app::*;
use crate::checking_errors::damaged;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::Machine;

/// What one idle call returned: A, the carry, the bytes it wrote below the stack pointer scr_idle
/// was called with, CKS_PHASE before and after it, and the bytes the shell had pushed when
/// checking_idle's first instruction ran, its return address included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub action: u8,
    pub carry: bool,
    pub depth: u16,
    pub before: u8,
    pub after: u8,
    pub shell: u16,
    /// Whether the checking screen was still the top one at the return.
    pub stays: bool,
}

/// What an idle call of the checking screen did, from the phase it started in, the phase it left and
/// its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    StepStays,
    StepRedraws,
    Chunk,
    EndsCrcRun,
    Finishing,
    AfterFinish,
    Other,
}

pub const CLASSES: [Class; 6] = [
    Class::StepStays,
    Class::StepRedraws,
    Class::Chunk,
    Class::EndsCrcRun,
    Class::Finishing,
    Class::AfterFinish,
];

const PHASE_STEP: u8 = 0;
const PHASE_READ: u8 = 1;
const PHASE_DONE: u8 = 2;

impl Sample {
    pub fn class(&self) -> Class {
        if !self.stays {
            return Class::Other;
        }
        match (self.before, self.after, self.action, self.carry) {
            (PHASE_STEP, PHASE_STEP, STAY, true) => Class::StepStays,
            (PHASE_STEP, PHASE_STEP, REDRAW, true) => Class::StepRedraws,
            (PHASE_READ, PHASE_READ, REDRAW, true) => Class::Chunk,
            (PHASE_READ, PHASE_STEP, REDRAW, true) => Class::EndsCrcRun,
            (PHASE_STEP, PHASE_DONE, REDRAW, false) => Class::Finishing,
            (PHASE_DONE, PHASE_DONE, STAY, false) => Class::AfterFinish,
            _ => Class::Other,
        }
    }
}

/// The least and the greatest depth and the number of calls of each class in `samples`.
pub fn by_class(samples: &[Sample]) -> std::collections::BTreeMap<Class, (u16, u16, usize)> {
    let mut classes = std::collections::BTreeMap::new();
    for s in samples {
        let entry = classes.entry(s.class()).or_insert((u16::MAX, 0, 0));
        *entry = (entry.0.min(s.depth), entry.1.max(s.depth), entry.2 + 1);
    }
    classes
}

/// Runs the shell's idle calls one at a time: at the call of scr_idle fills the 256 bytes below the
/// stack pointer, and at its return checks that interrupts are as they were at the call (on, or off
/// when `mask` turns them off for the call, so no handler's pushes are in the depth) and the speed
/// port is `speed`, and measures how deep the call wrote. `each` sees every sample and may change
/// the machine for the next call; it answers true to stop.
pub fn idle_calls(
    what: &str,
    run: &mut Run,
    speed: u8,
    mask: bool,
    mut before: impl FnMut(&mut Run, usize),
    mut each: impl FnMut(&mut Run, &Sample, usize) -> bool,
) -> Vec<Sample> {
    let work = run.shell.app.label("app_work");
    let call = work + 5;
    let back = work + 8;
    let idle = run.shell.app.label("checking_idle");
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
        if mask {
            set_interrupts(run, 0);
        }
        let before_phase = run.byte("CKS_PHASE");
        let sp = run.m().regs().sp;
        let kept = run.m().read_ram(sp - 256, 256);
        let fill: Vec<u8> = (0..256u16).map(|i| (i as u8).wrapping_mul(7) | 1).collect();
        run.m().load_ram(sp - 256, &fill);
        let on_screen = run.top_kind() == KIND_CHECKING;
        let mut shell = 0;
        if on_screen {
            run.run_to_addr("the call of checking_idle", idle, WAIT);
            shell = sp - run.m().regs().sp;
        }
        run.run_to_addr("the return of scr_idle", back, WAIT);
        let regs = run.m().regs();
        assert_eq!(
            regs.iff1,
            u8::from(!mask),
            "idle call {}: interrupts as at the call",
            all.len()
        );
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
            before: before_phase,
            after: run.byte("CKS_PHASE"),
            shell,
            stays: run.top_kind() == KIND_CHECKING,
        };
        all.push(sample);
        if mask {
            set_interrupts(run, 1);
        }
        run.m().resume(1);
        if each(run, &sample, all.len()) {
            return all;
        }
        assert!(all.len() < 5_000, "the scenario never ended");
    }
}

fn set_interrupts(run: &mut Run, on: u8) {
    let mut regs = run.m().regs();
    regs.iff1 = on;
    regs.iff2 = on;
    run.m().set_regs(&regs);
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
/// at the idle point, CK_PAUSED, a CRC that is wrong, a table lost and a reader that fails. Run once
/// more with interrupts off at each call, each class of call writes the depths the header gives.
#[test]
fn every_idle_call_of_the_checking_screen_returns_with_interrupts_on_and_the_speed_it_had() {
    let mut every = Vec::new();
    for (speed, mask) in [(0u8, false), (1, false), (0, true)] {
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
            mask,
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
            mask,
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
            mask,
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
            mask,
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
                mask,
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
            if mask {
                every.extend(all4);
            }
        }
        // A CRC that is wrong.
        let mut run = boot(damaged(false), None);
        let all5 = idle_calls(
            "damaged CRC",
            &mut run,
            speed,
            mask,
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
            mask,
            |_, _| {},
            |r, _, n| {
                n > chunks_of(padded()) + 20 && {
                    r.m().press_key(Machine::KEY_ON);
                    true
                }
            },
        );
        if mask {
            for more in [all, all2, all3, all5, all6, all7] {
                every.extend(more);
            }
        }
    }
    let deepest = every.iter().map(|s| s.depth).max().unwrap();
    println!(
        "{} idle calls, the deepest wrote {deepest} bytes",
        every.len()
    );
    let classes = by_class(&every);
    for (class, (least, most, calls)) in &classes {
        println!("{class:?}: {calls} calls, {least} to {most} bytes");
    }
    // The least and greatest depth and the fewest calls of each class on the stub OS.
    let stub: [(Class, (u16, u16, usize)); 6] = [
        (Class::StepStays, (28, 28, 100)),
        (Class::StepRedraws, (28, 28, 2)),
        (Class::Chunk, (14, 14, 10)),
        (Class::EndsCrcRun, (30, 30, 2)),
        (Class::Finishing, (28, 28, 1)),
        (Class::AfterFinish, (10, 10, 1)),
    ];
    assert_eq!(stub.map(|c| c.0), CLASSES);
    for (class, (least, most, calls)) in stub {
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
    assert_eq!(
        deepest, 30,
        "the deepest call is the one that ends the CRC run"
    );
    let shell: std::collections::BTreeSet<u16> =
        every.iter().map(|s| s.shell).filter(|&b| b > 0).collect();
    println!("the shell's share of the stack at checking_idle: {shell:?}");
    assert_eq!(
        shell,
        [SHELL_STACK].into(),
        "the shell's share of the depth"
    );
}

/// The bytes on the stack when checking_idle's first instruction runs: the return address of
/// scr_idle and the one of checking_idle.
pub const SHELL_STACK: u16 = 4;
