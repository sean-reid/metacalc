use crate::model::Pair;
use crate::support::*;

#[test]
fn pushes_on_a_full_stack_keep_the_two_bottom_frames_and_the_newest_ones_in_order() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let start = pair.rig.frames();
    let mut pushed = vec![];
    for n in 1..=12 {
        let new = pair.fresh(KINDS[n % KINDS.len()].0);
        pushed.push(new);
        assert_eq!(pair.key(1, 0, Act::Push(new), &[]).res, DRAW);
        let mut want = start[..2].to_vec();
        if n <= 6 {
            want.extend(&start[2 + n..]);
            want.extend(&pushed);
        } else {
            want.extend(&pushed[n - 6..]);
        }
        assert_eq!(pair.rig.frames(), want, "after {n} pushes");
        assert_eq!(pair.rig.depth(), MAX_DEPTH);
    }
}

#[test]
fn popping_after_full_pushes_walks_back_through_the_newest_screens_to_the_second() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let start = pair.rig.frames();
    let mut pushed = vec![];
    for _ in 0..9 {
        let new = pair.fresh(1);
        pushed.push(new);
        pair.key(1, 0, Act::Push(new), &[]);
    }
    let mut walked = vec![];
    while pair.rig.depth() > 1 {
        walked.push(pair.rig.top().1);
        pair.key(6, 0, Act::Pop, &[]);
    }
    walked.push(pair.rig.top().1);
    let mut want: Vec<Frame> = pushed[3..].iter().rev().copied().collect();
    want.push(start[1]);
    want.push(start[0]);
    assert_eq!(walked, want);
}

#[test]
fn a_jump_on_a_full_stack_lands_the_screen_under_the_top() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let new = pair.fresh(2);
    pair.key(1, 0, Act::Push(new), &[]);
    let before = pair.rig.frames();
    let keyed = pair.key(5, 0, Act::Jump(0x0102), &[]);
    assert_eq!(keyed.calls.len(), 3);
    assert_eq!(pair.rig.frames(), before[..7]);
    assert_eq!(keyed.calls[2].frame, before[6]);
}

#[test]
fn a_stack_grown_and_shrunk_again_keeps_every_frame_byte_for_byte() {
    let mut pair = Pair::opened(REAL, 5);
    for (kind, _) in KINDS.iter().take(4) {
        for _ in 0..4 {
            let new = pair.fresh(*kind);
            pair.key(1, 0, Act::Push(new), &[]);
        }
        for _ in 0..3 {
            pair.key(6, 0, Act::Pop, &[]);
        }
    }
}
