//! The Libraries screen with a status table in RAM: the three kinds of row the model has, a stage
//! that is not started, one that is under way and one that is done or failed, against the model
//! for the same marks.

use crate::app::*;
use crate::builds::*;
use crate::shellrig::*;
use crate::table;
use mcui::Status;
use mcui::canvas::Call;

/// The app around the library of real proofs, one of which is wrong, cut in `parts` stages.
fn proofs(name: &str, parts: usize) -> &'static Built {
    let library = crate::fixture::proofs();
    let count = table::header_of(&library).1 as usize;
    assert!(count >= parts, "{count} assertions for {parts} stages");
    let ends: Vec<usize> = (1..=parts).map(|n| n * count / parts).collect();
    staged(name, &library, &ends)
}

pub fn three() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| proofs("pages-proofs-three", 3))
}

fn six() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| proofs("pages-proofs-six", 6))
}

fn count_of(built: &Built) -> usize {
    table::header_of(built.library.as_ref().unwrap()).1 as usize
}

/// The right text of each stage row the model draws, My proofs left out.
fn right_texts(ui: &mcui::Ui) -> Vec<String> {
    ui.frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            Call::ListRow { left, right, .. } if left != "My proofs" => Some(right),
            _ => None,
        })
        .collect()
}

/// A shell on `built` with the status table `status` in RAM, and the model with the same marks:
/// the frames are the same, and so is each key's effect.
fn with_status(built: &'static Built, status: &[Status]) -> (Shell, mcui::Ui) {
    let data = table::of(
        built.library.as_ref().unwrap(),
        &table::values(built, status),
    );
    let mut shell = booted(built, Some(&data));
    let mut ui = model::ui(built, status);
    shell.shows("the Libraries screen", &model::frame(&ui));
    for row in 0..rows(built) {
        let at = format!("row {row}");
        if row > 0 {
            press(&mut shell, &mut ui, built, DOWN, &at);
        }
        if row < built.stages.len() && built.stages.len() > 1 {
            press(&mut shell, &mut ui, built, F3, &at);
            press(&mut shell, &mut ui, built, CLEAR, &at);
        }
    }
    (shell, ui)
}

fn marks(built: &Built, checked: usize, failed: &[usize]) -> Vec<Status> {
    let mut status = vec![Status::Unchecked; count_of(built)];
    status[..checked].fill(Status::Checked);
    for &i in failed {
        status[i] = Status::Failed;
    }
    status
}

#[test]
fn a_table_with_nothing_checked_shows_every_stage_unchecked() {
    let built = three_stages();
    let (_, ui) = with_status(built, &marks(built, 0, &[]));
    assert_eq!(right_texts(&ui), ["?", "?", "?"]);
}

#[test]
fn a_table_with_some_checked_shows_the_stages_under_way_as_checked_of_total() {
    let built = three_stages();
    let n = count_of(built);
    let first = built.stages[0].end;
    let (_, ui) = with_status(built, &marks(built, first + 2, &[]));
    let texts = right_texts(&ui);
    assert_eq!(texts[0], mcglyph::CHECK.to_string());
    assert_eq!(texts[1], format!("{}/{}", first + 2, built.stages[1].end));
    assert_eq!(texts[2], format!("{}/{n}", first + 2));
}

#[test]
fn a_failure_in_the_first_stage_marks_every_stage_that_contains_it() {
    let built = three_stages();
    let (_, ui) = with_status(built, &marks(built, 0, &[0]));
    let cross = mcglyph::CROSS.to_string();
    assert_eq!(right_texts(&ui), [cross.clone(), cross.clone(), cross]);
}

#[test]
fn a_failure_in_the_last_stage_only_marks_that_stage() {
    let built = three_stages();
    let n = count_of(built);
    let (_, ui) = with_status(built, &marks(built, n - 1, &[n - 1]));
    let texts = right_texts(&ui);
    assert_eq!(texts[0], mcglyph::CHECK.to_string());
    assert_eq!(texts[1], mcglyph::CHECK.to_string());
    assert_eq!(texts[2], mcglyph::CROSS.to_string());
}

#[test]
fn a_table_with_everything_checked_shows_every_stage_checked() {
    let built = three_stages();
    let (_, ui) = with_status(built, &marks(built, count_of(built), &[]));
    assert_eq!(right_texts(&ui), vec![mcglyph::CHECK.to_string(); 3]);
}

#[test]
fn a_stage_about_shows_the_counts_of_its_stage() {
    let built = three_stages();
    let first = built.stages[0].end;
    let (mut shell, mut ui) = with_status(built, &marks(built, first + 2, &[first + 1]));
    for row in 0..3 {
        press(&mut shell, &mut ui, built, F3, &format!("row {row}"));
        press(&mut shell, &mut ui, built, CLEAR, &format!("row {row}"));
        press(&mut shell, &mut ui, built, DOWN, &format!("row {row}"));
    }
}

/// Every table the stream gives, over the library cut in 1, 3 and 8: every screen the keys reach
/// is the model's. The rows between them hold each kind of text.
#[test]
fn the_rows_of_random_tables_are_the_models() {
    let mut seed = 0x2545_F491;
    let mut seen = std::collections::BTreeSet::new();
    for built in [three(), six()] {
        for round in 0..40 {
            let checked = [0, 25, 60, 100][round % 4];
            let failing = [0, 0, 7, 0, 3][round % 5];
            let status = table::random_status(count_of(built), &mut seed, checked, failing);
            let (_, ui) = with_status(built, &status);
            for text in right_texts(&ui) {
                let kind = match text.chars().next() {
                    Some(c) if c == mcglyph::CHECK || c == mcglyph::CROSS || c == '?' => text,
                    Some(_) => "n/m".to_string(),
                    None => "empty".to_string(),
                };
                seen.insert(kind);
            }
        }
    }
    let want = [mcglyph::CHECK, mcglyph::CROSS, '?'].map(|c| c.to_string());
    for kind in want.iter().chain(["n/m".to_string()].iter()) {
        assert!(seen.contains(kind), "no table showed {kind:?}: {seen:?}");
    }
}

#[test]
fn a_status_table_with_a_failed_assertion_the_model_demotes_is_the_models() {
    let built = three();
    let n = count_of(built);
    let (_, ui) = with_status(built, &vec![Status::Checked; n]);
    let texts = right_texts(&ui);
    assert!(
        texts.iter().any(|t| t == &mcglyph::CROSS.to_string()),
        "the library's wrong proof fails the stage it is in: {texts:?}"
    );
}
