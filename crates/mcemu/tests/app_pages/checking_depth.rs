//! The rig that runs the shell's idle calls of the checking screen one at a time and measures the
//! stack each writes.

use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;

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

/// The classes of `Class` with the calls that stop the screen last.
pub const WITH_OTHER: [Class; 7] = [
    Class::StepStays,
    Class::StepRedraws,
    Class::Chunk,
    Class::EndsCrcRun,
    Class::Finishing,
    Class::AfterFinish,
    Class::Other,
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

/// What the 256 bytes below the stack pointer hold before an idle call. The two fills differ at every
/// byte, so a pushed byte equal to one of them is not equal to the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Odd,
    Inverse,
}

impl Fill {
    pub const BOTH: [Fill; 2] = [Fill::Odd, Fill::Inverse];

    fn byte(self, i: u8) -> u8 {
        let odd = i.wrapping_mul(7) | 1;
        match self {
            Fill::Odd => odd,
            Fill::Inverse => !odd,
        }
    }
}

/// Runs the shell's idle calls one at a time: at the call of scr_idle fills the 256 bytes below the
/// stack pointer, and at its return checks that interrupts are as they were at the call (on, or off
/// when `mask` turns them off for the call, so no handler's pushes are in the depth) and the speed
/// port is `speed`, and measures how deep the call wrote. `each` sees every sample and may change
/// the machine for the next call; it answers true to stop. `with` is the fill.
pub fn idle_calls(
    what: &str,
    run: &mut Run,
    speed: u8,
    mask: bool,
    with: Fill,
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
        let fill: Vec<u8> = (0..=255u8).map(|i| with.byte(i)).collect();
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

pub const STAY: u8 = 0;
pub const REDRAW: u8 = 1;

/// The bytes on the stack when checking_idle's first instruction runs: the return address of
/// scr_idle and the one of checking_idle.
pub const SHELL_STACK: u16 = 4;

/// The samples of two runs of one scenario, one with each fill: asserts they are the same calls and
/// that the two fills measured the same depth for each, and returns the samples.
pub fn agreed(what: &str, odd: &[Sample], inverse: &[Sample]) -> Vec<Sample> {
    assert_eq!(odd.len(), inverse.len(), "{what}: the same number of calls");
    for (n, (a, b)) in odd.iter().zip(inverse).enumerate() {
        assert_eq!(
            a, b,
            "{what}: call {n} measured differently with the two fills"
        );
    }
    odd.to_vec()
}

/// The interrupts-on depths of each class against the masked ones. Prints the greatest of each class
/// and overall, asserts that some call is deeper than the greatest masked call of its class, which
/// is what an interrupt that landed in the call adds, and returns the greatest of each class.
pub fn live_depths(masked: &[Sample], live: &[Sample]) -> std::collections::BTreeMap<Class, u16> {
    let masked = by_class(masked);
    let live_classes = by_class(live);
    let mut deeper = 0;
    for s in live {
        let Some(&(_, most, _)) = masked.get(&s.class()) else {
            continue;
        };
        if s.depth > most {
            deeper += 1;
        }
    }
    let overall = live.iter().map(|s| s.depth).max().expect("samples");
    println!(
        "interrupts on: {} calls, {deeper} deeper than the masked greatest of their class, the deepest wrote {overall} bytes",
        live.len()
    );
    for (class, (_, most, calls)) in &live_classes {
        println!("{class:?} with interrupts on: {calls} calls, greatest {most} bytes");
    }
    assert!(
        deeper > 0,
        "an interrupt landed in a call and made it deeper"
    );
    live_classes.into_iter().map(|(c, v)| (c, v.1)).collect()
}
