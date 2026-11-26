use crate::findrig::*;
use crate::findstages::*;
use crate::isolation::pattern;
use crate::support::*;

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
                assert_eq!(
                    (out.ix, out.iy),
                    (0x1357, 0x2468),
                    "{what}: index registers"
                );
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
            "range match"
            | "range none after"
            | "range none before"
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
        (
            RANGE,
            range_regs(PREFIX_AT, 4),
            vec![(PREFIX_AT, b"ax-1".to_vec())],
        ),
        (
            RANGE,
            range_regs(PREFIX_AT, 1),
            vec![(PREFIX_AT, b"q".to_vec())],
        ),
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
        (
            COPY,
            copy_regs(0, COUNT, COPY_AT),
            vec![(COPY_AT, vec![MARK; 0x100])],
        ),
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
