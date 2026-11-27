//! The find when the status table is lost after the list was entered, and when a reader call is
//! refused.

use crate::built::Built;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use crate::table;
use mcui::{Key, Kind, Status};

const PATTERN: &str = "TATATAATTTATTAAT";

/// The assertions the table checks: all past the first eight, and every one of them a theorem or an
/// axiom the prefixes below reach.
const CHECKED: [usize; 5] = [8, 9, 11, 12, 14];

fn built() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| pattern_built("pages-find-lost", PATTERN))
}

fn marks(n: usize) -> Vec<Status> {
    (0..n)
        .map(|i| {
            if CHECKED.contains(&i) {
                Status::Checked
            } else {
                Status::Unchecked
            }
        })
        .collect()
}

fn lose(list: &mut Lister, how: &str, data: &[u8]) {
    list.run.m().delete_var("MCSTAT");
    match how {
        "deleted" => {}
        "magic" => {
            let mut bad = data.to_vec();
            bad[0] ^= 0xFF;
            table::install(list.run.m(), &bad);
        }
        "size" => table::install(list.run.m(), &data[..data.len() - 1]),
        other => panic!("{other}"),
    }
}

#[test]
fn a_letter_under_unchecked_after_the_table_was_lost_lands_as_it_does_with_no_table() {
    let built = built();
    let n = count(built);
    let status = marks(n);
    let data = table_of(built, &status);
    let labels = labels_of(built);
    let mut landed = 0;
    for how in ["deleted", "magic", "size"] {
        for &hidden in &CHECKED {
            let mut list = Lister::open(built, Some((&data, &status)), 0);
            list.settle();
            list.go_to_kind(Kind::Unchecked);
            assert_eq!(list.pos().len, n - CHECKED.len(), "{how}: the table hides");
            assert_eq!(list.var("LV_TABLE"), 1);
            lose(&mut list, how, &data);
            list.status = Vec::new();
            list.ui = list.replayed();
            list.ui.idle();
            let label = labels[hidden].clone();
            list.type_text(&label);
            let frame = list.frame_bytes();
            assert_eq!(
                u16::from_le_bytes([frame[2], frame[3]]),
                hidden as u16,
                "{how}: {label} is on the cursor, though the table hid it"
            );
            assert!(!list.find().missed);
            assert_eq!(
                list.var("LV_TABLE"),
                1,
                "{how}: the list never saw the loss"
            );
            landed += 1;
        }
    }
    assert_eq!(landed, 15);
}

/// A list of the pattern library under `kind` over a table that checks `CHECKED`, with the reader
/// calls `range` and `walk` counted by hooks that never refuse (call 99), then `refuse` made to fail.
fn counted(kind: Kind, table: bool, refuse: &str) -> Lister {
    let built = built();
    let status = marks(count(built));
    let data = table_of(built, &status);
    let mut list = Lister::open(built, table.then_some((&data[..], &status[..])), 0);
    list.settle();
    list.go_to_kind(kind);
    list.fail_at(0, refuse, 1, 3);
    let other = if refuse == "far_lib_find_range" {
        "far_lib_find_walk"
    } else {
        "far_lib_find_range"
    };
    list.fail_at(1, other, 99, 3);
    list
}

#[test]
fn a_refused_reader_call_is_a_miss_that_moves_nothing_and_later_letters_search_no_more() {
    let cases = [
        (Kind::All, false, "far_lib_find_range"),
        (Kind::All, false, "far_lib_find_walk"),
        (Kind::Theorems, false, "far_lib_find_walk"),
        (Kind::Axioms, false, "far_lib_find_walk"),
        (Kind::Unchecked, false, "far_lib_find_walk"),
        (Kind::Unchecked, true, "far_lib_find_copy"),
    ];
    for (kind, table, refuse) in cases {
        let mut list = counted(kind, table, refuse);
        let before = list.frame_bytes();
        let blits = list.run.blits();
        list.tap(scans_for('a')[0]);
        list.run.next_blit();
        list.run.run(300_000);
        let at = format!("{kind:?} table {table}, {refuse}");
        assert_eq!(list.run.blits(), blits + 1, "{at}: a frame for the title");
        let find = list.find();
        assert_eq!(
            (find.len, find.missed, find.chars.as_str()),
            (1, true, "a"),
            "{at}"
        );
        assert_eq!(list.frame_bytes(), before, "{at}: nothing moved");
        assert_eq!(
            list.fail_calls(0),
            1,
            "{at}: the refused call was made once"
        );
        let others = u16::from(refuse != "far_lib_find_range");
        assert_eq!(list.fail_calls(1), others, "{at}: the other call");
        list.tap(scans_for('x')[0]);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(list.find().len, 2);
        assert!(list.find().missed);
        assert_eq!(list.fail_calls(0), 1, "{at}: no search after a miss");
        assert_eq!(list.fail_calls(1), others, "{at}: nor the other call");
    }
}
