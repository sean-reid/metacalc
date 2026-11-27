//! Every arm of list_key the find adds, traced at the entry and the return: the stack it writes,
//! interrupts and the speed at the exit, and what it answers; and a write watch over a letter press.

use crate::list_depth::kept;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Kind;
use std::collections::BTreeMap;

/// One arm: the text typed first, the key pressed under trace, and what list_key answers.
struct Arm {
    name: &'static str,
    typed: &'static str,
    key: u8,
    answer: u8,
}

const ARMS: usize = 11;

fn arms() -> [Arm; ARMS] {
    [
        Arm { name: "letter, first", typed: "", key: 0x2B, answer: 1 },
        Arm { name: "letter, later", typed: "p", key: 0x0D, answer: 1 },
        Arm { name: "letter, miss", typed: "", key: 0x1A, answer: 1 },
        Arm { name: "letter after a miss", typed: "zq", key: 0x2F, answer: 1 },
        Arm { name: "ninth letter", typed: "pm2.61d1", key: 0x2F, answer: 0 },
        Arm { name: "f2, prefix", typed: "p", key: F2, answer: 1 },
        Arm { name: "f2, none", typed: "", key: F2, answer: 0 },
        Arm { name: "clear, prefix", typed: "p", key: CLEAR, answer: 1 },
        Arm { name: "arrow, prefix", typed: "p", key: DOWN, answer: 1 },
        Arm { name: "right, prefix", typed: "p", key: RIGHT, answer: 1 },
        Arm { name: "f3, prefix", typed: "p", key: F3, answer: 1 },
    ]
}

fn open_list(kind: Kind, table: bool) -> Lister {
    let marks = every_mark(proofs_three());
    list_under(proofs_three(), 1, kind, table.then_some(&marks))
}

/// The depth of each arm under `kind`, with interrupts masked for the call or as the app runs.
pub fn depths(kind: Kind, table: bool, mask: bool, speed: u8) -> BTreeMap<&'static str, u16> {
    let mut found = BTreeMap::new();
    for mut arm in arms() {
        if arm.name == "letter, later" && kind == Kind::Axioms {
            arm.typed = "a";
            arm.key = 0x2A;
        }
        let mut list = open_list(kind, table);
        list.type_settled(arm.typed);
        list.run.m().press_key(arm.key);
        let t = list.trace("list_key", mask, speed);
        list.run.m().release_key(arm.key);
        if arm.name == "letter, later" {
            let find = list.frame_now()[7];
            assert_eq!(find & 0x80, 0, "{kind:?}: the second letter is a hit");
            assert_eq!(find & 0x0F, 2);
        }
        assert_eq!(t.action, arm.answer, "{kind:?} {}: the answer", arm.name);
        let depth = kept(&format!("{kind:?} {}", arm.name), &[t], mask, false, speed);
        found.insert(arm.name, depth);
    }
    assert_eq!(found.len(), ARMS);
    found
}

#[test]
fn every_arm_keeps_interrupts_and_speed_and_writes_these_depths_under_each_kind() {
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
    ] {
        for speed in [0, 1] {
            let masked = depths(kind, table, true, speed);
            let live = depths(kind, table, false, speed);
            println!("{kind:?} table {table} speed {speed}: {masked:?}");
            for (name, depth) in &masked {
                assert!(live[name] >= *depth, "{kind:?} {name}");
            }
            assert_eq!(
                &masked,
                &expected(kind, table),
                "{kind:?} table {table}, speed {speed}"
            );
        }
    }
}

/// The stack each arm writes on the stub OS, counting the caller's return address.
fn expected(kind: Kind, table: bool) -> BTreeMap<&'static str, u16> {
    let (letter_first, letter_later, miss, after, ninth, f2p, f2n, clr, arrow, right, f3) =
        match (kind, table) {
            (Kind::All, _) => (20, 20, 20, 4, 4, 4, 4, 4, 6, 6, 24),
            (Kind::Theorems, _) => (20, 20, 20, 4, 4, 4, 4, 4, 18, 18, 24),
            (Kind::Axioms, _) => (20, 22, 20, 4, 4, 4, 4, 4, 18, 18, 20),
            (Kind::Unchecked, true) => (26, 26, 22, 4, 4, 4, 4, 4, 24, 24, 22),
            (Kind::Unchecked, false) => (20, 20, 20, 4, 4, 4, 4, 4, 6, 6, 20),
        };
    BTreeMap::from([
        ("letter, first", letter_first),
        ("letter, later", letter_later),
        ("letter, miss", miss),
        ("letter after a miss", after),
        ("ninth letter", ninth),
        ("f2, prefix", f2p),
        ("f2, none", f2n),
        ("clear, prefix", clr),
        ("arrow, prefix", arrow),
        ("right, prefix", right),
        ("f3, prefix", f3),
    ])
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
            let mut list = open_list(kind, table);
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
            assert_eq!(frame[7] & 0x0F, (before[7] & 0x0F) + 1, "one character more");
            assert!(
                writes.iter().any(|w| w.addr == list.frame_addr() + 7),
                "byte 7 was written"
            );
        }
    }
}
