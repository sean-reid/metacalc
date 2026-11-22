//! store_open on the reference OS: creating, keeping and replacing the table, and the RAM it asks
//! for.

use crate::harness::*;
use crate::layout::{self, Table};
use crate::pair::Pair;
use crate::rig::*;

const SIZE: u16 = 842;
/// What store_open asks _EnoughMem for: the table and STORE_OVERHEAD.
pub const NEED: u16 = SIZE + 16;
/// What the OS takes for the table: its bytes, the size word and the 13-byte entry.
pub const TAKEN: u16 = SIZE + 2 + ENTRY;
/// What an AppVar with a one-letter name takes beyond its bytes: the size word and an 8-byte entry.
pub const FILLER_COST: u16 = 10;

fn fresh(crc: u32, count: u16) -> Vec<u8> {
    layout::encode(&Table::new(crc, count))
}

#[test]
#[ignore = "needs the reference ROM image"]
fn open_creates_a_zeroed_table_and_nothing_else() {
    let mut p = Pair::boot();
    let log = p.launch(&[Cmd::Locate, Cmd::Open(CRC, COUNT), Cmd::Locate]);
    assert_eq!(log[0].a, 1, "no table before the open");
    assert_eq!((log[1].a, log[1].b()), (ST_OK, ST_ABSENT));
    assert_eq!((log[2].a, log[2].word(2)), (0, SIZE));
    assert_eq!(p.only_the_table("created", SIZE), fresh(CRC, COUNT));
    p.leave();
    p.finish("create");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn opening_again_keeps_the_table_as_it_was() {
    let mut p = Pair::boot();
    p.launch(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Set(0, 1),
        Cmd::Set(1663, 2),
        Cmd::Set(3325, 3),
        Cmd::SetNext(1664),
    ]);
    let mut want = Table::new(CRC, COUNT);
    (want.values[0], want.values[1663], want.values[3325]) = (1, 2, 3);
    want.next = 1664;
    assert_eq!(p.only_the_table("written", SIZE), layout::encode(&want));
    p.leave();
    let log = p.launch(&[
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
    assert_eq!((log[5].a, log[5].word(0)), (ST_OK, 1664));
    assert_eq!(p.only_the_table("reopened", SIZE), layout::encode(&want));
    p.leave();
    p.finish("reopen");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn another_library_or_another_count_is_replaced_by_a_zeroed_table() {
    let mut p = Pair::boot();
    p.launch(&[Cmd::Open(CRC, COUNT), Cmd::Set(5, 1), Cmd::SetNext(9)]);
    p.leave();
    let log = p.launch(&[Cmd::Open(CRC ^ 1, COUNT), Cmd::Get(5), Cmd::GetNext]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_OTHER_LIBRARY), "CRC");
    assert_eq!((log[1].b(), log[2].word(0)), (0, 0));
    assert_eq!(p.only_the_table("another CRC", SIZE), fresh(CRC ^ 1, COUNT));
    p.leave();
    let log = p.launch(&[Cmd::Open(CRC ^ 1, 100), Cmd::Open(CRC ^ 1, 100)]);
    assert_eq!((log[0].a, log[0].b()), (ST_OK, ST_OTHER_LIBRARY), "count");
    assert_eq!((log[1].a, log[1].b()), (ST_OK, 0));
    assert_eq!(p.only_the_table("another count", 35), fresh(CRC ^ 1, 100));
    p.leave();
    p.finish("replace");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_damaged_header_is_replaced_by_a_zeroed_table() {
    // The byte to damage, its new value, the count to open with and what the store calls it. A
    // count of 104 in a 35-byte table is a size the header does not imply.
    let cases = [
        ("magic", 0, 0x4E, 100, ST_BAD_MAGIC),
        ("version", 1, 0x02, 100, ST_BAD_MAGIC),
        ("size", 6, 104, 104, ST_BAD_SIZE),
    ];
    for (what, at, byte, count, reason) in cases {
        let mut p = Pair::boot();
        p.launch(&[Cmd::Open(CRC, 100), Cmd::Set(3, 2)]);
        p.leave();
        poke_table(&mut p.os, at, &[byte]);
        let log = p.launch(&[Cmd::Get(3), Cmd::Open(CRC, count), Cmd::Get(3)]);
        assert_eq!(log[0].a, reason, "{what}: the launch kept the damage");
        assert_eq!((log[1].a, log[1].b()), (ST_OK, reason), "{what}");
        assert_eq!((log[2].a, log[2].b()), (ST_OK, 0), "{what}");
        let size = layout::size(count) as u16;
        assert_eq!(p.only_the_table(what, size), fresh(CRC, count), "{what}");
        p.leave();
        p.finish(what);
    }
}

/// A pair and the RAM free while the app waits with nothing made.
pub fn pair_and_free() -> (Pair, u16) {
    let mut p = Pair::boot();
    p.launch(&[]);
    let free = free_ram(&mut p.os);
    p.leave();
    (p, free)
}

/// A script that leaves exactly `left` bytes free with the filler F, shows that with two
/// _EnoughMem calls, runs `cmds`, shows that `after` bytes are free, and deletes F.
pub fn with_free(free: u16, left: u16, cmds: &[Cmd], after: u16) -> Vec<Cmd> {
    let mut script = vec![
        Cmd::Make('F', free - FILLER_COST - left),
        Cmd::Room(left),
        Cmd::Room(left + 1),
    ];
    script.extend(cmds);
    script.extend([Cmd::Room(after), Cmd::Room(after + 1), Cmd::Delete('F')]);
    script
}

/// The records of `cmds` in the log of a `with_free` script, after checking the script's own.
pub fn inner(log: &[Rec], what: &str) -> Vec<Rec> {
    let n = log.len();
    assert_eq!(log[0].a, 0, "{what}: the filler is made");
    assert_eq!((log[1].a, log[2].a), (0, 1), "{what}: free before");
    assert_eq!((log[n - 3].a, log[n - 2].a), (0, 1), "{what}: free after");
    assert_eq!(log[n - 1].a, 0, "{what}: the filler is deleted");
    log[3..n - 3].to_vec()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn open_needs_the_table_and_sixteen_bytes_free_and_refuses_cleanly_below() {
    for left in [6, NEED - 1] {
        let (mut p, free) = pair_and_free();
        let script = with_free(free, left, &[Cmd::Open(CRC, COUNT), Cmd::Locate], left);
        let log = inner(&p.launch(&script), "refused");
        assert_eq!((log[0].a, log[0].b()), (ST_NO_RAM, 0), "{left} bytes free");
        assert_eq!(log[1].a, 1, "{left} bytes free: nothing created");
        p.same_as_control("refused");
        p.leave();
        p.finish("no room");
    }
    let (mut p, free) = pair_and_free();
    println!("free RAM with no table: {free}; open asks for {NEED}, the OS takes {TAKEN}");
    let script = with_free(
        free,
        NEED,
        &[Cmd::Open(CRC, COUNT), Cmd::Locate],
        NEED - TAKEN,
    );
    let log = inner(&p.launch(&script), "created");
    assert_eq!(
        (log[0].a, log[0].b()),
        (ST_OK, ST_ABSENT),
        "{NEED} bytes free"
    );
    assert_eq!((log[1].a, log[1].word(2)), (0, SIZE));
    assert_eq!(p.only_the_table("just enough", SIZE), fresh(CRC, COUNT));
    p.leave();
    p.finish("just enough room");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_program_with_the_tables_name_is_not_the_table_and_is_left_alone() {
    let mut p = Pair::boot();
    p.both(write_a_program_with_the_tables_name);
    let program = |p: &mut Pair| {
        let e = find(&mut p.os, PROGRAM, VAR).expect("the program exists");
        let size = usize::from(word(&mut p.os, e.addr));
        ram(&mut p.os, e.addr, 2 + size)
    };
    let before = program(&mut p);
    assert_eq!(before, [1, 0, 0x31], "one token, a 1");
    let log = p.launch(&[Cmd::Locate, Cmd::Get(0), Cmd::Open(CRC, 100), Cmd::Get(0)]);
    assert_eq!(
        (log[0].a, log[1].a),
        (1, ST_ABSENT),
        "the program is not found"
    );
    assert_eq!((log[2].a, log[2].b()), (ST_OK, ST_ABSENT));
    assert_eq!(log[3].a, ST_OK);
    assert_eq!(p.only_the_table("beside a program", 35), fresh(CRC, 100));
    assert_eq!(program(&mut p), before);
    p.leave();
    p.finish("program");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_counts_change_nothing_but_their_buffers() {
    let mut p = Pair::boot();
    let log = p.launch(&[
        Cmd::Open(CRC, COUNT),
        Cmd::Set(700, 1),
        Cmd::Set(1000, 2),
        Cmd::Set(3325, 3),
        Cmd::Count,
        Cmd::CountTo(1001),
        Cmd::CountEnds(&[779, 1171, 3326]),
    ]);
    assert_eq!(log[4].counts(), [3323, 1, 1, 1]);
    assert_eq!(log[5].counts(), [999, 1, 1, 0]);
    assert_eq!(
        log[6].ends(),
        [[778, 1, 0, 0], [1169, 1, 1, 0], [3323, 1, 1, 1]]
    );
    let mut want = Table::new(CRC, COUNT);
    (want.values[700], want.values[1000], want.values[3325]) = (1, 2, 3);
    assert_eq!(p.only_the_table("counted", SIZE), layout::encode(&want));
    p.leave();
    p.finish("counts");
}
