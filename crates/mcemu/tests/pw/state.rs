use crate::guard::{Allowed, pattern, prime, stray};
use crate::limits::*;
use crate::proofs::*;
use crate::rig::*;
use crate::stackfree;
use crate::support::*;

/// Bytes of the caller's stack the header says pw_shape uses below its return address, and bytes
/// of the walk's own stack below FM_STACK_TOP.
pub const CALLER_DEPTH: usize = 10;
pub const PRIVATE_DEPTH: usize = 6;

pub struct Exit {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub n: u16,
    pub open: bool,
    pub block: u16,
    pub status: u8,
}

fn exit(name: &'static str, (bytes, n): (Vec<u8>, u16), status: u8) -> Exit {
    Exit {
        name,
        bytes,
        n,
        open: true,
        block: BLOCK,
        status,
    }
}

/// One exit of every status the header lists, the walk's own and the reader's, with the library
/// that gives it.
pub fn exits() -> Vec<Exit> {
    let fixture = fixture();
    let at = |label: &str| number_of(&fixture, label);
    let count = mcpack::library::decode(&fixture).unwrap().assertions.len() as u16;
    let with = |name, n, status| Exit {
        name,
        bytes: fixture.clone(),
        n,
        open: true,
        block: BLOCK,
        status,
    };
    let mut out = vec![
        with("ok", at("mp2s"), ST_OK),
        with("ok with a theorem referenced", at("ref1"), ST_OK),
        with("axiom", at("ax-mp"), NONE),
        with("syntax statement", at("wtest"), SYNTAX),
        with("past the count", count, ST_INDEX),
    ];
    out.push(Exit {
        open: false,
        ..with("closed", at("mp2"), ST_CLOSED)
    });
    out.push(Exit {
        block: 0xC000,
        ..with("block in bank C", at("mp2"), ST_ARG)
    });
    out.push(Exit {
        block: 0x989C,
        ..with("block on the state", at("mp2"), ST_ARG)
    });
    out.push(Exit {
        block: 0xBFFC,
        ..with("block across the end of bank B", at("mp2"), ST_ARG)
    });
    out.push(Exit {
        block: 0xBFF8,
        ..with("block at the end of bank B", at("mp2"), ST_OK)
    });
    out.push(exit("underflow", one(&["ax-mp"], &[Num(2)]), UNDERFLOW));
    out.push(exit("bad reference", one(&[], &[Num(7)]), BAD_REF));
    out.push(exit("save on nothing", one(&[], &[Save]), SAVE_EMPTY));
    out.push(exit(
        "two entries",
        one(&[], &[Num(1), Num(1)]),
        MANY_ENTRIES,
    ));
    out.push(exit("256 distinct goals", chain(128, true), GOALS));
    out.push(exit("257 stack entries", deep(257), STACK));
    out.push(exit(
        "513 saved steps",
        {
            let mut proof = vec![Num(1)];
            proof.extend(vec![Save; 513]);
            one(&[], &proof)
        },
        SAVED,
    ));
    out.push(exit("a tree past 24 bits", doubling(24), TOTAL));
    out.push(exit("513 references", references(513), REFS));
    let (good, n) = one(&[], &[Num(1)]);
    let l = layout(&good, usize::from(n));
    out.push(exit(
        "no entry",
        (
            edited(&good, |b| b[l.proof - 2..l.proof].copy_from_slice(&[0, 0])),
            n,
        ),
        NO_ENTRY,
    ));
    out.push(exit(
        "cut number",
        (edited(&good, |b| b[l.proof] = 0xF0), n),
        CUT,
    ));
    let mut src = String::from(BASE);
    src += &theorem("one", &[("h", "|- p")], "|- p", &[], &[Num(1)]);
    src += &theorem(
        "tgt",
        &[("h", "|- p")],
        "|- p",
        &["one"],
        &[Num(0), Num(1), Num(2)],
    );
    let bytes = packed(&src);
    let n = number_of(&bytes, "tgt");
    let l = layout(&bytes, usize::from(n));
    out.push(exit(
        "forward reference",
        (
            edited(&bytes, |b| {
                let at = l.refs_count + 2;
                b[at..at + 2].copy_from_slice(&n.to_le_bytes())
            }),
            n,
        ),
        FORWARD,
    ));
    let directory = get32(&bytes, 16) as usize;
    let used = number_of(&bytes, "one") as usize;
    out.push(exit(
        "a referenced record past the library",
        (
            edited(&bytes, |b| {
                let len = b.len() as u32 + 100;
                b[directory + 4 * used..directory + 4 * used + 4]
                    .copy_from_slice(&len.to_le_bytes())
            }),
            n,
        ),
        ST_BAD,
    ));
    out.push(exit(
        "a referenced directory entry past 24 bits",
        (edited(&bytes, |b| b[directory + 4 * used + 3] = 1), n),
        ST_BAD,
    ));
    let cut = kernel_only(&fixture_source());
    let last = mcpack::library::decode(&cut).unwrap().assertions.len() - 1;
    let l = layout(&cut, last);
    out.push(exit(
        "a proof past the library",
        (
            edited(&cut, |b| {
                b[l.proof - 2..l.proof].copy_from_slice(&60_000u16.to_le_bytes())
            }),
            last as u16,
        ),
        ST_BAD,
    ));
    out
}

/// A rig for an exit with the caller's ports and interrupt state, ready for the call.
pub fn rigged(e: &Exit, port5: u8, port7: u8, ei: bool) -> Rig {
    let mut rig = Rig::with_image(PW);
    rig.m.out_port(5, port5);
    rig.m.out_port(7, port7);
    rig.ei = ei;
    rig.load(&e.bytes);
    if e.open {
        rig.open_ok();
    }
    rig
}

pub fn exit_regs(e: &Exit) -> Regs {
    Regs {
        de: e.block,
        ..regs(e.n, provable(&e.bytes).unwrap())
    }
}

#[test]
fn ports_the_interrupt_state_the_speed_and_the_index_registers_come_back_at_every_exit() {
    let all = exits();
    let mut statuses = std::collections::BTreeSet::new();
    let mut seen = 0;
    for (port5, port7, ei, speed) in [
        (0u8, 0x81u8, false, 0u8),
        (2, 0x81, true, 1),
        (7, 0x84, true, 0),
        (4, 0x87, false, 1),
    ] {
        for e in &all {
            // Pages 82h to 87h are one page that bank B and bank C show alike, so a block at
            // the end of bank B is the caller's stack there.
            if e.name == "block at the end of bank B" && port7 != 0x81 {
                continue;
            }
            let mut rig = rigged(e, port5, port7, ei);
            rig.m.out_port(0x20, speed);
            let before = rig.m.in_port(0x20);
            let out = rig.call(SHAPE, exit_regs(e));
            let what = format!("{} at {port5}/{port7:#x}/{ei}/{speed}", e.name);
            assert_eq!(out.status(), e.status, "{what}");
            assert_eq!(
                (out.ix, out.iy),
                (0x1357, 0x2468),
                "{what}: index registers"
            );
            assert_eq!((out.port5, out.port7), (port5, port7), "{what}: ports");
            assert_eq!(out.iff, u8::from(ei), "{what}: interrupt flag");
            assert_eq!(rig.m.in_port(0x20), before, "{what}: speed");
            assert_eq!(out.af & 0x40 != 0, e.status == 0, "{what}: zero flag");
            statuses.insert(e.status);
            seen += 1;
        }
    }
    assert_eq!(seen, 4 * all.len() - 2);
    let want: Vec<u8> = [
        0, 2, 3, 4, 5, 9, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44,
    ]
    .to_vec();
    assert_eq!(statuses.into_iter().collect::<Vec<_>>(), want);
}

#[test]
fn the_shadow_registers_are_touched_only_by_a_read_and_only_with_interrupts_off() {
    let seed = Shadows {
        af: 0x1122,
        bc: 0x3344,
        de: 0x5566,
        hl: 0x7788,
    };
    for e in exits() {
        let mut rig = rigged(&e, 0, 0x81, true);
        let (out, shadows) = rig.call_shadowed(SHAPE, exit_regs(&e), seed);
        assert_eq!(out.status(), e.status, "{}", e.name);
        let reads = !matches!(e.status, ST_CLOSED | ST_INDEX | ST_ARG);
        assert_eq!(shadows != seed, reads, "{}: {shadows:x?}", e.name);
    }
}

fn depths(e: &Exit) -> (usize, usize) {
    let (mut caller, mut private) = (0, 0);
    for flip in [0u8, 0xFF] {
        let mut rig = rigged(e, 0, 0x81, false);
        for (page, from) in [(OS_PAGE, 0), (SPARE_PAGE, 0)] {
            for (at, byte) in rig.m.ram_page(page).iter_mut().enumerate().skip(from) {
                *byte = pattern(page, at) ^ flip;
            }
        }
        let out = rig.call(SHAPE, exit_regs(e));
        assert_eq!(out.status(), e.status, "{}", e.name);
        let page = rig.m.ram_page(OS_PAGE).to_vec();
        let lowest = (0..page.len())
            .find(|&at| page[at] != pattern(OS_PAGE, at) ^ flip)
            .unwrap();
        caller = caller.max(page.len() - lowest);
        let spare = rig.m.ram_page(SPARE_PAGE).to_vec();
        let top = usize::from(rig.label("PW_STACK_TOP") - 0xC000);
        let floor = top - usize::from(rig.label("PW_STACK_ROOM"));
        let lowest = (floor..top)
            .find(|&at| spare[at] != pattern(SPARE_PAGE, at) ^ flip)
            .unwrap_or(top);
        private = private.max(top - lowest);
    }
    (caller, private)
}

#[test]
fn the_stacks_go_exactly_as_deep_as_the_header_says_on_each_exit() {
    let mut table = Vec::new();
    for e in exits() {
        table.push((e.name, depths(&e)));
    }
    println!("{table:?}");
    let caller = table.iter().map(|&(_, (c, _))| c).max().unwrap();
    let private = table.iter().map(|&(_, (_, p))| p).max().unwrap();
    assert_eq!((caller, private), (2 + CALLER_DEPTH, PRIVATE_DEPTH));
    for (name, (c, p)) in &table {
        let session = matches!(
            *name,
            "ok" | "ok with a theorem referenced"
                | "block at the end of bank B"
                | "underflow"
                | "bad reference"
                | "save on nothing"
                | "two entries"
                | "256 distinct goals"
                | "257 stack entries"
                | "513 saved steps"
                | "a tree past 24 bits"
                | "no entry"
                | "cut number"
                | "forward reference"
                | "a referenced record past the library"
                | "a referenced directory entry past 24 bits"
        );
        assert_eq!(
            *p > 0,
            session,
            "{name}: the private stack is used by a session only"
        );
        assert!(*c <= 2 + CALLER_DEPTH, "{name}: {c}");
    }
}

#[test]
fn the_walk_writes_only_the_bytes_its_header_names() {
    let mut touched = std::collections::BTreeSet::new();
    for e in exits() {
        let mut rig = Rig::with_image(PW);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.load(&e.bytes);
        if e.open {
            rig.open_ok();
        }
        let mut allowed = Allowed::of(PW);
        let vars = rig.label("PW_VARS");
        let used = rig.label("PW_USED");
        let top = rig.label("PW_STACK_TOP");
        let tokens = rig.label("FM_TOKENS");
        let tokens_used = rig.label("PW_TOKENS_USED");
        let low_end = top - rig.label("PW_STACK_ROOM");
        allowed.extra.push((SPARE_PAGE, vars, usize::from(low_end - vars)));
        allowed
            .extra
            .push((SPARE_PAGE, top - PRIVATE_DEPTH as u16, PRIVATE_DEPTH));
        allowed
            .extra
            .push((SPARE_PAGE, tokens, usize::from(tokens_used)));
        if e.block < 0xC000 {
            allowed.extra.push((APP_PAGE, e.block, 8));
        }
        let block_before = rig.peek(BUF, 0x200);
        let out = rig.call(SHAPE, exit_regs(&e));
        assert_eq!(out.status(), e.status, "{}", e.name);
        let found = stray(&mut rig, &allowed);
        assert!(found.is_empty(), "{}: {found:x?}", e.name);
        if e.block == BLOCK && e.status != ST_OK {
            assert_eq!(rig.peek(BUF, 0x200), block_before, "{}: the block", e.name);
        }
        let spare = rig.m.ram_page(SPARE_PAGE).to_vec();
        for (what, at, len) in [
            ("variables", vars, usize::from(used)),
            ("entries", rig.label("PW_ENTRIES"), 8),
            ("starts", rig.label("PW_STARTS"), 2),
            ("saved", rig.label("PW_SAVED"), 1),
            ("sizes", rig.label("PW_SIZES"), 3),
            ("hypothesis ranks", rig.label("PW_HYPK"), 1),
            ("references", rig.label("PW_MEMO"), 2),
            ("steps", rig.label("PW_STEP"), 2),
            ("begins", rig.label("PW_BEGIN"), 2),
            ("applied", rig.label("PW_BY"), 2),
            ("link starts", rig.label("PW_LSTART"), 2),
            ("variable bitmap", rig.label("PW_PROVBITS"), 32),
            ("links", rig.label("PW_LINKS"), 1),
        ] {
            let from = usize::from(at - 0xC000);
            if (from..from + len).any(|i| spare[i] != pattern(SPARE_PAGE, i)) {
                touched.insert(what);
            }
        }
    }
    assert_eq!(
        touched.into_iter().collect::<Vec<_>>(),
        [
            "applied",
            "begins",
            "entries",
            "hypothesis ranks",
            "link starts",
            "links",
            "references",
            "saved",
            "sizes",
            "starts",
            "steps",
            "variable bitmap",
            "variables"
        ]
    );
}

#[test]
fn the_two_windows_with_the_stack_out_of_reach_hold_no_stack_instruction() {
    let at = |name: &str| usize::from(PW.labels_at(name) - PW.org);
    let first = stackfree::breaks(
        PW.code,
        at("PWS_WINDOW"),
        at("PWS_WINDOW_END"),
        &["store sp"],
    );
    assert!(first.is_empty(), "{first:x?}");
    let second = stackfree::breaks(PW.code, at("PWL_WINDOW"), at("PWL_WINDOW_END"), &[]);
    assert!(second.is_empty(), "{second:x?}");
    let peek = stackfree::breaks(PW.code, at("PWK_WINDOW"), at("PWK_WINDOW_END"), &[]);
    assert!(peek.is_empty(), "{peek:x?}");
    assert!(at("PWS_WINDOW_END") - at("PWS_WINDOW") > 15);
    assert!(at("PWK_WINDOW_END") - at("PWK_WINDOW") > 30);
    assert!(at("PWL_WINDOW_END") - at("PWL_WINDOW") >= 3);
}

trait LabelAt {
    fn labels_at(&self, name: &str) -> u16;
}

impl LabelAt for Image {
    fn labels_at(&self, name: &str) -> u16 {
        label_in(*self, name)
    }
}

#[test]
fn an_interrupt_at_any_point_changes_nothing() {
    let all = exits();
    for name in [
        "ok",
        "ok with a theorem referenced",
        "underflow",
        "forward reference",
    ] {
        let e = all.iter().find(|e| e.name == name).unwrap();
        let mut rig = rigged(e, 0, 0x81, true);
        rig.install_handler();
        let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
        let regs = exit_regs(e);
        for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
            *byte = pattern(SPARE_PAGE, at);
        }
        rig.seed_result();
        let quiet = rig.call(SHAPE, regs);
        assert_eq!(quiet.status(), e.status, "{name}");
        let result = rig.result_bytes();
        let key = |o: &Out| (o.af, o.bc, o.de, o.hl, o.iff, o.port5, o.port7);
        let (mut fired, mut after) = (0, 1u32);
        loop {
            rig.seed_result();
            let (out, taken) = rig.call_interrupted(SHAPE, regs, after);
            fired += usize::from(taken > 0);
            assert_eq!(key(&out), key(&quiet), "{name} after {after}");
            assert_eq!(rig.result_bytes(), result, "{name} after {after}");
            if taken == 0 && after > 50 {
                break;
            }
            after += 997;
        }
        let page = rig.m.ram_page(SPARE_PAGE).to_vec();
        let vars = usize::from(rig.label("PW_VARS") - 0xC000);
        let top = usize::from(rig.label("PW_STACK_TOP") - 0xC000);
        let tokens = usize::from(rig.label("FM_TOKENS") - 0xC000);
        let used = usize::from(rig.label("PW_TOKENS_USED"));
        let stray: Vec<usize> = (0..page.len())
            .filter(|&at| {
                page[at] != pattern(SPARE_PAGE, at)
                    && !(bounce..bounce + 256).contains(&at)
                    && !(vars..top).contains(&at)
                    && !(tokens..tokens + used).contains(&at)
            })
            .collect();
        assert!(stray.is_empty(), "{name}: {stray:x?}");
        println!("{name}: {fired} interrupts landed");
        assert!(fired > 6, "{name}: {fired}");
    }
}
