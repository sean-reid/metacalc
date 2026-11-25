//! Every key the app can produce, on every row of the Libraries screen and on the stage About,
//! against what the host model does with it: the screen, the depth, the frame, the position the
//! frame keeps, and whether a frame is blitted. A key that reaches scr_key is counted there.

use crate::app::*;
use crate::builds::*;
use crate::shellrig::*;
use mcui::{Key, State};

/// One key the app can produce: the scancodes that make it, the last being the key itself, and
/// the model's key for it, if the model has one.
pub struct Probe {
    pub name: &'static str,
    pub scans: &'static [u8],
    pub model: Option<Key>,
}

pub const PROBES: [Probe; 14] = [
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

/// A table of `built`'s library in which every assertion has a value, one in five failed, and the
/// statuses the model has for it: a check of any stage has nothing to verify.
pub fn settled_table(built: &Built) -> (Vec<u8>, Vec<mcui::Status>) {
    let library = built.library.as_ref().unwrap();
    let n = usize::from(crate::table::header_of(library).1);
    let status: Vec<mcui::Status> = (0..n)
        .map(|i| match i % 5 {
            4 => mcui::Status::Failed,
            _ => mcui::Status::Checked,
        })
        .collect();
    let data = crate::table::of(library, &crate::table::values(built, &status));
    (data, status)
}

/// F2 on a stage row of a library with every assertion settled: the checking screen opens and the
/// first idle call finishes it with nothing to verify, so the push and that frame are one or two
/// blits and the frame left is the model's.
pub fn opens_a_check(shell: &mut Shell, ui: &mut mcui::Ui, at: &str) {
    let blits = shell.blits();
    shell.tap(F2);
    ui.press(Key::F2);
    assert!(
        matches!(ui.state(), State::Checking { .. }),
        "{at}: the model's check"
    );
    shell.shows(at, &model::frame(ui));
    assert_eq!(
        (shell.depth(), shell.top_kind()),
        (ui.depth(), KIND_CHECKING),
        "{at}"
    );
    let drawn = shell.blits() - blits;
    assert!((1..=2).contains(&drawn), "{at}: {drawn} frames");
}

#[test]
fn every_key_the_app_can_produce_does_on_every_row_what_the_model_does() {
    for built in builds() {
        for row in 0..rows(built) {
            for probe in PROBES.iter().filter(|p| p.name != "clear") {
                let at = format!("{} stages, row {row}, {}", built.stages.len(), probe.name);
                if probe.name == "f2" && row + 1 < rows(built) {
                    let (data, status) = settled_table(built);
                    let mut shell = booted(built, Some(&data));
                    let mut ui = model::ui(built, &status);
                    for _ in 0..row {
                        press(&mut shell, &mut ui, built, DOWN, &at);
                    }
                    opens_a_check(&mut shell, &mut ui, &at);
                    continue;
                }
                let (mut shell, mut ui) = on_row(built, row);
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
                frame(KIND_CHECKING, row as u8, 0),
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

/// Whether the model's own Notice, the one My proofs opens on F1, closes on `key`.
pub fn pops_a_notice(key: Option<Key>) -> bool {
    let mut ui = model::ui(plain(), &[]);
    ui.press(Key::F1);
    assert_eq!(ui.depth(), 2, "the model opened a notice");
    assert_eq!(
        ui.state(),
        mcui::State::Notice {
            title: "My proofs".into()
        },
        "it is the notice of My proofs"
    );
    if let Some(key) = key {
        ui.press(key);
    }
    ui.depth() == 1
}

/// The notice that stands in for the model's theorem list or checking screen, opened by `open` on
/// `row`: the model has no Notice there, so the oracle is the notice's rule, that the keys that
/// close the model's Notice pop and the rest change nothing, over the model's frame for the
/// Libraries screen under it and its own frame for the stand-in.
#[test]
fn every_key_the_app_can_produce_does_on_the_notices_of_a_stage_what_the_models_notice_does() {
    for built in [three_stages(), six_stages()] {
        for row in 0..built.stages.len() {
            for open in [F1, ENTER] {
                for probe in &PROBES {
                    let (mut shell, ui) = on_row(built, row);
                    let title = built.stage_name(row);
                    let at = format!(
                        "{} stages, the notice of row {row} from {open:02X}, {}",
                        built.stages.len(),
                        probe.name
                    );
                    shell.tap(open);
                    assert_eq!((shell.depth(), shell.top_kind()), (2, KIND_NOTICE), "{at}");
                    shell.shows(&at, &model::unavailable(title));
                    let blits = shell.blits();
                    for &modifier in &probe.scans[..probe.scans.len() - 1] {
                        shell.tap_modifier(modifier);
                    }
                    shell.tap(*probe.scans.last().unwrap());
                    if pops_a_notice(probe.model) {
                        assert_eq!(shell.depth(), 1, "{at}: popped");
                        shell.shows(&at, &model::frame(&ui));
                        assert_eq!(shell.blits(), blits + 1, "{at}: one frame");
                    } else {
                        assert_eq!(shell.depth(), 2, "{at}: stays");
                        shell.shows(&at, &model::unavailable(title));
                        assert_eq!(shell.blits(), blits, "{at}: no frame");
                    }
                }
            }
        }
    }
}
