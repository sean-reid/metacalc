//! A key that moves nothing, an auto power down and the writes below RAM, under each kind.

use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};
use std::collections::BTreeSet;

const HEADER: &str = include_str!("../../../../app/src/screen_list.inc");

fn open_kind(kind: Kind, table: bool) -> Lister {
    let built = proofs_three();
    let (data, status) = every_mark(built);
    let mut list = Lister::open(built, table.then_some((&data[..], &status[..])), 1);
    list.settle();
    list.go_to_kind(kind);
    list
}

#[test]
fn a_key_that_moves_nothing_writes_the_scratch_the_header_names_and_nothing_else() {
    let mut seen: Vec<(Kind, bool, BTreeSet<&str>)> = Vec::new();
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, false),
        (Kind::Unchecked, true),
    ] {
        let mut list = open_kind(kind, table);
        let (len, written) = list.walk_stays(&format!("{kind:?} table {table}"), false);
        println!("{kind:?}, table {table}, {len} shown: a stay wrote {written:?}");
        seen.push((kind, table, written));
    }
    for name in STAY_WRITES {
        assert!(HEADER.contains(name), "the header names {name}");
    }
    let core = [
        "LK_CUR", "LK_FR", "LK_KEY", "LK_N", "LK_X", "LV_END", "LV_KIND",
    ];
    for (kind, table, written) in &seen {
        let want: BTreeSet<&str> = match (kind, table) {
            (Kind::All, _) => BTreeSet::new(),
            (Kind::Unchecked, true) => core.iter().copied().chain(["LK_AGAIN"]).collect(),
            _ => core.iter().copied().collect(),
        };
        assert_eq!(written, &want, "{kind:?}, table {table}");
    }
}

/// A list under `kind` after a power down and a key it ignores: the refresh is what the model draws
/// with the preview pending, the kind byte, the cursor and the top are as they were.
fn powered_down(kind: Kind, settled: bool) {
    println!("power down under {kind:?}, settled {settled}");
    let mut list = open_kind(kind, true);
    let moved = list.key(RIGHT, Key::Right) || list.key(LEFT, Key::Left);
    assert!(moved, "{kind:?}: a key that moves the cursor");
    if settled {
        list.settle();
        assert_eq!(list.var("LV_PREVIEW"), ROW);
    } else {
        assert_eq!(list.seen.0, PENDING);
        list.run.run(100_000);
        let (preview, count) = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
        assert_eq!(preview, PENDING, "the key left the preview pending");
        assert!((1..11).contains(&count), "the wait is part way: {count}");
    }
    let at = format!("{kind:?}, settled {settled}");
    let before = list.run.frames()[1].clone();
    assert_eq!(before[6], code(kind), "{at}");
    list.run.shell.power_down();
    let blits = list.run.blits();
    list.tap(LETTER);
    let frame = list.run.next_blit();
    list.seen = (list.var("LV_PREVIEW"), list.var("LV_COUNT"));
    list.run.run(2_000);
    assert_eq!(list.run.blits(), blits + 1, "{at}: one frame");
    assert_eq!(list.seen, (PENDING, 11), "{at}: pending, the wait begins");
    let pending = list.replayed();
    same_frame(&at, &frame, &crate::shellrig::model::frame(&pending));
    list.ui = pending;
    list.same_kind_position(&at);
    assert_eq!(
        list.run.frames()[1][..7],
        before[..7],
        "{at}: byte 6, cursor and top"
    );
    assert_eq!(list.var("LV_TABLE"), 1, "{at}: derived again");
    assert!(list.settle(), "{at}: the preview is laid out again");
}

#[test]
fn a_power_down_under_a_filter_keeps_the_kind_the_cursor_and_the_top() {
    for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        for settled in [true, false] {
            powered_down(kind, settled);
        }
    }
}

/// A watch over every address below the first RAM address records nothing across everything the
/// list does under each kind, from the watch's arming on the Libraries screen: `open` enters the list
/// and goes to its kind.
pub fn nothing_below_ram(list: &mut Lister, at: &str, walk: bool, open: impl FnOnce(&mut Lister)) {
    list.run.m().watch_writes(0, 0x8000);
    let quiet = |list: &mut Lister, what: &str| {
        let w = list.run.m().take_writes();
        assert!(
            w.is_empty(),
            "{at}, {what}: {} writes below RAM, the first {:?}",
            w.len(),
            w.first().map(|w| (w.addr, w.value, w.pc))
        );
    };
    open(list);
    quiet(list, "the first enter, its draw and the kinds");
    if walk {
        let len = list.pos().len;
        let plan = [
            (DOWN, Key::Down, len + 1),
            (UP, Key::Up, len + 1),
            (RIGHT, Key::Right, len / 5 + 2),
            (LEFT, Key::Left, len / 5 + 2),
        ];
        for (scan, key, times) in plan {
            for _ in 0..times {
                list.key(scan, key);
                list.skip_the_wait();
                list.settle();
                quiet(list, "the walk");
            }
        }
    }
    for scan_key in [
        (DOWN, Key::Down),
        (RIGHT, Key::Right),
        (UP, Key::Up),
        (LEFT, Key::Left),
        (UP, Key::Up),
        (UP, Key::Up),
    ] {
        list.key(scan_key.0, scan_key.1);
        list.skip_the_wait();
        list.settle();
        quiet(list, "an arrow and the idle calls");
    }
    for scan in [F2, F4, F5, LETTER] {
        let notice =
            matches!(scan, F2 | F4) || (scan == F5 && list.var("LV_SECT") == 1);
        list.tap(scan);
        list.run.run(600_000);
        if notice {
            assert_eq!(list.run.top_kind(), KIND_NOTICE, "{at}: {scan:02X}");
            list.tap(CLEAR);
            list.run.run(600_000);
            list.ui = list.replayed();
            list.skip_the_wait();
            list.settle();
        }
        quiet(list, "a notice and list_enter");
    }
    for _ in 0..4 {
        list.key(F3, Key::F3);
        list.skip_the_wait();
        list.settle();
        list.run.run(600_000);
        quiet(list, "F3");
    }
    list.run.m().unwatch_writes();
}

#[test]
fn nothing_is_written_below_ram_under_any_kind() {
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
    ] {
        let built = proofs_three();
        let (data, status) = every_mark(built);
        let mut list = Lister::at_stage(built, table.then_some((&data[..], &status[..])), 1);
        nothing_below_ram(&mut list, &format!("{kind:?}"), true, |list| {
            list.key(ENTER, Key::Enter);
            list.settle();
            list.go_to_kind(kind);
        });
    }
}

#[test]
fn the_watch_over_the_rom_records_a_write_there() {
    let mut m = mcemu::Machine::new(mcemu::STUB_ROM).expect("the stub boots");
    m.watch_writes(0, 0x8000);
    m.load_ram(0x9000, &[0xF3, 0x3E, 0x5A, 0x32, 0x34, 0x72, 0x76]);
    let run = m.run(0x9000, 50_000);
    assert_eq!(run.stop, mcemu::Stop::Halted);
    let w: Vec<(u16, u8)> = m.take_writes().iter().map(|w| (w.addr, w.value)).collect();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].0, 0x7234);
}
