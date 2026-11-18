//! Whole frames, pixel for pixel, against text art checked in under tests/frames. Set
//! MCUI_BLESS=1 to write the current frames instead of comparing.

mod support;

use mcui::Key::*;
use mcui::Ui;
use std::path::PathBuf;
use support::{open, press, typed, ui};

fn check(ui: &Ui, name: &str) {
    let frame = ui.frame();
    assert_eq!((frame.width(), frame.height()), (96, 64), "{name}");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/frames")
        .join(format!("{name}.txt"));
    let got = format!("{frame}\n");
    if std::env::var_os("MCUI_BLESS").is_some() {
        std::fs::write(&path, got).expect("write frame");
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        got == want,
        "{name} differs\n--- got\n{got}--- want\n{want}"
    );
}

#[test]
fn libraries() {
    check(&ui(), "libraries");
}

#[test]
fn list() {
    let mut ui = ui();
    press(&mut ui, &[Enter, Down, Down]);
    check(&ui, "list");
}

#[test]
fn list_find_with_no_match() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "synx");
    check(&ui, "list-find-miss");
}

#[test]
fn theorem_with_hypotheses() {
    let mut ui = ui();
    open(&mut ui, "syl");
    check(&ui, "theorem");
}

#[test]
fn theorem_folded_to_fit_with_the_cursor_below_the_root() {
    let mut ui = ui();
    open(&mut ui, "ax-many");
    press(&mut ui, &[Down, Right]);
    check(&ui, "theorem-scrolled");
}

#[test]
fn theorem_after_folding_under_the_cursor() {
    let mut ui = ui();
    open(&mut ui, "ax-2");
    press(&mut ui, &[Down, Right, Enter]);
    check(&ui, "theorem-folded");
}

#[test]
fn distinct_variables() {
    let mut ui = ui();
    open(&mut ui, "alid");
    press(&mut ui, &[F2]);
    check(&ui, "dist");
}

#[test]
fn proof_root() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3]);
    check(&ui, "proof-root");
}

#[test]
fn proof_subgoal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Down, Enter]);
    check(&ui, "proof-subgoal");
}

#[test]
fn not_available_yet() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F4]);
    check(&ui, "unavailable");
}

#[test]
fn about() {
    let mut ui = ui();
    press(&mut ui, &[F3]);
    check(&ui, "about");
}

#[test]
fn theorem_with_the_cursor_on_a_hypothesis_root() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[Left]);
    check(&ui, "theorem-hypothesis");
}

#[test]
fn proof_with_the_cursor_on_the_goal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Up]);
    check(&ui, "proof-goal");
}
