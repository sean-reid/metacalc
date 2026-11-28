//! Unchecked's search over a range longer than one run of the reader's copy: the answer comes from a later
//! run, and an earlier run's hit is not overwritten by a later run that has none.

use crate::built::staged_with;
use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
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

/// A library with `n` labels that start with "zq" and no other label that does, as one stage.
fn library_with(n: usize) -> &'static crate::built::Built {
    static BUILT: std::sync::Mutex<Vec<(usize, &'static crate::built::Built)>> =
        std::sync::Mutex::new(Vec::new());
    let mut cache = BUILT.lock().unwrap();
    if let Some(&(_, built)) = cache.iter().find(|(m, _)| *m == n) {
        return built;
    }
    let src: String = (0..n).map(|i| format!("zq{i} $a |- ph $.\n")).collect();
    let library = crate::fixture::custom(&src);
    let end = crate::table::header_of(&library).1 as usize;
    let built = staged_with(
        &format!("pages-find-zq{n}"),
        &library,
        &[mcui::Stage {
            name: "Runs".into(),
            title: "Runs".into(),
            end,
        }],
    );
    cache.push((n, built));
    built
}

/// The list under Unchecked over a table that checks every assertion but the one that is last in the
/// label order among the `n` that start with "zq", which is the only one in the second run once there are
/// more than 127. Returns it with that assertion's number.
fn last_unchecked(n: usize) -> (Lister, usize) {
    let built = library_with(n);
    let labels = labels_of(built);
    let mut owned: Vec<usize> = (0..labels.len())
        .filter(|&i| labels[i].starts_with("zq"))
        .collect();
    assert_eq!(owned.len(), n, "the range is exactly {n}");
    owned.sort_by(|&a, &b| labels[a].cmp(&labels[b]));
    let last = owned[n - 1];
    let status: Vec<Status> = (0..labels.len())
        .map(|i| {
            if i == last {
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
    assert_eq!(list.pos().len, 1, "the table hides all but one");
    (list, last)
}

#[test]
fn a_range_of_127_128_and_254_entries_takes_one_two_and_two_runs_and_finds_the_last_in_the_order() {
    for (n, runs) in [(127, 1), (128, 2), (254, 2)] {
        let (mut list, last) = last_unchecked(n);
        list.type_text("z");
        list.fail_at(0, "far_lib_find_copy", 99, 3);
        list.press_char('q');
        list.same_find(&format!("{n}: zq"));
        let frame = list.frame_bytes();
        let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        assert!(!list.find().missed, "{n}");
        assert_eq!(cursor, last, "{n}: the one unchecked assertion");
        println!(
            "{n} entries: {} copy calls for the letter q",
            list.fail_calls(0)
        );
        assert_eq!(
            usize::from(list.fail_calls(0)),
            runs,
            "{n}: runs of at most 127"
        );
    }
}

#[test]
fn a_copy_refused_in_the_second_run_is_a_miss_that_moves_nothing() {
    let (mut list, _) = last_unchecked(128);
    let before = list.frame_bytes();
    list.type_text("z");
    list.fail_at(0, "far_lib_find_copy", 2, 3);
    list.tap(scans_for('q')[0]);
    list.run.next_blit();
    list.run.run(300_000);
    let find = list.find();
    assert_eq!(
        (find.len, find.missed),
        (2, true),
        "the refused second run is a miss"
    );
    assert_eq!(list.fail_calls(0), 2, "both runs were asked for");
    assert_eq!(list.frame_bytes()[..2], before[..2]);
}

/// Under Axioms every lf_next is one far_lib_kind_next and every lf_prev one far_lib_kind_prev, so
/// counting those two counts the screen's calls after a hit: a hit with exactly three shown assertions
/// after it is the worst, four lf_next and five lf_prev (one and the four of lf_back4).
#[test]
fn the_first_row_after_a_hit_takes_nine_calls_at_the_most_and_the_worst_hit_takes_nine() {
    let built = pattern_built("pages-find-nine", &"A".repeat(24));
    let labels = labels_of(built);
    let mut worst = (0, 0, String::new());
    for label in labels
        .iter()
        .filter(|l| l.len() == 5 && l.ends_with(['0', '3']))
    {
        let mut list = Lister::open(built, None, 0);
        list.settle();
        list.go_to_kind(Kind::Axioms);
        let (head, last) = label.split_at(4);
        list.fail_at(0, "far_lib_kind_next", 99, 3);
        list.fail_at(1, "far_lib_kind_prev", 99, 3);
        let arm = crate::list_find_arms::Arm {
            name: "the last letter",
            typed: head.to_string(),
            press: crate::list_find_arms::Press::Char(last.chars().next().unwrap()),
            answer: 1,
        };
        list.type_settled(head);
        list.fail_at(0, "far_lib_kind_next", 99, 3);
        list.fail_at(1, "far_lib_kind_prev", 99, 3);
        list.trace_arm(
            &crate::list_find_arms::Arm {
                typed: String::new(),
                ..arm
            },
            true,
            0,
        );
        assert!(!list.find().missed, "{label}");
        let (next, prev) = (list.fail_calls(0), list.fail_calls(1));
        assert!(
            next + prev <= 9,
            "{label}: {next} lf_next and {prev} lf_prev"
        );
        if next + prev > worst.0 + worst.1 {
            worst = (next, prev, label.clone());
        }
    }
    println!(
        "the worst hit {:?}: {} lf_next, {} lf_prev",
        worst.2, worst.0, worst.1
    );
    assert_eq!((worst.0, worst.1), (4, 5));
    assert_eq!(worst.2, "ax-20");
}
