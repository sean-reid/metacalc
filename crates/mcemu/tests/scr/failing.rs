use crate::model::Pair;
use crate::support::*;

fn raw(pair: &mut Pair) -> Vec<u8> {
    let at = pair.rig.stack();
    pair.rig.peek(at, 128)
}

/// Fills the slots at and above `depth`, which are dead, with a pattern the test knows.
fn mark_dead(pair: &mut Pair, depth: usize) {
    let at = pair.rig.stack() + 16 * depth as u16;
    pair.rig.poke(at, &vec![0x5A; 16 * (MAX_DEPTH - depth)]);
}

/// The stack is as it was after a failed push: every slot below `depth` is live and unchanged, and
/// every dead slot above the one at `depth` still holds the pattern `mark_dead` wrote. The slot at
/// `depth` is dead and is where the push put its frame, so it is not compared. At depth 8 every
/// slot is live and all 128 bytes are compared.
fn unchanged(pair: &mut Pair, before: &[u8], depth: usize) {
    let after = raw(pair);
    for slot in (0..MAX_DEPTH).filter(|&slot| slot != depth) {
        let at = 16 * slot..16 * slot + 16;
        assert_eq!(
            after[at.clone()],
            before[at],
            "slot {slot} at depth {depth}"
        );
    }
    assert_eq!(pair.rig.depth(), depth);
}

#[test]
fn a_failing_enter_after_a_push_leaves_the_stack_as_it_was() {
    for ei in [true, false] {
        a_failing_enter_after_a_push_leaves_the_stack_as_it_was_in(ei);
    }
}

fn a_failing_enter_after_a_push_leaves_the_stack_as_it_was_in(ei: bool) {
    for depth in 1..=MAX_DEPTH {
        for status in [1, 77, 255] {
            let mut pair = Pair::opened_with(REAL, depth, ei);
            mark_dead(&mut pair, depth);
            let before = raw(&mut pair);
            let new = pair.fresh(3);
            let keyed = pair.key(1, 0, Act::Push(new), &[status]);
            assert_eq!(keyed.res, FAILED, "depth {depth}");
            assert_eq!(keyed.status, status);
            unchanged(&mut pair, &before, depth);
            let routines: Vec<Routine> = keyed.calls.iter().map(|c| c.routine).collect();
            assert_eq!(routines, [Routine::Key, Routine::Enter, Routine::Enter]);
            assert_eq!(keyed.calls[1].frame, new);
            let old_top = pair.mirror.frames[depth - 1];
            assert_eq!(keyed.calls[2].frame, old_top);
            assert_eq!(
                keyed.calls[2].at,
                pair.rig.stack() + 16 * (depth as u16 - 1)
            );
        }
    }
}

#[test]
fn the_frame_a_full_stack_would_have_evicted_comes_back() {
    for ei in [true, false] {
        the_frame_a_full_stack_would_have_evicted_comes_back_in(ei);
    }
}

fn the_frame_a_full_stack_would_have_evicted_comes_back_in(ei: bool) {
    for status in [1, 77, 255] {
        let mut pair = Pair::opened_with(REAL, MAX_DEPTH, ei);
        let before = pair.rig.frames();
        let new = pair.fresh(1);
        pair.key(1, 0, Act::Push(new), &[status]);
        assert_eq!(pair.rig.frames(), before);
        let again = pair.fresh(2);
        pair.key(1, 0, Act::Push(again), &[]);
        let mut want = before;
        want.remove(2);
        want.push(again);
        assert_eq!(pair.rig.frames(), want);
    }
}

#[test]
fn a_failed_push_below_a_full_stack_after_a_full_push_brings_nothing_back() {
    for ei in [true, false] {
        a_failed_push_below_a_full_stack_after_a_full_push_brings_nothing_back_in(ei);
    }
}

fn a_failed_push_below_a_full_stack_after_a_full_push_brings_nothing_back_in(ei: bool) {
    let mut pair = Pair::opened_with(REAL, MAX_DEPTH, ei);
    let evicting = pair.fresh(1);
    pair.key(1, 0, Act::Push(evicting), &[]);
    pair.key(6, 0, Act::Pop, &[]);
    let before = pair.rig.frames();
    mark_dead(&mut pair, MAX_DEPTH - 1);
    let raw_before = raw(&mut pair);
    assert_eq!(before.len(), MAX_DEPTH - 1);
    let new = pair.fresh(3);
    let keyed = pair.key(1, 0, Act::Push(new), &[9]);
    assert_eq!(keyed.res, FAILED);
    assert_eq!(pair.rig.frames(), before);
    unchanged(&mut pair, &raw_before, MAX_DEPTH - 1);
}

#[test]
fn a_second_failing_enter_reports_the_first_status_and_still_restores_the_stack() {
    for ei in [true, false] {
        a_second_failing_enter_reports_the_first_status_and_still_restores_the_stack_in(ei);
    }
}

fn a_second_failing_enter_reports_the_first_status_and_still_restores_the_stack_in(ei: bool) {
    for depth in [1, 4, 8] {
        let mut pair = Pair::opened_with(REAL, depth, ei);
        mark_dead(&mut pair, depth);
        let before = raw(&mut pair);
        let new = pair.fresh(2);
        let keyed = pair.key(1, 0, Act::Push(new), &[5, 6]);
        assert_eq!((keyed.res, keyed.status), (FAILED, 5));
        unchanged(&mut pair, &before, depth);
    }
}

#[test]
fn an_idle_push_that_fails_twice_reports_the_first_status() {
    for ei in [true, false] {
        for depth in [1, MAX_DEPTH] {
            let mut pair = Pair::opened_with(REAL, depth, ei);
            let new = pair.fresh(2);
            let keyed = pair.idle(Act::Push(new), true, &[5, 6]);
            assert_eq!((keyed.res, keyed.status), (FAILED, 5), "depth {depth}");
            assert_eq!(pair.rig.depth(), depth);
        }
    }
}

#[test]
fn a_failing_enter_after_a_pop_keeps_the_pop_and_reports_the_status() {
    for ei in [true, false] {
        a_failing_enter_after_a_pop_keeps_the_pop_and_reports_the_status_in(ei);
    }
}

fn a_failing_enter_after_a_pop_keeps_the_pop_and_reports_the_status_in(ei: bool) {
    for depth in 2..=MAX_DEPTH {
        let mut pair = Pair::opened_with(REAL, depth, ei);
        let before = pair.rig.frames();
        let keyed = pair.key(6, 0, Act::Pop, &[9]);
        assert_eq!((keyed.res, keyed.status), (FAILED, 9));
        assert_eq!(pair.rig.frames(), before[..depth - 1]);
        pair.rig.arm(&[0]);
        let retry = pair.rig.call("SCR_REFRESH", Regs::default());
        assert_eq!(retry.a(), 0);
        let log = pair.rig.log();
        assert_eq!(log.len(), 1, "the retry calls enter once");
        assert_eq!(log[0].routine, Routine::Enter);
        assert_eq!(log[0].frame, before[depth - 2]);
        assert_eq!(log[0].at, pair.rig.stack() + 16 * (depth as u16 - 2));
    }
}

#[test]
fn a_failing_enter_after_a_jump_skips_the_land() {
    for ei in [true, false] {
        a_failing_enter_after_a_jump_skips_the_land_in(ei);
    }
}

fn a_failing_enter_after_a_jump_skips_the_land_in(ei: bool) {
    for depth in 2..=MAX_DEPTH {
        let mut pair = Pair::opened_with(REAL, depth, ei);
        let keyed = pair.key(5, 0, Act::Jump(0x4321), &[200]);
        assert_eq!((keyed.res, keyed.status), (FAILED, 200));
        assert!(keyed.calls.iter().all(|c| c.routine != Routine::Land));
        assert_eq!(pair.rig.depth(), depth - 1);
    }
}

#[test]
fn the_stack_stays_usable_after_a_failure() {
    for ei in [true, false] {
        the_stack_stays_usable_after_a_failure_in(ei);
    }
}

fn the_stack_stays_usable_after_a_failure_in(ei: bool) {
    let mut pair = Pair::opened_with(REAL, MAX_DEPTH, ei);
    let new = pair.fresh(1);
    pair.key(1, 0, Act::Push(new), &[8]);
    for _ in 0..3 {
        let next = pair.fresh(2);
        pair.key(1, 0, Act::Push(next), &[]);
        pair.key(6, 0, Act::Pop, &[4]);
        pair.key(5, 0, Act::Jump(9), &[]);
    }
}
