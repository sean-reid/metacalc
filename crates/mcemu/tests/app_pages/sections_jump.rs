//! The list's jump to a section: from each title, under each kind, with and without a status table,
//! the list's frame is the model's after its jump, on the assertion the rule gives; and what the jump
//! does to byte 7 and bytes 8 to 15 of the list's frame, to the preview, and to the theorem screen
//! opened next.

use crate::app::Built;
use crate::list_kind_rig::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use mcui::{Key, Kind, Status};
use std::collections::BTreeMap;

/// Where a jump lands, by the rule: the first assertion the kind shows at or after the title's, the
/// last one it shows when none follows, and nowhere when it shows none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Landing {
    Direct(usize),
    Next(usize),
    Last(usize),
    Empty,
}

impl Landing {
    pub fn of(items: &[usize], at: usize) -> Landing {
        match items.iter().position(|&i| i >= at) {
            _ if items.is_empty() => Landing::Empty,
            Some(p) if items[p] == at => Landing::Direct(at),
            Some(p) => Landing::Next(items[p]),
            None => Landing::Last(*items.last().unwrap()),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Landing::Direct(_) => "direct",
            Landing::Next(_) => "next",
            Landing::Last(_) => "last",
            Landing::Empty => "empty",
        }
    }

    /// The assertion the cursor is on afterwards, 0 for a list that shows nothing.
    pub fn cursor(self) -> usize {
        match self {
            Landing::Direct(a) | Landing::Next(a) | Landing::Last(a) => a,
            Landing::Empty => 0,
        }
    }
}

/// The marks the tests with a table use: assertions 0 to 12 checked, so stages 1 and 2 have nothing
/// unchecked, and from there one in five failed, one in three of the rest checked.
pub fn marks(built: &Built) -> (Vec<u8>, Vec<Status>) {
    let status: Vec<Status> = (0..count(built))
        .map(|i| match i {
            _ if i < 13 => Status::Checked,
            _ if i % 5 == 0 => Status::Failed,
            _ if i % 3 == 0 => Status::Checked,
            _ => Status::Unchecked,
        })
        .collect();
    (table_of(built, &status), status)
}

pub const KINDS: [Kind; 4] = [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked];

/// The order a stage's titles are visited in: every title once, jumping up and down.
fn visit(len: usize) -> Vec<usize> {
    assert!(len % 17 != 0);
    (0..len).map(|i| i * 17 % len).collect()
}

/// One session: the list of `stage` under `kind`, then F5, the cursor to each title in turn, and
/// ENTER or F1. Returns how many jumps landed each way.
pub fn session(
    stage: usize,
    table: Option<&(Vec<u8>, Vec<Status>)>,
    kind: Kind,
) -> BTreeMap<&'static str, usize> {
    let built = rich();
    let mut walker = Walker::on_list(built, table.map(|t| (&t.0[..], &t.1[..])), stage);
    let list = &mut walker.list;
    list.go_to_kind(kind);
    let titles = &rich_titles()[..title_counts(built)[stage]];
    let mut landings = BTreeMap::new();
    for (n, t) in visit(titles.len()).into_iter().enumerate() {
        let at = format!("stage {stage}, {kind:?}, title {t}");
        let items = list.items();
        list.open_sections();
        list.goto_title(t);
        let landing = Landing::of(&items, titles[t].assertion);
        *landings.entry(landing.name()).or_insert(0) += 1;
        let (scan, key) = if n % 2 == 0 {
            (ENTER, Key::Enter)
        } else {
            (F1, Key::F1)
        };
        list.leave(scan, key);
        list.same_kind_position(&at);
        let frame = list.list_frame();
        let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        assert_eq!(cursor, landing.cursor(), "{at}: {landing:?}");
        assert_eq!(frame[7], 0, "{at}: the find");
    }
    landings
}

fn totals(runs: &[BTreeMap<&'static str, usize>]) -> BTreeMap<&'static str, usize> {
    let mut sum = BTreeMap::new();
    for run in runs {
        for (name, n) in run {
            *sum.entry(*name).or_insert(0) += n;
        }
    }
    sum
}

#[test]
fn a_jump_from_every_title_under_every_kind_is_the_models_with_no_status_table() {
    let mut runs = Vec::new();
    for stage in [1, 2, 3, 4, 5] {
        for kind in KINDS {
            runs.push(session(stage, None, kind));
        }
    }
    let sum = totals(&runs);
    println!("no table: {sum:?}");
    assert_eq!(
        sum,
        BTreeMap::from([("direct", 294), ("empty", 16), ("last", 8), ("next", 74)])
    );
}

#[test]
fn a_jump_from_every_title_under_every_kind_is_the_models_with_a_status_table() {
    let table = marks(rich());
    let mut runs = Vec::new();
    for stage in [1, 2, 3, 5] {
        for kind in KINDS {
            runs.push(session(stage, Some(&table), kind));
        }
    }
    let sum = totals(&runs);
    println!("with a table: {sum:?}");
    assert_eq!(
        sum,
        BTreeMap::from([("direct", 163), ("empty", 25), ("last", 5), ("next", 67)])
    );
}
