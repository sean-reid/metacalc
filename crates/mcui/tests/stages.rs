//! One library shown as three stages: the rows, their lists, checking, and the theorems under them.

mod support;

use mcui::Key::*;
use mcui::canvas::Call;
use mcui::{CheckStep, Installed, Kind, LoadError, Mark, Stage, State, Status, Statuses, Ui};
use support::{LONG, check_frame, index_of, pack_all, press, sectioned};

const CHECK: char = mcglyph::CHECK;
const CROSS: char = mcglyph::CROSS;

/// The sectioned logic: wi wal ax-1 ax-2 ax-mp | a1i a2i mpd | ax-5 alid ax-last.
const ENDS: [usize; 3] = [5, 8, 11];
const NAMES: [&str; 3] = ["base", "rules", "all"];
const LAST: [&str; 3] = ["ax-mp", "mpd", "ax-last"];

fn bytes() -> Vec<u8> {
    pack_all(&support::db(&sectioned())).expect("the sectioned logic packs")
}

fn stages() -> Vec<Stage> {
    (0..3)
        .map(|i| Stage {
            name: NAMES[i].into(),
            title: format!("Title of {}", NAMES[i]),
            end: ENDS[i],
        })
        .collect()
}

fn ui_with(bytes: Vec<u8>, table: Vec<Status>, stages: Vec<Stage>) -> Result<Ui, LoadError> {
    Ui::with_stages(
        vec![Installed {
            name: "logic".into(),
            bytes,
        }],
        vec![stages],
        Statuses {
            libraries: vec![table],
            my_proofs: 2,
        },
    )
}

/// The first `checked` assertions checked, the rest not, and the assertions named failed.
fn table(checked: usize, failed: &[&str]) -> Vec<Status> {
    let bytes = bytes();
    let mut table = vec![Status::Unchecked; ENDS[2]];
    table[..checked].fill(Status::Checked);
    for label in failed {
        table[index_of(&bytes, label)] = Status::Failed;
    }
    table
}

fn staged(table: Vec<Status>) -> Ui {
    ui_with(bytes(), table, stages()).expect("the library loads")
}

fn fresh() -> Ui {
    staged(table(0, &[]))
}

fn rows(ui: &Ui) -> Vec<(String, String)> {
    ui.frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            Call::ListRow { left, right, .. } => Some((left, right)),
            _ => None,
        })
        .collect()
}

fn read(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|&(l, r)| (l.to_string(), r.to_string()))
        .collect()
}

fn list(ui: &Ui) -> (usize, Option<String>, usize, usize) {
    match ui.state() {
        State::List {
            stage,
            label,
            cursor,
            len,
            ..
        } => (stage, label, cursor, len),
        other => panic!("not a list: {other:?}"),
    }
}

fn checking(ui: &Ui) -> (usize, usize, usize, usize, bool) {
    match ui.state() {
        State::Checking {
            stage,
            done,
            total,
            failed,
            finished,
            ..
        } => (stage, done, total, failed, finished),
        other => panic!("not the checking screen: {other:?}"),
    }
}

/// Opens the list of stage `stage` from the libraries screen.
fn open_stage(ui: &mut Ui, stage: usize) {
    for _ in 0..stage {
        ui.press(Down);
    }
    ui.press(Enter);
}

fn run_all(ui: &mut Ui) -> (usize, usize) {
    let (mut passed, mut failed) = (0, 0);
    loop {
        match ui.step_check() {
            CheckStep::Checked => passed += 1,
            CheckStep::Failed => failed += 1,
            CheckStep::Finished => return (passed, failed),
            CheckStep::Idle => panic!("the checking screen closed"),
        }
    }
}

#[test]
fn one_row_per_stage_then_my_proofs_each_with_its_own_progress() {
    let c = CHECK.to_string();
    let x = CROSS.to_string();
    let cases: [(Vec<Status>, [&str; 3]); 8] = [
        (table(0, &[]), ["?", "?", "?"]),
        (table(3, &[]), ["3/5", "3/8", "3/11"]),
        (table(5, &[]), [&c, "5/8", "5/11"]),
        (table(7, &[]), [&c, "7/8", "7/11"]),
        (table(11, &[]), [&c, &c, &c]),
        (table(11, &["a2i"]), [&c, &x, &x]),
        (table(11, &["alid"]), [&c, &c, &x]),
        (table(11, &["ax-mp"]), [&x, &x, &x]),
    ];
    for (n, (table, marks)) in cases.into_iter().enumerate() {
        let ui = staged(table);
        let want = read(&[
            ("base", marks[0]),
            ("rules", marks[1]),
            ("all", marks[2]),
            ("My proofs", "2"),
        ]);
        assert_eq!(rows(&ui), want, "case {n}");
    }
}

#[test]
fn a_failure_marks_every_stage_whose_range_holds_it_and_none_before_it() {
    let ui = staged(table(8, &["a2i"]));
    assert_eq!(
        rows(&ui),
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", &CROSS.to_string()),
            ("all", &CROSS.to_string()),
            ("My proofs", "2"),
        ])
    );
}

#[test]
fn the_cursor_walks_the_stage_rows_and_my_proofs_stops_at_the_end() {
    let mut ui = fresh();
    press(&mut ui, &[Down, Down, Down]);
    assert_eq!(ui.state(), State::Libraries { cursor: 3, top: 0 });
    ui.press(Down);
    assert_eq!(ui.state(), State::Libraries { cursor: 3, top: 0 });
    ui.press(Enter);
    assert!(matches!(ui.state(), State::Notice { .. }));
    press(&mut ui, &[Clear, Up, Up, Up, Up]);
    assert_eq!(ui.state(), State::Libraries { cursor: 0, top: 0 });
}

#[test]
fn each_stage_opens_its_own_list_that_ends_at_its_last_assertion() {
    for stage in 0..3 {
        let mut ui = fresh();
        open_stage(&mut ui, stage);
        assert_eq!(list(&ui), (stage, Some("wi".into()), 0, ENDS[stage]));
        press(&mut ui, &[Right; 40]);
        let (_, label, cursor, len) = list(&ui);
        assert_eq!((label.as_deref(), cursor), (Some(LAST[stage]), len - 1));
        press(&mut ui, &[Down, Down, Right]);
        assert_eq!(list(&ui), (stage, Some(LAST[stage].into()), len - 1, len));
        let frame = ui.frame();
        let counts = format!("0/{}", ENDS[stage]);
        assert!(support::title_is(&frame, NAMES[stage], &counts));
    }
}

#[test]
fn a_list_title_counts_only_its_stage() {
    let mut ui = staged(table(5, &[]));
    open_stage(&mut ui, 1);
    assert!(support::title_is(&ui.frame(), "rules", "5/8"));
    press(&mut ui, &[Clear, Up]);
    ui.press(Enter);
    assert!(support::title_is(&ui.frame(), "base", "5/5"));
}

#[test]
fn find_stays_inside_the_stage() {
    let mut ui = fresh();
    open_stage(&mut ui, 0);
    support::typed(&mut ui, "a1");
    assert_eq!(list(&ui).1.as_deref(), Some("ax-1"));
    let missed = format!("a1 {CROSS}");
    assert!(support::title_is(&ui.frame(), "base", &missed));
    ui.press(Clear);
    support::typed(&mut ui, "ax-m");
    assert_eq!(list(&ui).1.as_deref(), Some("ax-mp"));

    let mut ui = fresh();
    open_stage(&mut ui, 1);
    support::typed(&mut ui, "a1");
    assert_eq!(list(&ui).1.as_deref(), Some("a1i"));
    press(&mut ui, &[Clear]);
    support::typed(&mut ui, "alid");
    assert_eq!(list(&ui).1.as_deref(), Some("ax-1"));
    assert!(support::title_is(
        &ui.frame(),
        "rules",
        &format!("alid {CROSS}")[..]
    ));
}

#[test]
fn kind_counts_only_the_stage() {
    let kinds = |stage: usize, checked: usize| {
        let mut ui = staged(table(checked, &[]));
        open_stage(&mut ui, stage);
        let mut seen = Vec::new();
        for _ in 0..4 {
            let (_, _, _, len) = list(&ui);
            let State::List { kind, .. } = ui.state() else {
                unreachable!()
            };
            seen.push((kind, len));
            ui.press(F3);
        }
        seen
    };
    let kinds_of = |counts: [usize; 4]| {
        [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked]
            .into_iter()
            .zip(counts)
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(0, 3), kinds_of([5, 0, 5, 2]));
    assert_eq!(kinds(1, 3), kinds_of([8, 3, 5, 5]));
    assert_eq!(kinds(2, 3), kinds_of([11, 4, 7, 8]));
}

#[test]
fn the_sections_jump_stays_inside_the_stage() {
    for (stage, titles, lands) in [(0, 3, "ax-1"), (1, 7, "mpd"), (2, 10, "ax-last")] {
        let mut ui = fresh();
        open_stage(&mut ui, stage);
        ui.press(F5);
        let State::Sections { len, .. } = ui.state() else {
            panic!("not sections: {:?}", ui.state());
        };
        assert_eq!(len, titles, "stage {stage}");
        press(&mut ui, &[Down; 20]);
        let State::Sections { cursor, .. } = ui.state() else {
            unreachable!()
        };
        assert_eq!(cursor, titles - 1);
        ui.press(Enter);
        assert_eq!(list(&ui).1.as_deref(), Some(lands), "stage {stage}");
        assert_eq!(list(&ui).0, stage);
    }
}

#[test]
fn the_sections_screen_of_a_stage_opens_on_the_title_of_the_highlighted_row_and_is_named_after_it()
{
    let mut ui = fresh();
    open_stage(&mut ui, 1);
    press(&mut ui, &[Right; 40]);
    ui.press(F5);
    let State::Sections { title, .. } = ui.state() else {
        panic!("not sections");
    };
    assert_eq!(title.as_deref(), Some(LONG));
    assert!(support::title_is(&ui.frame(), "rules", "7/7"));
}

#[test]
fn a_stage_without_a_title_has_no_sections_key() {
    let bytes = support::logic_bytes();
    let len = mcpack::library::read(&bytes).unwrap().len();
    let stages = vec![Stage {
        name: "only".into(),
        title: String::new(),
        end: len,
    }];
    let mut ui = ui_with(bytes, vec![Status::Unchecked; len], stages).unwrap();
    ui.press(Enter);
    assert!(!support::key_label_is(&ui.frame(), 4, "Sect"));
    ui.press(F5);
    assert!(matches!(ui.state(), State::List { .. }));
}

#[test]
fn a_theorem_opened_from_two_stages_is_the_same_theorem_with_the_same_mark() {
    let open_in = |stage: usize| {
        let mut ui = staged(table(7, &["a1i"]));
        open_stage(&mut ui, stage);
        support::typed(&mut ui, "a1i");
        ui.press(Enter);
        ui
    };
    let (one, two) = (open_in(1), open_in(2));
    assert!(matches!(one.state(), State::Theorem { ref label, .. } if label == "a1i"));
    assert_eq!(one.state(), two.state());
    assert_eq!(one.frame(), two.frame());
    assert!(support::title_is(
        &one.frame(),
        "a1i",
        &format!("{} thm", Mark::Failed.glyph())
    ));
}

#[test]
fn running_a_stage_stops_at_its_end_and_the_next_run_carries_on() {
    let mut ui = fresh();
    ui.press(F2);
    assert_eq!(checking(&ui), (0, 0, 5, 0, false));
    assert_eq!(run_all(&mut ui), (5, 0));
    assert_eq!(checking(&ui), (0, 5, 5, 0, true));
    assert_eq!(ui.step_check(), CheckStep::Finished);
    ui.press(F1);
    assert_eq!(
        rows(&ui),
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", "5/8"),
            ("all", "5/11"),
            ("My proofs", "2")
        ])
    );
    press(&mut ui, &[Down, Down, F2]);
    assert_eq!(checking(&ui), (2, 5, 11, 0, false));
    assert_eq!(run_all(&mut ui), (6, 0));
    assert_eq!(checking(&ui), (2, 11, 11, 0, true));
    ui.press(F1);
    assert_eq!(
        rows(&ui)[..3],
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", &CHECK.to_string()),
            ("all", &CHECK.to_string())
        ])[..]
    );
}

#[test]
fn running_the_middle_stage_leaves_the_last_assertions_alone() {
    let mut ui = fresh();
    press(&mut ui, &[Down, F2]);
    assert_eq!(run_all(&mut ui), (8, 0));
    assert_eq!(checking(&ui), (1, 8, 8, 0, true));
    ui.press(Clear);
    assert_eq!(
        rows(&ui)[..3],
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", &CHECK.to_string()),
            ("all", "8/11")
        ])[..]
    );
}

#[test]
fn a_stage_whose_range_is_all_settled_says_so_even_when_later_ones_are_not() {
    let mut ui = staged(table(5, &[]));
    ui.press(F2);
    assert_eq!(checking(&ui), (0, 5, 5, 0, true));
    assert_eq!(ui.step_check(), CheckStep::Finished);
    assert!(support::text_at(&ui.frame(), 24, 1, "All checked", false));
    assert!(support::key_label_is(&ui.frame(), 0, "Done"));
    ui.press(F1);
    press(&mut ui, &[Down, F2]);
    assert_eq!(checking(&ui), (1, 5, 8, 0, false));
}

#[test]
fn a_failed_check_counts_in_the_stages_that_hold_it_and_not_in_the_ones_before() {
    let sound = bytes();
    let alid = index_of(&sound, "alid") as u16;
    let rejected = support::refreshed(support::patch_reference(&sound, "a1i", 0, alid));
    let open = |stage: usize| {
        let mut ui = ui_with(rejected.clone(), table(0, &[]), stages()).unwrap();
        for _ in 0..stage {
            ui.press(Down);
        }
        ui.press(F2);
        ui
    };
    let mut ui = open(0);
    assert_eq!(run_all(&mut ui), (5, 0));
    assert_eq!(checking(&ui), (0, 5, 5, 0, true));
    let mut ui = open(1);
    assert_eq!(run_all(&mut ui), (7, 1));
    assert_eq!(checking(&ui), (1, 8, 8, 1, true));
    ui.press(Clear);
    assert_eq!(
        rows(&ui)[..3],
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", &CROSS.to_string()),
            ("all", &CROSS.to_string())
        ])[..]
    );
}

#[test]
fn info_on_a_stage_shows_its_title_and_counts() {
    let mut ui = staged(table(5, &["a2i"]));
    press(&mut ui, &[Down, F3]);
    assert_eq!(
        ui.state(),
        State::StageAbout {
            library: 0,
            stage: 1
        }
    );
    let calls = ui.frame_calls().1;
    assert!(calls.contains(&Call::Title {
        left: "rules".into(),
        right: String::new()
    }));
    for text in ["Title of rules", "8 assertions", "5 checked", "1 failed"] {
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, Call::TextCentered { text: t, .. } if t == text)),
            "{text}"
        );
    }
    ui.press(Clear);
    assert!(matches!(ui.state(), State::Libraries { cursor: 1, .. }));
}

#[test]
fn info_on_my_proofs_is_the_program_info() {
    let mut ui = fresh();
    press(&mut ui, &[Down, Down, Down, F3]);
    assert_eq!(ui.state(), State::About);
}

#[test]
fn a_damaged_library_shows_damaged_on_every_stage_and_refuses_to_check() {
    let mut damaged = bytes();
    damaged[4] ^= 0xFF;
    let mut ui = ui_with(damaged, table(0, &[]), stages()).unwrap();
    let want = read(&[
        ("base", "damaged"),
        ("rules", "damaged"),
        ("all", "damaged"),
        ("My proofs", "2"),
    ]);
    assert_eq!(rows(&ui), want);
    press(&mut ui, &[Down, F2]);
    assert!(matches!(ui.state(), State::Notice { .. }));
}

#[test]
fn stages_that_do_not_end_at_the_last_assertion_or_do_not_rise_are_refused() {
    let with = |ends: &[usize]| {
        let stages = ends
            .iter()
            .map(|&end| Stage {
                name: "s".into(),
                title: String::new(),
                end,
            })
            .collect();
        ui_with(bytes(), table(0, &[]), stages).err()
    };
    assert!(with(&[5, 8, 11]).is_none());
    for ends in [&[5, 8][..], &[5, 12], &[8, 5, 11], &[5, 5, 11], &[0, 11]] {
        assert!(matches!(with(ends), Some(LoadError::Stages(_))), "{ends:?}");
    }
    let err = Ui::with_stages(
        vec![],
        vec![vec![]],
        Statuses {
            libraries: vec![],
            my_proofs: 0,
        },
    )
    .err();
    assert!(matches!(err, Some(LoadError::Stages(_))));
}

#[test]
fn one_stage_ending_at_the_last_assertion_is_the_library_under_the_stage_name() {
    let stage = Stage {
        name: "whole".into(),
        title: String::new(),
        end: ENDS[2],
    };
    let mut ui = ui_with(bytes(), table(3, &[]), vec![stage]).unwrap();
    assert_eq!(rows(&ui), read(&[("whole", "3/11"), ("My proofs", "2")]));
    ui.press(Enter);
    assert_eq!(list(&ui), (0, Some("wi".into()), 0, 11));
    ui.press(Clear);
    ui.press(F3);
    assert_eq!(ui.state(), State::About);
}

#[test]
fn the_three_stage_frames() {
    let mut ui = staged(table(5, &[]));
    ui.press(Down);
    check_frame(&ui.frame(), "stages-libraries");
    press(&mut ui, &[Enter, Down, Down, Down, Down, Down, Down]);
    check_frame(&ui.frame(), "stages-list");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn ring_with_the_stages_mcpack_reports_reads_779_1171_and_3326() {
    use mcpack::manifest::Manifest;
    let dir = std::path::PathBuf::from(std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES is set"));
    let manifests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
    let libraries = ["prop", "pred", "ring"]
        .map(|name| {
            let manifest = Manifest::load(&manifests.join(format!("{name}.toml"))).unwrap();
            let bytes = std::fs::read(dir.join(format!("{name}.mclib"))).unwrap();
            (manifest, bytes)
        })
        .to_vec();
    let ring = libraries[2].1.clone();
    let reported = mcpack::library::stages(libraries).expect("the shipped libraries are stages");
    let stages: Vec<Stage> = reported
        .iter()
        .map(|s| Stage {
            name: s.name.clone(),
            title: s.title.clone(),
            end: s.count,
        })
        .collect();
    let total = mcpack::library::read(&ring).unwrap().len();
    let mut checked = vec![Status::Unchecked; total];
    checked[..100].fill(Status::Checked);
    let ui = ui_with(ring, checked, stages).expect("ring loads with its stages");
    assert_eq!(
        rows(&ui),
        read(&[
            ("prop", "100/779"),
            ("pred", "100/1171"),
            ("ring", "100/3326"),
            ("My proofs", "2")
        ])
    );
    let mut ui = ui;
    for (stage, want) in [779, 1171, 3326].into_iter().enumerate() {
        press(&mut ui, &vec![Down; stage]);
        ui.press(F2);
        let State::Checking { total, done, .. } = ui.state() else {
            panic!("not checking: {:?}", ui.state());
        };
        assert_eq!((total, done), (want, 100));
        ui.press(Clear);
        press(&mut ui, &vec![Up; stage]);
    }
}

#[test]
fn a_run_goes_past_a_recorded_failure_to_the_end_of_its_stage() {
    let mut table = table(5, &[]);
    table[index_of(&bytes(), "a1i")] = Status::Failed;
    let mut ui = staged(table);
    press(&mut ui, &[Down, F2]);
    assert_eq!(checking(&ui), (1, 6, 8, 1, false));
    assert_eq!(run_all(&mut ui), (2, 0));
    assert_eq!(checking(&ui), (1, 8, 8, 1, true));
    ui.press(Clear);
    assert_eq!(
        rows(&ui)[..3],
        read(&[
            ("base", &CHECK.to_string()),
            ("rules", &CROSS.to_string()),
            ("all", &CROSS.to_string())
        ])[..]
    );
}

#[test]
fn more_stage_rows_than_the_screen_holds_scroll_and_open_the_stage_under_the_cursor() {
    let ends = [1, 2, 3, 4, 5, 6, 8, 11];
    let stages = ends
        .iter()
        .enumerate()
        .map(|(i, &end)| Stage {
            name: format!("s{i}"),
            title: String::new(),
            end,
        })
        .collect();
    let mut ui = ui_with(bytes(), table(0, &[]), stages).unwrap();
    assert_eq!(rows(&ui).len(), 6);
    press(&mut ui, &[Down; 8]);
    assert_eq!(ui.state(), State::Libraries { cursor: 8, top: 3 });
    let names: Vec<String> = rows(&ui).into_iter().map(|r| r.0).collect();
    assert_eq!(names, ["s3", "s4", "s5", "s6", "s7", "My proofs"]);
    press(&mut ui, &[Down, Up]);
    assert_eq!(ui.state(), State::Libraries { cursor: 7, top: 3 });
    ui.press(Enter);
    assert_eq!(list(&ui), (7, Some("wi".into()), 0, 11));
    ui.press(Clear);
    press(&mut ui, &[Up, Up, Up]);
    ui.press(Enter);
    assert_eq!(list(&ui), (4, Some("wi".into()), 0, 5));
    ui.press(Clear);
    press(&mut ui, &[Up; 10]);
    assert_eq!(ui.state(), State::Libraries { cursor: 0, top: 0 });
}
