use crate::model::Pair;
use crate::support::*;

#[test]
fn a_pushed_frame_is_what_a_push_from_a_key_does_at_every_depth() {
    for ei in [true, false] {
        for depth in 1..=MAX_DEPTH {
            let mut pair = Pair::opened_with(REAL, depth, ei);
            let new = pair.fresh(3);
            let pushed = pair.push_frame(new, &[]);
            assert_eq!(pushed.res, DRAW, "depth {depth}");
            let top = pair.rig.top().1;
            assert_eq!(top, new);
            assert_eq!(pair.rig.depth(), (depth + 1).min(MAX_DEPTH));
        }
    }
}

#[test]
fn a_pushed_frame_on_a_full_stack_evicts_the_frame_at_index_2() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let before = pair.rig.frames();
    let new = pair.fresh(2);
    pair.push_frame(new, &[]);
    let mut want = before;
    want.remove(2);
    want.push(new);
    assert_eq!(pair.rig.frames(), want);
}

#[test]
fn a_pushed_frame_whose_enter_fails_leaves_the_stack_as_it_was() {
    for depth in [1, 4, MAX_DEPTH] {
        let mut pair = Pair::opened(REAL, depth);
        let before = pair.rig.frames();
        let new = pair.fresh(3);
        let pushed = pair.push_frame(new, &[77]);
        assert_eq!((pushed.res, pushed.status), (FAILED, 77), "depth {depth}");
        assert_eq!(pair.rig.frames(), before);
    }
}

#[test]
fn a_pushed_frame_that_fails_twice_reports_the_second_status() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let new = pair.fresh(1);
    let pushed = pair.push_frame(new, &[5, 6]);
    assert_eq!((pushed.res, pushed.status), (FAILED, 6));
}

#[test]
fn a_pushed_frame_is_copied_so_the_caller_may_reuse_its_own() {
    let mut pair = Pair::opened(REAL, 2);
    let new = pair.fresh(2);
    pair.push_frame(new, &[]);
    let at = pair.rig.label("FK_SCRIPT") + 0x100;
    pair.rig.poke(at, &[0xEE; 16]);
    assert_eq!(pair.rig.top().1, new);
}

#[test]
fn a_pushed_frame_uses_no_more_stack_than_a_push_from_a_key() {
    let mut worst = 0;
    for depth in [1, MAX_DEPTH] {
        for statuses in [&[][..], &[3, 0][..]] {
            let mut pair = Pair::opened(REAL, depth);
            let new = pair.fresh(2);
            pair.mirror.push_frame(new, statuses);
            worst = worst.max(pair.rig.push_frame(new, statuses).stack);
        }
    }
    println!("scr_push_frame reached {worst} bytes of stack");
    assert!(worst <= 10, "{worst}");
}
