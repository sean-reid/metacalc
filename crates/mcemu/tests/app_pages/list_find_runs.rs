//! Unchecked's search over a range longer than one run of the reader's copy: the answer comes from a later
//! run, and an earlier run's hit is not overwritten by a later run that has none.

use crate::built::staged_with;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::{Kind, Status};

fn library_of_300() -> &'static crate::built::Built {
    static BUILT: std::sync::OnceLock<&'static crate::built::Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let src: String = (0..300).map(|i| format!("a{i} $a |- ph $.\n")).collect();
        let library = crate::fixture::custom(&src);
        let end = crate::table::header_of(&library).1 as usize;
        staged_with(
            "pages-find-runs",
            &library,
            &[mcui::Stage {
                name: "Runs".into(),
                title: "Runs".into(),
                end,
            }],
        )
    })
}

#[test]
fn a_range_of_several_runs_finds_the_least_unchecked_whichever_run_holds_it() {
    let built = library_of_300();
    let labels = labels_of(built);
    let n = count(built);
    let at = |label: &str| labels.iter().position(|l| l == label).expect(label);
    let (first, last) = (at("a0"), at("a299"));
    assert!(first < last);
    let owned = labels.iter().filter(|l| l.starts_with('a')).count();
    assert!(
        owned >= 300,
        "{owned} labels start with a: a range of three runs of 127"
    );
    let order: Vec<&String> = {
        let mut o: Vec<&String> = labels.iter().filter(|l| l.starts_with('a')).collect();
        o.sort();
        o
    };
    let late = order.iter().position(|l| l.as_str() == "a299").unwrap();
    assert!(
        late >= 127,
        "a299 is at place {late} in the label order, past the first run"
    );
    for unchecked in [vec![last], vec![first, last], vec![last, first + 1]] {
        let status: Vec<Status> = (0..n)
            .map(|i| {
                if unchecked.contains(&i) {
                    Status::Unchecked
                } else {
                    Status::Checked
                }
            })
            .collect();
        let data = table_of(built, &status);
        let mut list = Lister::open(built, Some((&data, &status)), 0);
        list.settle();
        list.go_to_kind(Kind::Unchecked);
        assert_eq!(list.pos().len, unchecked.len(), "the table hides the rest");
        list.type_text("a");
        assert!(!list.find().missed, "{unchecked:?}");
        let frame = list.frame_bytes();
        let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        assert_eq!(
            cursor,
            *unchecked.iter().min().unwrap(),
            "{unchecked:?}: the least"
        );
    }
}
