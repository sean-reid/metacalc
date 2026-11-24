use crate::model::Pair;
use crate::random::{Xorshift, floors, run};
use crate::support::*;

#[test]
fn a_framework_with_its_ram_moved_agrees_with_the_mirror_too() {
    assert_ne!(Rig::new(MOVED).stack(), Rig::new(REAL).stack());
    let mut pair = Pair::new(MOVED);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    let seen = run(&mut pair, &mut Xorshift(12345), 2000);
    println!("{seen:?}");
    floors(&seen, 2000);
}

/// Changes one byte of guarded range `range` and makes an ordinary framework call, which must
/// notice.
fn call_after_poking(range: usize, at_end: bool) {
    let mut rig = Rig::new(REAL);
    let (start, len) = rig.guarded()[range];
    let at = if at_end {
        start + len as u16 - 1
    } else {
        start
    };
    let byte = rig.peek(at, 1)[0];
    rig.poke(at, &[byte ^ 1]);
    rig.call("SCR_DEPTH", Regs::default());
}

/// What the panic of an ordinary call says after one byte of a guarded range changed.
fn noticed(range: usize, at_end: bool) -> String {
    match std::panic::catch_unwind(|| call_after_poking(range, at_end)) {
        Ok(()) => String::new(),
        Err(why) => why.downcast_ref::<String>().cloned().unwrap_or_default(),
    }
}

#[test]
fn a_call_notices_a_change_at_either_end_of_every_guarded_range() {
    for range in 0..5 {
        for at_end in [false, true] {
            let said = noticed(range, at_end);
            assert!(
                said.contains("bytes outside the framework's changed"),
                "range {range}, end {at_end}: {said:?}"
            );
        }
    }
}

#[test]
fn the_stack_use_stays_within_what_the_header_states() {
    let (mut worst, mut entry) = (0, 0);
    for depth in [1, 8] {
        for act in [Act::Stay, Act::Push([1; 16]), Act::Pop, Act::Jump(1)] {
            for statuses in [&[][..], &[3, 0][..]] {
                let mut pair = Pair::opened(REAL, depth);
                pair.mirror.press(1, 0, act, statuses);
                let (out, log) = pair.rig.key(1, 0, act, statuses);
                worst = worst.max(out.stack);
                entry = entry.max(Rig::entry_stack(&log));
            }
        }
    }
    println!("scr_key reached {worst} bytes of stack, {entry} at a routine's entry");
    assert_eq!(worst, 10);
    assert_eq!(entry, 4);
}

/// An interrupt after every `step` T-states of one call, up to past its measured length. Every
/// trial is compared with the mirror. Returns the trials made, the interrupts taken, the last
/// offset that took an interrupt and the call's length without interrupts.
fn sweep(
    depth: usize,
    act: impl Fn(&mut Pair) -> Act,
    statuses: &[u8],
    step: u32,
) -> (usize, usize, u32, u32) {
    let mut pair = Pair::new(REAL);
    pair.rig.install_handler();
    pair.reopen(depth);
    let first = act(&mut pair);
    pair.key(1, 0, first, statuses);
    let length = pair.key_spent;
    let (mut fired, mut trials, mut last_fired) = (0, 0, 0);
    let mut after = 1;
    loop {
        pair.reopen(depth);
        let act = act(&mut pair);
        let taken = pair.key_interrupted(act, statuses, after);
        trials += 1;
        fired += usize::from(taken > 0);
        if taken > 0 {
            last_fired = after;
        }
        if after > length {
            return (trials, fired, last_fired, length);
        }
        after += step;
    }
}

/// T-states at the end of a measured call that are the driver's own `di` and `halt`, where no
/// interrupt is taken.
const DRIVER_TAIL: u32 = 30;

fn swept(depth: usize, act: impl Fn(&mut Pair) -> Act, statuses: &[u8]) {
    let (trials, fired, last, length) = sweep(depth, act, statuses, 7);
    println!("{fired} of {trials} interrupts landed inside a call of {length} T-states");
    assert!(fired * 10 >= trials * 9, "{fired} of {trials}");
    assert!(
        length.abs_diff(last) <= 7 + DRIVER_TAIL,
        "the last interrupt was after {last} T-states of a call of {length}"
    );
}

#[test]
fn an_interrupt_at_any_point_of_a_push_on_a_full_stack_changes_nothing() {
    swept(MAX_DEPTH, |p| Act::Push(p.fresh(2)), &[]);
}

#[test]
fn an_interrupt_at_any_point_of_a_push_whose_enter_fails_changes_nothing() {
    swept(MAX_DEPTH, |p| Act::Push(p.fresh(3)), &[4, 0]);
}

#[test]
fn an_interrupt_at_any_point_of_a_pop_changes_nothing() {
    swept(5, |_| Act::Pop, &[]);
}

#[test]
fn an_interrupt_at_any_point_of_a_jump_changes_nothing() {
    swept(5, |_| Act::Jump(0x55AA), &[]);
}
