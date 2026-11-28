use crate::caps::{neighbours, snapshot};
use crate::guard::{Allowed, pattern, prime, stray};
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;

/// The reader's state bytes each entry may write, by the labels the build gives.
const SCRATCH: [(&str, u16); 3] = [("LR_BUFP", 2), ("LR_SCRATCH", 2), ("LR_TR", 3)];
const START: [(&str, u16); 19] = [
    ("LR_HOFF", 3),
    ("LR_WANT", 1),
    ("LR_WBASE", 3),
    ("LR_WP", 1),
    ("LR_Q", 2),
    ("LR_OFF", 3),
    ("LR_DN", 0x16),
    ("LR_BUFP", 2),
    ("LR_SCRATCH", 2),
    ("LR_TR", 3),
    ("LR_K", 1),
    ("LR_CN", 2),
    ("LR_HDR", 4),
    ("LR_DC", 32),
    ("LR_DCBASE", 2),
    ("LR_SR", 2),
    ("LR_N", 2),
    ("LR_ENT", 3),
    ("LR_WIN", 32),
];

fn state_changes(rig: &mut Rig, before: &[u8], named: &[(&str, u16)]) -> Vec<u16> {
    let at = rig.label("LIB_STATE");
    let after = rig.peek(at, before.len());
    let ok: Vec<std::ops::Range<u16>> = named
        .iter()
        .map(|&(n, len)| rig.label(n)..rig.label(n) + len)
        .collect();
    (0..before.len())
        .filter(|&i| before[i] != after[i])
        .map(|i| at + i as u16)
        .filter(|a| !ok.iter().any(|r| r.contains(a)))
        .collect()
}

/// A primed machine with the fixture open and the borrowed bytes' neighbours noted.
fn primed(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_image(PW);
    prime(&mut rig);
    rig.load(bytes);
    rig.open_ok();
    rig
}

fn allowed_for(rig: &mut Rig, block: usize) -> Allowed {
    let mut allowed = Allowed::of(PW);
    let l = |n: &str| rig.label(n);
    let low_end = l("PW_STACK_TOP") - l("PW_STACK_ROOM");
    allowed.extra.push((
        SPARE_PAGE,
        l("PW_VARS"),
        usize::from(low_end - l("PW_VARS")),
    ));
    allowed.extra.push((SPARE_PAGE, l("PW_STACK_TOP") - 16, 16));
    allowed
        .extra
        .push((SPARE_PAGE, l("FM_TOKENS"), usize::from(l("PW_TOKENS_USED"))));
    allowed.extra.push((APP_PAGE, BLOCK, block));
    allowed
}

#[test]
fn a_start_a_continue_and_a_find_write_only_the_bytes_the_header_names() {
    let bytes = fixture();
    let tc = provable(&bytes).unwrap();
    let n = number_of(&bytes, "mp2s");
    let mut rig = primed(&bytes);
    let regions = neighbours(&rig);
    let near = snapshot(&mut rig, &regions);
    let size = usize::from(rig.label("LIB_STATE_SIZE"));
    let state = rig.label("LIB_STATE");
    let mut touched = [false; 3];
    let mut calls = 0;
    rig.seed_result();
    let before = rig.peek(state, size);
    let mut out = rig.call(SHAPE, regs_with(n, tc, 4));
    let allowed = allowed_for(&mut rig, 8);
    assert_eq!(stray(&mut rig, &allowed), vec![], "start");
    assert_eq!(
        state_changes(&mut rig, &before, &START),
        vec![],
        "start: state"
    );
    assert_eq!(out.status(), MORE, "the precondition: a cut start");
    while out.status() == MORE {
        calls += 1;
        let before = rig.peek(state, size);
        out = rig.more(4);
        let allowed = allowed_for(&mut rig, 8);
        assert_eq!(stray(&mut rig, &allowed), vec![], "continue {calls}");
        assert_eq!(
            state_changes(&mut rig, &before, &SCRATCH),
            vec![],
            "continue: state"
        );
    }
    touched[0] = calls > 1;
    assert_eq!(out.status(), ST_OK);
    assert_ne!(rig.result_bytes(), [MARK; 8]);
    let place = 3;
    rig.seed_find();
    let before = rig.peek(state, size);
    let out = rig.find(place);
    assert_eq!(out.status(), ST_OK);
    let allowed = allowed_for(&mut rig, 47);
    assert_eq!(stray(&mut rig, &allowed), vec![], "find");
    assert_eq!(
        state_changes(&mut rig, &before, &SCRATCH),
        vec![],
        "find: state"
    );
    touched[1] = rig.find_bytes() != [MARK; FIND_SIZE];
    assert_eq!(
        near,
        snapshot(&mut rig, &regions),
        "the neighbours of the borrowed bytes"
    );
    touched[2] = true;
    assert_eq!(touched, [true; 3]);
}

#[test]
fn a_refused_continue_and_a_refused_find_write_nothing_on_the_spare_page_or_in_the_block() {
    let bytes = fixture();
    let n = number_of(&bytes, "mp2s");
    let tc = provable(&bytes).unwrap();
    let mut rig = primed(&bytes);
    let page = rig.m.ram_page(SPARE_PAGE).to_vec();
    rig.seed_result();
    assert_eq!(rig.more(5).status(), NO_WALK);
    rig.seed_find();
    assert_eq!(rig.find(1).status(), NO_TABLE);
    assert_eq!(rig.m.ram_page(SPARE_PAGE).to_vec(), page);
    assert_eq!(rig.result_bytes(), [MARK; 8]);
    assert_eq!(rig.find_bytes(), [MARK; FIND_SIZE]);
    let (status, _) = rig.sliced(n, tc, 5);
    assert_eq!(status, ST_OK);
    let page = rig.m.ram_page(SPARE_PAGE).to_vec();
    rig.seed_result();
    assert_eq!(rig.more(5).status(), NO_WALK, "after the walk ended");
    assert_eq!(rig.m.ram_page(SPARE_PAGE).to_vec(), page);
    assert_eq!(rig.result_bytes(), [MARK; 8]);
    for place in [0, 6, 0xFF_FFFF] {
        rig.seed_find();
        assert_eq!(rig.find(place).status(), PLACE);
        assert_eq!(rig.find_bytes(), [MARK; FIND_SIZE]);
    }
    assert_eq!(
        rig.find(5).status(),
        ST_OK,
        "the table is as the walk left it"
    );
}

/// The caller's ports, interrupt state, speed and index registers after every kind of call.
#[test]
fn ports_interrupts_speed_and_indexes_are_the_callers_after_a_continue_and_a_find_too() {
    let bytes = fixture();
    let n = number_of(&bytes, "mp2s");
    let tc = provable(&bytes).unwrap();
    let mut kinds = std::collections::BTreeSet::new();
    for (port5, port7, ei, speed) in [
        (0u8, 0x81u8, false, 0u8),
        (2, 0x81, true, 1),
        (7, 0x84, true, 0),
        (4, 0x87, false, 1),
    ] {
        let mut rig = Rig::with_image(PW);
        rig.m.out_port(5, port5);
        rig.m.out_port(7, port7);
        rig.ei = ei;
        rig.load(&bytes);
        rig.open_ok();
        rig.m.out_port(0x20, speed);
        let before = rig.m.in_port(0x20);
        let mut calls = vec![rig.find(1)];
        let mut out = rig.call(SHAPE, regs_with(n, tc, 4));
        calls.push(out);
        while out.status() == MORE {
            out = rig.more(4);
            calls.push(out);
        }
        calls.push(rig.more(4));
        calls.push(rig.find(2));
        calls.push(rig.find(0));
        calls.push(rig.find(1_000));
        for out in calls {
            let what = format!("{} at {port5}/{port7:#x}/{ei}/{speed}", out.status());
            assert_eq!((out.ix, out.iy), (0x1357, 0x2468), "{what}");
            assert_eq!(
                (out.port5, out.port7, out.iff),
                (port5, port7, u8::from(ei)),
                "{what}"
            );
            assert_eq!(rig.m.in_port(0x20), before, "{what}");
            assert_eq!(out.af & 0x40 != 0, out.status() == 0, "{what}");
            kinds.insert(out.status());
        }
    }
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        [0, MORE, NO_WALK, PLACE, NO_TABLE]
    );
}

/// Bytes of the caller's stack and of the walk's own the call used, below their tops, with both
/// primed before it.
fn depths(rig: &mut Rig, flip: u8, call: impl FnOnce(&mut Rig) -> Out) -> (u8, usize, usize) {
    for (at, byte) in rig.m.ram_page(OS_PAGE).iter_mut().enumerate() {
        *byte = pattern(OS_PAGE, at) ^ flip;
    }
    let top = rig.label("PW_STACK_TOP");
    let room = rig.label("PW_STACK_ROOM");
    for at in top - room..top {
        rig.m.ram_page(SPARE_PAGE)[usize::from(at - 0xC000)] =
            pattern(SPARE_PAGE, usize::from(at)) ^ flip;
    }
    let out = call(rig);
    let os = rig.m.ram_page(OS_PAGE).to_vec();
    let lowest = (0..os.len())
        .find(|&at| os[at] != pattern(OS_PAGE, at) ^ flip)
        .unwrap();
    let spare = rig.m.ram_page(SPARE_PAGE).to_vec();
    let own = (top - room..top)
        .find(|&at| spare[usize::from(at - 0xC000)] != pattern(SPARE_PAGE, usize::from(at)) ^ flip)
        .map_or(0, |at| usize::from(top - at));
    (out.status(), os.len() - lowest, own)
}

/// The deepest the caller's stack and the walk's own go in a start, a continue and a find, over
/// every status each gives on the fixture.
pub fn entry_depths() -> [(usize, usize); 3] {
    let bytes = fixture();
    let tc = provable(&bytes).unwrap();
    let mut worst = [(0, 0); 3];
    let mut note = |k: usize, (_, c, p): (u8, usize, usize)| {
        worst[k] = (worst[k].0.max(c), worst[k].1.max(p));
    };
    for flip in [0u8, 0xFF] {
        for name in ["mp2s", "use3", "ref1"] {
            let n = number_of(&bytes, name);
            let mut rig = opened(&bytes);
            note(2, depths(&mut rig, flip, |r| r.find(1)));
            note(1, depths(&mut rig, flip, |r| r.more(3)));
            let mut out = Out::default();
            let first = depths(&mut rig, flip, |r| {
                out = r.call(SHAPE, regs_with(n, tc, 3));
                out
            });
            note(0, first);
            while out.status() == MORE {
                note(
                    1,
                    depths(&mut rig, flip, |r| {
                        out = r.more(3);
                        out
                    }),
                );
            }
            for place in [1, 2, 1000, 0] {
                note(2, depths(&mut rig, flip, |r| r.find(place)));
            }
            note(1, depths(&mut rig, flip, |r| r.more(3)));
        }
    }
    worst
}

#[test]
fn the_stacks_of_a_start_a_continue_and_a_find_go_exactly_as_deep_as_the_header_says() {
    let got = entry_depths();
    println!("{got:?}");
    assert_eq!(got, [(12, 6), (4, 6), (4, 4)]);
}
