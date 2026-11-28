//! An interrupt forced at the deepest instruction of the Dist enter and of the Dist draw, with the timer's
//! interrupt pending, so that the depth with interrupts on is measured at its top: the masked figure plus
//! everything the handler writes, not the most a sampled run happened to see.
//!
//! The deepest instructions come from a trace of the stack pointer taken with interrupts on and no source
//! enabled (port 3 at 0), which shows where the code itself runs with them off: none of the deepest runs
//! with them off. The forced run is masked from the entry, so that the timer's interrupt stays pending, and
//! takes it after one of those instructions by turning interrupts on for that one instruction.
//!
//! On the reference OS at speed 0 the shell's timer is not pending at the draw's deepest instructions, so
//! the draw's 36 there is a bound the shell does not produce, reached only by running the clock on at a
//! HALT; at speed 1 it is produced, and the enter's 32 needs no wait at either speed.

use crate::checkrig::{Run, WAIT};
use crate::dist_depth::{expected, handler};
use crate::dist_lib::{self, SEVEN};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::Key;
use std::collections::BTreeSet;

/// Where the stack is lowest in a call.
struct Deepest {
    entry: u16,
    low: u16,
    /// The instructions after which the pointer is `low` and an interrupt is taken, as (physical pc,
    /// pointer before it).
    reachable: BTreeSet<(usize, u16)>,
    /// How many instructions leave the pointer at `low` with interrupts off or just after an EI.
    unreachable: usize,
}

/// Interrupts on and no source enabled: steps from the entry of `label` to its return and says where the
/// stack pointer is lowest on the stack the call started on.
fn deepest(run: &mut Run, label: &str, speed: u8) -> Deepest {
    let at = run.shell.app.label(label);
    run.run_to_addr(label, at, WAIT);
    run.m().out_port(0x20, speed);
    let mut regs = run.m().regs();
    regs.iff1 = 1;
    regs.iff2 = 1;
    run.m().set_regs(&regs);
    let port3 = run.m().in_port(3);
    run.m().out_port(3, 0);
    let entry = regs.sp;
    let ret = {
        let b = run.m().read_ram(entry, 2);
        u16::from_le_bytes([b[0], b[1]])
    };
    // Each instruction as (physical pc, pointer before, opcode, pointer after, flip-flop after).
    let mut steps = Vec::new();
    let mut returned = false;
    for _ in 0..3_000_000 {
        let r = run.m().regs();
        if r.pc == ret && r.sp == entry + 2 {
            returned = true;
            break;
        }
        let op = run.m().read_ram(r.pc, 1)[0];
        let phys = run.m().physical_pc();
        run.m().resume(1);
        let after = run.m().regs();
        steps.push((phys, r.sp, op, after.sp, after.iff1));
    }
    assert!(returned, "{label}: the return was not reached");
    run.m().out_port(3, port3);
    let on_stack = |sp: u16| sp <= entry && u32::from(sp) + 256 >= u32::from(entry);
    let low = steps
        .iter()
        .map(|s| s.3)
        .filter(|&sp| on_stack(sp))
        .min()
        .unwrap();
    let takes = |s: &(usize, u16, u8, u16, u8)| s.4 == 1 && s.2 != 0xFB;
    let at_low = steps.iter().filter(|s| s.3 == low);
    Deepest {
        entry,
        low,
        reachable: at_low
            .clone()
            .filter(|s| takes(s))
            .map(|s| (s.0, s.1))
            .collect(),
        unreachable: at_low.filter(|s| !takes(s)).count(),
    }
}

/// What a forced interrupt did.
pub struct Forced {
    pub depth: u16,
    pub handler: u16,
    pub handler_at: u16,
    pub at: (usize, u16),
    pub phase: u32,
    pub candidates: usize,
    pub answer: u8,
    /// Whether the clock had to run on at a HALT for the timer's interrupt to be pending there.
    pub waited: bool,
}

/// The call of `label` from its entry, masked, with the timer's interrupt taken after the first time the
/// code reaches `target` with it pending. None when it is pending there at no time.
fn take(
    run: &mut Run,
    label: &str,
    speed: u8,
    d: &Deepest,
    target: (usize, u16),
    idle: Option<u16>,
) -> Option<Forced> {
    let at = run.shell.app.label(label);
    run.run_to_addr(label, at, WAIT);
    run.m().out_port(0x20, speed);
    let mut regs = run.m().regs();
    regs.iff1 = 0;
    regs.iff2 = 0;
    run.m().set_regs(&regs);
    assert_eq!(regs.sp, d.entry, "{label}: the same stack as the trace");
    let ret = {
        let b = run.m().read_ram(d.entry, 2);
        u16::from_le_bytes([b[0], b[1]])
    };
    let kept = run.m().read_ram(d.entry - 256, 256);
    let fill: Vec<u8> = (0..=255u8).map(|i| i.wrapping_mul(7) | 1).collect();
    run.m().load_ram(d.entry - 256, &fill);
    let mut taken = None;
    let mut returned = false;
    for _ in 0..3_000_000 {
        let r = run.m().regs();
        if r.pc == ret && r.sp == d.entry + 2 {
            returned = true;
            break;
        }
        let here = (run.m().physical_pc(), r.sp);
        if taken.is_none() && here == target && wait_for_timer(run, idle) {
            let mut on = r;
            on.iff1 = 1;
            on.iff2 = 1;
            run.m().set_regs(&on);
            run.m().resume(1);
            let entered = run.m().regs();
            assert_eq!(entered.pc, 0x38, "{label}: the interrupt was taken");
            assert_eq!(
                entered.sp,
                d.low - 2,
                "{label}: at the lowest the stack gets"
            );
            let (mut low, mut low_at) = (entered.sp, entered.pc);
            let mut steps = 0;
            while run.m().regs().sp < d.low {
                let before = run.m().regs().pc;
                run.m().resume(1);
                let now = run.m().regs().sp;
                if now < low {
                    (low, low_at) = (now, before);
                }
                steps += 1;
                assert!(steps < 100_000, "{label}: the handler returned");
            }
            let mut back = run.m().regs();
            back.iff1 = 0;
            back.iff2 = 0;
            run.m().set_regs(&back);
            taken = Some((here, d.low - low, low_at));
            continue;
        }
        run.m().resume(1);
    }
    assert!(returned, "{label}: the return was not reached");
    let regs = run.m().regs();
    let after = run.m().read_ram(d.entry - 256, 256);
    run.m().load_ram(d.entry - 256, &kept);
    let lowest = after
        .iter()
        .zip(&fill)
        .position(|(a, f)| a != f)
        .map_or(0, |i| 256 - i as u16);
    let mut done = regs;
    done.iff1 = 1;
    done.iff2 = 1;
    run.m().set_regs(&done);
    run.m().out_port(0x20, 0);
    let (at, handler, handler_at) = taken?;
    Some(Forced {
        depth: lowest + 2,
        handler,
        handler_at,
        at,
        phase: 0,
        candidates: d.reachable.len(),
        answer: (regs.af >> 8) as u8,
        waited: idle.is_some(),
    })
}

/// Whether the timer's interrupt is pending, after letting the clock run at the HALT at `idle` with
/// interrupts off until it is when there is one, which leaves the registers as they were.
fn wait_for_timer(run: &mut Run, idle: Option<u16>) -> bool {
    let pending = |run: &mut Run| run.m().in_port(4) & 2 != 0;
    let Some(halt) = idle else {
        return pending(run);
    };
    let kept = run.m().regs();
    let mut at_halt = kept;
    at_halt.pc = halt;
    for _ in 0..1_000 {
        if pending(run) {
            break;
        }
        run.m().set_regs(&at_halt);
        run.m().resume(1_000);
    }
    run.m().set_regs(&kept);
    pending(run)
}

/// The Dist screen's frame after the traced call is the model's after the F2.
fn frame_after(walker: &mut Walker, what: &str) {
    walker.list.run.release(F2);
    walker.list.run.run(3_000_000);
    walker.list.ui.press(Key::F2);
    walker.list.log.push(Key::F2);
    same_frame(what, &walker.list.run.frame(), &walker.list.shown());
    assert_eq!(walker.list.run.top_kind(), KIND_DIST, "{what}");
}

/// The `routine` (`enter` or `draw`) of the Dist screen of `SEVEN` with the timer's interrupt taken after
/// each of its deepest instructions in turn, in the first run-up to the F2 in which one is pending there.
/// Returns what each forced interrupt did, and how many deepest instructions there are.
pub fn forced(
    on_theorem: &dyn Fn(&str) -> Walker,
    routine: &str,
    speed: u8,
    idle: Option<u16>,
) -> (Vec<Forced>, usize) {
    let label = format!("dist_{routine}");
    let want = expected()[routine].0;
    let ready = |phase: u32| {
        let mut walker = on_theorem(SEVEN);
        walker.list.run.run(phase * 41_113);
        walker.list.run.press(F2);
        walker.trace("theorem_key", true, speed);
        if routine == "draw" {
            walker.trace("dist_enter", true, speed);
        }
        walker
    };
    let mut walker = ready(0);
    let d = deepest(&mut walker.list.run, &label, speed);
    assert_eq!(
        d.entry + 2 - d.low,
        want,
        "{routine}: the trace's depth is the figure"
    );
    assert_eq!(
        d.unreachable, 0,
        "{routine}: every deepest instruction runs with interrupts on"
    );
    assert!(
        !d.reachable.is_empty(),
        "{routine}: instructions at the deepest point"
    );
    let mut found = Vec::new();
    for &target in &d.reachable {
        let before = found.len();
        for phase in 0..24 {
            let mut walker = ready(phase);
            let got = take(&mut walker.list.run, &label, speed, &d, target, None);
            if let Some(mut f) = got {
                f.phase = phase;
                f.candidates = d.reachable.len();
                frame_after(
                    &mut walker,
                    &format!("{routine} after the forced interrupt"),
                );
                found.push(f);
                break;
            }
            walker.list.run.release(F2);
            walker.list.run.run(3_000_000);
        }
        if found.len() > before || idle.is_none() {
            continue;
        }
        let mut walker = ready(0);
        let mut f = take(&mut walker.list.run, &label, speed, &d, target, idle)
            .expect("the timer is pending after the clock runs on");
        f.candidates = d.reachable.len();
        frame_after(
            &mut walker,
            &format!("{routine} after the forced interrupt, the clock run on"),
        );
        found.push(f);
    }
    (found, d.reachable.len())
}

fn on_theorem_stub(label: &str) -> Walker {
    let built = dist_lib::built();
    let n = index_of(&lib_of(built), label);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    walker
}

/// The deepest instructions of each routine: the enter's are lr_cmp24's from its push hl to its ld b,(hl),
/// the draw's are ui_text_width's from its push hl to the inc a that ends GLYPH_ADV.
fn deepest_count(routine: &str) -> usize {
    match routine {
        "enter" => 7,
        _ => 11,
    }
}

/// Whether the timer's interrupt is pending at the deepest instructions only after the clock runs on: the
/// reference OS's draw at speed 0, whose deepest instructions come 16,324 to 17,533 T-states after its entry
/// and the timer fires 34,128 after it.
fn needs_wait(real: bool, routine: &str, speed: u8) -> bool {
    real && routine == "draw" && speed == 0
}

fn check(os: &str, speed: u8, routine: &str, (found, total): (Vec<Forced>, usize), real: bool) {
    println!(
        "{os}, speed {speed}, {routine}: {} of {total} deepest instructions taken",
        found.len()
    );
    assert_eq!(
        total,
        deepest_count(routine),
        "{routine}: the deepest instructions"
    );
    assert_eq!(found.len(), total, "{routine}: an interrupt after each");
    for f in &found {
        println!(
            "   after {:06X} with sp {:04X}: depth {}, handler {} bytes lowest at {:04X}, run-up {}, clock run on {}",
            f.at.0, f.at.1, f.depth, f.handler, f.handler_at, f.phase, f.waited
        );
        assert_eq!(
            f.waited,
            needs_wait(real, routine, speed),
            "{routine} at speed {speed}: the timer pending without running the clock on"
        );
        assert_eq!(f.answer, 0, "{routine}: the status");
        assert_eq!(f.handler, handler(real), "{routine}: the handler's bytes");
        assert_eq!(
            f.depth,
            expected()[routine].0 + handler(real),
            "{routine} at speed {speed}"
        );
        if real {
            assert_eq!(f.handler_at, 0x047D, "{routine}: the keypad scan");
        }
    }
}

#[test]
fn on_the_stub_an_interrupt_at_a_deepest_instruction_adds_what_its_handler_writes() {
    for speed in [0u8, 1] {
        for routine in ["enter", "draw"] {
            let got = forced(&on_theorem_stub, routine, speed, None);
            check("stub", speed, routine, got, false);
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_an_interrupt_at_each_deepest_instruction_adds_the_keypad_scans_10_bytes() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let on_theorem = |label: &str| {
        let mut list = crate::list_rig::Lister::on_rom_of(built, 0);
        list.open_on_rom();
        let mut walker = Walker::new(list, 0, built);
        walker.gap = 600_000;
        walker.mirrored = false;
        walker.goto(index_of(&lib, label));
        walker.hold_preview();
        walker.open();
        walker
    };
    let idle = {
        let mut walker = on_theorem(SEVEN);
        let page = walker.list.run.m().read_flash(0, 0, 0x4000);
        page.iter()
            .position(|&b| b == 0x76)
            .expect("a HALT in the first page") as u16
    };
    for speed in [0u8, 1] {
        for routine in ["enter", "draw"] {
            let got = forced(&on_theorem, routine, speed, Some(idle));
            check("ROM", speed, routine, got, true);
        }
    }
}
