#[path = "../../../mcui/tests/support/mod.rs"]
mod model;

use crate::support::*;
use mcui::Key::*;
use mcui::{State, Ui};
use model::{extra_ui, logic_bytes, open, press, proving_bytes, typed, ui, ui_over, unchecked_ui};

/// Replays the calls that drew `ui`'s screen on the Z80 and requires all 768 bytes to equal the
/// model's frame; the pixels of laid out tokens, which the Z80 does not draw, are ORed in by the
/// test where the model drew them. Returns the number of calls replayed.
fn replay(name: &str, ui: &Ui) -> usize {
    let (frame, calls) = ui.frame_calls();
    assert_eq!(frame, ui.frame(), "{name}: recording changed the frame");
    let got = run(REAL, &ops(&calls));
    same_frame(name, &got.frame, &pack(&frame));
    calls.len()
}

#[test]
fn the_libraries_screen() {
    assert!(replay("libraries", &ui()) >= 3);
}

#[test]
fn a_theorem_list_with_the_cursor_below_the_first_row() {
    let mut ui = ui();
    press(&mut ui, &[Enter, Down, Down]);
    assert!(replay("list", &ui) >= 5);
}

#[test]
fn a_theorem_list_with_a_find_prefix_that_matches_nothing() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "synx");
    replay("list-find-miss", &ui);
}

#[test]
fn a_theorem_list_with_a_long_find_prefix() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "abcdefghijklmnopqrstuvwxyzabcd");
    replay("list-find-long", &ui);
}

#[test]
fn the_about_screen() {
    let mut ui = ui();
    press(&mut ui, &[F3]);
    assert!(replay("about", &ui) >= 4);
}

#[test]
fn a_notice_that_wraps_at_word_boundaries() {
    let mut broken = logic_bytes();
    broken[4] ^= 0xFF;
    let mut ui = ui_over(broken, &[], &[]);
    open(&mut ui, "syl");
    press(&mut ui, &[F3]);
    assert!(replay("notice-no-export", &ui) >= 4);
}

#[test]
fn a_notice_for_a_syntax_theorem() {
    let mut ui = extra_ui();
    open(&mut ui, "wtwice");
    press(&mut ui, &[F3]);
    replay("notice-syntax", &ui);
}

#[test]
fn a_walk_through_two_libraries_replays_exactly() {
    let mut replayed = 0;
    for (name, build) in [
        ("libraries", ui as fn() -> Ui),
        ("extra", extra_ui as fn() -> Ui),
    ] {
        let mut ui = build();
        for keys in [
            &[][..],
            &[Enter],
            &[Down],
            &[F2],
            &[F3],
            &[F4],
            &[Up],
            &[Enter],
        ] {
            press(&mut ui, keys);
            replay(name, &ui);
            replayed += 1;
        }
    }
    assert!(replayed >= 12, "only {replayed} frames replayed");
}

#[test]
fn theorem_screens_with_a_cursor_and_a_marker_replay_exactly() {
    for (theorem, keys) in [
        ("syl", &[Down, Right, Left, Down, Down, Enter, Up][..]),
        ("ax-many", &[Down, Right, Down, Down, Left]),
        ("ax-2", &[Down, Right, Enter, Left, Left]),
    ] {
        let mut ui = extra_ui();
        open(&mut ui, theorem);
        replay(theorem, &ui);
        for key in keys {
            press(&mut ui, &[*key]);
            replay(theorem, &ui);
        }
    }
}

fn prove(label: &str) -> Ui {
    let mut ui = ui_over(proving_bytes(), &[], &[]);
    open(&mut ui, label);
    ui.press(F4);
    ui
}

/// Applies candidate `label` of the open goal.
fn aply(ui: &mut Ui, label: &str) {
    for _ in 0..300 {
        match ui.state() {
            State::Proving { candidate, .. } if candidate.as_deref() == Some(label) => {
                ui.press(F1);
                return;
            }
            _ => ui.press(Down),
        }
    }
    panic!("no candidate {label}");
}

#[test]
fn the_proving_screen_at_a_root_goal_and_after_applying() {
    let mut ui = prove("a1i");
    assert!(replay("proving-root", &ui) >= 3);
    press(&mut ui, &[F2]);
    replay("proving-hyps", &ui);
    press(&mut ui, &[Clear]);
    replay("proving-back", &ui);
}

#[test]
fn the_tree_outline_and_its_cursor() {
    let mut ui = prove("mpd");
    aply(&mut ui, "ax-mp");
    aply(&mut ui, "a2i");
    replay("proving-goal", &ui);
    press(&mut ui, &[F5]);
    replay("proving-tree", &ui);
    for key in [Down, Down, Up, Enter, F5, Down] {
        press(&mut ui, &[key]);
        replay("proving-tree-walk", &ui);
    }
}

#[test]
fn the_checking_screen_at_the_start_midway_and_finished() {
    let mut ui = unchecked_ui(logic_bytes());
    press(&mut ui, &[F2]);
    replay("checking-start", &ui);
    for _ in 0..5 {
        ui.step_check();
    }
    assert!(replay("checking-midway", &ui) >= 5);
    while ui.step_check() != mcui::CheckStep::Finished {}
    replay("checking-finished", &ui);
}
