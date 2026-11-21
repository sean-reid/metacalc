use crate::model::Pair;
use crate::random::{Xorshift, floors, run};
use crate::support::*;

#[test]
fn a_framework_with_its_ram_moved_agrees_with_the_mirror_too() {
    assert_ne!(Rig::new(MOVED).stack(), Rig::new(REAL).stack());
    let mut pair = Pair::new(MOVED);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    let seen = run(&mut pair, &mut Xorshift(12345), 800);
    println!("{seen:?}");
    floors(&seen, 800);
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

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn a_call_notices_a_change_in_the_bytes_before_the_stack() {
    call_after_poking(0, true);
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn a_call_notices_a_change_in_the_byte_after_the_stack() {
    call_after_poking(1, false);
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn a_call_notices_a_change_in_the_bytes_before_the_variables() {
    call_after_poking(2, true);
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn a_call_notices_a_change_in_an_unclaimed_variable_byte() {
    call_after_poking(3, false);
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn a_call_notices_a_change_in_the_byte_after_the_variables() {
    call_after_poking(4, false);
}

#[test]
fn the_stack_use_stays_within_what_the_header_states() {
    let mut worst = 0;
    for depth in [1, 8] {
        for act in [Act::Stay, Act::Push([1; 16]), Act::Pop, Act::Jump(1)] {
            for statuses in [&[][..], &[3, 0][..]] {
                let mut pair = Pair::opened(REAL, depth);
                pair.mirror.press(1, 0, act, statuses);
                let (out, _) = pair.rig.key(1, 0, act, statuses);
                worst = worst.max(out.stack);
            }
        }
    }
    println!("scr_key reached {worst} bytes of stack");
    assert!(worst <= 10, "{worst}");
}

/// An interrupt after every `step` T-states of one call, up to past its measured length. Every
/// trial is compared with the mirror. Returns the trials made, the interrupts taken, the last
/// offset tried and the call's length without interrupts.
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
    let (mut fired, mut trials) = (0, 0);
    let mut after = 1;
    loop {
        pair.reopen(depth);
        let act = act(&mut pair);
        let taken = pair.key_interrupted(act, statuses, after);
        trials += 1;
        fired += usize::from(taken > 0);
        if after > length {
            return (trials, fired, after, length);
        }
        after += step;
    }
}

fn swept(depth: usize, act: impl Fn(&mut Pair) -> Act, statuses: &[u8]) {
    let (trials, fired, last, length) = sweep(depth, act, statuses, 7);
    println!("{fired} of {trials} interrupts landed inside a call of {length} T-states");
    assert!(fired * 10 >= trials * 9, "{fired} of {trials}");
    assert!(last > length, "the sweep stopped at {last} of {length}");
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
