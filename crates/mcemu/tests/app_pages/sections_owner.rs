//! On the reference OS with the shipped libraries, by real key presses, what the sections screen is for:
//! F5 on each stage's list from its top, middle and end, the screen walked by row and by page to both ends,
//! Go on several sections under All and under Axioms, CLEAR back, and F5 again after a jump, with every
//! frame the model's; and a jump from a spread of each stage's titles under every kind, with and without a
//! status table, counted by where the model says each one lands.

use crate::list_rig::*;
use crate::sections_depth::{ROM, Table};
use crate::sections_jump::Landing;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::lib_of;
use mcui::{Key, Kind};
use std::collections::BTreeMap;

/// The list's cursor moved by one key and its preview settled.
fn list_key(list: &mut Lister, scan: u8, key: Key) -> bool {
    let drew = list.key(scan, key);
    if drew {
        list.skip_the_wait();
        list.settle();
    }
    drew
}

/// Pages down while the cursor is more than a page short of `target` assertions into the list.
fn page_to(list: &mut Lister, target: usize) {
    while list.pos().cursor + 6 <= target {
        assert!(list_key(list, RIGHT, Key::Right), "a page down");
    }
}

/// The sections screen walked by row to both ends and then by page to both ends, one key past each end
/// staying, every frame compared. Returns the keys that moved.
fn walk(list: &mut Lister, stage: usize) -> usize {
    let len = list.sections().len;
    assert!(len > 6, "stage {stage} has {len} titles");
    let mut moves = 0;
    for (scan, key, stop) in [(DOWN, Key::Down, len - 1), (UP, Key::Up, 0)] {
        while list.sections().cursor != stop {
            assert!(list.sections_key(scan, key));
            moves += 1;
        }
        assert!(!list.sections_key(scan, key), "stage {stage}: past the end");
    }
    for (scan, key, stop) in [(RIGHT, Key::Right, len - 1), (LEFT, Key::Left, 0)] {
        while list.sections().cursor != stop {
            assert!(list.sections_key(scan, key));
            moves += 1;
        }
        assert!(!list.sections_key(scan, key), "stage {stage}: past the end");
    }
    moves
}

/// The session's keys of one stage: the moves that drew and the jumps, and the moves the model's positions
/// give, which are the walk's rows down from where F5 opened and up, and its pages both ways.
fn owner_session(stage: usize) -> (usize, usize, usize) {
    let mut list = Lister::on_rom(stage);
    list.open_on_rom();
    let at = format!("stage {stage}");
    let (mut moves, mut jumps, mut want) = (0, 0, 0);
    let items = list.pos().len;
    for place in ["top", "middle", "end"] {
        match place {
            "middle" => page_to(&mut list, items / 2),
            "end" => {
                while list_key(&mut list, RIGHT, Key::Right) {}
                assert_eq!(list.pos().cursor, items - 1, "{at}: the last assertion");
            }
            _ => assert_eq!(list.pos().cursor, 0),
        }
        let before = list.list_frame();
        let assertion = list.pos().cursor;
        let section = list.open_sections();
        println!(
            "{at}, F5 from the {place} (assertion {assertion}): title {}, first title {}",
            section.cursor, section.top
        );
        want += (section.len - 1 - section.cursor)
            + (section.len - 1)
            + 2 * (section.len - 1).div_ceil(6);
        moves += walk(&mut list, stage);
        list.leave(CLEAR, Key::Clear);
        assert_eq!(list.list_frame(), before, "{at}: CLEAR gives the list back");
    }
    for kind in [Kind::All, Kind::Axioms] {
        list.go_to_kind(kind);
        let items = list.items();
        assert!(items.len() > 6, "{at}: {kind:?} shows {} rows", items.len());
        for (n, t) in [0, 5, 31, 7, 100, 2, 60, 166, 13].into_iter().enumerate() {
            let before = list.list_frame();
            let len = list.open_sections().len;
            let t = t.min(len - 1);
            list.goto_title(t);
            if n % 4 == 3 {
                list.leave(CLEAR, Key::Clear);
                assert_eq!(list.list_frame(), before, "{at}: CLEAR after moving");
                continue;
            }
            let (scan, key) = if n % 2 == 0 {
                (ENTER, Key::Enter)
            } else {
                (F1, Key::F1)
            };
            list.leave(scan, key);
            list.same_kind_position(&format!("{at}, {kind:?}, title {t}"));
            jumps += 1;
        }
    }
    (moves, jumps, want)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_owners_session_has_the_models_frame_at_every_key() {
    let mut total = (0, 0);
    for stage in 0..3 {
        let (moves, jumps, want) = owner_session(stage);
        println!("stage {stage}: {moves} moves on the sections screen, {jumps} jumps");
        assert_eq!(
            moves, want,
            "stage {stage}: the moves the model's positions give"
        );
        assert_eq!(
            jumps, JUMPS_A_STAGE,
            "stage {stage}: nine titles under two kinds, two of the nine left by CLEAR"
        );
        total.0 += moves;
        total.1 += jumps;
    }
    println!("owner session: {} moves, {} jumps", total.0, total.1);
    assert_eq!(total.1, 3 * JUMPS_A_STAGE);
}

/// The jumps of one stage's session: nine titles under each of All and Axioms, of which the fourth and
/// the eighth are left by CLEAR.
const JUMPS_A_STAGE: usize = 2 * (9 - 2);

/// The titles of `stage` a jump is tried from: every `stride`th, the first and the last, and the first
/// title of each landing the model gives that the stride missed.
fn spread(landings: &[Landing], stride: usize) -> Vec<usize> {
    let mut seen: Vec<usize> = (0..landings.len()).step_by(stride).collect();
    seen.push(landings.len() - 1);
    for name in ["direct", "next", "last", "empty"] {
        let first = landings.iter().position(|l| l.name() == name);
        if let Some(t) = first.filter(|_| !seen.iter().any(|&s| landings[s].name() == name)) {
            seen.push(t);
        }
    }
    seen.sort_unstable();
    seen.dedup();
    seen
}

/// The kinds and marks of a jump session: each kind without a table, and with the marks of the
/// depth tests, which hide some assertions from Unchecked; Unchecked with every assertion checked.
const COMBOS: [(Kind, Table); 8] = [
    (Kind::All, Table::None),
    (Kind::Theorems, Table::None),
    (Kind::Axioms, Table::None),
    (Kind::All, Table::Marks),
    (Kind::Theorems, Table::Marks),
    (Kind::Axioms, Table::Marks),
    (Kind::Unchecked, Table::Marks),
    (Kind::Unchecked, Table::AllChecked),
];

/// The model's landing of every title of `stage` under `kind` and `table`, from a list opened there.
fn landings_of(kind: Kind, table: Table, stage: usize) -> (Lister, Vec<Landing>) {
    let list = (ROM.list_at)(kind, table, stage);
    let titles = lib_of(list.built).titles()[..title_counts(list.built)[stage]].to_vec();
    let items = list.items();
    let landings = titles
        .iter()
        .map(|t| Landing::of(&items, t.assertion))
        .collect();
    (list, landings)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_jump_from_a_spread_of_each_stages_titles_is_the_models_under_every_kind() {
    let mut census: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut every: BTreeMap<(usize, &'static str), usize> = BTreeMap::new();
    for stage in [0, 1, 2] {
        for (kind, table) in COMBOS {
            let (mut list, landings) = landings_of(kind, table, stage);
            for l in &landings {
                *every.entry((stage, l.name())).or_insert(0) += 1;
            }
            let stride = [6, 13, 11][stage];
            for (n, t) in spread(&landings, stride).into_iter().enumerate() {
                let at = format!("stage {stage}, {kind:?}, {table:?}, title {t}");
                let items = list.items();
                let len = list.open_sections().len;
                assert_eq!(len, landings.len(), "{at}: the count");
                list.goto_title(t);
                let landing = landings[t];
                *census.entry(landing.name()).or_insert(0) += 1;
                let (scan, key) = if n % 2 == 0 {
                    (ENTER, Key::Enter)
                } else {
                    (F1, Key::F1)
                };
                list.leave(scan, key);
                list.same_kind_position(&at);
                let frame = list.list_frame();
                let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
                if items.is_empty() {
                    assert_eq!(landing, Landing::Empty, "{at}");
                }
                assert_eq!(cursor, landing.cursor(), "{at}: {landing:?}");
                assert_eq!(frame[7], 0, "{at}: the find");
            }
        }
    }
    println!("landings of the jumps: {census:?}");
    println!("landings of every title of every stage under the eight lists: {every:?}");
    let last = |stage: usize| every.get(&(stage, "last")).copied().unwrap_or(0);
    println!(
        "titles that land on the last shown assertion: prop {}, pred {}, ring {}",
        last(0),
        last(1),
        last(2)
    );
    assert!(
        last(0) > 0,
        "prop has titles after its last shown assertion"
    );
    assert_eq!(last(2), 0, "ring has none, under any kind");
    for name in ["direct", "next", "last", "empty"] {
        assert!(census.get(name).copied().unwrap_or(0) > 0, "a {name} jump");
    }
}
