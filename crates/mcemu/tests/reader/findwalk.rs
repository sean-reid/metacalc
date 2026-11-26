use crate::damaged::offset_of;
use crate::findrig::*;
use crate::support::*;
use mcpack::library::{CHUNK, LABEL_ORDER};

const SMALL: [&str; 12] = [
    "b", "ab", "a", "abc", "ba", "bb", "a-b", "A", "abd", "b1", "aa", "zz",
];

fn small() -> (Model, Vec<u8>) {
    let labels = strings(&SMALL);
    let bits = pseudo(labels.len(), 7);
    assert!(bits.iter().filter(|&&t| t).count() > 3);
    assert!(bits.iter().filter(|&&t| !t).count() > 3);
    let bytes = library(&labels, Some(&bits));
    (Model::new(&labels, Some(bits)), bytes)
}

/// The distinct ranges of a prefix sweep, with the whole section and an empty range added.
fn ranges(model: &Model) -> Vec<(u16, u16)> {
    let mut found = vec![(0, model.len()), (3, 0)];
    for prefix in sweep_prefixes(model) {
        let range = model.range(&prefix).unwrap();
        if !found.contains(&range) {
            found.push(range);
        }
    }
    found
}

#[derive(Default, Debug)]
struct Tally {
    found: usize,
    bound: usize,
    index: usize,
    arg: usize,
}

fn check_walk(
    rig: &mut Rig,
    model: &Model,
    range: (u16, u16),
    s: u16,
    b: u16,
    kind: u8,
    t: &mut Tally,
) {
    let want = model.walk(range, s, b, kind);
    let out = ask_walk(rig, range, s, b, kind);
    let what = format!("range {range:?} start {s} bound {b} kind {kind}");
    match want {
        Ok((n, _)) => {
            assert_eq!((out.status(), out.hl), (ST_OK, n), "{what}");
            if n < b {
                t.found += 1;
            } else {
                assert_eq!(n, b);
                t.bound += 1;
            }
        }
        Err(status) => {
            assert_eq!(out.status(), status, "{what}");
            if status == ST_INDEX {
                t.index += 1
            } else {
                t.arg += 1
            }
        }
    }
}

#[test]
fn the_walk_over_every_range_kind_start_and_bound_of_a_small_library_is_the_rules_answer() {
    let (model, bytes) = small();
    let mut rig = opened(&bytes);
    let found = ranges(&model);
    assert!(found.len() > 20, "{} ranges", found.len());
    assert!(found.iter().any(|r| r.1 == 0) && found.iter().any(|r| r.1 == 1));
    assert!(found.iter().any(|r| r.1 > 2));
    let mut t = Tally::default();
    for &range in &found {
        for kind in [AXIOM, THEOREM, EVERY] {
            for start in 0..=model.len() {
                for bound in 0..=model.len() {
                    check_walk(&mut rig, &model, range, start, bound, kind, &mut t);
                }
            }
        }
    }
    println!("{} ranges: {t:?}", found.len());
    assert!(t.found > 1250, "{t:?}");
    assert!(t.bound > 4900, "{t:?}");
    assert!(t.index > 5300, "{t:?}");
    assert_eq!(t.arg, 0);
}

#[test]
fn arguments_the_walk_refuses_are_refused_with_their_status_in_order() {
    let (model, bytes) = small();
    let mut rig = opened(&bytes);
    let whole = (0, 12);
    let mut t = Tally::default();
    for kind in [2u8, 3, 0x80, 0xFE] {
        check_walk(&mut rig, &model, whole, 0, 12, kind, &mut t);
        check_walk(&mut rig, &model, (40, 40), 99, 98, kind, &mut t);
    }
    assert_eq!(
        t.arg, 8,
        "a kind that is not 0, 1 or FFh is 2 before any index is looked at"
    );
    let cases = [
        ((0, 13), 0, 12, "a range one past the section"),
        ((12, 1), 0, 12, "first at the count and one entry"),
        ((13, 0), 0, 12, "first past the count"),
        ((0xFFFF, 2), 0, 12, "first and count wrap 16 bits"),
        ((0, 12), 0, 13, "bound past the count"),
        ((0, 12), 5, 4, "start past the bound"),
        ((0, 12), 13, 13, "start and bound past the count"),
    ];
    for (range, s, b, what) in cases {
        for kind in [AXIOM, THEOREM, EVERY] {
            let out = ask_walk(&mut rig, range, s, b, kind);
            assert_eq!(out.status(), ST_INDEX, "{what} under {kind}");
        }
    }
    for (range, s, b) in [((12, 0), 12, 12), ((0, 12), 12, 12), ((0, 12), 0, 0)] {
        let out = ask_walk(&mut rig, range, s, b, EVERY);
        assert_eq!((out.status(), out.hl), (ST_OK, b), "{range:?} {s} {b}");
    }
}

#[test]
fn a_block_that_is_not_allowed_is_refused_with_2() {
    let (_, bytes) = small();
    let mut rig = opened(&bytes);
    let state = rig.label("LIB_STATE");
    let size = rig.label("LIB_STATE_SIZE");
    for (at, what) in [
        (0x7FFFu16, "bank A"),
        (0xBFF8, "ends one past bank B"),
        (0xC000, "bank C"),
        (state, "the state block"),
        (state + size - 1, "ends in the state block"),
        (state - 8, "starts below the state block and ends in it"),
    ] {
        let out = rig.call(WALK, walk_regs(at));
        assert_eq!(out.status(), ST_ARG, "{what}");
    }
    for (at, what) in [
        (0xBFF7u16, "ends on the last byte of bank B"),
        (state - 9, "ends one below the state block"),
        (state + size, "starts one past the block"),
    ] {
        rig.poke(at, &block(0, 12, 0, 12, EVERY));
        let out = rig.call(WALK, walk_regs(at));
        assert_eq!((out.status(), out.hl), (ST_OK, 0), "{what}");
    }
}

/// Candidates the walk tests against the bitmap, counted by watching the first byte of lr_hdr,
/// which only a bitmap read writes in a walk.
fn bitmap_reads(rig: &mut Rig, range: (u16, u16), s: u16, b: u16, kind: u8) -> (u16, usize) {
    let at = rig.label("LR_HDR");
    rig.poke(BLOCK_AT, &block(range.0, range.1, s, b, kind));
    rig.m.watch_writes(at, 1);
    let out = rig.call(WALK, walk_regs(BLOCK_AT));
    let writes = rig.m.take_writes();
    rig.m.unwatch_writes();
    assert_eq!(out.status(), ST_OK);
    (out.hl, writes.len())
}

fn reads_of(labels: &[&str], kind: u8) -> (usize, Model) {
    let labels = strings(labels);
    let bits = vec![true; labels.len()];
    let model = Model::new(&labels, Some(bits.clone()));
    let mut rig = opened(&library(&labels, Some(&bits)));
    let range = model.range(b"k").unwrap();
    assert_eq!(range, (0, labels.len() as u16));
    let (n, reads) = bitmap_reads(&mut rig, range, 0, labels.len() as u16, kind);
    let (want, want_reads) = model.walk(range, 0, labels.len() as u16, kind).unwrap();
    assert_eq!((n, reads), (want, want_reads));
    (reads, model)
}

#[test]
fn only_a_candidate_below_the_best_so_far_is_tested_against_the_bitmap() {
    let falling = ["k5", "k4", "k3", "k2", "k1", "k0"];
    let rising = ["k0", "k1", "k2", "k3", "k4", "k5"];
    let (falls, model) = reads_of(&falling, THEOREM);
    assert_eq!(
        model.order,
        [5, 4, 3, 2, 1, 0],
        "numbers fall along the labels"
    );
    assert_eq!(
        falls, 6,
        "a range in falling number order tests each candidate"
    );
    let (rises, model) = reads_of(&rising, THEOREM);
    assert_eq!(model.order, [0, 1, 2, 3, 4, 5]);
    assert_eq!(rises, 1, "a range in rising number order tests one");
    assert_eq!(
        reads_of(&falling, EVERY).0,
        0,
        "every assertion reads no bitmap"
    );
    assert_eq!(reads_of(&rising, EVERY).0, 0);
    let all_axioms = strings(&rising);
    let mut rig = opened(&library(&all_axioms, Some(&vec![true; 6])));
    let (n, reads) = bitmap_reads(&mut rig, (0, 6), 0, 6, AXIOM);
    assert_eq!(
        (n, reads),
        (6, 6),
        "no axiom: every candidate is tested and none lowers the best"
    );
}

#[test]
fn candidates_outside_start_and_bound_are_not_tested() {
    let labels = strings(&["k5", "k4", "k3", "k2", "k1", "k0"]);
    let bits = vec![true; 6];
    let mut rig = opened(&library(&labels, Some(&bits)));
    let model = Model::new(&labels, Some(bits));
    let mut seen = Vec::new();
    for (start, bound) in [
        (0u16, 6u16),
        (2, 6),
        (0, 4),
        (2, 4),
        (3, 3),
        (6, 6),
        (5, 6),
        (0, 1),
        (4, 5),
    ] {
        let want = model.walk((0, 6), start, bound, THEOREM).unwrap();
        let got = bitmap_reads(&mut rig, (0, 6), start, bound, THEOREM);
        assert_eq!(got, (want.0, want.1), "start {start} bound {bound}");
        seen.push(got.1);
    }
    assert_eq!(seen, [6, 4, 4, 2, 0, 0, 1, 1, 1]);
}

#[test]
fn the_reads_of_every_walk_of_a_small_library_are_the_rules_count() {
    let (model, bytes) = small();
    let mut rig = opened(&bytes);
    let (mut total, mut zero, mut many) = (0, 0, 0);
    for range in ranges(&model) {
        for kind in [AXIOM, THEOREM] {
            for (s, b) in [(0u16, 12u16), (3, 9), (0, 5), (7, 12), (4, 4), (1, 11)] {
                let (want, reads) = model.walk(range, s, b, kind).unwrap();
                assert_eq!(
                    bitmap_reads(&mut rig, range, s, b, kind),
                    (want, reads),
                    "{range:?} {s} {b} {kind}"
                );
                total += 1;
                zero += usize::from(reads == 0);
                many += usize::from(reads > 2);
            }
        }
    }
    println!("{total} walks, {zero} with no read, {many} with more than two");
    assert!(zero > 150 && many > 10, "{zero} {many}");
}

/// Labels `ax-0` to `ax-{n-1}`, whose label order is far from their number order, in a library
/// whose section has a chunk boundary with 40 entries or more on each side.
fn crossing() -> (Model, Vec<u8>, usize) {
    for n in (3000..4600usize).step_by(11) {
        let labels: Vec<String> = (0..n).map(|i| format!("ax-{i}")).collect();
        let bits = pseudo(n, 21);
        let bytes = library(&labels, Some(&bits));
        let at = offset_of(&bytes, LABEL_ORDER) as usize + 2;
        let end = at + 2 * n;
        let boundary = end / CHUNK * CHUNK;
        if boundary > at + 80 && boundary + 80 < end {
            let entry = (boundary - at) / 2;
            return (Model::new(&labels, Some(bits)), bytes, entry);
        }
    }
    panic!("no library with the label order section across a chunk");
}

#[test]
fn a_range_across_a_chunk_and_a_range_longer_than_a_window_are_walked_whole() {
    let (model, bytes, entry) = crossing();
    let n = model.len();
    let mut rig = opened(&bytes);
    let mut t = Tally::default();
    let mut crossed = 0;
    for prefix in [&b"ax-"[..], b"ax-1", b"ax-12", b"ax-2", b"ax-3"] {
        let range = model.range(prefix).unwrap();
        let (first, count) = (usize::from(range.0), usize::from(range.1));
        assert!(
            count > 40,
            "{prefix:?}: a range longer than the 32 byte window holds"
        );
        let spans = first + 1 < entry && entry + 1 < first + count;
        crossed += usize::from(spans);
        let members: Vec<u16> = {
            let mut m = model.order[first..first + count].to_vec();
            m.sort();
            m
        };
        let (lo, hi, mid) = (
            members[0],
            members[members.len() - 1],
            members[members.len() / 2],
        );
        let around = |m: u16| [m.saturating_sub(1), m, (m + 1).min(n)];
        let mut points: Vec<u16> = [lo, hi, mid, 0, n].into_iter().flat_map(around).collect();
        points.sort();
        points.dedup();
        for kind in [AXIOM, THEOREM, EVERY] {
            for &s in &points {
                for &b in &points {
                    if s <= b {
                        check_walk(&mut rig, &model, range, s, b, kind, &mut t);
                    }
                }
            }
        }
    }
    println!("{crossed} ranges across the chunk: {t:?}");
    assert!(
        crossed >= 1,
        "a range that spans the boundary at entry {entry}"
    );
    assert!(t.found > 100 && t.bound > 100, "{t:?}");
}

#[test]
fn a_walk_that_ends_inside_the_chunk_and_one_that_starts_after_it_agree_with_the_rule() {
    let (model, bytes, entry) = crossing();
    let mut rig = opened(&bytes);
    let n = model.len();
    let mut t = Tally::default();
    for (first, count) in [
        (entry as u16 - 40, 40u16),
        (entry as u16 - 1, 2),
        (entry as u16 - 17, 33),
        (entry as u16, 40),
        (entry as u16 - 15, 17),
    ] {
        for kind in [AXIOM, THEOREM, EVERY] {
            for (s, b) in [(0, n), (n / 2, n), (0, n / 2), (n / 3, 2 * n / 3)] {
                check_walk(&mut rig, &model, (first, count), s, b, kind, &mut t);
            }
        }
    }
    assert!(t.found > 40 && t.bound > 3, "{t:?}");
}
