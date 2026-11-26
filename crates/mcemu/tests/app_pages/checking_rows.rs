//! The checking screen on a stage other than the first, and on a stage with nothing left to check,
//! against the host model: the stage's own end bounds the job, and the frames, the stop and the
//! Libraries screen after it are the model's for the row the cursor was on.

use crate::app::*;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use crate::table;
use crate::ui_support::same_frame;
use mcui::Key;
use std::sync::OnceLock;

/// The proofs library as three stages of equal size.
fn three() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let library = crate::fixture::proofs();
        let n = usize::from(table::header_of(&library).1);
        staged("pages-checking-rows", &library, &[n / 3, 2 * n / 3, n])
    })
}

/// The model on Libraries with the cursor on `row` and these values in the status table.
fn libraries_at(built: &Built, values: &[u8], row: usize) -> Vec<u8> {
    let mut ui = model::ui(built, &status_of(values));
    for _ in 0..row {
        ui.press(Key::Down);
    }
    model::frame(&ui)
}

/// A shell on `built` with the cursor on `row`, the move's own frame taken.
fn on_row(built: &'static Built, table: Option<&[u8]>, row: usize) -> Run {
    let mut run = Run::boot(built, table);
    for _ in 0..row {
        run.tap(DOWN);
        run.next_blit();
    }
    assert_eq!(run.frames()[0][1], row as u8, "the cursor is on the row");
    run
}

/// Run on the second and the third stage of a library with no table: the reading frames, the steps'
/// frames and the finished frame are the model's for that row, the job stops at that stage's end
/// and no assertion past it is verified.
#[test]
fn the_frames_of_a_check_on_the_second_and_third_stage_are_the_models_for_that_row() {
    let built = three();
    let library = built.library.as_ref().unwrap();
    let verdicts = host_verdicts(library);
    let ends: Vec<usize> = built.stages.iter().map(|s| s.end).collect();
    assert!(ends[0] < ends[1] && ends[1] < ends[2] && ends[2] == verdicts.len());
    assert!(
        ends[1] > EVERY,
        "stage 1 ends past the first frame of the steps"
    );
    for row in [1, 2] {
        let end = ends[row];
        let mut run = on_row(built, None, row);
        run.m().press_key(F2);
        let (mut reading, mut middle, mut frames) = (0usize, 0usize, 0usize);
        let (last, values) = loop {
            let (frame, values) = run.next_frame();
            frames += 1;
            assert!(frames < 40, "row {row}: the check never ended");
            let Some(values) = values else {
                shows(
                    &format!("row {row}: reading frame {frames}"),
                    &frame,
                    &reading_model(built, row, frames.saturating_sub(2)),
                );
                reading += 1;
                continue;
            };
            let settled = values.iter().filter(|&&v| v != 0).count();
            assert_eq!(values, expected_values(built, &verdicts, settled));
            shows(
                &format!("row {row}: {settled} settled"),
                &frame,
                &checking_model(built, &values, row),
            );
            if values[..end].iter().all(|&v| v != 0) {
                break (frame, values);
            }
            middle += usize::from(settled > 0 && settled < end);
        };
        run.m().release_key(F2);
        assert!(reading >= 2, "row {row}: {reading} reading frames");
        assert!(middle >= 1, "row {row}: {middle} frames in the middle");
        assert!(
            values[end..].iter().all(|&v| v == 0),
            "row {row}: nothing past the end"
        );
        assert_eq!(usize::from(run.verifies()), end, "row {row}");
        let finished = checking_model(built, &values, row);
        assert!(matches!(
            finished.state(),
            mcui::State::Checking { finished: true, stage, .. } if stage == row
        ));
        shows(&format!("row {row}: finished"), &last, &finished);
        assert_eq!(run.frames()[1][..3], [KIND_CHECKING, row as u8, 1]);
        assert_eq!(
            usize::from(run.next_number()),
            end,
            "row {row}: next is the stage's end"
        );
    }
}

/// F1 on the second and the third stage in the middle of the steps: the screen pops, "next" is the
/// first assertion not verified, and the Libraries frame is the model's with the cursor still on
/// the row.
#[test]
fn a_stop_on_the_second_and_third_stage_returns_to_libraries_on_that_row() {
    let built = three();
    for row in [1, 2] {
        let mut run = on_row(built, Some(&empty_table(built)), row);
        run.m().press_key(F2);
        run.next_blit();
        run.m().release_key(F2);
        run.next_blit();
        run.tap(F1);
        let frame = run.next_blit();
        let values = run.values().unwrap();
        let settled = values.iter().filter(|&&v| v != 0).count();
        assert!(
            settled >= EVERY && settled < built.stages[row].end,
            "row {row}: {settled}"
        );
        assert_eq!(usize::from(run.next_number()), settled);
        assert_eq!((run.depth(), run.frames()[0][1]), (1, row as u8));
        same_frame(
            &format!("Libraries after a stop on row {row}"),
            &frame,
            &libraries_at(built, &values, row),
        );
        assert_ne!(
            frame,
            libraries_at(built, &values, 0),
            "row {row}: the cursor is not on the first row"
        );
    }
}

/// Run on a stage whose assertions all have a value: the first frame is the model's with nothing
/// left to check, no kernel call is made, the next frame is the finished one, and F1, which says
/// Done, returns to Libraries with "next" at the end.
#[test]
fn run_on_a_stage_that_is_already_checked_makes_no_kernel_call_and_ends_on_done() {
    let built = proofs();
    let library = built.library.as_ref().unwrap();
    let verdicts = host_verdicts(library);
    let n = verdicts.len();
    let data = table::of(library, &expected_values(built, &verdicts, n));
    let mut run = Run::boot(built, Some(&data));
    run.m().press_key(F2);
    let (first, values) = run.next_frame();
    let values = values.expect("the table");
    assert!(
        values.iter().all(|&v| v != 0),
        "every assertion has a value"
    );
    let model = checking_model(built, &values, 0);
    shows("the first frame", &first, &model);
    assert_eq!(
        run.frames()[1][..3],
        [KIND_CHECKING, 0, 0],
        "not yet finished"
    );
    run.m().release_key(F2);
    let (finished, _) = run.next_frame();
    shows("the finished frame", &finished, &model);
    assert_eq!(run.frames()[1][..3], [KIND_CHECKING, 0, 1], "finished");
    run.run(3_000_000);
    assert_eq!(run.verifies(), 0, "no kernel verify call");
    assert_eq!(run.crc_calls(), (0, 0), "no CRC run");
    let mut kept = data.clone();
    kept[8..10].copy_from_slice(&u16::try_from(n).unwrap().to_le_bytes());
    assert_eq!(
        run.raw_table(),
        Some(kept),
        "the table is as it was, with next at the end"
    );
    run.tap(F1);
    let back = run.next_blit();
    shows(
        "Libraries after Done",
        &back,
        &model::ui(built, &status_of(&values)),
    );
    assert_eq!(run.depth(), 1);
}
