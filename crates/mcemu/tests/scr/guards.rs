use crate::model::Pair;
use crate::random::{Xorshift, run};
use crate::support::*;

#[test]
fn a_framework_with_its_ram_moved_agrees_with_the_mirror_too() {
    assert_ne!(Rig::new(MOVED).stack(), Rig::new(REAL).stack());
    let mut pair = Pair::new(MOVED);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    run(&mut pair, &mut Xorshift(12345), 800);
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn the_guard_check_catches_a_change_in_the_byte_after_the_stack() {
    let mut rig = Rig::new(REAL);
    let at = rig.stack() + 128;
    rig.poke(at, &[MARK ^ 1]);
    rig.check_guards();
}

#[test]
#[should_panic(expected = "bytes outside the framework's changed")]
fn the_guard_check_catches_a_change_in_a_variable_byte_the_framework_does_not_own() {
    let mut rig = Rig::new(REAL);
    let at = rig.label("SCR_VARS") + rig.label("SCR_USED");
    rig.poke(at, &[0]);
    rig.check_guards();
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

fn sweep(
    depth: usize,
    act: impl Fn(&mut Pair) -> Act,
    statuses: &[u8],
    step: u32,
) -> (usize, usize) {
    let mut pair = Pair::new(REAL);
    pair.rig.install_handler();
    let (mut fired, mut trials) = (0, 0);
    let mut after = 1;
    loop {
        pair.reopen(depth);
        let act = act(&mut pair);
        let taken = pair.key_interrupted(act, statuses, after);
        trials += 1;
        fired += usize::from(taken > 0);
        if taken == 0 && after > 100 {
            return (fired, trials);
        }
        after += step;
    }
}

#[test]
fn an_interrupt_at_any_point_of_a_push_on_a_full_stack_changes_nothing() {
    let (fired, trials) = sweep(MAX_DEPTH, |p| Act::Push(p.fresh(2)), &[], 7);
    println!("{fired} of {trials} interrupts landed inside the call");
    assert!(fired > 150, "{fired}");
}

#[test]
fn an_interrupt_at_any_point_of_a_push_whose_enter_fails_changes_nothing() {
    let (fired, _) = sweep(MAX_DEPTH, |p| Act::Push(p.fresh(3)), &[4, 0], 7);
    assert!(fired > 150, "{fired}");
}

#[test]
fn an_interrupt_at_any_point_of_a_pop_or_a_jump_changes_nothing() {
    let (fired, _) = sweep(5, |_| Act::Pop, &[], 7);
    assert!(fired > 50, "{fired}");
    let (fired, _) = sweep(5, |_| Act::Jump(0x55AA), &[], 7);
    assert!(fired > 50, "{fired}");
}
