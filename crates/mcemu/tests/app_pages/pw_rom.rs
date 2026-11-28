//! The proof walk through its far entries on the reference OS with the shipped ring library,
//! ignored in the default run: what lib_proof, the uncut walk, one slice and pw_find cost, printed
//! and pinned. Each theorem is timed in turn, so lib_proof finds the record window on another page.

use crate::app::*;
use crate::list_kind_real::ring;
use crate::realos::BASE;
use mcpack::library::{Step, decode};

const BLOCK_AT: u16 = 0xA100;
const BUDGET: u16 = 26;
const MARK: u8 = 0xE5;
const LIB_PROOF_MEDIAN: u32 = 17_871;
const LIB_PROOF_WORST: u32 = 42_913;
const WALK_MEDIAN: u32 = 69_962;
const WALK_WORST: u32 = 1_175_420;
const PER_STEP: u32 = 2_719;
const SLICE_START: u32 = 107_566;
const SLICE_MORE: u32 = 104_213;
const FIND_DEEPEST: u32 = 46_066;
const FIND_WIDEST: u32 = 26_744;
const MORE: u8 = 45;
const SYNTAX: u8 = 42;

/// The id of the provable typecode in the symbol table: a constant of typecode FFh named `|-`.
fn provable(bytes: &[u8]) -> u8 {
    let symbols = usize::from(bytes[12]) + usize::from(bytes[13]);
    let mut at = 20;
    for id in 0..symbols {
        let len = usize::from(bytes[at + 1]);
        if bytes[at] == 0xFF && &bytes[at + 2..at + 2 + len] == b"|-" {
            return id as u8;
        }
        at += 2 + len;
    }
    panic!("no provable typecode");
}

/// A figure within 2% above and 5% below the one measured: the slow path must be the one taken.
fn near(what: &str, got: u32, measured: u32) {
    assert!(
        got <= measured + measured / 50 && got >= measured - measured / 20,
        "{what}: {got} against {measured} measured"
    );
}

fn call(app: &mut App, entry: &str, input: In) -> Done {
    let target = app.label(entry);
    app.timed(BASE, target, input)
}

fn proof(app: &mut App, n: u16) -> Done {
    call(
        app,
        "far_lib_proof",
        In {
            a: 0x55,
            hl: n,
            f: 0xFF,
            ..In::default()
        },
    )
}

fn shape(app: &mut App, tc: u8, n: u16, budget: u16) -> Done {
    app.m.load_ram(BLOCK_AT, &[MARK; 8]);
    call(
        app,
        "far_pw_shape",
        In {
            a: tc,
            bc: budget,
            de: BLOCK_AT,
            hl: n,
            f: 0xFF,
        },
    )
}

fn more(app: &mut App, budget: u16) -> Done {
    call(
        app,
        "far_pw_more",
        In {
            a: 0x55,
            bc: budget,
            de: BLOCK_AT,
            f: 0xFF,
            ..In::default()
        },
    )
}

fn find(app: &mut App, place: u32) -> Done {
    app.m.load_ram(BLOCK_AT, &[MARK; 47]);
    call(
        app,
        "far_pw_find",
        In {
            a: (place >> 16) as u8,
            hl: place as u16,
            de: BLOCK_AT,
            f: 0xFF,
            bc: 0x1234,
        },
    )
}

fn u24(b: &[u8]) -> u32 {
    u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16
}

struct Ring {
    labels: Vec<String>,
    theorems: Vec<usize>,
    steps: Vec<usize>,
}

impl Ring {
    fn of(bytes: &[u8]) -> Ring {
        let library = decode(bytes).unwrap();
        Ring {
            labels: library.assertions.iter().map(|a| a.label.clone()).collect(),
            theorems: (0..library.assertions.len())
                .filter(|&n| library.assertions[n].theorem)
                .collect(),
            steps: library
                .assertions
                .iter()
                .map(|a| {
                    a.proof
                        .iter()
                        .filter(|s| matches!(s, Step::Number(_)))
                        .count()
                })
                .collect(),
        }
    }
}

fn median_and_worst(costs: &[(usize, u32)]) -> ((usize, u32), (usize, u32)) {
    let mut sorted = costs.to_vec();
    sorted.sort_by_key(|&(n, t)| (t, n));
    (sorted[sorted.len() / 2], *sorted.last().unwrap())
}

/// Walks `n` to its end with `budget` and returns the T-states of the start and the most of any
/// call after it, and the calls.
fn sliced(app: &mut App, tc: u8, n: u16, budget: u16) -> (u8, u32, u32, usize) {
    let mut out = shape(app, tc, n, budget);
    let start = out.tstates;
    let (mut worst, mut calls) = (0, 0);
    while out.status() == MORE {
        out = more(app, budget);
        worst = worst.max(out.tstates);
        calls += 1;
    }
    (out.status(), start, worst, calls)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_proof_walk_costs_through_its_far_entries_on_the_reference_os_is_printed_and_pinned() {
    let mut list = ring(None);
    let bytes = list.built.library.clone().unwrap();
    let ring = Ring::of(&bytes);
    let tc = provable(&bytes);
    let app = &mut list.run.shell.app;

    let mut proofs = Vec::new();
    for &n in &ring.theorems {
        let done = proof(app, n as u16);
        assert_eq!(done.status(), 0, "{}", ring.labels[n]);
        proofs.push((n, done.tstates));
    }
    let (median, worst) = median_and_worst(&proofs);
    println!(
        "lib_proof over ring: median {} ({}), worst {} ({})",
        median.1, ring.labels[median.0], worst.1, ring.labels[worst.0]
    );
    near("lib_proof median", median.1, LIB_PROOF_MEDIAN);
    near("lib_proof worst", worst.1, LIB_PROOF_WORST);

    let mut walks = Vec::new();
    let (mut syntax, mut steps, mut spent) = (0, 0usize, 0u64);
    for &n in &ring.theorems {
        let done = shape(app, tc, n as u16, 0);
        if done.status() == SYNTAX {
            assert_eq!(app.m.read_ram(BLOCK_AT, 8), [MARK; 8], "{}", ring.labels[n]);
            syntax += 1;
            continue;
        }
        assert_eq!(done.status(), 0, "{}", ring.labels[n]);
        let block = app.m.read_ram(BLOCK_AT, 8);
        assert_eq!(
            usize::from(block[0]) | usize::from(block[1]) << 8,
            ring.steps[n],
            "{}: the steps",
            ring.labels[n]
        );
        walks.push((n, done.tstates));
        steps += ring.steps[n];
        spent += u64::from(done.tstates);
    }
    assert_eq!((syntax, walks.len()), (2, 3049));
    let (median, worst) = median_and_worst(&walks);
    println!(
        "uncut walk over ring: median {} ({}, {} steps), worst {} ({}, {} steps), {} a step over {steps} steps",
        median.1,
        ring.labels[median.0],
        ring.steps[median.0],
        worst.1,
        ring.labels[worst.0],
        ring.steps[worst.0],
        spent / steps as u64
    );
    assert_eq!(ring.labels[worst.0], "oprabidw");
    near("uncut median", median.1, WALK_MEDIAN);
    near("uncut worst", worst.1, WALK_WORST);
    near("a step over ring", (spent / steps as u64) as u32, PER_STEP);

    let (mut start_worst, mut more_worst, mut calls) = (0, 0, 0);
    for &(n, _) in &walks {
        let (status, start, worst, c) = sliced(app, tc, n as u16, BUDGET);
        assert_eq!(status, 0, "{}", ring.labels[n]);
        start_worst = start_worst.max(start);
        more_worst = more_worst.max(worst);
        calls += c;
    }
    println!(
        "budget {BUDGET}: worst start {start_worst}, worst continue {more_worst}, {calls} continues"
    );
    assert!(calls > 0);
    near("worst start", start_worst, SLICE_START);
    near("worst continue", more_worst, SLICE_MORE);

    for (label, pin) in [("tfrlem9", FIND_DEEPEST), ("tz7.44-2", FIND_WIDEST)] {
        let n = ring.labels.iter().position(|l| l == label).unwrap();
        let done = shape(app, tc, n as u16, 0);
        assert_eq!(done.status(), 0, "{label}");
        let total = u24(&app.m.read_ram(BLOCK_AT + 5, 3));
        let (mut worst, mut at, mut deepest) = (0, 0, 0);
        for place in 1..=total {
            let done = find(app, place);
            assert_eq!(done.status(), 0, "{label}: place {place}");
            let block = app.m.read_ram(BLOCK_AT, 47);
            assert_ne!(block, [MARK; 47]);
            deepest = deepest.max(block[6]);
            if done.tstates > worst {
                (worst, at) = (done.tstates, place);
            }
        }
        println!("find in {label}: total {total}, depth {deepest}, worst {worst} at place {at}");
        near(&format!("find in {label}"), worst, pin);
    }
}
