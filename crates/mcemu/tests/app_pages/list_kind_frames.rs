//! The list's filter by kind on the stub OS against the host model: F3 round the four kinds from
//! the first, the middle and the last row, the walks over what each kind shows, and a filter with one,
//! five, six and no match.

use crate::app::*;
use crate::builds::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Key, Kind, Status};

#[derive(Default, Debug)]
struct Seen {
    rounds: usize,
    kept: usize,
    blanked: usize,
    empty: usize,
    with_table: usize,
    starts: [usize; 3],
}

/// F3 four times round from `place`: 0 the first row, 1 the middle, 2 the last.
fn round(
    built: &'static Built,
    table: Option<(&[u8], &[Status])>,
    stage: usize,
    place: usize,
    seen: &mut Seen,
) {
    let mut list = Lister::open(built, table, stage);
    list.settle();
    let len = list.pos().len;
    let rows = match place {
        0 => 0,
        1 => (len / 10).max(1),
        _ => len / 5 + 2,
    };
    for _ in 0..rows {
        list.key(RIGHT, Key::Right);
        list.settle();
    }
    let cursor = list.pos().cursor;
    match place {
        0 => assert_eq!(cursor, 0, "the round starts from the first row"),
        1 => assert!(
            len < 12 || (cursor > 0 && cursor + 1 < len),
            "the round starts from a middle row: {cursor} of {len}"
        ),
        _ => assert_eq!(cursor + 1, len, "the round starts from the last row"),
    }
    seen.starts[place] += 1;
    seen.with_table += usize::from(table.is_some());
    let at = format!(
        "{} stages, stage {stage}, place {place}",
        built.stages.len()
    );
    for n in 0..4 {
        let step = format!("{at}, F3 {n}");
        list.key(F3, Key::F3);
        list.same_kind_position(&step);
        assert_eq!(list.kind_byte(), code(list.kind()), "{step}");
        if list.seen.0 == PENDING {
            seen.blanked += 1;
        } else {
            seen.kept += 1;
        }
        seen.empty += usize::from(list.pos().len == 0);
        list.settle();
        seen.rounds += 1;
    }
    assert_eq!(list.kind(), Kind::All, "{at}: four presses come round");
}

#[test]
fn f3_four_times_round_from_the_first_the_middle_and_the_last_row_of_every_stage() {
    let mut seen = Seen::default();
    let proofs = proofs_three();
    for built in [with_library(), three_stages(), proofs] {
        let mut seed = 0x2468_ACE1;
        let status = crate::table::random_status(count(built), &mut seed, 50, 4);
        let data = table_of(built, &status);
        for with_table in [false, true] {
            let table = with_table.then_some((&data[..], &status[..]));
            for stage in 0..built.stages.len().max(1) {
                for place in 0..3 {
                    round(built, table, stage, place, &mut seen);
                }
            }
        }
    }
    println!("{seen:?}");
    assert!(seen.rounds > 160, "{seen:?}");
    assert!(seen.kept > 70, "{seen:?}");
    assert!(seen.blanked > 85, "{seen:?}");
    assert!(seen.empty > 20, "{seen:?}");
    assert_eq!(seen.starts.iter().sum::<usize>() * 4, seen.rounds);
    assert_eq!(seen.with_table * 2, seen.starts.iter().sum::<usize>());
    assert_eq!(seen.starts, [14, 14, 14]);
}

#[test]
fn every_kind_is_walked_by_row_and_by_page_over_everything_it_shows() {
    let proofs = proofs_three();
    let (data, status) = every_mark(proofs);
    let mut shown = std::collections::BTreeMap::new();
    for (built, table) in [
        (proofs, Some((&data[..], &status[..]))),
        (proofs, None),
        (three_stages(), None),
    ] {
        for stage in 0..built.stages.len() {
            for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
                let mut list = Lister::open(built, table, stage);
                list.settle();
                list.go_to_kind(kind);
                let at = format!("stage {stage}, {kind:?}, table {}", table.is_some());
                let len = list.walk_kind(&at);
                *shown.entry(code(kind)).or_insert(0) += len;
            }
        }
    }
    println!("assertions walked by kind: {shown:?}");
    assert!(shown.values().all(|&n| n > 20), "{shown:?}");
}

#[test]
fn a_filter_with_one_five_six_or_no_match_is_the_models() {
    let mut counts = Vec::new();
    for (name, pattern) in [
        ("pages-kinds-a", "TATATATATAT"),
        ("pages-kinds-b", "AAATAAA"),
        ("pages-kinds-c", "TTATTT"),
    ] {
        let built = pattern_built(name, pattern);
        let theorems = pattern.matches('T').count();
        let axioms = pattern.matches('A').count();
        for (kind, want) in [(Kind::Theorems, theorems), (Kind::Axioms, axioms)] {
            let mut list = Lister::open(built, None, 0);
            list.settle();
            list.go_to_kind(kind);
            assert_eq!(
                list.pos().len,
                want,
                "{pattern} {kind:?}: the model's count"
            );
            assert_eq!(list.walk_kind(&format!("{pattern} {kind:?}")), want);
            counts.push(want);
        }
    }
    counts.sort_unstable();
    assert_eq!(counts, [1, 1, 5, 5, 6, 6]);
}

#[test]
fn unchecked_with_one_five_or_six_marks_not_checked_is_the_models() {
    let built = with_library();
    let n = count(built);
    for k in [1usize, 5, 6] {
        let mut status = vec![Status::Checked; n];
        for i in 0..k {
            status[(i * n) / k + 1] = Status::Unchecked;
        }
        let data = table_of(built, &status);
        let mut list = Lister::open(built, Some((&data, &status)), 0);
        list.settle();
        list.go_to_kind(Kind::Unchecked);
        assert_eq!(list.pos().len, k, "the model's count");
        assert_eq!(list.walk_kind(&format!("{k} unchecked")), k);
    }
}

#[test]
fn a_filter_that_shows_nothing_says_none_and_its_keys_do_what_an_empty_stages_do() {
    let built = pattern_built("pages-kinds-none", "AAAAAAA");
    let mut list = Lister::open(built, None, 0);
    list.settle();
    list.go_to_kind(Kind::Theorems);
    assert_eq!(list.pos().len, 0);
    assert_eq!(list.walk_kind("no theorem"), 0);
    for scan in [ENTER, F1, F4] {
        let mut list = Lister::open(built, None, 0);
        list.settle();
        list.go_to_kind(Kind::Theorems);
        list.tap(scan);
        list.run.next_blit();
        list.run.run(300_000);
        assert_eq!(list.run.top_kind(), KIND_NOTICE, "{scan:02X}: the notice");
    }
}
