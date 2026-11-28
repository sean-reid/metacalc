//! What store_least_not_one asks of the OS and of the table: one OS call and no more, and a table
//! read only for a number inside start..limit and below the best so far.

use super::least::{leak, least, table};
use super::rig::*;
use super::stubos::stub_label;

const ENTRY: &str = "STORE_LEAST_NOT_ONE";

/// (OS calls to _ChkFindSym, table reads, answer) of one call of the routine over `values`.
/// A table read is a run of store_locate, and each is followed by one of store_unpack.
fn asked(rig: &mut Rig, list: &[u16], start: u16, limit: u16) -> (usize, usize, Result<u16, u8>) {
    let list = leak(list);
    let cmds = [Cmd::LeastNotOne(start, limit, list)];
    let entry = label(ENTRY);
    let os = (0, 0x4000 + stub_label("CHK_FIND_SYM") as u16);
    let (calls, took) = rig.hits(&cmds, entry, os);
    let locate = rig.hits(&cmds, entry, (BASE, label("STORE_LOCATE"))).0;
    let unpack = rig.hits(&cmds, entry, (BASE, label("STORE_UNPACK"))).0;
    assert_eq!(locate, unpack, "a read locates and unpacks once");
    assert!(took > 1000, "{took} T-states");
    let log = rig.run(&cmds);
    let got = match log[0].a {
        ST_OK => Ok(log[0].word(0)),
        status => Err(status),
    };
    (calls, locate, got)
}

struct Case {
    what: &'static str,
    list: Vec<u16>,
    start: u16,
    limit: u16,
    reads: usize,
}

fn case(what: &'static str, list: Vec<u16>, start: u16, limit: u16, reads: usize) -> Case {
    Case {
        what,
        list,
        start,
        limit,
        reads,
    }
}

fn cases() -> Vec<Case> {
    let up = |from: u16, to: u16| (from..to).collect::<Vec<u16>>();
    let down = |from: u16, to: u16| (from..to).rev().collect::<Vec<u16>>();
    vec![
        case("an empty list", vec![], 0, 300, 0),
        case("every number below the start", up(10, 30), 100, 300, 0),
        case(
            "every number at or above the limit",
            up(200, 220),
            0,
            100,
            0,
        ),
        case("every number equal to the limit", vec![100; 20], 0, 100, 0),
        case(
            "a number twenty times, the least of it",
            vec![50; 20],
            0,
            300,
            1,
        ),
        case("a rising list", up(50, 70), 0, 300, 1),
        case("a falling list", down(50, 70), 0, 300, 20),
        case(
            "the best, then numbers above it",
            [vec![50], up(51, 70)].concat(),
            0,
            300,
            1,
        ),
        case(
            "numbers outside start..limit around one inside",
            vec![5, 299, 400, 150, 9, 99, 600, 150, 300],
            100,
            300,
            2,
        ),
        case("a rising list of 127", up(100, 227), 0, 300, 1),
        case("a falling list of 127", down(100, 227), 0, 300, 127),
    ]
}

#[test]
fn a_number_is_read_from_the_table_only_inside_start_and_limit_and_below_the_best_so_far() {
    let zeros = vec![0u8; 300];
    let mut rig = table(&zeros, 0xFF);
    let mut table_reads = 0;
    for c in cases() {
        let (calls, reads, got) = asked(&mut rig, &c.list, c.start, c.limit);
        println!("{}: {calls} OS call, {reads} reads, {got:?}", c.what);
        assert_eq!(reads, c.reads, "{}", c.what);
        assert_eq!(got, least(&zeros, &c.list, c.start, c.limit), "{}", c.what);
        assert_eq!(calls, 1, "{}", c.what);
        table_reads += reads;
    }
    assert_eq!(table_reads, 1 + 1 + 20 + 1 + 2 + 1 + 127);
}

#[test]
fn every_number_is_read_when_none_lowers_the_best() {
    let ones = vec![1u8; 300];
    let mut rig = table(&ones, 0xFF);
    for (what, list, reads) in [
        ("a rising list of ones", (50..70).collect::<Vec<u16>>(), 20),
        ("a falling list of ones", (50..70).rev().collect(), 20),
        ("a number twenty times", vec![50; 20], 20),
    ] {
        let (calls, got, answer) = asked(&mut rig, &list, 0, 300);
        assert_eq!((calls, got, answer), (1, reads, Ok(300)), "{what}");
    }
}

#[test]
fn one_os_call_is_made_whatever_the_table_the_list_or_the_limits_are() {
    let mut rig = Rig::new();
    let (calls, reads, got) = asked(&mut rig, &[1, 2, 3], 0, 10);
    assert_eq!((calls, reads, got), (1, 0, Err(ST_ABSENT)), "no table");
    let zeros = vec![0u8; 300];
    let mut rig = table(&zeros, 0xFF);
    for (what, list, start, limit, status) in [
        (
            "a limit above the count",
            vec![1u16, 2, 3],
            0u16,
            301u16,
            ST_RANGE,
        ),
        ("a start above the limit", vec![1, 2, 3], 5, 4, ST_RANGE),
    ] {
        let (calls, reads, got) = asked(&mut rig, &list, start, limit);
        assert_eq!((calls, reads, got), (1, 0, Err(status)), "{what}");
    }
    let (calls, reads, got) = asked(&mut rig, &(0..127).collect::<Vec<u16>>(), 0, 300);
    assert_eq!((calls, reads, got), (1, 1, Ok(0)), "a list of 127");
    let mut rig = table(&[], 0xFF);
    let (calls, reads, got) = asked(&mut rig, &[0, 1, 2], 0, 0);
    assert_eq!(
        (calls, reads, got),
        (1, 0, Ok(0)),
        "a table of no assertions"
    );
}
