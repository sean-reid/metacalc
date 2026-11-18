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
        ui.press(pause);
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
    ui.press(Clear);
    check(&ui, "checking-finished-libraries");
}

#[test]
fn a_bad_proof_fails_and_the_marks_follow() {
    let mut ui = unchecked_ui(bad_proof());
    press(&mut ui, &[F2]);
    assert_eq!(run_all(&mut ui), (12, 1));
    assert_eq!(checking(ui.state()), (13, 13, 1, true));
    check(&ui, "checking-failures");
    ui.press(Clear);
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
    ui.press(Clear);
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
fn a_library_whose_checksum_is_wrong_fails_every_assertion() {
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    let mut ui = unchecked_ui(bytes);
    press(&mut ui, &[F2]);
    assert_eq!(run_all(&mut ui), (0, 13));
}

#[test]
fn on_does_nothing_off_the_checking_screen() {
    let mut ui = unchecked_ui(logic_bytes());
    ui.press(On);
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
