//! Whole screens replayed on the Z80. Statements, goals, hypotheses and tree lines are laid
//! out by the host, so they arrive as ink layers that the test ORs into the frame where the model
//! drew them; bars are the exception and go through ui_bar. What these tests prove is the
//! layering of the Z80 calls (title, rows, soft keys, bars, inverts, markers and the order they
//! and the ink go in); every replay prints how many of the lit pixels the Z80 calls also light on
//! their own and requires some. Glyph drawing itself is proved in draw.rs and font.rs, and the
//! mutation test below shows that a wrong glyph fails a replay.

#[path = "../../../mcui/tests/support/mod.rs"]
mod model;

use crate::support::*;
use mcui::Key::*;
use mcui::canvas::Call;
use mcui::{State, Ui};
use model::{extra_ui, logic_bytes, open, press, proving_bytes, typed, ui, ui_over, unchecked_ui};
use std::io::Write;

fn lit(frame: &[u8]) -> u32 {
    frame.iter().map(|b| b.count_ones()).sum()
}

/// Pixels lit in both frames.
fn lit_in_both(a: &[u8], b: &[u8]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x & y).count_ones()).sum()
}

/// Replays the calls that drew `ui`'s screen on the Z80 and requires all 768 bytes to equal the
/// model's frame. Returns the number of calls replayed.
fn replay(name: &str, ui: &Ui) -> usize {
    let (frame, calls) = ui.frame_calls();
    assert_eq!(frame, ui.frame(), "{name}: recording changed the frame");
    let script = ops(&calls);
    let got = run(REAL, &script);
    same_frame(name, &got.frame, &pack(&frame));
    let drawn: Vec<Op> = script
        .iter()
        .filter(|op| !matches!(op, Op::Ink(_)))
        .cloned()
        .collect();
    let z80_only = run(REAL, &drawn).frame;
    let (z80, all) = (lit_in_both(&z80_only, &got.frame), lit(&got.frame));
    assert!(z80 > 0, "{name}: no pixel comes from a Z80 call");
    assert!(z80 <= all);
    let _ = writeln!(
        std::io::stderr(),
        "{name}: {z80} of {all} lit pixels are also lit by the Z80 calls alone ({}%)",
        z80 * 100 / all
    );
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
fn a_theorem_list_while_its_preview_waits() {
    let ink = |ui: &Ui| {
        let (_, calls) = ui.frame_calls();
        calls.iter().filter(|c| matches!(c, Call::Ink(_))).count()
    };
    let mut ui = ui();
    press(&mut ui, &[Enter]);
    let settled = ink(&ui);
    ui.press(Down);
    assert!(matches!(ui.state(), State::List { pending: true, .. }));
    let frame = ui.frame();
    assert!((48..56).all(|y| (0..96).all(|x| !frame.get(x, y))));
    assert!(ink(&ui) < settled);
    assert!(replay("list-pending", &ui) >= 4);
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
    let no_ink = |ui: &Ui| {
        let calls = ui.frame_calls().1;
        assert!(
            !calls.iter().any(|c| matches!(c, Call::Ink(_))),
            "the checking screen records host ink, which the replay would OR over a broken bar"
        );
    };
    no_ink(&ui);
    replay("checking-start", &ui);
    for _ in 0..5 {
        ui.step_check();
    }
    no_ink(&ui);
    assert!(replay("checking-midway", &ui) >= 5);
    let bar = |ui: &Ui| {
        ui.frame_calls().1.into_iter().find_map(|c| match c {
            Call::Bar { done, total, .. } => Some((done, total)),
            _ => None,
        })
    };
    let (done, total) = bar(&ui).expect("the checking screen records a bar call");
    assert!(
        0 < done && done < total,
        "a half drawn bar: {done} of {total}"
    );
    while ui.step_check() != mcui::CheckStep::Finished {}
    no_ink(&ui);
    replay("checking-finished", &ui);
}

#[test]
fn the_proof_reading_screen_at_the_root_a_subgoal_and_the_goal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3]);
    replay("proof-root", &ui);
    press(&mut ui, &[Down, Enter]);
    replay("proof-subgoal", &ui);
    let mut ui = self::ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Up]);
    replay("proof-goal", &ui);
}

#[test]
fn the_distinct_variables_screen_short_and_scrolled() {
    let mut ui = ui();
    open(&mut ui, "alid");
    press(&mut ui, &[F2]);
    replay("dist", &ui);
    let mut ui = extra_ui();
    open(&mut ui, "ax-dvs");
    press(&mut ui, &[F2]);
    replay("dist-top", &ui);
    press(&mut ui, &[Down; 10]);
    replay("dist-scrolled", &ui);
}

#[test]
fn a_wrong_glyph_in_the_font_table_fails_a_replay() {
    let ui = ui();
    let (frame, calls) = ui.frame_calls();
    let table = mcglyph::z80::Table::builtin();
    let mut font: Vec<u8> = table.slots().iter().flatten().copied().collect();
    let mut script = vec![Op::SetFont(font.clone())];
    script.extend(ops(&calls));
    same_frame("intact table", &run(REAL, &script).frame, &pack(&frame));
    let l = (usize::from(table.code('L')) - 1) * 8;
    font[l + 2] ^= 0xFF;
    script[0] = Op::SetFont(font);
    let got = run(REAL, &script);
    assert_ne!(
        got.frame,
        pack(&frame),
        "a corrupted glyph row went unnoticed"
    );
}
