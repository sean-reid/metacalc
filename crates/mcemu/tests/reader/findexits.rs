use crate::damaged::{offset_of, row_of, with};
use crate::findrig::*;
use crate::isolation::{Allowed, pattern, prime, stray};
use crate::secrig::*;
use crate::support::*;
use mcpack::library::{KINDS, LABEL_ORDER};

/// Bytes of stack the header says the deepest exit of the three calls uses below its return address.
const STACK_DEPTH: usize = 10;
const COUNT: u16 = 40;

fn labels() -> Vec<String> {
    (0..COUNT).map(|i| format!("ax-{i}")).collect()
}

fn bits() -> Vec<bool> {
    pseudo(usize::from(COUNT), 5)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Closed,
    NoSection,
    Open,
    /// The library ends inside the label order section.
    CutOrder,
    /// The kinds bitmap is where no byte can be read, the label order section is whole.
    CutKinds,
}

const STAGES: [Stage; 5] = [
    Stage::Closed,
    Stage::NoSection,
    Stage::Open,
    Stage::CutOrder,
    Stage::CutKinds,
];

fn staged_at(stage: Stage, junk: Option<u8>, ports: Option<(u8, u8, bool)>) -> Rig {
    let good = library(&labels(), Some(&bits()));
    let bytes = if stage == Stage::NoSection {
        with(&good, |b| b[row_of(&good, LABEL_ORDER)] = 77)
    } else {
        good.clone()
    };
    let mut rig = Rig::new();
    if let Some(flip) = junk {
        for page in 0..8u8 {
            for (at, byte) in rig.m.ram_page(page).iter_mut().enumerate() {
                *byte = pattern(page, at) ^ flip;
            }
        }
    }
    if let Some((port5, port7, ei)) = ports {
        rig.m.out_port(5, port5);
        rig.m.out_port(7, port7);
        rig.ei = ei;
    }
    rig.load(&bytes);
    if stage != Stage::Closed {
        rig.open_ok();
    }
    match stage {
        Stage::CutOrder => {
            let at = offset_of(&good, LABEL_ORDER) + 5;
            rig.poke(rig.label("LR_LENGTH"), &at.to_le_bytes()[..3]);
        }
        Stage::CutKinds => {
            let end = rig.state_word("LR_LENGTH", 3) as u32;
            rig.poke(rig.label("LS_KD"), &end.to_le_bytes()[..3]);
        }
        _ => {}
    }
    rig
}

struct Exit {
    name: &'static str,
    target: &'static str,
    regs: Regs,
    status: u8,
    /// Prepares the caller's memory.
    pokes: Vec<(u16, Vec<u8>)>,
}

fn range_exit(name: &'static str, prefix: &[u8], len: u8, at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: RANGE,
        regs: range_regs(at, len),
        status,
        pokes: if at < 0xC000 { vec![(at, prefix.to_vec())] } else { Vec::new() },
    }
}

fn walk_exit(name: &'static str, b: [u8; 9], at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: WALK,
        regs: walk_regs(at),
        status,
        pokes: vec![(BLOCK_AT, b.to_vec())],
    }
}

fn copy_exit(name: &'static str, first: u16, count: u16, at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: COPY,
        regs: copy_regs(first, count, at),
        status,
        pokes: vec![(COPY_AT, vec![MARK; 0x100])],
    }
}

fn exits(stage: Stage) -> Vec<Exit> {
    let whole = |kind| block(0, COUNT, 0, COUNT, kind);
    match stage {
        Stage::Closed => vec![
            range_exit("closed range", b"ax-1", 4, PREFIX_AT, ST_CLOSED),
            walk_exit("closed walk", whole(EVERY), BLOCK_AT, ST_CLOSED),
            copy_exit("closed copy", 0, 5, COPY_AT, ST_CLOSED),
        ],
        Stage::NoSection => vec![
            range_exit("none range", b"ax-1", 4, PREFIX_AT, ST_NONE),
            walk_exit("none walk", whole(EVERY), BLOCK_AT, ST_NONE),
            copy_exit("none copy", 0, 5, COPY_AT, ST_NONE),
        ],
        Stage::CutOrder => vec![
            range_exit("cut range", b"ax-1", 4, PREFIX_AT, ST_BAD),
            walk_exit("cut walk", whole(EVERY), BLOCK_AT, ST_BAD),
            copy_exit("cut copy", 0, 5, COPY_AT, ST_BAD),
        ],
        Stage::CutKinds => vec![walk_exit("cut kinds walk", whole(THEOREM), BLOCK_AT, ST_BAD)],
        Stage::Open => vec![
            range_exit("range match", b"ax-1", 4, PREFIX_AT, ST_OK),
            range_exit("range none after", b"q", 1, PREFIX_AT, ST_OK),
            range_exit("range none before", b",", 1, PREFIX_AT, ST_OK),
            range_exit("range length 0", b"a", 0, PREFIX_AT, ST_ARG),
            range_exit("range length 9", b"ax-1ax-1a", 9, PREFIX_AT, ST_ARG),
            range_exit("range pointer", b"ax-1", 4, 0xC000, ST_ARG),
            range_exit("range pointer at the end of bank B", b"ax-1", 4, 0xBFFC, ST_OK),
            walk_exit("walk every", whole(EVERY), BLOCK_AT, ST_OK),
            walk_exit("walk theorem", whole(THEOREM), BLOCK_AT, ST_OK),
            walk_exit("walk axiom none", block(0, 1, 0, COUNT, AXIOM), BLOCK_AT, ST_OK),
            walk_exit("walk empty range", block(3, 0, 0, COUNT, THEOREM), BLOCK_AT, ST_OK),
            walk_exit("walk bad kind", whole(2), BLOCK_AT, ST_ARG),
            walk_exit("walk bad block", whole(EVERY), 0xC000, ST_ARG),
            walk_exit("walk bad range", block(0, COUNT + 1, 0, COUNT, EVERY), BLOCK_AT, ST_INDEX),
            walk_exit("walk bad bound", block(0, COUNT, 0, COUNT + 1, EVERY), BLOCK_AT, ST_INDEX),
            walk_exit("walk bad start", block(0, COUNT, 5, 4, EVERY), BLOCK_AT, ST_INDEX),
            copy_exit("copy run", 0, 5, COPY_AT, ST_OK),
            copy_exit("copy long run", 0, COUNT, COPY_AT, ST_OK),
            copy_exit("copy no entries", 3, 0, 0xC000, ST_OK),
            copy_exit("copy bad buffer", 0, 5, 0xC000, ST_ARG),
            copy_exit("copy bad range", 0, COUNT + 1, COPY_AT, ST_INDEX),
        ],
    }
}

fn run(rig: &mut Rig, e: &Exit) -> Out {
    for (at, bytes) in &e.pokes {
        rig.poke(*at, bytes);
    }
    rig.call(e.target, e.regs)
}

#[test]
fn ports_5_and_7_the_interrupt_state_and_the_index_registers_come_back_at_every_exit() {
    let mut seen = 0;
    for stage in STAGES {
        for (port5, port7, ei) in [
            (0u8, 0x81u8, false),
            (2, 0x81, true),
            (7, 0x84, true),
            (4, 0x87, false),
        ] {
            for e in exits(stage) {
                let mut rig = staged_at(stage, None, Some((port5, port7, ei)));
                let what = format!("{} at {port5}/{port7:#x}/{ei}", e.name);
                let out = run(&mut rig, &e);
                assert_eq!(out.status(), e.status, "{what}");
                assert_eq!((out.ix, out.iy), (0x1357, 0x2468), "{what}: index registers");
                assert_eq!((out.port5, out.port7), (port5, port7), "{what}: ports");
                assert_eq!(out.iff, u8::from(ei), "{what}: interrupt flag");
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 4 * (3 + 3 + 3 + 1 + 21));
}

/// Bytes of the OS page's stack a call wrote, the driver's return address included, from pages
/// filled twice with patterns that differ in every byte so that no written byte can hide.
fn stack_used(stage: Stage, e: &Exit) -> usize {
    let mut most = 0;
    for flip in [0u8, 0xFF] {
        let mut rig = staged_at(stage, Some(flip), None);
        for (at, byte) in rig.m.ram_page(OS_PAGE).iter_mut().enumerate() {
            *byte = pattern(OS_PAGE, at) ^ flip;
        }
        let out = run(&mut rig, e);
        assert_eq!(out.status(), e.status, "{}", e.name);
        let page = rig.m.ram_page(OS_PAGE).to_vec();
        let lowest = (0..page.len())
            .find(|&at| page[at] != pattern(OS_PAGE, at) ^ flip)
            .unwrap();
        most = most.max(page.len() - lowest);
    }
    most
}

#[test]
fn the_stack_goes_exactly_as_deep_as_the_header_says_on_each_exit() {
    let mut deepest = 0;
    let mut table = Vec::new();
    for stage in STAGES {
        for e in exits(stage) {
            let used = stack_used(stage, &e);
            table.push((e.name, used));
            deepest = deepest.max(used);
        }
    }
    println!("{table:?}");
    assert_eq!(deepest, 2 + STACK_DEPTH);
    let mut deep = 0;
    for (name, used) in &table {
        let want = match *name {
            "range match" | "range none after" | "range none before"
            | "range pointer at the end of bank B" => 12,
            "cut range" | "cut kinds walk" | "walk theorem" | "walk axiom none" => 10,
            _ => 6,
        };
        assert_eq!(*used, want, "{name}");
        deep += usize::from(want > 6);
    }
    assert_eq!(deep, 8);
    assert_eq!(table.len(), 31);
}

#[test]
fn an_interrupt_at_any_point_of_the_three_calls_changes_nothing() {
    let bytes = library(&labels(), Some(&bits()));
    let mut rig = Rig::with_library(&bytes);
    rig.install_handler();
    rig.ei = true;
    rig.open_ok();
    let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
    let state = rig.section_state();
    for (target, regs, pokes) in [
        (RANGE, range_regs(PREFIX_AT, 4), vec![(PREFIX_AT, b"ax-1".to_vec())]),
        (RANGE, range_regs(PREFIX_AT, 1), vec![(PREFIX_AT, b"q".to_vec())]),
        (
            WALK,
            walk_regs(BLOCK_AT),
            vec![(BLOCK_AT, block(0, COUNT, 0, COUNT, THEOREM).to_vec())],
        ),
        (
            WALK,
            walk_regs(BLOCK_AT),
            vec![(BLOCK_AT, block(0, COUNT, 0, COUNT, EVERY).to_vec())],
        ),
        (COPY, copy_regs(0, COUNT, COPY_AT), vec![(COPY_AT, vec![MARK; 0x100])]),
    ] {
        for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
            *byte = pattern(SPARE_PAGE, at);
        }
        for (at, bytes) in &pokes {
            rig.poke(*at, bytes);
        }
        let quiet = rig.call(target, regs);
        assert_eq!(quiet.status(), ST_OK, "{target}");
        let key = |o: &Out| (o.af, o.bc, o.de, o.hl, o.iff, o.port5, o.port7);
        let (mut fired, mut after) = (0, 1u32);
        loop {
            let (out, taken) = rig.call_interrupted(target, regs, after);
            fired += usize::from(taken > 0);
            assert_eq!(key(&out), key(&quiet), "{target} after {after}");
            let page = rig.m.ram_page(SPARE_PAGE).to_vec();
            let stray: Vec<usize> = (0..page.len())
                .filter(|&at| {
                    page[at] != pattern(SPARE_PAGE, at) && !(bounce..bounce + 256).contains(&at)
                })
                .collect();
            assert!(stray.is_empty(), "{target} after {after}: {stray:x?}");
            if taken == 0 && after > 50 {
                break;
            }
            after += 53;
        }
        println!("{target}: {fired} interrupts landed inside the call");
        assert!(fired > 40, "{target}: {fired} interrupts landed");
    }
    assert_eq!(rig.section_state(), state);
}

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
    assert_eq!(decoded(&bytes).assertions[usize::from(n)].hypotheses.len(), 6);
    let mut rig = opened(&bytes);
    let mut check = |what: &str, target: &str, regs: Regs, pokes: &[(u16, Vec<u8>)], status: u8, window: bool, cache: bool| {
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
        assert!(before.0 != 0 && before.2 != 0xFFFF && before.3 == u32::from(n), "{what}: {before:?}");
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
            assert_eq!((after.0, after.1), (before.0, before.1), "{what}: the window is kept");
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
    check("range", RANGE, range_regs(PREFIX_AT, 5), &pre(b"ax-00"), ST_OK, true, false);
    check("range, none", RANGE, range_regs(PREFIX_AT, 1), &pre(b"q"), ST_OK, true, false);
    check("range, length 0", RANGE, range_regs(PREFIX_AT, 0), &pre(b"q"), ST_ARG, false, false);
    check("range, pointer", RANGE, range_regs(0xC000, 3), &[], ST_ARG, false, false);
    check("walk", WALK, walk_regs(BLOCK_AT), &blk(block(0, c, 0, c, THEOREM)), ST_OK, true, true);
    check("walk, every", WALK, walk_regs(BLOCK_AT), &blk(block(0, 5, 0, c, EVERY)), ST_OK, true, true);
    check("walk, empty range", WALK, walk_regs(BLOCK_AT), &blk(block(0, 0, 0, c, EVERY)), ST_OK, true, true);
    check("walk, bad range", WALK, walk_regs(BLOCK_AT), &blk(block(0, c + 1, 0, c, EVERY)), ST_INDEX, true, true);
    check("walk, bad start", WALK, walk_regs(BLOCK_AT), &blk(block(0, c, 5, 4, EVERY)), ST_INDEX, true, true);
    check("walk, bad kind", WALK, walk_regs(BLOCK_AT), &blk(block(0, c, 0, c, 7)), ST_ARG, false, false);
    check("walk, bad block", WALK, walk_regs(0xC000), &[], ST_ARG, false, false);
    check("copy", COPY, copy_regs(0, 20, COPY_AT), &[], ST_OK, false, false);
    check("copy, none", COPY, copy_regs(0, 0, COPY_AT), &[], ST_OK, false, false);
    check("copy, bad range", COPY, copy_regs(0, c + 1, COPY_AT), &[], ST_INDEX, false, false);
    let _ = KINDS;
}
