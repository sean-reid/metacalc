//! The checking screen as a user drives it on the stub OS: Run from Libraries to "All checked", a
//! stop by F1, CLEAR or ON in the middle of the reading and of the steps and a second Run, the
//! CRC that is wrong, a table that disappears, and what the table holds each time.

use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcemu::Machine;

/// The proofs library's check from the Libraries screen to the end, with no stop: every mark is the
/// host's, each assertion was verified once, and the last frame is the model's finished screen.
#[test]
fn run_on_a_stage_checks_every_assertion_once_and_ends_on_all_checked() {
    let built = proofs();
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let n = verdicts.len();
    let mut run = Run::boot(built, None);
    run.m().press_key(F2);
    let mut frames = 0;
    let (last, values) = loop {
        let (frame, values) = run.next_frame();
        frames += 1;
        if let Some(values) = values
            && values.iter().all(|&v| v != 0)
        {
            break (frame, values);
        }
        assert!(frames < 100, "the check never ended");
    };
    run.m().release_key(F2);
    assert_eq!(
        values,
        expected_values(built, &verdicts, n),
        "the host's verdicts"
    );
    assert_eq!(
        usize::from(run.verifies()),
        n,
        "each assertion verified once"
    );
    let finished = checking_model(built, &values, 0);
    assert!(matches!(
        finished.state(),
        mcui::State::Checking { finished: true, .. }
    ));
    shows("the finished screen", &last, &finished);
    // The reading frames, the steps' frames and the last: one for the push, one for crc_begin,
    // then the single chunk ends the run and the steps' first frame follows.
    let chunks = chunks_of(built);
    let steps = (0..n).step_by(EVERY).count();
    assert_eq!(frames, chunks + 1 + steps + 1, "the frames of a whole run");
    assert_eq!(run.depth(), 2);
    assert_eq!(
        run.next_number(),
        u16::try_from(n).unwrap(),
        "next is N once finished"
    );
    run.tap(CLEAR);
    let libraries = run.next_blit();
    shows(
        "Libraries after All checked",
        &libraries,
        &model::ui(built, &status_of(&values)),
    );
    assert_eq!(run.depth(), 1);
}

/// Where a stop happens: in the reading phase, or after some steps.
#[derive(Clone, Copy, Debug)]
pub enum At {
    Reading,
    Stepping,
}

/// A run of the stub on the padded library for `At::Reading`, at the blit of the fourth frame,
/// with two chunks read, or on the proofs library for `At::Stepping`, at the blit of the second
/// frame, EVERY steps in.
fn running(at: At) -> Run {
    let (built, table, blits) = match at {
        At::Reading => (padded(), None, 4),
        At::Stepping => (proofs(), Some(empty_table(proofs())), 2),
    };
    let mut run = Run::boot(built, table.as_deref());
    run.m().press_key(F2);
    run.next_blit();
    run.m().release_key(F2);
    for _ in 1..blits {
        run.next_blit();
    }
    run
}

/// With a check open at its third frame on the stage that ends at `end`: a stop by `stop`, F1 or
/// CLEAR, or ON for none, pops the screen, the table holds "next" as the first assertion not
/// verified and Libraries shows the table's counts; ON held on does not leave the app or reach
/// Libraries; and a second Run carries on and finishes with no assertion verified twice.
pub fn stopped_and_run_again(mut run: Run, at: At, stop: Option<u8>, end: usize) {
    let name = format!("{at:?} stopped by {stop:02X?}");
    let built = run.built;
    assert_eq!(run.depth(), 2, "{name}: the screen is open");
    let keys = run.scr_keys();
    let verifies = run.verifies();
    let crc = run.crc_calls();
    if matches!(at, At::Reading) {
        assert_eq!(crc.0, 1, "{name}: the CRC run began");
        assert!(crc.1 >= 2, "{name}: {} chunks read", crc.1);
    }
    match stop {
        Some(key) => run.tap(key),
        None => run.m().press_key(Machine::KEY_ON),
    }
    run.run(30_000_000);
    let frame = run.frame();
    assert_eq!(run.depth(), 1, "{name}: popped");
    assert_eq!(run.top_kind(), KIND_LIBRARIES, "{name}");
    assert_eq!(
        run.verifies(),
        verifies,
        "{name}: the stop came before the next step"
    );
    let values = run.values();
    match at {
        At::Reading => {
            assert_eq!(values, None, "{name}: reading creates no table");
            same_frame(&name, &frame, &model::libraries(built));
        }
        At::Stepping => {
            let values = values.expect("the table");
            let settled = values.iter().filter(|&&v| v != 0).count();
            assert!(settled > 0 && settled < end, "{name}: part way, {settled}");
            assert!(
                values[..settled].iter().all(|&v| v != 0),
                "{name}: in order"
            );
            assert_eq!(
                usize::from(run.next_number()),
                settled,
                "{name}: next is the first assertion not verified"
            );
            shows(&name, &frame, &model::ui(built, &status_of(&values)));
        }
    }
    if stop.is_none() {
        let blits = run.blits();
        run.run(3_000_000);
        assert_eq!(run.depth(), 1, "{name}: the app did not leave");
        assert_eq!(run.blits(), blits, "{name}: nothing was drawn");
        run.m().release_key(Machine::KEY_ON);
        run.run(3_000_000);
        assert_eq!(run.blits(), blits, "{name}: still nothing");
        assert_eq!(run.scr_keys(), keys, "{name}: no key reached Libraries");
        run.tap(F4);
    }
    assert_eq!(run.depth(), 1);
    run.m().press_key(F2);
    let (last, values) = loop {
        let (frame, values) = run.next_frame();
        if let Some(values) = values
            && values[..end].iter().all(|&v| v != 0)
        {
            break (frame, values);
        }
    };
    run.m().release_key(F2);
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    assert_eq!(
        values[..end],
        expected_values(built, &verdicts, end)[..end],
        "{name}"
    );
    assert_eq!(
        usize::from(run.verifies()),
        end,
        "{name}: none verified twice"
    );
    if matches!(at, At::Reading) {
        let chunks = u16::try_from(chunks_of(built)).unwrap();
        assert_eq!(
            run.crc_calls(),
            (2, crc.1 + chunks),
            "{name}: the second Run began the CRC run again and read every chunk"
        );
    }
    shows(&name, &last, &checking_model(built, &values, 0));
}

#[test]
fn a_stop_in_the_middle_pops_keeps_the_progress_and_a_second_run_finishes_the_job() {
    for at in [At::Reading, At::Stepping] {
        for stop in [Some(F1), Some(CLEAR), None] {
            let end = usize::from(crate::table::header_of(running_library(at)).1);
            stopped_and_run_again(running(at), at, stop, end);
        }
    }
}

fn running_library(at: At) -> &'static [u8] {
    match at {
        At::Reading => padded().library.as_deref().unwrap(),
        At::Stepping => proofs().library.as_deref().unwrap(),
    }
}

/// A check of more than 256 assertions, with a table: the frames' counts are the table's, the table
/// ends whole and complete, and each assertion was verified once.
#[test]
fn a_check_of_more_than_256_assertions_runs_to_the_end() {
    let built = padded();
    let n = usize::from(crate::table::header_of(built.library.as_ref().unwrap()).1);
    assert!(n > 1000, "{n} assertions");
    let mut run = Run::boot(built, Some(&empty_table(built)));
    run.m().press_key(F2);
    let mut frames = 0;
    let values = loop {
        let (frame, values) = run.next_frame();
        frames += 1;
        let values = values.expect("the table is whole at every frame");
        assert_eq!(
            run.top_kind(),
            KIND_CHECKING,
            "frame {frames}: the screen is open"
        );
        let settled = values.iter().filter(|&&v| v != 0).count();
        if settled == n {
            shows("finished", &frame, &checking_model(built, &values, 0));
            break values;
        }
        assert!(frames < 200, "the check never ended");
    };
    run.m().release_key(F2);
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    assert_eq!(values, expected_values(built, &verdicts, n));
    assert_eq!(usize::from(run.verifies()), n);
}
