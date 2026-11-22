//! The questions the check job asks the table: whether one exists for a library, whether a list of
//! assertions all hold 1, and where the next unchecked assertion is. Each is checked against a
//! count by hand, and none may write.

use super::layout::{self, Table};
use super::rig::*;

fn leak(list: Vec<u16>) -> &'static [u16] {
    Box::leak(list.into_boxed_slice())
}

fn loaded(values: Vec<u8>, seed: &mut u32) -> (Rig, Table) {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, values.len() as u16);
    t.values = values;
    t.next = (xorshift(seed) % 5) as u16;
    let pad = xorshift(seed) as u8;
    rig.m
        .create_var(VAR, &layout::encode_padded(&t, pad))
        .unwrap();
    (rig, t)
}

fn random_values(count: usize, seed: &mut u32, ones: u32) -> Vec<u8> {
    (0..count)
        .map(|_| {
            if xorshift(seed) % 100 < ones {
                1
            } else {
                (xorshift(seed) & 3) as u8
            }
        })
        .collect()
}

#[test]
fn has_says_yes_only_for_a_table_with_this_crc_and_count() {
    let mut t = Table::new(CRC, COUNT);
    t.values[5] = 2;
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    let before = rig.raw();
    let log = rig.run(&[
        Cmd::Has(CRC, COUNT),
        Cmd::Has(CRC ^ 1, COUNT),
        Cmd::Has(CRC ^ 0x8000_0000, COUNT),
        Cmd::Has(CRC, COUNT + 1),
        Cmd::Has(CRC, COUNT - 1),
        Cmd::Has(CRC, 0),
    ]);
    let got: Vec<u8> = log.iter().map(|r| r.a).collect();
    assert_eq!(
        got,
        [
            ST_OK,
            ST_OTHER_LIBRARY,
            ST_OTHER_LIBRARY,
            ST_OTHER_LIBRARY,
            ST_OTHER_LIBRARY,
            ST_OTHER_LIBRARY
        ]
    );
    assert_eq!(rig.raw(), before, "asking writes nothing");
}

#[test]
fn has_reports_why_a_table_is_not_there_and_creates_nothing() {
    let mut rig = Rig::new();
    let log = rig.run(&[Cmd::Has(CRC, COUNT)]);
    assert_eq!(log[0].a, ST_ABSENT);
    assert!(rig.raw().is_none(), "no table was created");

    let good = layout::encode(&Table::new(CRC, COUNT));
    let mut wrong_magic = good.clone();
    wrong_magic[0] = 0x4E;
    let mut wrong_version = good.clone();
    wrong_version[1] = 2;
    let long = [good.clone(), vec![0]].concat();
    let short = good[..good.len() - 1].to_vec();
    for (bytes, want) in [
        (wrong_magic, ST_BAD_MAGIC),
        (wrong_version, ST_BAD_MAGIC),
        (long, ST_BAD_SIZE),
        (short, ST_BAD_SIZE),
    ] {
        let mut rig = Rig::new();
        rig.m.create_var(VAR, &bytes).unwrap();
        let log = rig.run(&[Cmd::Has(CRC, COUNT)]);
        assert_eq!(log[0].a, want);
        assert_eq!(rig.raw().unwrap(), bytes, "the variable is as it was");
    }
}

#[test]
fn has_leaves_an_archived_table_archived() {
    let mut rig = Rig::new();
    rig.m
        .create_var(VAR, &layout::encode(&Table::new(CRC, COUNT)))
        .unwrap();
    rig.m.archive_var(VAR);
    let log = rig.run(&[Cmd::Has(CRC, COUNT)]);
    assert_eq!(log[0].a, ST_ARCHIVED);
    assert!(rig.m.read_var(VAR).unwrap().archived);
}

#[test]
fn all_ones_answers_for_every_list_as_a_count_by_hand_does() {
    let mut seed = 0x0A11_0E55;
    for count in [1usize, 3, 4, 5, 63, 64, 300, 1000] {
        let (mut rig, t) = loaded(random_values(count, &mut seed, 80), &mut seed);
        let before = rig.raw();
        for _ in 0..5 {
            let mut script = Vec::new();
            let mut want = Vec::new();
            for _ in 0..8 {
                let len = (xorshift(&mut seed) % 24) as usize;
                let list: Vec<u16> = (0..len)
                    .map(|_| {
                        let n = (xorshift(&mut seed) as usize % count) as u16;
                        if xorshift(&mut seed).is_multiple_of(5) {
                            0x8000 | xorshift(&mut seed) as u16
                        } else {
                            n
                        }
                    })
                    .collect();
                want.push(
                    list.iter()
                        .filter(|&&e| e & 0x8000 == 0)
                        .all(|&e| t.values[usize::from(e)] == 1) as u8,
                );
                script.push(Cmd::AllOnes(leak(list)));
            }
            let log = rig.run(&script);
            let got: Vec<(u8, u8)> = log.iter().map(|r| (r.a, r.b())).collect();
            let want: Vec<(u8, u8)> = want.iter().map(|&w| (ST_OK, w)).collect();
            assert_eq!(got, want, "count {count}");
        }
        assert_eq!(rig.raw(), before, "count {count}: a lookup writes nothing");
    }
}

#[test]
fn all_ones_looks_at_the_last_entry_and_stops_at_the_first_that_is_not_one() {
    let mut t = Table::new(CRC, 40);
    t.values.fill(1);
    t.values[9] = 3;
    t.values[21] = 2;
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    let log = rig.run(&[
        Cmd::AllOnes(&[0, 1, 2, 3]),
        Cmd::AllOnes(&[0, 1, 2, 9]),
        Cmd::AllOnes(&[9, 0, 1, 2]),
        Cmd::AllOnes(&[0, 21]),
        Cmd::AllOnes(&[4, 39]),
        Cmd::AllOnes(&[]),
        Cmd::AllOnes(&[8000]),
    ]);
    let got: Vec<(u8, u8)> = log.iter().map(|r| (r.a, r.b())).collect();
    assert_eq!(
        got,
        [
            (ST_OK, 1),
            (ST_OK, 0),
            (ST_OK, 0),
            (ST_OK, 0),
            (ST_OK, 1),
            (ST_OK, 1),
            (ST_RANGE, 0)
        ]
    );
}

#[test]
fn all_ones_skips_dummies_and_refuses_an_entry_past_the_count() {
    let mut t = Table::new(CRC, 10);
    t.values = vec![1, 0, 1, 1, 2, 1, 1, 1, 1, 3];
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    let log = rig.run(&[
        Cmd::AllOnes(&[0x8001, 0x8004, 0xFFFF]),
        Cmd::AllOnes(&[0x8001, 2, 0x8004, 4]),
        Cmd::AllOnes(&[0, 2, 10]),
        Cmd::AllOnes(&[0, 2, 0x7FFE]),
        Cmd::AllOnes(&[0, 1, 10]),
        Cmd::AllOnes(&[9]),
        Cmd::AllOnes(&[5, 6, 7, 8]),
    ]);
    let got: Vec<(u8, u8)> = log.iter().map(|r| (r.a, r.b())).collect();
    assert_eq!(
        got,
        [
            (ST_OK, 1),
            (ST_OK, 0),
            (ST_RANGE, 0),
            (ST_RANGE, 0),
            (ST_OK, 0),
            (ST_OK, 0),
            (ST_OK, 1)
        ]
    );
}

#[test]
fn all_ones_and_next_zero_report_a_missing_table() {
    let mut rig = Rig::new();
    let log = rig.run(&[Cmd::AllOnes(&[1, 2]), Cmd::NextZero(0, 0)]);
    assert_eq!((log[0].a, log[0].b()), (ST_ABSENT, 0));
    assert_eq!(log[1].a, ST_ABSENT);
    assert!(rig.raw().is_none());
}

fn first_zero(values: &[u8], from: usize, end: usize) -> usize {
    (from..end).find(|&i| values[i] == 0).unwrap_or(end)
}

#[test]
fn next_zero_finds_the_first_zero_below_the_end_as_a_scan_by_hand_does() {
    let mut seed = 0x2E50_0001;
    for count in [
        1usize, 2, 3, 4, 5, 7, 8, 9, 63, 64, 65, 255, 256, 1000, 3326,
    ] {
        for ones in [0, 50, 97, 100] {
            let (mut rig, t) = loaded(random_values(count, &mut seed, ones), &mut seed);
            let mut script = Vec::new();
            let mut want = Vec::new();
            for _ in 0..30 {
                let end = xorshift(&mut seed) as usize % (count + 1);
                let from = xorshift(&mut seed) as usize % (end + 1);
                script.push(Cmd::NextZero(from as u16, end as u16));
                want.push(first_zero(&t.values, from, end) as u16);
            }
            script.push(Cmd::NextZero(0, count as u16));
            want.push(first_zero(&t.values, 0, count) as u16);
            let before = rig.raw();
            let log = rig.run(&script);
            let got: Vec<(u8, u16)> = log.iter().map(|r| (r.a, r.word(0))).collect();
            let want: Vec<(u8, u16)> = want.iter().map(|&w| (ST_OK, w)).collect();
            assert_eq!(got, want, "count {count}, {ones}% ones");
            assert_eq!(rig.raw(), before, "count {count}: a scan writes nothing");
        }
    }
}

#[test]
fn next_zero_passes_whole_bytes_and_stops_inside_a_byte() {
    let mut t = Table::new(CRC, 64);
    t.values.fill(2);
    t.values[37] = 0;
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    let log = rig.run(&[
        Cmd::NextZero(0, 64),
        Cmd::NextZero(0, 37),
        Cmd::NextZero(0, 38),
        Cmd::NextZero(36, 64),
        Cmd::NextZero(37, 64),
        Cmd::NextZero(38, 64),
        Cmd::NextZero(64, 64),
        Cmd::NextZero(3, 3),
    ]);
    let got: Vec<u16> = log.iter().map(|r| r.word(0)).collect();
    assert_eq!(got, [37, 37, 37, 37, 37, 64, 64, 3]);
}

#[test]
fn next_zero_refuses_a_range_that_is_not_in_the_table() {
    let mut t = Table::new(CRC, 10);
    t.values.fill(1);
    t.values[4] = 0;
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    let log = rig.run(&[
        Cmd::NextZero(0, 11),
        Cmd::NextZero(6, 5),
        Cmd::NextZero(0, 0xFFFF),
        Cmd::NextZero(0xFFFF, 0xFFFF),
        Cmd::NextZero(5, 10),
    ]);
    let got: Vec<(u8, u16)> = log.iter().map(|r| (r.a, r.word(0))).collect();
    assert_eq!(got[0].0, ST_RANGE);
    assert_eq!(got[1].0, ST_RANGE);
    assert_eq!(got[2].0, ST_RANGE);
    assert_eq!(got[3].0, ST_RANGE);
    assert_eq!(got[4], (ST_OK, 10));
}
