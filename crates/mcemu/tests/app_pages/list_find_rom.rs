//! The find calls through their far entries on the reference OS with the shipped ring library,
//! ignored in the default run: what the screen pays for each call, printed, and pinned within 2%.

use crate::app::*;
use crate::list_kind_real::ring;
use crate::realos::BASE;
use mcemu::{Reached, Regs};
use mcui::Status;

const PREFIX_AT: u16 = 0xA600;
const BLOCK_AT: u16 = 0xA610;
const COPY_AT: u16 = 0xA100;
const FLAGS: u16 = 0x89F0;
const LIMIT: u32 = 100_000_000;
const FAR_READ_PIN: u32 = 1_414;

struct Ring {
    labels: Vec<Vec<u8>>,
    theorems: Vec<bool>,
    order: Vec<u16>,
}

impl Ring {
    fn of(built: &Built) -> Ring {
        let library = crate::support::decoded(built.library.as_ref().unwrap());
        let labels: Vec<Vec<u8>> = library
            .assertions
            .iter()
            .map(|a| a.label.as_bytes().to_vec())
            .collect();
        let mut order: Vec<u16> = (0..labels.len() as u16).collect();
        order.sort_by(|&a, &b| labels[usize::from(a)].cmp(&labels[usize::from(b)]));
        Ring {
            theorems: library.assertions.iter().map(|a| a.theorem).collect(),
            labels,
            order,
        }
    }

    fn label_at(&self, i: usize) -> &[u8] {
        &self.labels[usize::from(self.order[i])]
    }

    fn range(&self, prefix: &[u8]) -> (u16, u16) {
        let first = (0..self.order.len())
            .take_while(|&i| self.label_at(i) < prefix)
            .count();
        let count = (first..self.order.len())
            .take_while(|&i| self.label_at(i).starts_with(prefix))
            .count();
        (first as u16, count as u16)
    }

    fn walk(&self, (first, count): (u16, u16), bound: u16, kind: u8) -> u16 {
        self.order[usize::from(first)..usize::from(first + count)]
            .iter()
            .copied()
            .filter(|&n| {
                n < bound && (kind == 0xFF || self.theorems[usize::from(n)] == (kind == 1))
            })
            .min()
            .unwrap_or(bound)
    }
}

/// `target` on the entry page from a driver that sets IX too, timed from its first instruction to
/// the entry page's `screen_ret`, less what the same driver takes to call a plain `ret`.
fn timed_ix(app: &mut App, target: u16, input: In, ix: u16) -> Done {
    let overhead = run(app, app.label("screen_ret"), In::default(), 0).tstates;
    let mut done = run(app, target, input, ix);
    done.tstates -= overhead;
    done
}

fn run(app: &mut App, target: u16, input: In, ix: u16) -> Done {
    let stop = app.label("screen_ret");
    let [lo, hi] = target.to_le_bytes();
    let [slo, shi] = stop.to_le_bytes();
    app.poke(app.driver, &[0xCD, lo, hi, 0xC3, slo, shi]);
    app.m.out_port(0x06, BASE);
    app.m.set_regs(&Regs {
        af: u16::from(input.a) << 8 | u16::from(input.f),
        bc: input.bc,
        de: input.de,
        hl: input.hl,
        ix,
        iy: FLAGS,
        sp: app.stack,
        pc: app.driver,
        af2: 0x2222,
        bc2: 0x3333,
        de2: 0x4444,
        hl2: 0x5555,
        i: 0xFF,
        iff1: 0,
        iff2: 0,
        im: 1,
    });
    let start = app.m.clock();
    let reached = app.m.run_to(BASE, stop, LIMIT);
    assert_eq!(reached, Reached::Target, "the call to {target:04X}");
    let tstates = app.m.clock().wrapping_sub(start);
    Done {
        regs: app.m.regs(),
        tstates,
        page: app.m.in_port(0x06),
    }
}

/// A measured cost is within 2% under and 1% over what it was when its pin was set.
fn pinned(costs: &[(String, u32, u32)]) {
    for (what, took, measured) in costs {
        let (low, high) = (measured - measured / 50, measured + measured / 100);
        assert!(
            (low..=high).contains(took),
            "{what}: {took} T-states, measured {measured}"
        );
    }
}

fn range_cost(app: &mut App, ring: &Ring, prefix: &[u8]) -> (u32, (u16, u16)) {
    app.poke(PREFIX_AT, prefix);
    let target = app.label("far_lib_find_range");
    let done = app.timed(
        BASE,
        target,
        In {
            a: 0x55,
            bc: (prefix.len() as u16) << 8,
            de: PREFIX_AT,
            hl: 0xFEED,
            f: 0xFF,
        },
    );
    let want = ring.range(prefix);
    assert_eq!(
        (done.status(), done.regs.hl, done.regs.bc),
        (0, want.0, want.1)
    );
    (done.tstates, want)
}

fn walk_cost(app: &mut App, ring: &Ring, range: (u16, u16), bound: u16, kind: u8) -> u32 {
    let mut block = Vec::new();
    for word in [range.0, range.1, 0, bound] {
        block.extend(word.to_le_bytes());
    }
    block.push(kind);
    app.poke(BLOCK_AT, &block);
    let target = app.label("far_lib_find_walk");
    let done = app.timed(
        BASE,
        target,
        In {
            a: 0x55,
            hl: BLOCK_AT,
            f: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(
        (done.status(), done.regs.hl),
        (0, ring.walk(range, bound, kind))
    );
    done.tstates
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_find_calls_cost_through_their_far_entries_on_the_reference_os_is_printed_and_pinned() {
    let mut list = ring(None);
    let built = list.built;
    let ring = Ring::of(built);
    let app = &mut list.run.shell.app;
    let n = ring.labels.len() as u16;
    let sorted = |i: usize| ring.label_at(i).to_vec();
    let cut = |l: Vec<u8>| l[..l.len().min(8)].to_vec();
    let eight = (0..ring.order.len())
        .map(sorted)
        .find(|l| l.len() == 8)
        .unwrap();
    let absent = (b'a'..=b'z')
        .find(|&c| ring.range(&[c]).1 == 0)
        .expect("a letter no label starts with");
    let cases: Vec<(&str, Vec<u8>, u32)> = vec![
        ("s", b"s".to_vec(), 95_665),
        ("sg", b"sg".to_vec(), 92_520),
        ("a whole 8 character label", eight, 117_506),
        ("the first label", cut(sorted(0)), 112_462),
        ("the last label", cut(sorted(ring.order.len() - 1)), 58_812),
        ("below the first label", vec![b'-' - 1], 54_672),
        ("above the last label", vec![b'~'], 50_291),
        ("a letter no label starts with", vec![absent], 54_692),
    ];
    let mut costs: Vec<(String, u32, u32)> = Vec::new();
    for (what, prefix, pin) in &cases {
        let (took, got) = range_cost(app, &ring, prefix);
        println!(
            "range, {what} ({:?}): {took} T-states, {got:?}",
            String::from_utf8_lossy(prefix)
        );
        costs.push((format!("range, {what}"), took, *pin));
    }
    let (mut worst, mut worst_at, mut total, mut calls) = (0, Vec::new(), 0u64, 0u64);
    let (mut worst_miss, mut miss_at) = (0, Vec::new());
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for label in &ring.labels {
        for p in 1..=label.len().min(8) {
            let prefix = label[..p].to_vec();
            if seen.contains(&prefix) {
                continue;
            }
            seen.push(prefix.clone());
            let (took, _) = range_cost(app, &ring, &prefix);
            total += u64::from(took);
            calls += 1;
            if took > worst {
                (worst, worst_at) = (took, prefix);
            }
        }
        let mut miss = label[..label.len().min(8)].to_vec();
        let last = miss.len() - 1;
        miss[last] = miss[last].wrapping_add(1);
        if ring.range(&miss).1 == 0 && calls % 5 == 0 {
            let (took, _) = range_cost(app, &ring, &miss);
            if took > worst_miss {
                (worst_miss, miss_at) = (took, miss);
            }
        }
    }
    println!(
        "range over every prefix of 1 to 8 characters of ring's labels: {calls} prefixes, worst {worst} T-states at {:?}, mean {}",
        String::from_utf8_lossy(&worst_at),
        total / calls
    );
    println!(
        "range that ends in no match, over the labels after every fifth prefix call with the last byte one up: worst {worst_miss} T-states at {:?}",
        String::from_utf8_lossy(&miss_at)
    );
    assert_eq!((calls, &worst_at[..]), (6338, &b"eleqtrrd"[..]));
    assert_eq!(&miss_at[..], b"sylan9er");
    costs.push(("range, the worst of every prefix".into(), worst, 120_540));
    costs.push((
        "range, the mean of every prefix".into(),
        (total / calls) as u32,
        107_914,
    ));
    costs.push(("range, no match, the worst".into(), worst_miss, 61_010));
    let s = ring.range(b"s");
    let df = ring.range(b"df");
    assert_eq!((s.1, df.1), (375, 192));
    for (what, range, bound, kind, pin) in [
        ("s, ring's end, FFh", s, n, 0xFF, 172_412),
        ("s, ring's end, theorems", s, n, 1, 183_272),
        ("s, ring's end, axioms", s, n, 0, 690_681),
        ("df, ring's end, theorems", df, n, 1, 242_053),
        ("s, bound 779, FFh", s, 779, 0xFF, 172_259),
        ("s, bound 779, axioms", s, 779, 0, 353_229),
        ("s, bound 1171, FFh", s, 1171, 0xFF, 172_412),
        ("s, bound 1171, axioms", s, 1171, 0, 433_443),
        ("df, bound 779, theorems", df, 779, 1, 99_604),
        ("df, bound 1171, theorems", df, 1171, 1, 106_519),
    ] {
        let took = walk_cost(app, &ring, range, bound, kind);
        println!("walk {what}: {took} T-states");
        costs.push((format!("walk {what}"), took, pin));
    }
    let mut copied = 0;
    let (mut first, mut count) = (s.0, s.1);
    let target = app.label("far_lib_find_copy");
    let pins = [14_400, 14_424, 13_845];
    let mut run_at = 0;
    while count > 0 {
        app.poke(COPY_AT, &[0xE5; 0x100]);
        let done = app.timed(
            BASE,
            target,
            In {
                a: 0x55,
                bc: count,
                de: COPY_AT,
                hl: first,
                f: 0xFF,
            },
        );
        assert_eq!(done.status(), 0);
        println!(
            "copy of {} entries: {} T-states",
            done.regs.hl, done.tstates
        );
        costs.push((
            format!("copy of {} entries", done.regs.hl),
            done.tstates,
            pins[run_at],
        ));
        copied += done.tstates;
        first += done.regs.hl;
        count -= done.regs.hl;
        run_at += 1;
    }
    println!("copy of s in its runs: {copied} T-states");
    costs.push(("copy of s in its runs".into(), copied, 42_669));
    let far_read = app.timed(
        BASE,
        app.label("far_lib_read"),
        In {
            bc: 1,
            de: COPY_AT,
            hl: 0x40,
            f: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(far_read.status(), 0);
    println!("far_lib_read of one byte: {} T-states", far_read.tstates);
    costs.push((
        "far_lib_read of one byte".into(),
        far_read.tstates,
        FAR_READ_PIN,
    ));
    pinned(&costs);
}

/// The store call over the runs of "s" on a table of ring's size where every assertion is 1 and
/// where none is, each run with the best so far as its limit.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_store_call_costs_over_the_runs_of_s_on_the_reference_os_is_printed_and_pinned() {
    let mut costs: Vec<(String, u32, u32)> = Vec::new();
    for (name, status, pins, whole) in [
        (
            "every assertion 1",
            Status::Checked,
            [90_343, 90_305, 86_865],
            267_513,
        ),
        (
            "no assertion 1",
            Status::Unchecked,
            [40_699, 37_478, 36_497],
            114_674,
        ),
    ] {
        let probe = ring(None);
        let n = crate::list_kind_rig::count(probe.built);
        drop(probe);
        let mut list = ring(Some(&vec![status; n]));
        let built = list.built;
        let ring = Ring::of(built);
        let app = &mut list.run.shell.app;
        let s = ring.range(b"s");
        let numbers: Vec<u16> = ring.order[usize::from(s.0)..usize::from(s.0 + s.1)].to_vec();
        let (mut limit, mut total) = (n as u16, 0);
        let target = app.label("store_least_not_one");
        for (at, run_of) in numbers.chunks(127).enumerate() {
            let bytes: Vec<u8> = run_of.iter().flat_map(|v| v.to_le_bytes()).collect();
            app.poke(COPY_AT, &bytes);
            let done = timed_ix(
                app,
                target,
                In {
                    a: 0x55,
                    bc: run_of.len() as u16,
                    de: 0,
                    hl: COPY_AT,
                    f: 0xFF,
                },
                limit,
            );
            assert_eq!(done.status(), 0, "{name}");
            let want = run_of
                .iter()
                .copied()
                .filter(|&v| v < limit && (status != Status::Checked))
                .min()
                .unwrap_or(limit);
            assert_eq!(done.regs.hl, want, "{name}");
            println!(
                "{name}: a run of {} with limit {limit}: {} T-states, answer {want}",
                run_of.len(),
                done.tstates
            );
            costs.push((format!("{name}, run {at}"), done.tstates, pins[at]));
            total += done.tstates;
            limit = want;
        }
        println!("{name}: 375 numbers in three runs: {total} T-states");
        costs.push((format!("{name}, the three runs"), total, whole));
    }
    pinned(&costs);
}
