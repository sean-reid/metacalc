use crate::findrig::*;
use crate::findstages::*;
use crate::isolation::{Allowed, prime, stray};
use crate::support::*;

#[test]
fn the_calls_write_only_the_state_the_bounce_buffer_and_the_callers_bytes() {
    let bytes = library(&labels(), Some(&bits()));
    let mut rig = Rig::new();
    rig.m.out_port(5, 0);
    prime(&mut rig);
    rig.load(&bytes);
    rig.open_ok();
    let mut allowed = Allowed::of(DEFAULT);
    allowed.extra.push((APP_PAGE, 0xBFF0, 16));
    let state = rig.label("LIB_STATE");
    let size = usize::from(rig.label("LIB_STATE_SIZE"));
    let persistent = |rig: &mut Rig| {
        let mut all = rig.peek(state + 0x10, 0x30);
        all.extend(rig.peek(state + 0xA8, size - 0xA8));
        all
    };
    let before = persistent(&mut rig);
    let mut ran = 0;
    for e in exits(Stage::Open) {
        run(&mut rig, &e);
        ran += 1;
        assert_eq!(
            stray(&mut rig, &allowed),
            Vec::<(u8, u16)>::new(),
            "{}",
            e.name
        );
        assert_eq!(
            persistent(&mut rig),
            before,
            "{}: the facts, the descriptor and the sections are kept",
            e.name
        );
    }
    assert_eq!(ran, 21);
}

#[test]
fn each_call_drops_the_record_window_and_the_directory_cache_as_the_header_says() {
    let bytes = crate::sections::headed(120, 3, 0);
    let count = decoded(&bytes).assertions.len();
    let n = 42u16;
    assert_eq!(
        decoded(&bytes).assertions[usize::from(n)].hypotheses.len(),
        6
    );
    let mut rig = opened(&bytes);
    let mut check = |what: &str,
                     target: &str,
                     regs: Regs,
                     pokes: &[(u16, Vec<u8>)],
                     status: u8,
                     window: bool,
                     cache: bool| {
        rig.hypothesis(n, 0);
        rig.label_of(n);
        let held = |rig: &mut Rig| {
            (
                rig.state_byte("LR_WLEN"),
                rig.state_byte("LR_WP"),
                rig.state_word("LR_DCBASE", 2),
                rig.state_word("LR_DN", 2),
            )
        };
        let before = held(&mut rig);
        assert!(
            before.0 != 0 && before.2 != 0xFFFF && before.3 == u32::from(n),
            "{what}: {before:?}"
        );
        for (at, bytes) in pokes {
            rig.poke(*at, bytes);
        }
        let out = rig.call(target, regs);
        assert_eq!(out.status(), status, "{what}");
        let after = held(&mut rig);
        assert_eq!(after.3, u32::from(n), "{what}: the descriptor is kept");
        if window {
            assert_eq!((after.0, after.1), (0, 0), "{what}: the window is dropped");
        } else {
            assert_eq!(
                (after.0, after.1),
                (before.0, before.1),
                "{what}: the window is kept"
            );
        }
        if cache {
            assert_eq!(after.2, 0xFFFF, "{what}: the cache is dropped");
        } else {
            assert_eq!(after.2, before.2, "{what}: the cache is kept");
        }
    };
    let c = count as u16;
    let pre = |p: &[u8]| vec![(PREFIX_AT, p.to_vec())];
    let blk = |b: [u8; 9]| vec![(BLOCK_AT, b.to_vec())];
    check(
        "range",
        RANGE,
        range_regs(PREFIX_AT, 5),
        &pre(b"ax-00"),
        ST_OK,
        true,
        false,
    );
    check(
        "range, none",
        RANGE,
        range_regs(PREFIX_AT, 1),
        &pre(b"q"),
        ST_OK,
        true,
        false,
    );
    check(
        "range, length 0",
        RANGE,
        range_regs(PREFIX_AT, 0),
        &pre(b"q"),
        ST_ARG,
        false,
        false,
    );
    check(
        "range, pointer",
        RANGE,
        range_regs(0xC000, 3),
        &[],
        ST_ARG,
        false,
        false,
    );
    check(
        "walk",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, c, 0, c, THEOREM)),
        ST_OK,
        true,
        true,
    );
    check(
        "walk, every",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, 5, 0, c, EVERY)),
        ST_OK,
        true,
        true,
    );
    check(
        "walk, empty range",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, 0, 0, c, EVERY)),
        ST_OK,
        true,
        true,
    );
    check(
        "walk, bad range",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, c + 1, 0, c, EVERY)),
        ST_INDEX,
        true,
        true,
    );
    check(
        "walk, bad start",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, c, 5, 4, EVERY)),
        ST_INDEX,
        true,
        true,
    );
    check(
        "walk, bad kind",
        WALK,
        walk_regs(BLOCK_AT),
        &blk(block(0, c, 0, c, 7)),
        ST_ARG,
        false,
        false,
    );
    check(
        "walk, bad block",
        WALK,
        walk_regs(0xC000),
        &[],
        ST_ARG,
        false,
        false,
    );
    check(
        "copy",
        COPY,
        copy_regs(0, 20, COPY_AT),
        &[],
        ST_OK,
        false,
        false,
    );
    check(
        "copy, none",
        COPY,
        copy_regs(0, 0, COPY_AT),
        &[],
        ST_OK,
        false,
        false,
    );
    check(
        "copy, bad range",
        COPY,
        copy_regs(0, c + 1, COPY_AT),
        &[],
        ST_INDEX,
        false,
        false,
    );
}
