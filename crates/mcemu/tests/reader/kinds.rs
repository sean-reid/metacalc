use crate::damaged::{both, offset_of, row_of, with};
use crate::isolation::pattern;
use crate::secrig::*;
use crate::support::*;
use mcpack::library::{KINDS, decode, pack_kernel};

const NEXT: &str = "LIB_KIND_NEXT";
const PREV: &str = "LIB_KIND_PREV";
const AXIOM: u8 = 0;
const THEOREM: u8 = 1;
const NO_MATCH: u16 = 0xFFFF;

/// Bytes of stack the header says the two calls use below their return address.
const STACK_DEPTH: usize = 6;

/// `bits.len()` axioms with the bitmap of the kinds section overwritten by `bits`, true for a
/// theorem, so that the reader sees a pattern the records do not need to agree with.
pub fn kinds_library(bits: &[bool]) -> Vec<u8> {
    let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
    for i in 0..bits.len() {
        src += &format!("ax-{i} $a |- p $.\n");
    }
    let bytes = pack_all(&parse(&src));
    let at = offset_of(&bytes, KINDS) as usize + 2;
    with(&bytes, |b| {
        b[at..at + bits.len().div_ceil(8)].fill(0);
        for (n, _) in bits.iter().enumerate().filter(|(_, t)| **t) {
            b[at + n / 8] |= 1 << (n % 8);
        }
    })
}

fn opened_with(bits: &[bool]) -> Rig {
    let mut rig = Rig::with_library(&kinds_library(bits));
    rig.open_ok();
    rig
}

/// The rule of lib_kind_next, from the header and nothing else.
pub fn mirror_next(bits: &[bool], kind: u8, from: u16, end: u16) -> Result<u16, u8> {
    if kind > 1 {
        return Err(ST_ARG);
    }
    if usize::from(end) > bits.len() || from > end {
        return Err(ST_INDEX);
    }
    let found = (from..end).find(|&n| bits[usize::from(n)] == (kind == THEOREM));
    Ok(found.unwrap_or(end))
}

/// The rule of lib_kind_prev.
pub fn mirror_prev(bits: &[bool], kind: u8, from: u16) -> Result<u16, u8> {
    if kind > 1 {
        return Err(ST_ARG);
    }
    if usize::from(from) > bits.len() {
        return Err(ST_INDEX);
    }
    let found = (0..from)
        .rev()
        .find(|&n| bits[usize::from(n)] == (kind == THEOREM));
    Ok(found.unwrap_or(NO_MATCH))
}

/// Registers a bare `ret` fails on: A and C hold junk, IX and IY hold marks, HL holds `from`.
fn seeded(kind: u8, from: u16, end: u16) -> Regs {
    Regs {
        a: 0x55,
        bc: u16::from(kind) << 8 | 0x33,
        de: end,
        hl: from,
        ix: 0x1357,
        iy: 0x2468,
    }
}

pub fn ask_next(rig: &mut Rig, kind: u8, from: u16, end: u16) -> (u8, u16) {
    let out = rig.call(NEXT, seeded(kind, from, end));
    (out.status(), out.hl)
}

pub fn ask_prev(rig: &mut Rig, kind: u8, from: u16) -> (u8, u16) {
    let out = rig.call(PREV, seeded(kind, from, 0x7777));
    (out.status(), out.hl)
}

fn agrees(got: (u8, u16), want: Result<u16, u8>, what: &str) {
    match want {
        Ok(n) => assert_eq!(got, (ST_OK, n), "{what}"),
        Err(status) => assert_eq!(got.0, status, "{what}"),
    }
}

#[derive(Default, Debug)]
struct Tally {
    matched: usize,
    none: usize,
    arg: usize,
    index: usize,
}

impl Tally {
    fn add(&mut self, want: Result<u16, u8>, no_match: u16) {
        match want {
            Ok(n) if n == no_match => self.none += 1,
            Ok(_) => self.matched += 1,
            Err(ST_ARG) => self.arg += 1,
            Err(ST_INDEX) => self.index += 1,
            Err(other) => panic!("status {other}"),
        }
    }
}

fn pseudo(n: usize, seed: u32) -> Vec<bool> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            x >> 29 & 1 == 1
        })
        .collect()
}

/// Every call of the two routines against the rule: `from` and `end` over the whole range and
/// two past it, for each kind in `kinds`.
fn sweep_all(rig: &mut Rig, bits: &[bool], kinds: &[u8], tally: &mut Tally) {
    let count = bits.len() as u16;
    for &kind in kinds {
        for from in 0..=count + 1 {
            for end in 0..=count + 1 {
                let want = mirror_next(bits, kind, from, end);
                let got = ask_next(rig, kind, from, end);
                agrees(got, want, &format!("next {kind} {from} {end}"));
                tally.add(want, end);
            }
            let want = mirror_prev(bits, kind, from);
            agrees(
                ask_prev(rig, kind, from),
                want,
                &format!("prev {kind} {from}"),
            );
            tally.add(want, NO_MATCH);
        }
    }
}

#[test]
fn every_kind_from_and_end_of_a_small_library_is_the_rules_answer() {
    let mut tally = Tally::default();
    for (n, seed) in [(21usize, 1u32), (29, 2), (16, 3), (9, 4), (1, 5)] {
        let bits = pseudo(n, seed);
        assert!(n < 3 || (bits.contains(&true) && bits.contains(&false)));
        let mut rig = opened_with(&bits);
        sweep_all(&mut rig, &bits, &[AXIOM, THEOREM, 2], &mut tally);
    }
    println!("{tally:?}");
    assert!(tally.matched > 1600, "{tally:?}");
    assert!(tally.none > 350, "{tally:?}");
    assert!(tally.arg > 2000, "{tally:?}");
    assert!(tally.index > 2000, "{tally:?}");
}

#[test]
fn a_lone_match_or_a_lone_miss_at_every_bit_is_found_from_every_start() {
    let mut tally = Tally::default();
    for n in [1usize, 8, 9, 24, 29] {
        for pos in 0..n {
            for lone in [true, false] {
                let mut bits = vec![!lone; n];
                bits[pos] = lone;
                let mut rig = opened_with(&bits);
                let ends: Vec<u16> = [pos.saturating_sub(1), pos, pos + 1, pos + 2, n / 2, n]
                    .into_iter()
                    .filter(|&e| e <= n)
                    .map(|e| e as u16)
                    .collect();
                for kind in [AXIOM, THEOREM] {
                    for from in 0..=n as u16 {
                        for &end in &ends {
                            let want = mirror_next(&bits, kind, from, end);
                            let got = ask_next(&mut rig, kind, from, end);
                            agrees(
                                got,
                                want,
                                &format!("n {n} pos {pos} next {kind} {from} {end}"),
                            );
                            tally.add(want, end);
                        }
                        let want = mirror_prev(&bits, kind, from);
                        let got = ask_prev(&mut rig, kind, from);
                        agrees(got, want, &format!("n {n} pos {pos} prev {kind} {from}"));
                        tally.add(want, NO_MATCH);
                    }
                }
            }
        }
    }
    println!("{tally:?}");
    assert!(tally.matched > 20_000, "{tally:?}");
    assert!(tally.none > 9_000, "{tally:?}");
    assert_eq!(tally.arg, 0);
    assert!(tally.index > 15_000, "{tally:?}");
}

#[test]
fn a_match_beside_an_end_in_the_middle_of_a_byte_is_in_or_out_by_one() {
    let lone = |pos: usize| {
        let mut bits = vec![false; 29];
        bits[pos] = true;
        opened_with(&bits)
    };
    for (pos, found) in [(10usize, 10u16), (11, 11), (12, 11)] {
        let mut rig = lone(pos);
        assert_eq!(
            ask_next(&mut rig, THEOREM, 0, 11),
            (ST_OK, found),
            "match at {pos}"
        );
    }
    for (pos, found) in [(10usize, 10u16), (11, NO_MATCH), (12, NO_MATCH)] {
        let mut rig = lone(pos);
        assert_eq!(
            ask_prev(&mut rig, THEOREM, 11),
            (ST_OK, found),
            "match at {pos}"
        );
    }
    let mut rig = lone(11);
    assert_eq!(ask_prev(&mut rig, THEOREM, 12), (ST_OK, 11));
    assert_eq!(ask_next(&mut rig, THEOREM, 11, 12), (ST_OK, 11));
    assert_eq!(ask_next(&mut rig, THEOREM, 12, 12), (ST_OK, 12));
}

#[test]
fn a_set_bit_past_the_last_assertion_is_never_reported() {
    for n in [1usize, 5, 8, 13, 21] {
        let bits = vec![false; n];
        let mut bytes = kinds_library(&bits);
        let pad = n.div_ceil(8) * 8 - n;
        let last = offset_of(&bytes, KINDS) as usize + 2 + n.div_ceil(8) - 1;
        bytes[last] = if pad == 0 { 0 } else { 0xFFu8 << (8 - pad) };
        assert!(pad == 0 || bytes[last] != 0);
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        let count = n as u16;
        for from in 0..=count {
            assert_eq!(
                ask_next(&mut rig, THEOREM, from, count),
                (ST_OK, count),
                "n {n} {from}"
            );
        }
        assert_eq!(
            ask_prev(&mut rig, THEOREM, count),
            (ST_OK, NO_MATCH),
            "n {n}"
        );
    }
}

/// Two matches with `gap` whole bytes of misses between them, at bit `low` of the byte before
/// the gap and bit `high` of the byte after it.
fn gapped(gap: usize, low: usize, high: usize) -> (Vec<bool>, u16, u16) {
    let first = 8 * 3 + low;
    let second = 8 * (3 + gap + 1) + high;
    let mut bits = vec![false; second + 20];
    bits[first] = true;
    bits[second] = true;
    (bits, first as u16, second as u16)
}

#[test]
fn a_run_of_whole_bytes_without_a_match_is_crossed_in_both_directions_at_every_length() {
    let mut longer = 0;
    for gap in 26..=70usize {
        for (low, high) in [(0usize, 7usize), (7, 0), (3, 3)] {
            let (bits, first, second) = gapped(gap, low, high);
            let count = bits.len() as u16;
            let mut rig = opened_with(&bits);
            let what = format!("gap {gap} {low} {high}");
            assert_eq!(
                ask_next(&mut rig, THEOREM, 0, count),
                (ST_OK, first),
                "{what}"
            );
            assert_eq!(
                ask_next(&mut rig, THEOREM, first + 1, count),
                (ST_OK, second),
                "{what}"
            );
            assert_eq!(
                ask_next(&mut rig, THEOREM, second + 1, count),
                (ST_OK, count),
                "{what}"
            );
            assert_eq!(
                ask_next(&mut rig, THEOREM, first + 1, second),
                (ST_OK, second),
                "{what}"
            );
            assert_eq!(
                ask_prev(&mut rig, THEOREM, count),
                (ST_OK, second),
                "{what}"
            );
            assert_eq!(
                ask_prev(&mut rig, THEOREM, second),
                (ST_OK, first),
                "{what}"
            );
            assert_eq!(
                ask_prev(&mut rig, THEOREM, first),
                (ST_OK, NO_MATCH),
                "{what}"
            );
            let inverse: Vec<bool> = bits.iter().map(|b| !b).collect();
            let mut rig = opened_with(&inverse);
            assert_eq!(
                ask_next(&mut rig, AXIOM, first + 1, count),
                (ST_OK, second),
                "{what}"
            );
            assert_eq!(ask_prev(&mut rig, AXIOM, second), (ST_OK, first), "{what}");
            longer += usize::from(second - first > 8 * 32);
        }
    }
    assert!(longer > 100, "{longer} runs longer than the 32 byte window");
}

/// A library whose bitmap holds a chunk boundary with at least 40 bytes of it on each side, and
/// the number of bitmap bytes before the boundary.
fn crossing_library() -> (Vec<u8>, usize, Vec<bool>) {
    for count in 3000..4200usize {
        let mut bits = vec![false; count];
        bits[3] = true;
        bits[count - 5] = true;
        let bytes = kinds_library(&bits);
        let at = offset_of(&bytes, KINDS) as usize + 2;
        let end = at + count.div_ceil(8);
        let boundary = end / CHUNK * CHUNK;
        if boundary > at + 40 && boundary + 40 < end {
            return (bytes, boundary - at, bits);
        }
    }
    panic!("no library with a bitmap across a chunk");
}

#[test]
fn a_run_across_a_chunk_boundary_is_crossed_in_both_directions() {
    let (bytes, before, bits) = crossing_library();
    let count = bits.len() as u16;
    assert!(before > 40 && before + 40 < bits.len().div_ceil(8));
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    assert_eq!(ask_next(&mut rig, THEOREM, 0, count), (ST_OK, 3));
    assert_eq!(ask_next(&mut rig, THEOREM, 4, count), (ST_OK, count - 5));
    assert_eq!(ask_prev(&mut rig, THEOREM, count), (ST_OK, count - 5));
    assert_eq!(ask_prev(&mut rig, THEOREM, count - 5), (ST_OK, 3));
    assert_eq!(ask_prev(&mut rig, THEOREM, 3), (ST_OK, NO_MATCH));
    let around = (8 * before) as u16;
    for from in [
        around - 300,
        around - 17,
        around - 1,
        around,
        around + 9,
        around + 300,
    ] {
        assert_eq!(
            ask_next(&mut rig, THEOREM, from, count),
            (ST_OK, count - 5),
            "from {from}"
        );
        assert_eq!(ask_prev(&mut rig, THEOREM, from), (ST_OK, 3), "from {from}");
        let want = mirror_next(&bits, AXIOM, from, from + 40).unwrap();
        assert_eq!(
            ask_next(&mut rig, AXIOM, from, from + 40),
            (ST_OK, want),
            "from {from}"
        );
    }
}

fn without_kinds() -> [(&'static str, Vec<u8>); 2] {
    let good = kinds_library(&pseudo(40, 9));
    let row = row_of(&good, KINDS);
    let db = parse("$c wff |- $.\n$v p $.\nwp $f wff p $.\nax-1 $a |- p $.\nax-2 $a |- p $.\n");
    let all: Vec<_> = (0..db.stmts.len() as u32).collect();
    [
        ("an unknown id", with(&good, |b| b[row] = 77)),
        ("no trailer", pack_kernel(&db, &all).unwrap()),
    ]
}

#[test]
fn a_library_without_a_kinds_section_answers_9_whatever_it_was_packed_with() {
    for (what, bytes) in without_kinds() {
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        assert_eq!(rig.sections().1 & HAVE_KINDS, 0, "{what}");
        assert_eq!(ask_next(&mut rig, THEOREM, 0, 2).0, ST_NONE, "{what}");
        assert_eq!(ask_next(&mut rig, AXIOM, 0, 2).0, ST_NONE, "{what}");
        assert_eq!(ask_prev(&mut rig, THEOREM, 1).0, ST_NONE, "{what}");
        assert_eq!(ask_prev(&mut rig, AXIOM, 1).0, ST_NONE, "{what}");
        assert_eq!(
            ask_next(&mut rig, 5, 0, 2).0,
            ST_ARG,
            "the kind is checked first: {what}"
        );
        assert_eq!(ask_next(&mut rig, THEOREM, 9, 2).0, ST_NONE, "{what}");
    }
    let mut rig = Rig::with_library(&kinds_library(&pseudo(40, 9)));
    rig.open_ok();
    assert_eq!(rig.sections().1 & HAVE_KINDS, HAVE_KINDS);
}

#[test]
fn a_kinds_section_that_does_not_fit_the_library_is_refused_at_open_as_the_decoder_refuses_it() {
    let good = kinds_library(&pseudo(21, 7));
    let at = offset_of(&good, KINDS) as usize;
    let length = row_of(&good, KINDS) + 5;
    assert_eq!(&good[at..at + 2], &[21, 0]);
    let cases = [
        ("count 20", with(&good, |b| b[at] = 20)),
        ("count 22", with(&good, |b| b[at] = 22)),
        ("count 0", with(&good, |b| b[at] = 0)),
        ("count 65535", with(&good, |b| b[at..at + 2].fill(0xFF))),
        ("length 4", with(&good, |b| b[length] = 4)),
        ("length 3", with(&good, |b| b[length] = 3)),
        ("length 2", with(&good, |b| b[length] = 2)),
        ("length 0", with(&good, |b| b[length] = 0)),
    ];
    for (what, bytes) in cases {
        let (_, status, host) = both(&bytes);
        assert_eq!((status, host), (ST_SECTION, false), "{what}");
    }
    let past_the_table = with(&good, |b| b[length + 2] = 1);
    let (_, status, host) = both(&past_the_table);
    assert_eq!((status, host), (ST_TABLE, false), "a length past the table");
    let (mut rig, status, host) = both(&good);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(ask_next(&mut rig, THEOREM, 0, 21).0, ST_OK);
    for count in [8usize, 9, 16, 17] {
        let bytes = kinds_library(&pseudo(count, 3));
        let at = offset_of(&bytes, KINDS) as usize;
        let one_more = with(&bytes, |b| b[at] = count as u8 + 1);
        assert_eq!(both(&one_more).1, ST_SECTION, "count {count} + 1");
        let one_fewer = with(&bytes, |b| b[at] = count as u8 - 1);
        let (status, host) = (both(&one_fewer).1, both(&one_fewer).2);
        assert_eq!((status, host), (ST_SECTION, false), "count {count} - 1");
        let (_, status, host) = both(&bytes);
        assert_eq!((status, host), (ST_OK, true), "count {count}");
    }
}

#[test]
fn a_set_padding_bit_opens_in_the_reader_and_is_refused_by_the_decoder() {
    let bits = pseudo(13, 5);
    let mut padded = kinds_library(&bits);
    let at = offset_of(&padded, KINDS) as usize;
    padded[at + 3] |= 0x80;
    let padded = with(&padded, |_| {});
    let (mut rig, status, host) = both(&padded);
    assert_eq!((status, host), (ST_OK, false));
    assert!(decode(&padded).is_err());
    let mut tally = Tally::default();
    sweep_all(&mut rig, &bits, &[AXIOM, THEOREM], &mut tally);
    assert!(tally.matched > 100 && tally.none > 50, "{tally:?}");
}

/// Where a rig for the exits stands: closed, with no kinds section, open, or open with the
/// library cut short under the bitmap.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Closed,
    NoSection,
    Open,
    Cut,
}

const STAGES: [Stage; 4] = [Stage::Closed, Stage::NoSection, Stage::Open, Stage::Cut];

fn bits40() -> Vec<bool> {
    let mut bits = vec![false; 40];
    bits[7] = true;
    bits[33] = true;
    bits
}

/// A rig at `stage`; with `junk`, every page of RAM starts filled with the pattern xor `junk`.
fn staged(stage: Stage, junk: Option<u8>) -> Rig {
    staged_at(stage, junk, None)
}

/// `staged` with ports 5 and 7 and the interrupt flag set as given before the library opens.
fn staged_at(stage: Stage, junk: Option<u8>, ports: Option<(u8, u8, bool)>) -> Rig {
    let good = kinds_library(&bits40());
    let bytes = if stage == Stage::NoSection {
        with(&good, |b| b[row_of(&good, KINDS)] = 77)
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
    if stage == Stage::Cut {
        let at = offset_of(&good, KINDS) + 3;
        rig.poke(rig.label("LR_LENGTH"), &at.to_le_bytes()[..3]);
    }
    rig
}

struct Exit {
    name: &'static str,
    target: &'static str,
    regs: Regs,
    status: u8,
}

fn exit(name: &'static str, target: &'static str, regs: Regs, status: u8) -> Exit {
    Exit {
        name,
        target,
        regs,
        status,
    }
}

fn exits(stage: Stage) -> Vec<Exit> {
    let r = seeded;
    match stage {
        Stage::Closed => vec![
            exit("closed next", NEXT, r(THEOREM, 0, 40), ST_CLOSED),
            exit("closed prev", PREV, r(THEOREM, 40, 0), ST_CLOSED),
        ],
        Stage::NoSection => vec![
            exit("none next", NEXT, r(THEOREM, 0, 40), ST_NONE),
            exit("none prev", PREV, r(THEOREM, 40, 0), ST_NONE),
        ],
        Stage::Cut => vec![
            exit("cut next", NEXT, r(THEOREM, 0, 40), ST_BAD),
            exit("cut prev", PREV, r(THEOREM, 40, 0), ST_BAD),
        ],
        Stage::Open => vec![
            exit("next match", NEXT, r(THEOREM, 0, 40), ST_OK),
            exit("next none", NEXT, r(THEOREM, 8, 33), ST_OK),
            exit("next from end", NEXT, r(THEOREM, 5, 5), ST_OK),
            exit("next bad kind", NEXT, r(2, 0, 40), ST_ARG),
            exit("next end past count", NEXT, r(THEOREM, 0, 41), ST_INDEX),
            exit("next from past end", NEXT, r(THEOREM, 6, 5), ST_INDEX),
            exit("prev match", PREV, r(THEOREM, 40, 0), ST_OK),
            exit("prev none", PREV, r(THEOREM, 7, 0), ST_OK),
            exit("prev from 0", PREV, r(THEOREM, 0, 0), ST_OK),
            exit("prev bad kind", PREV, r(9, 40, 0), ST_ARG),
            exit("prev from past count", PREV, r(THEOREM, 41, 0), ST_INDEX),
        ],
    }
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
                let out = rig.call(e.target, e.regs);
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
    assert_eq!(seen, 4 * (2 + 2 + 2 + 11));
}

/// Bytes of the OS page's stack a call wrote, the driver's return address included, from pages
/// filled twice with patterns that differ in every byte so that no written byte can hide.
fn stack_used(stage: Stage, e: &Exit) -> usize {
    let mut most = 0;
    for flip in [0u8, 0xFF] {
        let mut rig = staged(stage, Some(flip));
        for (at, byte) in rig.m.ram_page(OS_PAGE).iter_mut().enumerate() {
            *byte = pattern(OS_PAGE, at) ^ flip;
        }
        let out = rig.call(e.target, e.regs);
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
fn the_stack_goes_exactly_as_deep_as_the_header_says_on_the_deepest_exit() {
    let mut deepest = 0;
    for stage in STAGES {
        for e in exits(stage) {
            let used = stack_used(stage, &e);
            println!(
                "stack used by {}: {used} bytes, the driver's return address included",
                e.name
            );
            let want = match e.name {
                "next match" | "next none" | "prev match" | "prev none" | "cut next"
                | "cut prev" => 8,
                _ => 6,
            };
            assert_eq!(used, want, "{}", e.name);
            deepest = deepest.max(used);
        }
    }
    assert_eq!(deepest, 2 + STACK_DEPTH);
}

#[test]
fn an_interrupt_at_any_point_of_the_two_calls_changes_nothing() {
    let (bytes, _, bits) = crossing_library();
    let mut rig = Rig::with_library(&bytes);
    rig.install_handler();
    rig.ei = true;
    rig.open_ok();
    let count = bits.len() as u16;
    let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
    let state = rig.section_state();
    for (target, regs, want) in [
        (NEXT, seeded(THEOREM, 0, count), 3u16),
        (NEXT, seeded(THEOREM, 4, count), count - 5),
        (NEXT, seeded(THEOREM, count - 4, count), count),
        (PREV, seeded(THEOREM, count, 0), count - 5),
        (PREV, seeded(THEOREM, count - 5, 0), 3),
        (PREV, seeded(THEOREM, 3, 0), NO_MATCH),
    ] {
        for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
            *byte = pattern(SPARE_PAGE, at);
        }
        let quiet = rig.call(target, regs);
        assert_eq!((quiet.status(), quiet.hl), (ST_OK, want), "{target}");
        let (mut fired, mut after) = (0, 1u32);
        loop {
            let (out, taken) = rig.call_interrupted(target, regs, after);
            fired += usize::from(taken > 0);
            let key = |o: &Out| (o.af, o.bc, o.de, o.hl, o.iff, o.port5, o.port7);
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
            after += 97;
        }
        println!("{target}: {fired} interrupts landed inside the call");
        assert!(fired > 20, "{target}: {fired} interrupts landed");
    }
    assert_eq!(rig.section_state(), state);
}

#[test]
fn the_record_window_survives_a_kinds_call_between_two_steps_of_a_walk() {
    let bytes = crate::sections::headed(120, 3, 0);
    let library = decoded(&bytes);
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    let count = library.assertions.len() as u16;
    let mut walked = 0;
    for (n, assertion) in library.assertions.iter().enumerate().skip(10).take(40) {
        let n = n as u16;
        for k in 0..assertion.hypotheses.len() as u8 {
            let alone = rig.hypothesis(n, k);
            rig.hypothesis(n, 0);
            assert_eq!(ask_next(&mut rig, THEOREM, 0, count).0, ST_OK);
            assert_eq!(ask_prev(&mut rig, AXIOM, count).0, ST_OK);
            let after = rig.hypothesis(n, k);
            assert_eq!(alone, after, "{n} {k}");
            walked += 1;
        }
        let before = rig.conclusion(n);
        assert_eq!(ask_next(&mut rig, AXIOM, n, count).0, ST_OK);
        assert_eq!(rig.conclusion(n), before);
        assert_eq!(before.1.len(), assertion.conclusion.len());
    }
    assert!(walked > 20, "{walked} hypotheses walked");
}

#[test]
fn a_kinds_call_drops_the_record_window_only_when_it_reads_the_bitmap() {
    let good = crate::sections::headed(120, 3, 0);
    let count = decoded(&good).assertions.len() as u16;
    let no_section = with(&good, |b| b[row_of(&good, KINDS)] = 77);
    let n = 42;
    assert_eq!(
        decoded(&good).assertions[usize::from(n)].hypotheses.len(),
        2
    );
    let (mut kept, mut dropped) = (0, 0);
    let mut probe =
        |rig: &mut Rig, what: &str, target: &str, regs: Regs, status: u8, reads: bool| {
            rig.hypothesis(n, 0);
            let window = |rig: &mut Rig| (rig.state_byte("LR_WLEN"), rig.state_byte("LR_WP"));
            let held = window(rig);
            assert_ne!(held.0, 0, "{what}: a window to keep");
            let out = rig.call(target, regs);
            assert_eq!(out.status(), status, "{what}");
            if reads {
                assert_eq!(window(rig).0, 0, "{what}: the read drops the window");
                dropped += 1;
            } else {
                assert_eq!(
                    window(rig),
                    held,
                    "{what}: nothing read, the window is kept"
                );
                kept += 1;
            }
        };
    let mut rig = Rig::with_library(&good);
    rig.open_ok();
    probe(
        &mut rig,
        "next reads",
        NEXT,
        seeded(THEOREM, 10, 60),
        ST_OK,
        true,
    );
    probe(
        &mut rig,
        "prev reads",
        PREV,
        seeded(AXIOM, 60, 0),
        ST_OK,
        true,
    );
    probe(
        &mut rig,
        "next, from = end",
        NEXT,
        seeded(THEOREM, 30, 30),
        ST_OK,
        false,
    );
    probe(
        &mut rig,
        "prev, from 0",
        PREV,
        seeded(AXIOM, 0, 0),
        ST_OK,
        false,
    );
    probe(
        &mut rig,
        "next, kind 2",
        NEXT,
        seeded(2, 10, 60),
        ST_ARG,
        false,
    );
    probe(
        &mut rig,
        "prev, kind 2",
        PREV,
        seeded(2, 60, 0),
        ST_ARG,
        false,
    );
    let past = count + 1;
    probe(
        &mut rig,
        "next, end past the count",
        NEXT,
        seeded(THEOREM, 10, past),
        ST_INDEX,
        false,
    );
    probe(
        &mut rig,
        "prev, from past the count",
        PREV,
        seeded(AXIOM, past, 0),
        ST_INDEX,
        false,
    );
    let mut rig = Rig::with_library(&no_section);
    rig.open_ok();
    probe(
        &mut rig,
        "next, no section",
        NEXT,
        seeded(THEOREM, 10, 60),
        ST_NONE,
        false,
    );
    probe(
        &mut rig,
        "prev, no section",
        PREV,
        seeded(AXIOM, 60, 0),
        ST_NONE,
        false,
    );
    assert_eq!((dropped, kept), (2, 8));
}

#[test]
fn the_packed_bitmap_of_a_library_with_theorems_is_what_the_reader_finds() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let bits: Vec<bool> = decoded(&bytes)
        .assertions
        .iter()
        .map(|a| a.theorem)
        .collect();
    assert!(bits.contains(&true) && bits.contains(&false));
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    let mut tally = Tally::default();
    sweep_all(&mut rig, &bits, &[AXIOM, THEOREM], &mut tally);
    assert!(tally.matched > 0 && tally.none > 0, "{tally:?}");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_answer_as_the_rule_does() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        let bits: Vec<bool> = decoded(&bytes)
            .assertions
            .iter()
            .map(|a| a.theorem)
            .collect();
        let count = bits.len() as u16;
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        let mut tally = Tally::default();
        for kind in [AXIOM, THEOREM] {
            for from in (0..=count).step_by(7).chain([count]) {
                let want = mirror_prev(&bits, kind, from);
                agrees(
                    ask_prev(&mut rig, kind, from),
                    want,
                    &format!("{name} prev {kind} {from}"),
                );
                tally.add(want, NO_MATCH);
                for end in [from, from + 1, from + 40, from + 300, count] {
                    if end <= count {
                        let want = mirror_next(&bits, kind, from, end);
                        let got = ask_next(&mut rig, kind, from, end);
                        agrees(got, want, &format!("{name} next {kind} {from} {end}"));
                        tally.add(want, end);
                    }
                }
            }
        }
        println!("{name}: {tally:?}");
        let (matched, none) = match name {
            "prop" => (800, 400),
            "pred" => (1300, 580),
            _ => (3900, 1550),
        };
        assert!(
            tally.matched > matched && tally.none > none,
            "{name}: {tally:?}"
        );
    }
}

/// The longest run of the other kind between two assertions of `kind`: (start, next match).
fn longest_gap(bits: &[bool], kind: u8) -> (u16, u16) {
    let hits: Vec<usize> = (0..bits.len())
        .filter(|&n| bits[n] == (kind == THEOREM))
        .collect();
    let (a, b) = hits
        .windows(2)
        .map(|w| (w[0], w[1]))
        .max_by_key(|&(a, b)| b - a)
        .unwrap();
    (a as u16, b as u16)
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_two_calls_cost_over_ring_is_printed_and_pinned() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/ring.mclib")).unwrap();
    let bits: Vec<bool> = decoded(&bytes)
        .assertions
        .iter()
        .map(|a| a.theorem)
        .collect();
    let count = bits.len() as u16;
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    let mut costs: Vec<(String, u32, u32)> = Vec::new();
    let mut run = |rig: &mut Rig, what: &str, target: &str, regs: Regs, pinned: u32| {
        let took = rig.cost(target, regs);
        println!("{what}: {took} T-states");
        costs.push((what.to_string(), took, pinned));
    };
    let (a, b) = longest_gap(&bits, AXIOM);
    assert_eq!((a, b), (298, 567));
    run(
        &mut rig,
        "next axiom across 298 to 567",
        NEXT,
        seeded(AXIOM, a + 1, count),
        8116,
    );
    run(
        &mut rig,
        "prev axiom across 567 to 298",
        PREV,
        seeded(AXIOM, b, 0),
        7823,
    );
    let (a, b) = longest_gap(&bits, THEOREM);
    assert_eq!((a, b), (2988, 3019));
    run(
        &mut rig,
        "next theorem across ring's 30",
        NEXT,
        seeded(THEOREM, a + 1, count),
        3278,
    );
    run(
        &mut rig,
        "prev theorem across ring's 30",
        PREV,
        seeded(THEOREM, b, 0),
        3073,
    );
    run(
        &mut rig,
        "next over one assertion",
        NEXT,
        seeded(AXIOM, 0, 1),
        2540,
    );
    run(
        &mut rig,
        "prev over one assertion",
        PREV,
        seeded(AXIOM, 1, 0),
        2733,
    );
    let none = kinds_library(&vec![false; usize::from(count)]);
    let mut rig = Rig::with_library(&none);
    rig.open_ok();
    run(
        &mut rig,
        "next with no match to the end",
        NEXT,
        seeded(THEOREM, 0, count),
        66546,
    );
    run(
        &mut rig,
        "prev with no match to the start",
        PREV,
        seeded(THEOREM, count, 0),
        66157,
    );
    for (what, took, pinned) in &costs {
        let (low, high) = (pinned - pinned / 50, pinned + pinned / 100);
        assert!(
            (low..=high).contains(took),
            "{what}: {took} T-states, measured {pinned}"
        );
    }
}
