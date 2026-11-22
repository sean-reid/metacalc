use super::layout::{self, Table};
use super::rig::*;

#[test]
fn open_creates_the_table_zeroed_when_there_is_none() {
    let mut rig = Rig::new();
    let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_ABSENT));
    let var = rig.m.read_var(VAR).unwrap();
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
fn another_librarys_crc_or_count_recreates_the_table_zeroed() {
    for (crc, count) in [(CRC ^ 1, COUNT), (CRC, COUNT + 1), (CRC, 100), (!CRC, 7)] {
        let mut rig = Rig::new();
        let mut old = Table::new(CRC, COUNT);
        old.values.fill(3);
        old.next = 99;
        rig.m.create_var(VAR, &layout::encode(&old)).unwrap();
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
        rig.m.create_var(VAR, &bytes).unwrap();
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
        rig.m.create_var(VAR, &bytes).unwrap();
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
    assert!(rig.m.read_var(VAR).is_none());
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
    rig.m.create_var(VAR, &layout::encode(&old)).unwrap();
    // Deleting the old table frees its bytes, its size word and its 13-byte entry, and the new
    // one then wants the table plus OVERHEAD.
    let freed = layout::size(COUNT) + 2 + 13;
    let short = least_free() - freed - 1;
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], short);
    assert_eq!((log[0].a, log[0].b()), (ST_NO_RAM, 0));
    assert!(rig.m.read_var(VAR).is_none());

    rig.m.create_var(VAR, &layout::encode(&old)).unwrap();
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
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    rig.m.create_var("OTHER", &[1; 30]).unwrap();
    rig.m.archive_var(VAR);
    assert!(rig.m.read_var(VAR).unwrap().archived);
    let log = rig.run(&[Cmd::Open(CRC, COUNT), Cmd::Get(3000)]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, WAS_ARCHIVED));
    assert_eq!(log[1].b(), 3);
    assert!(!rig.m.read_var(VAR).unwrap().archived);
    assert_eq!(rig.table().unwrap(), t);
    assert_eq!(rig.m.read_var("OTHER").unwrap().data, [1; 30]);
}

#[test]
fn an_archived_table_without_room_stays_archived_and_every_call_says_so() {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, COUNT);
    t.values[5] = 1;
    rig.m.create_var(VAR, &layout::encode(&t)).unwrap();
    rig.m.archive_var(VAR);
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
    let var = rig.m.read_var(VAR).unwrap();
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
    rig.m.create_var(VAR, &bytes).unwrap();
    rig.m.archive_var(VAR);
    let log = rig.run_free(&[Cmd::Open(CRC, COUNT)], need - 1);
    assert_eq!((log[0].a, log[0].b()), (ST_ARCHIVED, 0));
    let var = rig.m.read_var(VAR).unwrap();
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
    rig.m.create_var(VAR, &layout::encode(&old)).unwrap();
    rig.m.archive_var(VAR);
    let log = rig.run(&[Cmd::Open(CRC, COUNT)]);
    assert_eq!(
        (log[0].a, log[0].b()),
        (ST_OK, WAS_ARCHIVED | ST_OTHER_LIBRARY)
    );
    assert_eq!(rig.table().unwrap(), Table::new(CRC, COUNT));
}
