//! The same checks at counts that put the table's last byte, its size and its blocks on edges.

use super::layout::{self, Table};
use super::rig::*;

const COUNTS: [u16; 7] = [0, 1, 3, 4, 3326, 3328, 65535];

fn random_table(count: u16, seed: &mut u32) -> (Table, u8) {
    let mut t = Table::new(CRC, count);
    t.values = (0..count).map(|_| (xorshift(seed) & 3) as u8).collect();
    t.next = (xorshift(seed) % (u32::from(count) + 1)) as u16;
    (t, xorshift(seed) as u8)
}

fn loaded(count: u16, seed: &mut u32) -> (Rig, Table, u8) {
    let mut rig = Rig::new();
    let (t, pad) = random_table(count, seed);
    rig.m
        .create_var(VAR, &layout::encode_padded(&t, pad))
        .unwrap();
    (rig, t, pad)
}

#[test]
fn get_and_set_at_the_first_the_last_and_one_past_the_last() {
    let mut seed = 0xC0FF_EE11;
    for count in COUNTS {
        let (mut rig, mut t, pad) = loaded(count, &mut seed);
        let mut picks = vec![count];
        if count > 0 {
            picks.extend([0, count - 1]);
        }
        let mut script = vec![Cmd::Open(CRC, count)];
        let mut set_to = Vec::new();
        for n in &picks {
            let v = (xorshift(&mut seed) & 3) as u8;
            set_to.push(v);
            script.extend([Cmd::Get(*n), Cmd::Set(*n, v), Cmd::Get(*n)]);
        }
        let log = rig.run(&script);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, 0), "count {count}");
        for (i, (n, v)) in picks.iter().zip(&set_to).enumerate() {
            let [first, set, second] = &log[1 + 3 * i..4 + 3 * i] else {
                unreachable!()
            };
            if *n < count {
                assert_eq!(
                    first.b(),
                    t.values[usize::from(*n)],
                    "count {count} get {n}"
                );
                assert_eq!((first.a, set.a, second.a), (ST_OK, ST_OK, ST_OK));
                assert_eq!(second.b(), *v, "count {count} get {n} after set");
                t.values[usize::from(*n)] = *v;
            } else {
                assert_eq!((first.a, set.a, second.a), (ST_RANGE, ST_RANGE, ST_RANGE));
            }
        }
        assert_eq!(
            rig.raw().unwrap(),
            layout::encode_padded(&t, pad),
            "count {count}"
        );
    }
}

#[test]
fn reset_at_every_count() {
    let mut seed = 0xBADC_0DE5;
    for count in COUNTS {
        let (mut rig, _, _) = loaded(count, &mut seed);
        let log = rig.run(&[Cmd::Open(CRC, count), Cmd::Reset, Cmd::GetNext, Cmd::Count]);
        assert_eq!(log[1].a, ST_OK);
        assert_eq!(log[2].word(0), 0);
        assert_eq!(log[3].counts(), [count, 0, 0, 0], "count {count}");
        assert_eq!(
            rig.raw().unwrap(),
            layout::encode(&Table::new(CRC, count)),
            "count {count}"
        );
    }
}

#[test]
fn the_count_pass_at_every_count() {
    let mut seed = 0x0DDB_A110;
    for count in COUNTS {
        let (mut rig, t, _) = loaded(count, &mut seed);
        let log = rig.run(&[Cmd::Open(CRC, count), Cmd::Count]);
        let mut want = [0u16; 4];
        for v in &t.values {
            want[usize::from(*v)] += 1;
        }
        assert_eq!((log[1].a, log[1].counts()), (ST_OK, want), "count {count}");
    }
}

#[test]
fn open_creates_a_zeroed_table_at_every_count() {
    for count in COUNTS {
        let mut rig = Rig::new();
        let log = rig.run(&[Cmd::Open(CRC, count)]);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_ABSENT), "count {count}");
        assert_eq!(
            rig.raw().unwrap(),
            layout::encode(&Table::new(CRC, count)),
            "count {count}"
        );
    }
}

#[test]
fn an_archived_table_comes_back_unchanged_at_every_count() {
    let mut seed = 0xFEED_F00D;
    // The stub keeps an archive in one 16 KB flash page, so the largest table does not fit.
    for count in COUNTS.into_iter().filter(|&c| c < 65535) {
        let (mut rig, t, pad) = loaded(count, &mut seed);
        rig.m.archive_var(VAR);
        let log = rig.run(&[Cmd::Open(CRC, count), Cmd::GetNext]);
        assert_eq!(
            (log[0].a, log[0].b()),
            (ST_OK, WAS_ARCHIVED),
            "count {count}"
        );
        assert_eq!(log[1].word(0), t.next);
        let var = rig.m.read_var(VAR).unwrap();
        assert!(!var.archived);
        assert_eq!(var.data, layout::encode_padded(&t, pad), "count {count}");
    }
}
