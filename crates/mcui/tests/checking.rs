//! Screen 6: checking a library one assertion at a time, against the kernel.

mod support;

use mcui::Key::*;
use mcui::{CheckStep, State, Ui};
use support::{check_frame, logic_bytes, press, refreshed, unchecked_ui};

fn check(ui: &Ui, name: &str) {
    check_frame(&ui.frame(), name);
}

fn checking(state: State) -> (usize, usize, usize, bool) {
    match state {
        State::Checking {
            done,
            total,
            failed,
            finished,
            ..
        } => (done, total, failed, finished),
        other => panic!("not the checking screen: {other:?}"),
    }
}

fn steps(ui: &mut Ui, n: usize) {
    for _ in 0..n {
        ui.step_check();
    }
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

fn bad_proof() -> Vec<u8> {
    let sound = logic_bytes();
    let lib = mcpack::library::read(&sound).unwrap();
    let alid = lib.iter().position(|a| a.label == "alid").unwrap() as u16;
    refreshed(support::patch_reference(&sound, "a1i", 0, alid))
}

#[test]
fn the_run_key_opens_the_checking_screen_at_the_start() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[F2]);
    assert_eq!(ui.depth(), 2);
    assert_eq!(checking(ui.state()), (0, 13, 0, false));
    check(&ui, "checking-start");
}

#[test]
fn checking_advances_one_assertion_per_step() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[F2]);
    assert_eq!(ui.step_check(), CheckStep::Checked);
    assert_eq!(checking(ui.state()), (1, 13, 0, false));
    steps(&mut ui, 4);
    assert_eq!(checking(ui.state()), (5, 13, 0, false));
    check(&ui, "checking-midway");
}

#[test]
fn stop_pauses_and_opening_the_screen_again_resumes() {
    for pause in [F1, Clear, On] {
        let mut ui = unchecked_ui(logic_bytes());
        press(&mut ui, &[F2]);
        steps(&mut ui, 5);
        ui.press_settled(pause);
        assert_eq!(ui.depth(), 1);
        assert_eq!(ui.step_check(), CheckStep::Idle);
        if pause == F1 {
            check(&ui, "checking-paused");
        }
        press(&mut ui, &[F2]);
        assert_eq!(checking(ui.state()), (5, 13, 0, false));
        steps(&mut ui, 8);
        assert_eq!(checking(ui.state()), (13, 13, 0, true));
    }
}

#[test]
fn a_finished_check_says_so_and_stays_finished() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[F2]);
    assert_eq!(run_all(&mut ui), (13, 0));
    assert_eq!(checking(ui.state()), (13, 13, 0, true));
    assert_eq!(ui.step_check(), CheckStep::Finished);
    check(&ui, "checking-finished");
    ui.press_settled(Clear);
    check(&ui, "checking-finished-libraries");
}

#[test]
fn the_first_key_of_a_check_reads_stop_while_it_runs_and_done_when_it_is_finished_and_leaves() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[F2]);
    steps(&mut ui, 3);
    assert!(support::key_label_is(&ui.frame(), 0, "Stop"));
    run_all(&mut ui);
    let frame = ui.frame();
    assert!(support::key_label_is(&frame, 0, "Done"));
    assert!(!support::key_label_is(&frame, 0, "Stop"));
    ui.press_settled(F1);
    assert_eq!(ui.depth(), 1);
    assert!(matches!(ui.state(), State::Libraries { .. }));
}

#[test]
fn a_bad_proof_fails_and_the_marks_follow() {
    let mut ui = unchecked_ui(bad_proof());
    press(&mut ui, &[F2]);
    assert_eq!(run_all(&mut ui), (12, 1));
    assert_eq!(checking(ui.state()), (13, 13, 1, true));
    check(&ui, "checking-failures");
    ui.press_settled(Clear);
    check(&ui, "checking-failures-libraries");
    press(&mut ui, &[Enter, F2]);
    support::typed(&mut ui, "a1i");
    check(&ui, "checking-failures-list");
    press(&mut ui, &[Clear, Clear, F2, Clear]);
    support::open(&mut ui, "syl");
    assert!(support::title_is(&ui.frame(), "syl", "~ thm"));
}

#[test]
fn a_failure_shows_while_the_check_is_still_running() {
    let mut ui = unchecked_ui(bad_proof());
    press(&mut ui, &[F2]);
    steps(&mut ui, 7);
    assert_eq!(checking(ui.state()), (7, 13, 1, false));
    check(&ui, "checking-failure-midway");
}

#[test]
fn a_full_check_marks_every_assertion_on_the_list_and_libraries_screens() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[Enter]);
    press(&mut ui, &[F3, F3, F3]);
    assert!(matches!(ui.state(), State::List { len: 13, .. }));
    press(&mut ui, &[Clear, Clear, F2]);
    run_all(&mut ui);
    ui.press_settled(Clear);
    check(&ui, "checking-all-libraries");
    press(&mut ui, &[Enter, F3, F3, F3]);
    assert!(matches!(ui.state(), State::List { len: 0, .. }));
}

#[test]
fn a_status_from_before_is_kept_and_only_unchecked_assertions_are_run() {
    let mut ui = support::ui_with(&["a2i", "mpd"], &["id"]);
    press(&mut ui, &[F2]);
    assert_eq!(checking(ui.state()), (11, 13, 1, false));
    assert_eq!(run_all(&mut ui), (2, 0));
    assert_eq!(checking(ui.state()), (13, 13, 1, true));
}

#[test]
fn the_run_key_on_my_proofs_is_a_notice() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[Down, F2]);
    assert!(matches!(ui.state(), State::Notice { .. }));
}

#[test]
fn an_empty_library_is_finished_at_once() {
    let db = support::db(support::LOGIC);
    let bytes = mcpack::library::pack(&db, &[]).unwrap();
    let mut ui = unchecked_ui(bytes);
    press(&mut ui, &[F2]);
    assert_eq!(checking(ui.state()), (0, 0, 0, true));
    assert_eq!(ui.step_check(), CheckStep::Finished);
}

#[test]
fn a_library_whose_checksum_is_wrong_says_so_before_it_is_opened_and_does_not_run() {
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    let mut ui = unchecked_ui(bytes);
    let frame = ui.frame();
    let width = mcglyph::Font::builtin().text_width("damaged");
    assert!(support::text_at(&frame, 1, 1, "logic", true));
    assert!(support::text_at(&frame, 95 - width, 1, "damaged", true));
    check(&ui, "libraries-damaged");
    ui.press_settled(F2);
    assert!(matches!(ui.state(), State::Notice { ref title } if title == "Verify"));
    let frame = ui.frame();
    assert!(support::key_label_is(&frame, 0, "Back"));
    assert!(support::text_at(&frame, 1, 0, "Verify", true));
    check(&ui, "checking-damaged");
    ui.press_settled(Clear);
    assert_eq!(ui.depth(), 1);
    assert_eq!(ui.step_check(), CheckStep::Idle);
    press(&mut ui, &[Enter, F3, F3, F3]);
    assert!(matches!(ui.state(), State::List { len: 13, .. }));
}

#[test]
fn nothing_of_a_damaged_library_is_trusted_but_its_installed_table_is_kept() {
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    let table = vec![mcui::Status::Checked; 13];
    let lib = mcui::data::Lib::new(
        mcui::Installed {
            name: "logic".into(),
            bytes: bytes.clone(),
        },
        table.clone(),
    )
    .unwrap();
    assert!(lib.damaged());
    assert_eq!((lib.checked(), lib.failed(), lib.settled()), (0, 0, 0));
    assert!((0..13).all(|i| lib.mark(i) == mcui::Mark::Unchecked));
    assert!((0..13).all(|i| lib.status(i) == mcui::Status::Checked));
    let mut ui = support::ui_over(bytes, &[], &[]);
    press(&mut ui, &[Enter]);
    assert!(support::title_is(&ui.frame(), "logic", "0/13"));
    press(&mut ui, &[F3, F3, F3]);
    assert!(matches!(ui.state(), State::List { len: 13, .. }));
}

#[test]
fn on_does_nothing_off_the_checking_screen() {
    let mut ui = unchecked_ui(logic_bytes());
    ui.press_settled(On);
    assert_eq!(ui.depth(), 1);
    press(&mut ui, &[Enter, On]);
    assert_eq!(ui.depth(), 2);
}

#[test]
fn a_forward_reference_is_not_counted_as_a_failure_before_anything_ran() {
    let sound = logic_bytes();
    let a1i = support::index_of(&sound, "a1i") as u16;
    let mut ui = unchecked_ui(refreshed(support::patch_reference(&sound, "a1i", 0, a1i)));
    press(&mut ui, &[F2]);
    assert_eq!(checking(ui.state()), (0, 13, 0, false));
    assert_eq!(run_all(&mut ui), (12, 1));
    assert_eq!(checking(ui.state()), (13, 13, 1, true));
}

#[test]
fn a_reference_past_the_end_of_the_library_fails_its_assertion_without_a_panic() {
    let sound = logic_bytes();
    let mut ui = unchecked_ui(refreshed(support::patch_reference(&sound, "a1i", 0, 5000)));
    press(&mut ui, &[F2]);
    assert_eq!(run_all(&mut ui), (12, 1));
    assert_eq!(checking(ui.state()), (13, 13, 1, true));
}

/// Steps the real libraries in `MCUI_LIBS` through the kernel and prints how long each took.
#[test]
#[ignore = "needs MCUI_LIBS, a directory of built libraries"]
fn stepping_through_a_real_library_is_quick() {
    let dir = std::env::var("MCUI_LIBS").expect("MCUI_LIBS names the libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        let total = mcpack::library::read(&bytes).unwrap().len();
        let mut ui = unchecked_ui(bytes);
        press(&mut ui, &[F2]);
        let started = std::time::Instant::now();
        let (passed, failed) = run_all(&mut ui);
        println!(
            "{name}: {total} assertions, {passed} passed, {failed} failed in {:?}",
            started.elapsed()
        );
        assert_eq!(passed + failed, total);
        assert_eq!(failed, 0, "{name}");
    }
}

/// Loading the real libraries in `MCUI_LIBS` with every entry checked demotes none of them.
#[test]
#[ignore = "needs MCUI_LIBS, a directory of built libraries"]
fn loading_a_shipped_library_demotes_no_checked_entry() {
    let dir = std::env::var("MCUI_LIBS").expect("MCUI_LIBS names the libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        let total = mcpack::library::read(&bytes).unwrap().len();
        let installed = mcui::data::Installed {
            name: name.into(),
            bytes,
        };
        let lib = mcui::data::Lib::new(installed, vec![mcui::Status::Checked; total]).unwrap();
        let demoted = (0..total).filter(|&i| lib.status(i) != mcui::Status::Checked);
        assert_eq!(demoted.count(), 0, "{name}");
        assert_eq!(lib.checked(), total, "{name}");
    }
}

/// A library of 6000 axioms, several chunks of 16 KB.
fn many_chunks() -> Vec<u8> {
    let mut src = String::from("$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n");
    for n in 1..=6000 {
        src.push_str(&format!("ax{n} $a |- ph $.\n"));
    }
    support::pack_all(&support::db(&src)).expect("the library packs")
}

fn reading(state: State) -> (usize, usize, usize, bool, Option<(usize, usize)>) {
    match state {
        State::Checking {
            done,
            total,
            failed,
            finished,
            reading,
            ..
        } => (done, total, failed, finished, reading),
        other => panic!("not the checking screen: {other:?}"),
    }
}

#[test]
fn a_library_with_no_table_is_read_a_chunk_at_a_time_before_its_assertions() {
    let bytes = many_chunks();
    let chunks = bytes.len().div_ceil(1 << 14);
    assert_eq!(
        chunks, 11,
        "the library is eleven chunks, the last one short"
    );
    assert_ne!(bytes.len() % (1 << 14), 0);
    let mut ui = unchecked_ui(bytes);
    assert!(!ui.begin_reading(), "no checking screen is open");
    assert!(!ui.read_chunk());
    press(&mut ui, &[F2]);
    assert!(!ui.read_chunk(), "the screen is not reading yet");
    assert_eq!(reading(ui.state()).4, None);
    assert!(ui.begin_reading());
    assert_eq!(reading(ui.state()), (0, 6000, 0, false, Some((0, 11))));
    check(&ui, "checking-reading-start");
    for read in 1..=3 {
        assert!(ui.read_chunk());
        assert_eq!(reading(ui.state()).4, Some((read, 11)));
    }
    check(&ui, "checking-reading-midway");
    for read in 4..=11 {
        assert!(ui.read_chunk());
        let want = if read < 11 { Some((read, 11)) } else { None };
        assert_eq!(reading(ui.state()).4, want);
        if read == 10 {
            check(&ui, "checking-reading-last-chunk");
        }
    }
    assert_eq!(reading(ui.state()), (0, 6000, 0, false, None));
    assert!(!ui.read_chunk());
    assert_eq!(ui.step_check(), CheckStep::Checked);
}

#[test]
fn the_reading_phase_has_no_assertion_label_and_no_failed_count_and_the_stop_key() {
    let mut ui = unchecked_ui(many_chunks());
    press(&mut ui, &[F2]);
    ui.begin_reading();
    let frame = ui.frame();
    let font = mcglyph::Font::builtin();
    assert!(support::text_at(&frame, 1, 0, "logic", true));
    assert!(support::text_at(
        &frame,
        95 - font.text_width("0/6000"),
        0,
        "0/6000",
        true
    ));
    let width = font.text_width("Reading library");
    assert!(support::text_at(
        &frame,
        (96 - width) / 2,
        1,
        "Reading library",
        false
    ));
    assert!(
        (24..32).all(|y| (0..96).all(|x| !frame.get(x, y))),
        "the row of the assertion's label is blank"
    );
    let failed = font.text_width("0 failed");
    assert!(!support::text_at(
        &frame,
        (96 - failed) / 2,
        5,
        "0 failed",
        false
    ));
    let pauses = font.text_width("ON pauses");
    assert!(support::text_at(
        &frame,
        (96 - pauses) / 2,
        6,
        "ON pauses",
        false
    ));
    assert!(support::key_label_is(&frame, 0, "Stop"));
}

#[test]
fn stop_pops_the_reading_phase_and_opening_the_screen_again_starts_with_the_assertions() {
    for pause in [F1, Clear, On] {
        let mut ui = unchecked_ui(many_chunks());
        press(&mut ui, &[F2]);
        ui.begin_reading();
        assert!(ui.read_chunk());
        ui.press_settled(pause);
        assert_eq!(ui.depth(), 1);
        press(&mut ui, &[F2]);
        assert_eq!(reading(ui.state()), (0, 6000, 0, false, None));
    }
}
