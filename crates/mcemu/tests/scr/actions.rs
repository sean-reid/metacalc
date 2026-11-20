use crate::model::Pair;
use crate::support::*;

const KEY_LETTER: u8 = 12;

fn routines(calls: &[Rec]) -> Vec<Routine> {
    calls.iter().map(|c| c.routine).collect()
}

fn raw_stack(pair: &mut Pair) -> Vec<u8> {
    let at = pair.rig.stack();
    pair.rig.peek(at, 128)
}

#[test]
fn init_clears_the_stack_pushes_the_frame_and_enters_it() {
    let mut pair = Pair::new(REAL);
    let at = pair.rig.stack();
    pair.rig.poke(at, &[0x77; 128]);
    let first = pair.fresh(2);
    assert_eq!(pair.init(first, &[]), 0);
    let raw = raw_stack(&mut pair);
    assert_eq!(raw[..16], first);
    assert!(
        raw[16..].iter().all(|&b| b == 0),
        "the rest of the stack is clear"
    );
    let (_, log) = pair.rig.init(first, &[]);
    assert_eq!(routines(&log), [Routine::Enter]);
    assert_eq!((log[0].kind, log[0].at, log[0].frame), (2, at, first));
}

#[test]
fn init_hands_back_a_failing_enter_with_one_screen_open() {
    let mut pair = Pair::new(REAL);
    let first = pair.fresh(3);
    assert_eq!(pair.init(first, &[9]), 9);
    assert_eq!(pair.rig.depth(), 1);
    assert_eq!(pair.rig.frames(), [first]);
}

#[test]
fn a_second_init_replaces_the_stack() {
    let mut pair = Pair::opened(REAL, 6);
    let first = pair.fresh(1);
    pair.init(first, &[]);
    assert_eq!(pair.rig.frames(), [first]);
    let raw = raw_stack(&mut pair);
    assert!(raw[16..].iter().all(|&b| b == 0));
}

#[test]
fn stay_and_redraw_call_only_key_with_the_key_it_was_given() {
    for depth in 1..=MAX_DEPTH {
        for (act, res) in [
            (Act::Stay, IDLE),
            (Act::Redraw, DRAW),
            (Act::Other(9), IDLE),
        ] {
            let mut pair = Pair::opened(REAL, depth);
            let before = raw_stack(&mut pair);
            let top = pair.mirror.frames[depth - 1];
            let keyed = pair.key(KEY_LETTER, b'Q', act, &[]);
            assert_eq!(keyed.res, res, "{act:?} at depth {depth}");
            assert_eq!(routines(&keyed.calls), [Routine::Key]);
            let call = &keyed.calls[0];
            assert_eq!((call.a, call.b, call.frame), (KEY_LETTER, b'Q', top));
            assert_eq!(raw_stack(&mut pair), before, "{act:?} at depth {depth}");
        }
    }
}

#[test]
fn push_below_a_full_stack_adds_the_frame_on_top_and_enters_it() {
    for depth in 1..MAX_DEPTH {
        let mut pair = Pair::opened(REAL, depth);
        let before = pair.rig.frames();
        let new = pair.fresh(KINDS[depth % KINDS.len()].0);
        let keyed = pair.key(1, 0, Act::Push(new), &[]);
        assert_eq!(keyed.res, DRAW);
        assert_eq!(routines(&keyed.calls), [Routine::Key, Routine::Enter]);
        let mut want = before;
        want.push(new);
        assert_eq!(pair.rig.frames(), want, "depth {depth}");
        let enter = &keyed.calls[1];
        assert_eq!(enter.at, pair.rig.stack() + 16 * depth as u16);
        assert_eq!(enter.frame, new);
    }
}

#[test]
fn push_on_a_full_stack_removes_the_frame_at_index_two() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let before = pair.rig.frames();
    let new = pair.fresh(2);
    let keyed = pair.key(1, 0, Act::Push(new), &[]);
    assert_eq!(keyed.res, DRAW);
    let mut want = before.clone();
    want.remove(2);
    want.push(new);
    assert_eq!(pair.rig.frames(), want);
    assert_eq!(&want[..2], &before[..2]);
    assert_eq!(&want[2..7], &before[3..]);
    assert_eq!(routines(&keyed.calls), [Routine::Key, Routine::Enter]);
    assert_eq!(keyed.calls[1].at, pair.rig.stack() + 16 * 7);
}

#[test]
fn pop_removes_the_top_frame_and_enters_the_one_it_uncovers() {
    for depth in 2..=MAX_DEPTH {
        let mut pair = Pair::opened(REAL, depth);
        let before = pair.rig.frames();
        let keyed = pair.key(6, 0, Act::Pop, &[]);
        assert_eq!(keyed.res, DRAW);
        assert_eq!(routines(&keyed.calls), [Routine::Key, Routine::Enter]);
        assert_eq!(pair.rig.frames(), before[..depth - 1]);
        let enter = &keyed.calls[1];
        assert_eq!(enter.frame, before[depth - 2]);
        assert_eq!(enter.at, pair.rig.stack() + 16 * (depth as u16 - 2));
    }
}

#[test]
fn pop_with_one_screen_open_reports_leave_and_changes_nothing() {
    let mut pair = Pair::opened(REAL, 1);
    let before = raw_stack(&mut pair);
    let keyed = pair.key(6, 0, Act::Pop, &[]);
    assert_eq!(keyed.res, LEAVE);
    assert_eq!(routines(&keyed.calls), [Routine::Key]);
    assert_eq!(raw_stack(&mut pair), before);
    assert_eq!(pair.rig.depth(), 1);
}

#[test]
fn popping_down_to_one_screen_then_again_leaves_with_the_stack_untouched() {
    let mut pair = Pair::opened(REAL, MAX_DEPTH);
    let bottom = pair.rig.frames()[0];
    for left in (1..MAX_DEPTH).rev() {
        assert_eq!(pair.key(6, 0, Act::Pop, &[]).res, DRAW);
        assert_eq!(pair.rig.depth(), left);
    }
    let before = raw_stack(&mut pair);
    for _ in 0..3 {
        assert_eq!(pair.key(6, 0, Act::Pop, &[]).res, LEAVE);
        assert_eq!(raw_stack(&mut pair), before);
    }
    assert_eq!(pair.rig.frames(), [bottom]);
}

#[test]
fn jump_enters_the_uncovered_screen_and_then_lands_it_on_the_assertion() {
    for depth in 2..=MAX_DEPTH {
        let mut pair = Pair::opened(REAL, depth);
        let before = pair.rig.frames();
        let keyed = pair.key(5, 0, Act::Jump(0xBEEF), &[]);
        assert_eq!(keyed.res, DRAW);
        assert_eq!(
            routines(&keyed.calls),
            [Routine::Key, Routine::Enter, Routine::Land]
        );
        let land = &keyed.calls[2];
        assert_eq!((land.de, land.frame), (0xBEEF, before[depth - 2]));
        assert_eq!(land.at, keyed.calls[1].at);
        assert_eq!(pair.rig.frames(), before[..depth - 1]);
    }
}

#[test]
fn jump_with_one_screen_open_does_nothing() {
    let mut pair = Pair::opened(REAL, 1);
    let before = raw_stack(&mut pair);
    let keyed = pair.key(5, 0, Act::Jump(7), &[]);
    assert_eq!(keyed.res, IDLE);
    assert_eq!(routines(&keyed.calls), [Routine::Key]);
    assert_eq!(raw_stack(&mut pair), before);
}

#[test]
fn every_action_from_every_depth_agrees_with_the_model() {
    for depth in 1..=MAX_DEPTH {
        for kind in KINDS.map(|(kind, _)| kind) {
            for act in 0..6 {
                let mut pair = Pair::opened(REAL, depth);
                let top = pair.mirror.frames.len() - 1;
                pair.mirror.frames[top][0] = kind;
                let mut raw = pair.mirror.frames[top];
                raw[0] = kind;
                let at = pair.rig.stack() + 16 * top as u16;
                pair.rig.poke(at, &raw);
                let new = pair.fresh(kind);
                let act = match act {
                    0 => Act::Stay,
                    1 => Act::Redraw,
                    2 => Act::Push(new),
                    3 => Act::Pop,
                    4 => Act::Jump(0x1234),
                    _ => Act::Other(77),
                };
                pair.key(KEY_LETTER, b'Z', act, &[]);
            }
        }
    }
}

#[test]
fn draw_calls_only_the_top_screens_draw_and_refresh_only_its_enter() {
    for depth in 1..=MAX_DEPTH {
        let mut pair = Pair::opened(REAL, depth);
        let before = raw_stack(&mut pair);
        pair.rig.arm(&[]);
        pair.rig.call("SCR_DRAW", Regs::default());
        let log = pair.rig.log();
        assert_eq!(
            log.iter().cloned().map(Rec::given).collect::<Vec<_>>(),
            pair.mirror.draw()
        );
        pair.rig.arm(&[4]);
        let out = pair.rig.call("SCR_REFRESH", Regs::default());
        assert_eq!(out.a(), 4);
        let log = pair.rig.log();
        assert_eq!(routines(&log), [Routine::Enter]);
        assert_eq!(log[0].frame, pair.mirror.frames[depth - 1]);
        assert_eq!(raw_stack(&mut pair), before);
        assert_eq!(pair.rig.depth(), depth);
    }
}

#[test]
fn the_variables_use_the_bytes_the_header_says() {
    let rig = Rig::new(REAL);
    assert_eq!(rig.label("SCR_USED"), 41);
}
