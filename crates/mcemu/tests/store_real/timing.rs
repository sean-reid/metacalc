//! What the store's routines cost on the reference OS, beside the stub's figures. Each figure is
//! the window of `call_window`: the routine from its first instruction to its return.

use crate::harness::*;
use crate::rig::*;

const ENDS: &[u16] = &[779, 1171, 3326];

/// T-states of the store routine `entry` on the reference OS and on the stub, at its
/// `skip + 1`th call in `cmds`. Each runs on a machine booted for it: the OS defragments its
/// apps at the sixteenth launch after a boot.
fn windows(cmds: &[Cmd], entry: &str, skip: usize) -> (u32, u32) {
    let real = launch_window(&mut boot(), cmds, entry, skip);
    (real, Rig::new().window(cmds, label(entry), skip))
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_routines_cost_on_the_reference_os_stays_inside_its_budgets() {
    let open = Cmd::Open(CRC, COUNT);
    let one = [open, Cmd::Get(1663)];
    let rows = [
        (
            "store_open, created",
            windows(&[open], "STORE_OPEN", 0),
            66_000,
        ),
        (
            "store_open, found",
            windows(&[open, open], "STORE_OPEN", 1),
            7_100,
        ),
        (
            "store_data, the lookup",
            windows(&one, "STORE_DATA", 0),
            6_600,
        ),
        ("store_get", windows(&one, "STORE_GET", 0), 7_100),
        (
            "store_set",
            windows(&[open, Cmd::Set(1663, 2)], "STORE_SET", 0),
            7_300,
        ),
        (
            "store_count",
            windows(&[open, Cmd::Count], "STORE_COUNT", 0),
            208_000,
        ),
        (
            "store_count_ends, three ends",
            windows(&[open, Cmd::CountEnds(ENDS)], "STORE_COUNT_ENDS", 0),
            212_000,
        ),
    ];
    for (name, (real, stub), ceiling) in rows {
        println!("{name}: {real} T-states on OS 2.43, {stub} on the stub");
        assert!(real < ceiling, "{name}: {real}");
    }
    // store_data is the one lookup, and the OS's costs 2,341 more than the stub's: the real
    // _ChkFindSym and bcall against the stub's. Every other routine that finds the table is
    // dearer on the OS by exactly that, which shows each makes one lookup and nothing else that
    // differs between the two.
    let (data, others) = (rows[2], [rows[1], rows[3], rows[4], rows[5], rows[6]]);
    let lookup = data.1.0 - data.1.1;
    assert_eq!(lookup, 2_341, "the OS's lookup against the stub's");
    for (name, (real, stub), _) in others {
        assert_eq!(real - stub, lookup, "{name}: the OS costs one lookup more");
    }

    // The OS walks the named entries from the oldest, so a table created after other variables
    // is found later.
    let mut crowd: Vec<Cmd> = (b'A'..=b'T').map(|c| Cmd::Make(char::from(c), 1)).collect();
    crowd.extend([open, Cmd::Get(1663)]);
    let behind = launch_window(&mut boot(), &crowd, "STORE_GET", 0);
    println!("store_get with 20 AppVars created before the table: {behind} T-states");
    assert!(behind > rows[3].1.0 && behind < 12_800, "{behind}");
}
