//! The status table module, run by a real app on the stub OS: the app is launched the way
//! TI-OS launches it, reads its commands from an AppVar and logs what the module returned. The
//! table is checked against a layout written separately in `store/layout.rs`.

#[path = "store/layout.rs"]
mod layout;
#[path = "store/rig.rs"]
mod rig;

use layout::Table;
use rig::*;

const CRC: u32 = 0xCBF4_3926;
const COUNT: u16 = 3326;

fn xorshift(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

#[test]
fn open_creates_the_table_zeroed_when_there_is_none() {
    let mut rig = Rig::new();
    let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_ABSENT));
    let var = rig.m.read_var("MCSTAT").unwrap();
    assert_eq!(var.data.len(), 842);
    assert!(!var.archived);
    let t = layout::decode(&var.data).unwrap();
    assert_eq!(t, Table::new(CRC, COUNT));
}

#[test]
fn reopening_keeps_what_was_written() {
    let mut rig = Rig::new();
    rig.run(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Set(0, 1),
        Cmd::Set(1663, 2),
        Cmd::Set(3325, 3),
        Cmd::SetNext(1664),
    ]);
    let log = rig.run(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Get(0),
        Cmd::Get(1663),
        Cmd::Get(3325),
        Cmd::Get(1),
        Cmd::GetNext,
    ]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, 0), "kept as it was");
    let values: Vec<u8> = log[1..5].iter().map(|r| r.b()).collect();
    assert_eq!(values, [1, 2, 3, 0]);
    assert_eq!(log[5].word(0), 1664);
    assert!(log.iter().all(|r| r.a == ST_OK));
}

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
            assert_eq!(rig.table().unwrap(), want, "n {n} v {v}");
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
    rig.m.create_var("MCSTAT", &layout::encode(&model)).unwrap();
    for n in [0u16, 3, 4, 7, 8, 1000, 1001, 1002, 1003, 3324, 3325] {
        let v = (xorshift(&mut seed) & 3) as u8;
        let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Set(n, v)]);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, 0));
        model.values[usize::from(n)] = v;
        assert_eq!(rig.table().unwrap(), model, "after setting {n} to {v}");
    }
}

#[test]
fn another_librarys_crc_or_count_recreates_the_table_zeroed() {
    for (crc, count) in [(CRC ^ 1, COUNT), (CRC, COUNT + 1), (CRC, 100), (!CRC, 7)] {
        let mut rig = Rig::new();
        let mut old = Table::new(CRC, COUNT);
        old.values.fill(3);
        old.next = 99;
        rig.m.create_var("MCSTAT", &layout::encode(&old)).unwrap();
        let log = rig.run(&[Cmd::Open(crc, count)]);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_OTHER_LIBRARY));
        assert_eq!(rig.table().unwrap(), Table::new(crc, count));
    }
}

#[test]
fn a_wrong_magic_or_version_recreates_the_table() {
    for (at, to) in [(0, 0x4E), (0, 0), (1, 2), (1, 0)] {
        let mut rig = Rig::new();
        let mut bytes = layout::encode(&Table::new(CRC, COUNT));
        bytes[at] = to;
        rig.m.create_var("MCSTAT", &bytes).unwrap();
        let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
        assert_eq!(
            (log[0].a, log[0].b()),
            (ST_OK, ST_BAD_MAGIC),
            "byte {at} = {to}"
        );
        assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
    }
}

#[test]
fn a_size_the_header_does_not_imply_recreates_the_table() {
    let good = layout::encode(&Table::new(CRC, COUNT));
    let cases = [
        [good.clone(), vec![0]].concat(),
        good[..good.len() - 1].to_vec(),
        good[..10].to_vec(),
        good[..9].to_vec(),
        good[..3].to_vec(),
        vec![],
    ];
    for bytes in cases {
        let mut rig = Rig::new();
        let len = bytes.len();
        rig.m.create_var("MCSTAT", &bytes).unwrap();
        let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
        assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_BAD_SIZE), "size {len}");
        assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
    }
}

/// The least free RAM `store_open` goes on with: it asks _EnoughMem for the table plus OVERHEAD.
fn least_free() -> usize {
    layout::size(COUNT) + OVERHEAD
}

#[test]
fn not_enough_ram_leaves_nothing_half_created() {
    let need = least_free();
    let mut rig = Rig::new();
    let before = rig.run_free(&[Cmd::Open(CRC, COUNT)], need - 1);
    assert_eq!((before[0].a, before[0].b()), (ST_NO_RAM, 0));
    assert!(rig.m.read_var("MCSTAT").is_none());
    assert_eq!(rig.m.free_ram(), need - 1);
    let log = rig.run(&[Cmd::Get(0), Cmd::Set(0, 1), Cmd::GetNext]);
    assert!(log.iter().all(|r| r.a == ST_ABSENT));

    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], need);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_ABSENT));
    assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
}

#[test]
fn a_table_that_is_replaced_for_lack_of_room_leaves_no_variable() {
    let mut rig = Rig::new();
    let mut old = Table::new(CRC ^ 1, COUNT);
    old.values.fill(1);
    rig.m.create_var("MCSTAT", &layout::encode(&old)).unwrap();
    // Deleting the old table frees its bytes, its size word and its 13-byte entry, and the new
    // one then wants the table plus OVERHEAD.
    let freed = layout::size(COUNT) + 2 + 13;
    let short = least_free() - freed - 1;
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], short);
    assert_eq!((log[0].a, log[0].b()), (ST_NO_RAM, 0));
    assert!(rig.m.read_var("MCSTAT").is_none());

    rig.m.create_var("MCSTAT", &layout::encode(&old)).unwrap();
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], short + 1);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_OTHER_LIBRARY));
}

#[test]
fn an_archived_table_comes_back_to_ram_when_there_is_room() {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, COUNT);
    t.values[10] = 2;
    t.values[3000] = 3;
    t.next = 77;
    rig.m.create_var("MCSTAT", &layout::encode(&t)).unwrap();
    rig.m.create_var("OTHER", &[1; 30]).unwrap();
    rig.m.archive_var("MCSTAT");
    assert!(rig.m.read_var("MCSTAT").unwrap().archived);
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(3000)]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, WAS_ARCHIVED));
    assert_eq!(log[1].b(), 3);
    assert!(!rig.m.read_var("MCSTAT").unwrap().archived);
    assert_eq!(rig.table().unwrap(), t);
    assert_eq!(rig.m.read_var("OTHER").unwrap().data, [1; 30]);
}

#[test]
fn an_archived_table_without_room_stays_archived_and_every_call_says_so() {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, COUNT);
    t.values[5] = 1;
    rig.m.create_var("MCSTAT", &layout::encode(&t)).unwrap();
    rig.m.archive_var("MCSTAT");
    let script = [
        Cmd::Open(CRC, COUNT),
        Cmd::Get(5),
        Cmd::Set(5, 2),
        Cmd::GetNext,
        Cmd::SetNext(1),
        Cmd::Count,
        Cmd::Reset,
    ];
    let log = rig.run_free(&script, least_free() - 1);
    assert!(log.iter().all(|r| r.a == ST_ARCHIVED), "{log:?}");
    let var = rig.m.read_var("MCSTAT").unwrap();
    assert!(var.archived);
    assert_eq!(layout::decode(&var.data).unwrap(), t);

    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], least_free());
    assert_eq!((log[0].a, log[0].b()), (ST_OK, WAS_ARCHIVED));
}

#[test]
fn an_archived_table_larger_than_its_header_implies_is_judged_by_its_own_size() {
    let mut bytes = layout::encode(&Table::new(CRC, COUNT));
    bytes.extend([0; 3000]);
    let need = bytes.len() + OVERHEAD;
    let mut rig = Rig::new();
    rig.m.create_var("MCSTAT", &bytes).unwrap();
    rig.m.archive_var("MCSTAT");
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], need - 1);
    assert_eq!((log[0].a, log[0].b()), (ST_ARCHIVED, 0));
    let var = rig.m.read_var("MCSTAT").unwrap();
    assert!(var.archived);
    assert_eq!(var.data, bytes);
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], need);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, WAS_ARCHIVED | ST_BAD_SIZE));
    assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
}

#[test]
fn an_archived_table_of_another_library_is_replaced_after_it_is_brought_back() {
    let mut rig = Rig::new();
    let mut old = Table::new(CRC ^ 5, COUNT);
    old.values.fill(2);
    rig.m.create_var("MCSTAT", &layout::encode(&old)).unwrap();
    rig.m.archive_var("MCSTAT");
    let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
    assert_eq!(
        (log[0].a, log[0].b()),
        (ST_OK, WAS_ARCHIVED | ST_OTHER_LIBRARY)
    );
    assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
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
fn reset_marks_everything_unchecked_and_keeps_the_header() {
    let mut rig = Rig::new();
    let log = rig.run(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Set(0, 1),
        Cmd::Set(3325, 3),
        Cmd::SetNext(500),
        Cmd::Reset,
        Cmd::Get(0),
        Cmd::Get(3325),
        Cmd::GetNext,
    ]);
    assert_eq!(log[4].a, ST_OK);
    assert_eq!((log[5].b(), log[6].b(), log[7].word(0)), (0, 0, 0));
    assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
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
    assert_eq!(rig.table().unwrap(), want);
}

#[test]
fn a_variable_made_between_launches_below_the_appvars_moves_the_table_up() {
    let mut rig = Rig::new();
    let first = [Cmd::Open(CRC, COUNT), Cmd::Set(1, 1), Cmd::Locate];
    let log = rig.run(&first);
    let before = log[2].word(0);
    rig.m.create_typed(1, "L1", &[7; 27]).unwrap();
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(1), Cmd::Locate]);
    assert_eq!((log[0].a, log[0].b(), log[1].b()), (ST_OK, 0, 1));
    // The old script goes, 13 bytes and its size word, and the list's 29 come in.
    assert_eq!(log[2].word(0), before - 15 + 29);
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

/// T-states of one command, from a run of it a hundred times against a run of nothing.
fn cost(rig: &mut Rig, cmd: Cmd, setup: &[Cmd]) -> u32 {
    let mut base = setup.to_vec();
    base.push(Cmd::Repeat(100));
    base.push(Cmd::Nop);
    let mut with = setup.to_vec();
    with.push(Cmd::Repeat(100));
    with.push(cmd);
    let (_, t0) = rig.run_timed(&base, None);
    let (_, t1) = rig.run_timed(&with, None);
    (t1 - t0) / 100
}

#[test]
fn timings_stay_inside_their_budgets() {
    let mut rig = Rig::new();
    let open = [Cmd::Open(CRC, COUNT)];
    let open_found = cost(&mut rig, open[0], &open);
    rig.m.delete_var("MCSTAT");
    let (_, bare) = rig.run_timed(&[Cmd::Nop], None);
    let (_, created) = rig.run_timed(&open, None);
    let open_created = created - bare;
    let get = cost(&mut rig, Cmd::Get(1663), &open);
    let set = cost(&mut rig, Cmd::Set(1663, 2), &open);
    let count = cost(&mut rig, Cmd::Count, &open);
    eprintln!(
        "T-states on the stub at 6 MHz from flash: open found {open_found}, open created {open_created}, get {get}, set {set}, count of {COUNT} {count}"
    );
    assert!(open_found < 20_000, "{open_found}");
    assert!(open_created < 60_000, "{open_created}");
    assert!(get < 10_000, "{get}");
    assert!(set < 10_000, "{set}");
    assert!(count < 250_000, "{count}");
}
