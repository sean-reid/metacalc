use super::layout::{self, Table};
use super::rig::*;

#[test]
fn every_value_reads_back_at_the_edges_and_at_byte_boundaries() {
    let mut rig = Rig::new();
    for n in [0u16, 1, 2, 3, 4, 5, 7, 8, 9, 1662, 1663, 3324, 3325] {
        for v in 0..4u8 {
            let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(n, v), Cmd::Get(n)]);
            assert_eq!(log[1].a, ST_OK, "set {n}");
            assert_eq!((log[2].a, log[2].b()), (ST_OK, v), "get {n} after set {v}");
            let mut want = Table::new(CRC, COUNT);
            want.values[usize::from(n)] = v;
            assert_eq!(rig.raw().unwrap(), layout::encode(&want), "n {n} v {v}");
            rig.m.delete_var("MCSTAT");
        }
    }
}

#[test]
fn a_set_leaves_every_neighbour_alone() {
    let mut rig = Rig::new();
    let mut seed = 0x1234_5679;
    let mut model = Table::new(CRC, COUNT);
    model.values = (0..COUNT)
        .map(|_| (xorshift(&mut seed) & 3) as u8)
        .collect();
    let padding = 0b1011_0000 | xorshift(&mut seed) as u8;
    rig.m
        .create_var("MCSTAT", &layout::encode_padded(&model, padding))
        .unwrap();
    for n in [0u16, 3, 4, 7, 8, 1000, 1001, 1002, 1003, 3324, 3325] {
        let v = (xorshift(&mut seed) & 3) as u8;
        let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(n, v)]);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, 0));
        model.values[usize::from(n)] = v;
        assert_eq!(
            rig.raw().unwrap(),
            layout::encode_padded(&model, padding),
            "after setting {n} to {v}: padding bits stay as they were"
        );
    }
}

#[test]
fn numbers_and_values_out_of_range_are_refused_and_change_nothing() {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, COUNT);
    t.values.iter_mut().step_by(7).for_each(|v| *v = 2);
    rig.m.create_var("MCSTAT", &layout::encode(&t)).unwrap();
    let log = rig.run(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Get(COUNT),
        Cmd::Get(u16::MAX),
        Cmd::Set(COUNT, 3),
        Cmd::Set(u16::MAX, 1),
        Cmd::Set(5, 4),
        Cmd::Set(5, 255),
        Cmd::SetNext(COUNT + 1),
        Cmd::SetNext(u16::MAX),
        Cmd::Get(COUNT - 1),
        Cmd::SetNext(COUNT),
        Cmd::GetNext,
    ]);
    assert!(log[1..9].iter().all(|r| r.a == ST_RANGE), "{log:?}");
    assert_eq!(log[9].a, ST_OK);
    assert_eq!(log[10].a, ST_OK);
    assert_eq!(log[11].word(0), COUNT);
    t.next = COUNT;
    assert_eq!(rig.table().unwrap(), t);
}

#[test]
fn a_table_of_no_assertions_refuses_every_number() {
    let mut rig = Rig::new();
    let log = rig.run(&[
        Cmd::Open(CRC, 0),
        Cmd::Get(0),
        Cmd::Set(0, 1),
        Cmd::SetNext(0),
        Cmd::Count,
    ]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_ABSENT));
    assert_eq!((log[1].a, log[2].a), (ST_RANGE, ST_RANGE));
    assert_eq!(log[3].a, ST_OK);
    assert_eq!(log[4].counts(), [0; 4]);
    assert_eq!(rig.m.read_var("MCSTAT").unwrap().data.len(), 10);
}

#[test]
fn the_next_number_survives_a_relaunch_and_leaves_the_table_alone() {
    let mut rig = Rig::new();
    rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(9, 1), Cmd::SetNext(1234)]);
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::GetNext, Cmd::Get(9)]);
    assert_eq!(log[1].word(0), 1234);
    assert_eq!(log[2].b(), 1);
    let t = rig.table().unwrap();
    assert_eq!(t.next, 1234);
    assert_eq!(t.values[9], 1);
}

#[test]
fn reset_marks_everything_unchecked_zeroes_the_padding_and_keeps_the_header() {
    let mut rig = Rig::new();
    let mut seed = 5;
    let mut t = Table::new(CRC, COUNT);
    t.values = (0..COUNT)
        .map(|_| (xorshift(&mut seed) & 3) as u8)
        .collect();
    t.next = 900;
    rig.m
        .create_var("MCSTAT", &layout::encode_padded(&t, 0xFF))
        .unwrap();
    let log = rig.run(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Reset,
        Cmd::Get(0),
        Cmd::Get(3325),
        Cmd::GetNext,
    ]);
    assert_eq!(log[1].a, ST_OK);
    assert_eq!((log[2].b(), log[3].b(), log[4].word(0)), (0, 0, 0));
    assert_eq!(rig.raw().unwrap(), layout::encode(&Table::new(CRC, COUNT)));
}

#[test]
fn a_next_number_above_the_count_is_refused() {
    let mut rig = Rig::new();
    rig.run(&[Cmd::Open(CRC, COUNT), Cmd::SetNext(COUNT)]);
    let mut bytes = rig.raw().unwrap();
    for (next, want) in [
        (COUNT, ST_OK),
        (COUNT + 1, ST_RANGE),
        (u16::MAX, ST_RANGE),
        (0, ST_OK),
    ] {
        bytes[8..10].copy_from_slice(&next.to_le_bytes());
        rig.m.delete_var("MCSTAT");
        rig.m.create_var("MCSTAT", &bytes).unwrap();
        let log = rig.run(&[Cmd::GetNext]);
        assert_eq!(log[0].a, want, "next {next}");
        if want == ST_OK {
            assert_eq!(log[0].word(0), next);
        }
    }
}

#[test]
fn the_count_pass_agrees_with_a_recount_over_random_tables() {
    let mut seed = 0x9E37_79B9;
    let mut rig = Rig::new();
    let mut counts_seen = vec![
        0u16,
        1,
        2,
        3,
        4,
        5,
        62 * 4,
        63 * 4,
        63 * 4 + 1,
        64 * 4 + 3,
        3326,
    ];
    counts_seen.extend((0..12).map(|_| (xorshift(&mut seed) % 4000) as u16));
    for count in counts_seen {
        let mut t = Table::new(CRC, count);
        t.values = (0..count)
            .map(|_| (xorshift(&mut seed) & 3) as u8)
            .collect();
        let padding = xorshift(&mut seed) as u8;
        rig.m.delete_var("MCSTAT");
        rig.m
            .create_var("MCSTAT", &layout::encode_padded(&t, padding))
            .unwrap();
        let log = rig.run(&[Cmd::Open(CRC, count), Cmd::Count]);
        assert_eq!((log[1].a, log[0].b()), (ST_OK, 0), "count {count}");
        let mut want = [0u16; 4];
        for v in &t.values {
            want[usize::from(*v)] += 1;
        }
        assert_eq!(log[1].counts(), want, "count {count}");
    }
}

#[test]
fn the_count_pass_of_a_uniform_table_has_one_full_counter() {
    let mut rig = Rig::new();
    for v in 0..4u8 {
        let mut t = Table::new(CRC, COUNT);
        t.values.fill(v);
        rig.m.delete_var("MCSTAT");
        rig.m.create_var("MCSTAT", &layout::encode(&t)).unwrap();
        let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Count]);
        let mut want = [0; 4];
        want[usize::from(v)] = COUNT;
        assert_eq!(log[1].counts(), want);
    }
}

#[test]
fn the_table_moves_between_accesses_and_the_module_follows_it() {
    let mut rig = Rig::new();
    let log = rig.run(&[
        Cmd::Make('A', 100),
        Cmd::Make('B', 50),
        Cmd::Open(CRC, COUNT),
        Cmd::Set(5, 2),
        Cmd::Locate,
        Cmd::Delete('A'),
        Cmd::Locate,
        Cmd::Set(6, 3),
        Cmd::Get(5),
        Cmd::Get(6),
        Cmd::Delete('B'),
        Cmd::Locate,
        Cmd::Set(7, 1),
        Cmd::Get(5),
        Cmd::Get(6),
        Cmd::Get(7),
        Cmd::Make('C', 40),
        Cmd::Locate,
        Cmd::Get(5),
    ]);
    assert!(log.iter().all(|r| r.a == ST_OK), "{log:?}");
    let at = |i: usize| log[i].word(0);
    assert_eq!(at(6), at(4) - 102, "deleting A moved the table down");
    assert_eq!(at(11), at(6) - 52, "so did deleting B");
    assert_eq!(
        at(17),
        at(11),
        "a variable made after it leaves it where it is"
    );
    assert_eq!(log[4].word(2), 842);
    assert_eq!((log[8].b(), log[9].b()), (2, 3));
    assert_eq!((log[13].b(), log[14].b(), log[15].b()), (2, 3, 1));
    assert_eq!(log[18].b(), 2);
    let mut want = Table::new(CRC, COUNT);
    want.values[5] = 2;
    want.values[6] = 3;
    want.values[7] = 1;
    assert_eq!(rig.raw().unwrap(), layout::encode(&want));
}

#[test]
fn a_list_made_inside_a_launch_leaves_the_table_where_it_is() {
    let mut rig = Rig::new();
    let log = rig.run(&[
        Cmd::Make('A', 50),
        Cmd::Open(CRC, COUNT),
        Cmd::Set(5, 2),
        Cmd::Locate,
        Cmd::List(2),
        Cmd::Locate,
        Cmd::Set(6, 3),
        Cmd::Get(5),
        Cmd::Delete('A'),
        Cmd::Locate,
        Cmd::Get(6),
        Cmd::List(1),
        Cmd::Get(5),
        Cmd::Set(7, 1),
    ]);
    assert!(log.iter().all(|r| r.a == ST_OK), "{log:?}");
    let at = |i: usize| log[i].word(0);
    assert_eq!(at(5), at(3), "TI-OS puts a new list above the table");
    assert_eq!(at(9), at(5) - 52, "deleting A below it moved it down");
    assert_eq!((log[7].b(), log[10].b(), log[12].b()), (2, 3, 2));
    let mut want = Table::new(CRC, COUNT);
    want.values[5] = 2;
    want.values[6] = 3;
    want.values[7] = 1;
    assert_eq!(rig.raw().unwrap(), layout::encode(&want));
}

#[test]
fn a_list_that_grows_between_launches_moves_the_table_up() {
    let mut rig = Rig::new();
    rig.m.delete_var("GUARDA");
    rig.m.create_typed(1, "L1", &[0; 12]).unwrap();
    rig.m.create_var("GUARDA", &[0xA7; 61]).unwrap();
    let first = [Cmd::Open(CRC, COUNT), Cmd::Set(1, 1), Cmd::Locate];
    let log = rig.run(&first);
    let before = log[2].word(0);
    rig.m.grow_var(1, "L1", 17);
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(1), Cmd::Locate]);
    assert_eq!((log[0].a, log[0].b(), log[1].b()), (ST_OK, 0, 1));
    // The old script goes, 13 bytes and its size word, and the list grows by 17.
    assert_eq!(log[2].word(0), before - 15 + 17);
}

#[test]
fn many_moves_never_corrupt_the_neighbours() {
    let mut rig = Rig::new();
    let letters = ['A', 'B', 'C', 'D', 'E'];
    let mut script: Vec<Cmd> = letters
        .iter()
        .enumerate()
        .map(|(i, &l)| Cmd::Make(l, 20 + 37 * i as u16))
        .collect();
    script.push(Cmd::Open(CRC, COUNT));
    let mut want = Table::new(CRC, COUNT);
    let mut seed = 77;
    for l in letters {
        for _ in 0..3 {
            let n = (xorshift(&mut seed) % u32::from(COUNT)) as u16;
            let v = (xorshift(&mut seed) & 3) as u8;
            script.push(Cmd::Set(n, v));
            want.values[usize::from(n)] = v;
        }
        script.push(Cmd::Delete(l));
    }
    for n in 0..COUNT {
        if n % 311 == 0 {
            script.push(Cmd::Get(n));
        }
    }
    let log = rig.run(&script);
    assert!(log.iter().all(|r| r.a == ST_OK));
    let gets = &log[log.len() - 11..];
    for (rec, n) in gets.iter().zip((0..COUNT).step_by(311)) {
        assert_eq!(rec.b(), want.values[usize::from(n)], "get {n}");
    }
    assert_eq!(rig.table().unwrap(), want);
}

#[test]
fn a_table_the_user_deleted_is_absent_and_open_makes_a_new_one() {
    let mut rig = Rig::new();
    rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(1, 1)]);
    rig.m.delete_var("MCSTAT");
    let log = rig.run(&[Cmd::Get(1), Cmd::Set(1, 1), Cmd::Count, Cmd::Reset]);
    assert!(log.iter().all(|r| r.a == ST_ABSENT));
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(1)]);
    assert_eq!((log[0].b(), log[1].b()), (ST_ABSENT, 0));
}

#[test]
fn a_ram_clear_loses_the_table_and_open_starts_over() {
    let mut rig = Rig::new();
    rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(1, 1)]);
    rig.m.clear_ram();
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(1)]);
    assert_eq!((log[0].a, log[0].b(), log[1].b()), (ST_OK, ST_ABSENT, 0));
}

#[test]
fn a_table_edited_under_the_module_is_noticed_on_the_next_access() {
    let mut rig = Rig::new();
    rig.run(&[Cmd::Open(CRC, COUNT)]);
    let mut bytes = rig.m.read_var("MCSTAT").unwrap().data;
    bytes[0] = 0;
    rig.m.delete_var("MCSTAT");
    rig.m.create_var("MCSTAT", &bytes).unwrap();
    let log = rig.run(&[Cmd::Get(1), Cmd::Set(1, 1), Cmd::Count]);
    assert!(log.iter().all(|r| r.a == ST_BAD_MAGIC));
    bytes[0] = 0x4D;
    bytes.pop();
    rig.m.delete_var("MCSTAT");
    rig.m.create_var("MCSTAT", &bytes).unwrap();
    let log = rig.run(&[Cmd::Get(1), Cmd::Reset]);
    assert!(log.iter().all(|r| r.a == ST_BAD_SIZE));
}
