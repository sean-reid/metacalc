//! Every arm of list_key the find adds, traced at the entry and the return: the stack it writes,
//! interrupts and the speed at the exit, and what it answers; and a write watch over a letter press.

use crate::list_find_arms::*;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use mcui::Kind;
use std::collections::BTreeMap;

fn open_list(kind: Kind, table: bool, stage: usize) -> Lister {
    let marks = every_mark(proofs_three());
    list_under(proofs_three(), stage, kind, table.then_some(&marks))
}

/// The greatest of each arm over the proofs library's stages 1 and 2.
fn over_stages(
    kind: Kind,
    table: bool,
    mask: bool,
    speed: u8,
    sweep: bool,
) -> BTreeMap<&'static str, u16> {
    let mut greatest: BTreeMap<&'static str, u16> = BTreeMap::new();
    for stage in [1, 2] {
        let open = || open_list(kind, table, stage);
        for (name, depth) in depths(&open, sweep.then_some(&PLAN), mask, speed) {
            let at = greatest.entry(name).or_default();
            *at = (*at).max(depth);
        }
    }
    greatest
}

/// The sweeps over the stub's list: every move, and F3 from items spread to the last.
const PLAN: Plan = Plan {
    cap: usize::MAX,
    f3_items: &[0, 1, 2, 4, 7, 11, 16, 22, LAST],
};

const CASES: [(Kind, bool); 8] = [
    (Kind::All, false),
    (Kind::Theorems, false),
    (Kind::Axioms, false),
    (Kind::Unchecked, true),
    (Kind::Unchecked, false),
    (Kind::All, true),
    (Kind::Theorems, true),
    (Kind::Axioms, true),
];

#[test]
fn every_arm_keeps_interrupts_and_speed_and_writes_these_depths_under_each_kind() {
    let mut wrong = Vec::new();
    for (kind, table) in CASES {
        for speed in [0, 1] {
            let masked = over_stages(kind, table, true, speed, true);
            let live = over_stages(kind, table, false, speed, false);
            println!("{kind:?} table {table} speed {speed}: {masked:?}");
            for (name, depth) in &live {
                assert!(*depth >= masked[name], "{kind:?} {name}");
            }
            if masked != expected(kind, table) {
                wrong.push(format!("{kind:?} table {table}, speed {speed}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the depths differ from the table in {wrong:?}"
    );
}

#[test]
fn a_letter_on_a_library_without_the_section_keeps_interrupts_and_speed_and_writes_this_depth() {
    for speed in [0, 1] {
        for mask in [true, false] {
            let mut list = Lister::open(crate::list_find_keys::stripped_built(), None, 0);
            list.settle();
            let depth = no_section_depth(&mut list, mask, speed);
            println!("no section, masked {mask}, speed {speed}: {depth}");
            if mask {
                assert_eq!(depth, NO_SECTION, "speed {speed}");
            } else {
                assert!(depth >= NO_SECTION);
            }
        }
    }
}

/// What a letter writes with LV_HASO 0, the caller's return address counted.
const NO_SECTION: u16 = 2;

/// The stack each arm writes on the stub OS, counting the caller's return address.
fn expected(kind: Kind, table: bool) -> BTreeMap<&'static str, u16> {
    let row: [u16; 12] = match (kind, table) {
        (Kind::All, _) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 6, 6, 24],
        (Kind::Theorems, _) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 24],
        (Kind::Axioms, false) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 20],
        (Kind::Axioms, true) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 28],
        (Kind::Unchecked, true) => [26, 26, 26, 4, 4, 2, 4, 4, 4, 26, 26, 22],
        (Kind::Unchecked, false) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 10, 10, 20],
    };
    NAMES.iter().copied().zip(row).collect()
}

#[test]
fn a_letter_press_writes_only_the_frame_and_the_variables_the_header_names() {
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
    ] {
        for (typed, b) in [("", b's'), ("p", b'm'), ("p", b'z'), ("pz", b'x')] {
            let mut list = open_list(kind, table, 1);
            list.type_settled(typed);
            let may = letter_may_write(&mut list);
            let before = list.frame_now();
            list.run.shell.app.m.watch_writes(0x8000, 0x8000);
            let done = list.call_key(12, b);
            let writes = list.run.shell.app.m.take_writes();
            list.run.shell.app.m.unwatch_writes();
            assert_eq!(done.status(), 1, "{kind:?} {typed:?} {}", b as char);
            let outside: Vec<u16> = writes
                .iter()
                .map(|w| w.addr)
                .filter(|&a| !in_ranges(&may, a))
                .collect();
            assert!(
                outside.is_empty(),
                "{kind:?} {typed:?} {}: wrote outside the header's names: {outside:04X?}",
                b as char
            );
            let frame = list.frame_now();
            assert_eq!(frame[..2], before[..2], "the kind of screen and the stage");
            assert_eq!(frame[6], before[6], "the kind");
            assert_eq!(
                frame[7] & 0x0F,
                (before[7] & 0x0F) + 1,
                "one character more"
            );
            assert!(
                writes.iter().any(|w| w.addr == list.frame_addr() + 7),
                "byte 7 was written"
            );
        }
    }
}

/// Every byte of the app's regions, from the build's label file, but the driver's own RAM, read through
/// the machine.
fn snapshot(list: &mut Lister) -> Vec<(u16, u8)> {
    let own = list.run.shell.app.scratch.clone();
    let regions = crate::list_kind_stays::app_regions(list.built);
    let mut bytes = Vec::new();
    for (_, at, size) in regions {
        let data = list.run.peek(at, usize::from(size));
        for (i, b) in data.into_iter().enumerate() {
            let addr = at + i as u16;
            if !own.iter().any(|&(s, e)| (s..e).contains(&addr)) {
                bytes.push((addr, b));
            }
        }
    }
    bytes
}

fn changed(before: &[(u16, u8)], after: &[(u16, u8)]) -> Vec<u16> {
    assert_eq!(before.len(), after.len());
    before
        .iter()
        .zip(after)
        .filter(|(a, b)| a != b)
        .map(|(a, _)| a.0)
        .collect()
}

/// F2 and CLEAR with no find, a ninth letter and a refused character write nothing at all, and a letter
/// after a miss writes byte 7 and the next character of the frame: every byte of the app's regions is
/// compared, not the status.
#[test]
fn what_adds_nothing_changes_no_byte_and_a_letter_after_a_miss_changes_two() {
    for (kind, table) in CASES {
        let at = format!("{kind:?} table {table}");
        let nothing: [(&str, &str, u8, u8, u8); 4] = [
            ("F2 with no find", "", 8, 0, 0),
            ("CLEAR with no find", "", 6, 0, 3),
            ("a ninth letter", "pm2.61d1", 12, b'x', 0),
            ("a refused character", "p", 12, b'/', 0),
        ];
        for (what, typed, key, b, answer) in nothing {
            let mut list = open_list(kind, table, 1);
            list.type_settled(typed);
            let before = snapshot(&mut list);
            let done = list.call_key(key, b);
            let after = snapshot(&mut list);
            assert_eq!(done.status(), answer, "{at}, {what}: the answer");
            let moved = changed(&before, &after);
            assert!(moved.is_empty(), "{at}, {what}: wrote {moved:04X?}");
        }
        let mut list = open_list(kind, table, 1);
        list.type_settled("q");
        assert!(list.find().missed, "{at}: q misses");
        let frame = list.frame_addr();
        let before = snapshot(&mut list);
        let done = list.call_key(12, b'e');
        let after = snapshot(&mut list);
        assert_eq!(done.status(), 1, "{at}: a letter after a miss redraws");
        assert_eq!(
            changed(&before, &after),
            [frame + 7, frame + 9],
            "{at}: byte 7 and the next character"
        );
    }
}
