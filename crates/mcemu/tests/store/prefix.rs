//! Counts over a prefix of the table and over several prefixes in one pass: against a count by
//! hand, with refusals that write nothing, and against the totals of the host model.

use super::layout::{self, Table};
use super::rig::*;

/// Counts that put the last byte, the 63-byte blocks and the largest table on edges.
const COUNTS: [u16; 12] = [1, 3, 4, 5, 252, 253, 255, 256, 257, 3326, 3328, 65535];

const REAL_ENDS: &[u16] = &[779, 1171, 3326];

fn by_hand(values: &[u8], end: usize) -> [u16; 4] {
    let mut counts = [0u16; 4];
    for v in &values[..end] {
        counts[usize::from(*v)] += 1;
    }
    counts
}

/// A rig holding a random table whose last byte's spare bits are random too.
fn loaded(count: u16, seed: &mut u32) -> (Rig, Table) {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, count);
    t.values = (0..count).map(|_| (xorshift(seed) & 3) as u8).collect();
    let pad = xorshift(seed) as u8;
    rig.m
        .create_var(VAR, &layout::encode_padded(&t, pad))
        .unwrap();
    (rig, t)
}

fn leak(ends: Vec<u16>) -> &'static [u16] {
    Box::leak(ends.into_boxed_slice())
}

/// `n` distinct ends in 0..=count, rising, with the first few often in one byte.
fn random_ends(count: u16, seed: &mut u32) -> Vec<u16> {
    let wanted = 1 + (xorshift(seed) % 7) as usize;
    let span = if xorshift(seed) & 1 == 0 {
        u32::from(count) + 1
    } else {
        (u32::from(count) + 1).min(24)
    };
    let mut ends: Vec<u16> = (0..wanted)
        .map(|_| (xorshift(seed) % span) as u16)
        .collect();
    if span < u32::from(count) + 1 {
        ends.push(count);
    }
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[test]
fn a_prefix_counts_only_the_entries_below_n() {
    let mut seed = 0x51ED_2701;
    for count in COUNTS {
        let (mut rig, t) = loaded(count, &mut seed);
        let c = usize::from(count);
        let mut ns = vec![0, 1, 2, 3, 4, 5, 7, 8, 9, c / 3, c.saturating_sub(1), c];
        ns.retain(|&n| n <= c);
        ns.dedup();
        let mut script = vec![Cmd::Open(CRC, count)];
        script.extend(ns.iter().map(|&n| Cmd::CountTo(n as u16)));
        script.push(Cmd::Count);
        let before = rig.raw();
        let log = rig.run(&script);
        for (rec, n) in log[1..].iter().zip(&ns) {
            assert_eq!(
                (rec.a, rec.counts()),
                (ST_OK, by_hand(&t.values, *n)),
                "count {count} prefix {n}"
            );
        }
        let whole = log.last().unwrap();
        assert_eq!(whole.counts(), by_hand(&t.values, c), "count {count} whole");
        assert_eq!(rig.raw(), before, "count {count}: counting writes nothing");
    }
}

#[test]
fn a_prefix_past_the_table_is_refused_and_writes_nothing() {
    let mut seed = 0x0BAD_5EED;
    for count in [1, 4, 252, 3326, 65534] {
        let (mut rig, _) = loaded(count, &mut seed);
        let log = rig.run(&[
            Cmd::Open(CRC, count),
            Cmd::CountTo(count + 1),
            Cmd::CountTo(u16::MAX),
        ]);
        for rec in &log[1..] {
            assert_eq!(rec.a, ST_RANGE, "count {count}");
            assert_eq!(rec.counts(), [0xA5A5; 4], "count {count}: the buffer");
        }
    }
}

#[test]
fn one_pass_gives_every_end_its_own_counts() {
    let mut seed = 0xC0DE_F00D;
    for count in COUNTS {
        let (mut rig, t) = loaded(count, &mut seed);
        let mut lists: Vec<Vec<u16>> = vec![vec![count], vec![0], vec![0, count]];
        if count > 8 {
            lists.push((1..=8).collect());
            lists.push(vec![count - 1, count]);
            lists.push(vec![4, 8, count]);
        }
        if count == 3326 {
            lists.push(REAL_ENDS.to_vec());
        }
        for _ in 0..9 {
            lists.push(random_ends(count, &mut seed));
        }
        for batch in lists.chunks(4) {
            let mut script = vec![Cmd::Open(CRC, count)];
            script.extend(batch.iter().map(|l| Cmd::CountEnds(leak(l.clone()))));
            let log = rig.run(&script);
            for (rec, ends) in log[1..].iter().zip(batch) {
                assert_eq!((rec.a, rec.b()), (ST_OK, ends.len() as u8));
                let want: Vec<[u16; 4]> = ends
                    .iter()
                    .map(|&e| by_hand(&t.values, usize::from(e)))
                    .collect();
                assert_eq!(rec.ends(), want, "count {count} ends {ends:?}");
                assert!(
                    rec.untouched_past(ends.len()),
                    "count {count} ends {ends:?}: past K"
                );
            }
        }
    }
}

#[test]
fn the_three_real_ends_in_one_pass_equal_three_separate_prefixes() {
    let mut seed = 0x3A3A_3A3A;
    let (mut rig, t) = loaded(3326, &mut seed);
    let log = rig.run(&[
        Cmd::Open(CRC, 3326),
        Cmd::CountEnds(REAL_ENDS),
        Cmd::CountTo(779),
        Cmd::CountTo(1171),
        Cmd::CountTo(3326),
        Cmd::Count,
    ]);
    let pass = log[1].ends();
    assert_eq!(pass.len(), 3);
    assert!(log[1].untouched_past(3), "past the third result");
    for (i, end) in REAL_ENDS.iter().enumerate() {
        assert_eq!(pass[i], log[2 + i].counts(), "end {end}");
        assert_eq!(pass[i], by_hand(&t.values, usize::from(*end)));
    }
    assert_eq!(pass[2], log[5].counts(), "the last end is the whole table");
}

#[test]
fn a_bad_list_is_refused_and_writes_nothing() {
    const LISTS: [&[u16]; 12] = [
        &[],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
        &[5, 5],
        &[6, 3],
        &[10, 3327],
        &[3327],
        &[0, 0],
        &[3326, 3326],
        &[1, 2, 3, 3],
        &[65535],
        &[3000, 2999],
        &[7, 3326, 3325],
    ];
    let mut seed = 0xFACE_FEED;
    let (mut rig, t) = loaded(3326, &mut seed);
    let before = rig.raw();
    for lists in LISTS.chunks(4) {
        let mut script = vec![Cmd::Open(CRC, 3326)];
        script.extend(lists.iter().map(|l| Cmd::CountEnds(l)));
        let log = rig.run(&script);
        for (rec, ends) in log[1..].iter().zip(lists) {
            assert_eq!(rec.a, ST_RANGE, "ends {ends:?}");
            assert!(rec.untouched(), "ends {ends:?}: the results");
        }
    }
    assert_eq!(rig.raw(), before);
    let log = rig.run(&[Cmd::Open(CRC, 3326), Cmd::CountEnds(&[8, 3326])]);
    assert_eq!(
        log[1].ends(),
        [by_hand(&t.values, 8), by_hand(&t.values, 3326)],
        "a list after refusals"
    );
}

#[test]
fn eight_ends_are_the_most_and_the_buffer_past_them_is_not_written() {
    let mut seed = 0x8888_0001;
    let (mut rig, t) = loaded(3326, &mut seed);
    let eight: &[u16] = &[1, 2, 3, 4, 5, 6, 7, 3326];
    let log = rig.run(&[Cmd::Open(CRC, 3326), Cmd::CountEnds(eight)]);
    assert_eq!(log[1].a, ST_OK);
    let want: Vec<[u16; 4]> = eight
        .iter()
        .map(|&e| by_hand(&t.values, e.into()))
        .collect();
    assert_eq!(log[1].ends(), want);
    assert!(log[1].untouched_past(8));
}

/// Counts of one value that cross 255 and 256, the edges of a byte-wide tally, with the last byte
/// partial before, at and after the edge.
const UNIFORM_NS: [u16; 15] = [
    251, 252, 253, 254, 255, 256, 257, 511, 512, 513, 1023, 1024, 1025, 2047, 3326,
];

/// A rig holding a table of 3326 entries that are all `v`.
fn uniform(v: u8, seed: &mut u32) -> Rig {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, 3326);
    t.values = vec![v; 3326];
    let pad = xorshift(seed) as u8;
    rig.m
        .create_var(VAR, &layout::encode_padded(&t, pad))
        .unwrap();
    rig
}

fn only(v: u8, n: u16) -> [u16; 4] {
    let mut counts = [0; 4];
    counts[usize::from(v)] = n;
    counts
}

#[test]
fn a_table_of_one_value_counts_past_the_edges_of_a_byte_wide_tally() {
    let mut seed = 0x0101_0101;
    for v in 0..4 {
        let mut rig = uniform(v, &mut seed);
        let mut script = vec![Cmd::Open(CRC, 3326)];
        script.extend(UNIFORM_NS.map(Cmd::CountTo));
        script.push(Cmd::Count);
        let log = rig.run(&script);
        for (rec, n) in log[1..].iter().zip(UNIFORM_NS) {
            assert_eq!(rec.counts(), only(v, n), "value {v} prefix {n}");
        }
        assert_eq!(
            log.last().unwrap().counts(),
            only(v, 3326),
            "value {v} whole"
        );
    }
}

#[test]
fn a_table_of_one_value_counts_the_same_through_lists_of_ends() {
    const LISTS: [&[u16]; 4] = [
        &[251, 252, 253, 254, 255, 256, 257, 511],
        &[255, 256],
        &[511, 512, 513, 1023, 1024, 1025, 2047, 3326],
        &[0, 255, 256, 3326],
    ];
    let mut seed = 0x0202_0202;
    for v in 0..4 {
        let mut rig = uniform(v, &mut seed);
        let mut script = vec![Cmd::Open(CRC, 3326)];
        script.extend(LISTS.map(Cmd::CountEnds));
        let log = rig.run(&script);
        for (rec, ends) in log[1..].iter().zip(LISTS) {
            let want: Vec<[u16; 4]> = ends.iter().map(|&e| only(v, e)).collect();
            assert_eq!(rec.ends(), want, "value {v} ends {ends:?}");
            assert!(
                rec.untouched_past(ends.len()),
                "value {v} ends {ends:?}: past K"
            );
        }
    }
}

#[test]
fn a_tail_that_carries_a_tally_past_255_reaches_the_high_byte() {
    let mut seed = 0x0303_0303;
    for others in [1usize, 2, 3] {
        let mut t = Table::new(CRC, 3326);
        t.values = vec![1; 3326];
        for i in 0..others {
            t.values[17 + 40 * i] = 2;
        }
        let mut rig = Rig::new();
        let pad = xorshift(&mut seed) as u8;
        rig.m
            .create_var(VAR, &layout::encode_padded(&t, pad))
            .unwrap();
        let ns = [256u16, 257, 258, 259];
        let mut script = vec![Cmd::Open(CRC, 3326), Cmd::CountEnds(&[256, 257, 258, 259])];
        script.extend(ns.map(Cmd::CountTo));
        let log = rig.run(&script);
        let want: Vec<[u16; 4]> = ns.iter().map(|&n| by_hand(&t.values, n.into())).collect();
        assert!(want.iter().any(|c| c[1] > 255), "the table reaches 256");
        assert_eq!(log[1].ends(), want, "{others} other entries, by one pass");
        for (rec, want) in log[2..].iter().zip(&want) {
            assert_eq!(rec.counts(), *want, "{others} other entries, by prefix");
        }
    }
}

/// What the host model calls checked, failed and settled over a prefix, and the prefix's length,
/// from the four counts alone.
fn model_of(c: [u16; 4]) -> (usize, usize, usize, usize) {
    let n = c.map(usize::from);
    (
        n[1] + n[3],
        n[2],
        n[1] + n[2] + n[3],
        n[0] + n[1] + n[2] + n[3],
    )
}

mod model {
    use super::*;
    use mcui::data::{Installed, Lib, Stage};
    use mcui::{Mark, Status};

    /// The value the app stores for a mark.
    fn value(mark: Mark) -> u8 {
        match mark {
            Mark::Unchecked => 0,
            Mark::Checked => 1,
            Mark::Failed => 2,
            Mark::Resting => 3,
        }
    }

    /// The statuses of a round: random, mostly unchecked.
    pub fn random_status(len: usize, seed: &mut u32) -> Vec<Status> {
        let statuses = [Status::Unchecked, Status::Checked, Status::Failed];
        (0..len)
            .map(|_| match xorshift(seed) % 8 {
                0..=2 => statuses[1],
                3 => statuses[2],
                _ => statuses[0],
            })
            .collect()
    }

    /// Loads `bytes` with `status` and the stages ending at `ends`, stores the model's marks as the
    /// table, and compares the Z80's counts of every stage's prefix, and of the empty prefix, with
    /// the model's. A forward reference would make the model mark an unchecked assertion failed,
    /// which the table cannot hold; the exporter refuses such a library, so none is counted here.
    /// Returns how many assertions of each value the table held.
    pub fn agree(bytes: &[u8], ends: &[usize], status: Vec<Status>, label: &str) -> [usize; 4] {
        let len = status.len();
        let stages: Vec<Stage> = ends
            .iter()
            .map(|&end| Stage {
                name: format!("s{end}"),
                title: String::new(),
                end,
            })
            .collect();
        let lib = Lib::with_stages(
            Installed {
                name: label.to_string(),
                bytes: bytes.to_vec(),
            },
            status,
            stages,
        )
        .expect("the library loads with its stages");
        let mut t = Table::new(CRC, len as u16);
        t.values = (0..len).map(|i| value(lib.mark(i))).collect();
        let mut tally = [0; 4];
        for v in &t.values {
            tally[usize::from(*v)] += 1;
        }
        let mut rig = Rig::new();
        rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
        let wire: Vec<u16> = std::iter::once(0)
            .chain(ends.iter().map(|&e| e as u16))
            .collect();
        let mut script = vec![
            Cmd::Open(CRC, len as u16),
            Cmd::CountEnds(leak(wire.clone())),
        ];
        script.extend(wire.iter().map(|&e| Cmd::CountTo(e)));
        let log = rig.run(&script);
        let pass = log[1].ends();
        assert_eq!(pass.len(), wire.len(), "{label}");
        assert_eq!(pass[0], [0; 4], "{label}: the empty prefix");
        assert_eq!(log[2].counts(), [0; 4], "{label}: the empty prefix alone");
        for (i, &end) in ends.iter().enumerate() {
            let want = lib.counts(i);
            for (how, counts) in [("pass", pass[i + 1]), ("prefix", log[3 + i].counts())] {
                assert_eq!(
                    counts,
                    by_hand(&t.values, end),
                    "{label} stage {i} ending at {end}, by {how}: each value's own count"
                );
                let (checked, failed, settled, total) = model_of(counts);
                assert_eq!(
                    (want.checked, want.failed, want.settled, want.total),
                    (checked, failed, settled, total),
                    "{label} stage {i} ending at {end}, by {how}: {counts:?}"
                );
            }
        }
        tally
    }
}

#[test]
fn the_counts_give_the_models_totals_for_random_statuses_and_stages() {
    let bytes = crate::fixture::proofs();
    let len = mcpack::library::read(&bytes).unwrap().len();
    let mut seed = 0x70_7A15;
    let mut tally = [0; 4];
    for round in 0..40 {
        let mut ends: Vec<usize> = (0..1 + xorshift(&mut seed) % 3)
            .map(|_| 1 + (xorshift(&mut seed) as usize) % (len - 1))
            .collect();
        ends.push(len);
        ends.sort_unstable();
        ends.dedup();
        let status = model::random_status(len, &mut seed);
        let got = model::agree(&bytes, &ends, status, &format!("fixture round {round}"));
        for (all, one) in tally.iter_mut().zip(got) {
            *all += one;
        }
    }
    assert!(
        tally.iter().all(|&n| n > 0),
        "every value occurred: {tally:?}"
    );
}

#[test]
fn a_resting_assertion_is_stored_as_three_and_counted_as_settled() {
    use mcui::Status;
    let bytes = crate::fixture::proofs();
    let assertions = mcpack::library::read(&bytes).unwrap();
    let (user, used) = assertions
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            a.references.iter().find_map(|r| match r {
                mcpack::library::Reference::Assertion(j) if *j < i => Some((i, *j)),
                _ => None,
            })
        })
        .expect("a proof uses an earlier assertion");
    let len = assertions.len();
    let mut status = vec![Status::Checked; len];
    status[used] = Status::Unchecked;
    status[(0..len).rev().find(|&k| k != user && k != used).unwrap()] = Status::Failed;
    let tally = model::agree(&bytes, &[user + 1, len], status, "hand built");
    assert!(
        tally.iter().all(|&n| n > 0),
        "every value occurred: {tally:?}"
    );
}

#[test]
#[ignore = "needs MC_LIBRARIES, a directory holding ring.mclib"]
fn the_counts_give_the_models_totals_for_ring_at_its_three_stages() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES is set");
    let bytes = std::fs::read(std::path::Path::new(&dir).join("ring.mclib")).unwrap();
    let mut seed = 0x21_4C0DE;
    let len = mcpack::library::read(&bytes).unwrap().len();
    for round in 0..3 {
        let status = model::random_status(len, &mut seed);
        model::agree(&bytes, &[779, 1171, 3326], status, &format!("ring {round}"));
    }
}
