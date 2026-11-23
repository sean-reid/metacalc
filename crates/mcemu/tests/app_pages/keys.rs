//! Every key the app can produce, on every row of the Libraries screen and on the stage About,
//! against what the host model does with it: the screen, the depth, the frame, the position the
//! frame keeps, and whether a frame is blitted. A key that reaches scr_key is counted there.

use crate::app::*;
use crate::builds::*;
use crate::shellrig::*;
use mcui::Key;

/// One key the app can produce: the scancodes that make it, the last being the key itself, and
/// the model's key for it, if the model has one.
struct Probe {
    name: &'static str,
    scans: &'static [u8],
    model: Option<Key>,
}

const PROBES: [Probe; 14] = [
    Probe {
        name: "up",
        scans: &[UP],
        model: Some(Key::Up),
    },
    Probe {
        name: "down",
        scans: &[DOWN],
        model: Some(Key::Down),
    },
    Probe {
        name: "left",
        scans: &[LEFT],
        model: Some(Key::Left),
    },
    Probe {
        name: "right",
        scans: &[RIGHT],
        model: Some(Key::Right),
    },
    Probe {
        name: "enter",
        scans: &[ENTER],
        model: Some(Key::Enter),
    },
    Probe {
        name: "clear",
        scans: &[CLEAR],
        model: Some(Key::Clear),
    },
    Probe {
        name: "f1",
        scans: &[F1],
        model: Some(Key::F1),
    },
    Probe {
        name: "f2",
        scans: &[F2],
        model: Some(Key::F2),
    },
    Probe {
        name: "f3",
        scans: &[F3],
        model: Some(Key::F3),
    },
    Probe {
        name: "f4",
        scans: &[F4],
        model: Some(Key::F4),
    },
    Probe {
        name: "f5",
        scans: &[F5],
        model: Some(Key::F5),
    },
    Probe {
        name: "a letter",
        scans: &[LETTER],
        model: Some(Key::Letter('A')),
    },
    Probe {
        name: "2nd and 1, no key",
        scans: &[SECOND, ONE],
        model: None,
    },
    Probe {
        name: "a digit",
        scans: &[ONE],
        model: Some(Key::Letter('1')),
    },
];

/// A shell on `built` with the cursor on `row`, and the model with it.
fn on_row(built: &'static Built, row: usize) -> (Shell, mcui::Ui) {
    let mut shell = booted(built, None);
    let mut ui = model::ui(built, &[]);
    for _ in 0..row {
        press(
            &mut shell,
            &mut ui,
            built,
            DOWN,
            &format!("on the way to row {row}"),
        );
    }
    (shell, ui)
}

#[test]
fn every_key_the_app_can_produce_does_on_every_row_what_the_model_does() {
    for built in builds() {
        for row in 0..rows(built) {
            for probe in PROBES.iter().filter(|p| p.name != "clear") {
                let (mut shell, mut ui) = on_row(built, row);
                let at = format!("{} stages, row {row}, {}", built.stages.len(), probe.name);
                press_keys(&mut shell, &mut ui, built, probe.scans, probe.model, &at);
            }
        }
    }
}

#[test]
fn every_key_the_app_can_produce_does_on_about_and_the_notices_of_my_proofs_what_the_model_does() {
    for built in [plain(), three_stages()] {
        let last = rows(built) - 1;
        for open in [F3, F1, F2] {
            for probe in &PROBES {
                let (mut shell, mut ui) = on_row(built, last);
                let at = format!(
                    "{} stages, My proofs, {open:02X}, {}",
                    built.stages.len(),
                    probe.name
                );
                press(&mut shell, &mut ui, built, open, &at);
                assert_eq!(shell.depth(), 2, "{at}: opened");
                press_keys(&mut shell, &mut ui, built, probe.scans, probe.model, &at);
            }
        }
    }
}

#[test]
fn clear_leaves_the_app_from_every_row() {
    for built in builds() {
        for row in 0..rows(built) {
            let (mut shell, _) = on_row(built, row);
            shell.dirty_dead();
            shell.leave(CLEAR);
            shell.assert_regions_put_away();
        }
    }
}

#[test]
fn every_key_the_app_can_produce_does_on_the_stage_about_what_the_model_does() {
    for built in [three_stages(), six_stages()] {
        for row in 0..built.stages.len() {
            for probe in &PROBES {
                let (mut shell, mut ui) = on_row(built, row);
                let at = format!(
                    "{} stages, the About of row {row}, {}",
                    built.stages.len(),
                    probe.name
                );
                press(&mut shell, &mut ui, built, F3, &at);
                assert_eq!(shell.top_kind(), KIND_STAGE, "{at}: opened");
                press_keys(&mut shell, &mut ui, built, probe.scans, probe.model, &at);
            }
        }
    }
}

#[test]
fn the_cursor_stops_at_both_ends_and_the_first_row_shown_follows_it() {
    let built = six_stages();
    let mut shell = booted(built, None);
    let mut ui = model::ui(built, &[]);
    let at = |cursor, top| mcui::State::Libraries { cursor, top };
    for step in 0..5 {
        press(&mut shell, &mut ui, built, DOWN, &format!("down {step}"));
    }
    assert_eq!(ui.state(), at(5, 0), "the window has not moved yet");
    for step in 0..4 {
        press(
            &mut shell,
            &mut ui,
            built,
            DOWN,
            &format!("down to the end {step}"),
        );
    }
    assert_eq!(
        ui.state(),
        at(6, 1),
        "on My proofs with the window at the bottom"
    );
    for step in 0..6 {
        press(&mut shell, &mut ui, built, UP, &format!("up {step}"));
    }
    assert_eq!(ui.state(), at(0, 0), "back on the first row");
    for step in 0..3 {
        press(
            &mut shell,
            &mut ui,
            built,
            UP,
            &format!("up at the top {step}"),
        );
    }
    for step in 0..6 {
        press(
            &mut shell,
            &mut ui,
            built,
            DOWN,
            &format!("down again {step}"),
        );
    }
    for step in 0..2 {
        press(
            &mut shell,
            &mut ui,
            built,
            UP,
            &format!("up from the bottom {step}"),
        );
    }
    assert_eq!(
        ui.state(),
        at(4, 1),
        "the window stays while the cursor is in it"
    );
    for step in 0..4 {
        press(
            &mut shell,
            &mut ui,
            built,
            UP,
            &format!("up past the window {step}"),
        );
    }
    assert_eq!(ui.state(), at(0, 0), "the window follows the cursor up");
}

#[test]
fn the_frames_the_keys_push_hold_the_notice_or_the_stage() {
    let built = three_stages();
    for row in 0..rows(built) {
        let stage = row < built.stages.len();
        let frame = |kind: u8, byte1: u8, byte2: u8| {
            let mut frame = vec![0u8; 16];
            frame[..3].copy_from_slice(&[kind, byte1, byte2]);
            frame
        };
        let (about, run, open) = if stage {
            (
                frame(KIND_STAGE, row as u8, 0),
                frame(KIND_NOTICE, NOTICE_VERIFY, 0),
                frame(KIND_NOTICE, NOTICE_LIST, row as u8),
            )
        } else {
            (
                frame(KIND_ABOUT, 0, 0),
                frame(KIND_NOTICE, NOTICE_PICK, 0),
                frame(KIND_NOTICE, NOTICE_PROOFS, 0),
            )
        };
        for (keys, want) in [
            (&[F3][..], about),
            (&[F2][..], run),
            (&[F1, ENTER][..], open),
        ] {
            for &key in keys {
                let (mut shell, _) = on_row(built, row);
                shell.tap(key);
                assert_eq!(shell.frames()[1], want, "row {row}, key {key:02X}");
            }
        }
    }
}
