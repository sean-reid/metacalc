//! An archived table on the reference OS. The test archives it the way a user does, in the memory
//! menu; store_open brings it back.

use crate::harness::*;
use crate::layout::{self, Table};
use crate::open::{NEED, inner, pair_and_free, with_free};
use crate::pair::Pair;
use crate::rig::*;

const SIZE: u16 = 842;

/// A pair whose table holds `t` and is in the archive.
fn archived(t: &Table, script: &[Cmd]) -> Pair {
    let mut p = Pair::boot();
    p.launch(script);
    assert_eq!(
        p.only_the_table("before the archive", SIZE),
        layout::encode(t)
    );
    p.leave();
    p.toggle_archive();
    assert_eq!(
        table(&mut p.os),
        Some((layout::encode(t), true)),
        "archived"
    );
    p
}

fn written() -> (Table, Vec<Cmd>) {
    let mut t = Table::new(CRC, COUNT);
    (t.values[7], t.values[3325], t.next) = (2, 3, 99);
    let script = vec![
        Cmd::Open(CRC, COUNT),
        Cmd::Set(7, 2),
        Cmd::Set(3325, 3),
        Cmd::SetNext(99),
    ];
    (t, script)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_archived_table_is_refused_by_every_routine_until_open_brings_it_back() {
    let (t, script) = written();
    let mut p = archived(&t, &script);
    let log = p.launch(&[
        Cmd::Get(7),
        Cmd::Set(7, 1),
        Cmd::GetNext,
        Cmd::SetNext(0),
        Cmd::Count,
        Cmd::CountTo(5),
        Cmd::CountEnds(&[5]),
        Cmd::Reset,
        Cmd::Locate,
        Cmd::Open(CRC, COUNT),
        Cmd::Locate,
        Cmd::Get(7),
        Cmd::Get(3325),
        Cmd::GetNext,
    ]);
    for (i, rec) in log[..8].iter().enumerate() {
        assert_eq!(rec.a, ST_ARCHIVED, "record {i}, command {}", rec.cmd);
    }
    assert_eq!(
        log[5].counts(),
        [0xA5A5; 4],
        "the refused count wrote nothing"
    );
    assert!(log[6].untouched(), "the refused pass wrote nothing");
    assert_eq!(log[8].a, 2, "the OS has it in the archive");
    assert_eq!((log[9].a, log[9].b()), (ST_OK, WAS_ARCHIVED));
    assert_eq!(log[10].a, 0, "the OS has it in RAM");
    assert_eq!((log[11].b(), log[12].b(), log[13].word(0)), (2, 3, 99));
    assert_eq!(p.only_the_table("brought back", SIZE), layout::encode(&t));
    p.leave();
    p.finish("archived");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_archived_table_stays_archived_below_the_need_and_comes_back_at_it() {
    let (t, script) = written();
    let mut p = archived(&t, &script);
    p.launch(&[]);
    let free = free_ram(&mut p.os);
    p.leave();
    println!("free RAM with the table archived: {free}");
    let cmds = [Cmd::Open(CRC, COUNT), Cmd::Locate, Cmd::Get(7)];
    let log = inner(
        &p.launch(&with_free(free, NEED - 1, &cmds, NEED - 1)),
        "short",
    );
    assert_eq!((log[0].a, log[0].b()), (ST_ARCHIVED, 0));
    assert_eq!((log[1].a, log[2].a), (2, ST_ARCHIVED), "still archived");
    assert_eq!(table(&mut p.os), Some((layout::encode(&t), true)));
    p.leave();
    // The entry stayed in the symbol table, so the move takes the bytes and the size word.
    let left = NEED - SIZE - 2;
    let log = inner(&p.launch(&with_free(free, NEED, &cmds, left)), "enough");
    assert_eq!((log[0].a, log[0].b()), (ST_OK, WAS_ARCHIVED));
    assert_eq!((log[1].a, log[2].a, log[2].b()), (0, ST_OK, 2));
    assert_eq!(p.only_the_table("brought back", SIZE), layout::encode(&t));
    p.leave();
    p.finish("archived, scarce");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_archived_table_of_another_library_is_brought_back_and_replaced() {
    let mut t = Table::new(CRC ^ 5, COUNT);
    t.values[11] = 1;
    let mut p = archived(&t, &[Cmd::Open(CRC ^ 5, COUNT), Cmd::Set(11, 1)]);
    let log = p.launch(&[Cmd::Open(CRC, COUNT), Cmd::Get(11)]);
    assert_eq!(
        (log[0].a, log[0].b()),
        (ST_OK, WAS_ARCHIVED | ST_OTHER_LIBRARY)
    );
    assert_eq!((log[1].a, log[1].b()), (ST_OK, 0));
    let zeroed = layout::encode(&Table::new(CRC, COUNT));
    assert_eq!(p.only_the_table("replaced", SIZE), zeroed);
    p.leave();
    p.finish("archived, another library");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn free_ram_is_what_it_was_before_the_table_once_it_is_deleted() {
    let (mut p, free) = pair_and_free();
    p.launch(&[Cmd::Open(CRC, COUNT)]);
    assert_eq!(free_ram(&mut p.os), free - SIZE - 2 - ENTRY);
    p.leave();
    p.launch(&[Cmd::Drop]);
    assert_eq!(free_ram(&mut p.os), free);
    p.leave();
    p.finish("free RAM");
}
