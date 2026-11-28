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
fn over_stages(kind: Kind, table: bool, mask: bool, speed: u8, sweep: bool) -> BTreeMap<&'static str, u16> {
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
    assert!(wrong.is_empty(), "the depths differ from the table in {wrong:?}");
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
        (Kind::Axioms, _) => [20, 20, 20, 4, 4, 2, 4, 4, 4, 22, 22, 20],
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
