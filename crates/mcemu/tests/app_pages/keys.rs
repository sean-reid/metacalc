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
                if probe.name == "f2" && row + 1 < rows(built) {
                    continue;
                }
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
fn pops_a_notice(key: Option<Key>) -> bool {
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

const WALK: [u8; 34] = [
    F3, CLEAR, ENTER, F1, F4, CLEAR, DOWN, F3, ENTER, F1, CLEAR, DOWN, F4, F1, UP, F3, CLEAR, DOWN,
    DOWN, F3, CLEAR, F1, ENTER, F4, CLEAR, DOWN, F3, CLEAR, UP, UP, UP, UP, F3, CLEAR,
];

/// One session, one key after another, the screen and the model compared after each: a fixed walk
/// through every kind of screen, then a pseudo-random stream of keys, never CLEAR on the Libraries
/// screen, which leaves. A stage's list and checking screens are notices in the app; while one is
/// open the oracle is the notice's rule (see above) and the model rests on the Libraries screen
/// under it, which is rebuilt by replaying the keys it took.
#[test]
fn a_long_walk_through_every_kind_of_screen_in_one_session_is_the_models_at_every_step() {
    let stream = [
        UP, DOWN, F1, F4, F3, ENTER, CLEAR, F4, F5, LEFT, RIGHT, DOWN, F3, F1,
    ];
    for built in [three_stages(), six_stages()] {
        let library = built.library.as_ref().unwrap();
        let n = usize::from(crate::table::header_of(library).1);
        let status: Vec<mcui::Status> = (0..n)
            .map(|i| match i % 3 {
                0 => mcui::Status::Checked,
                _ => mcui::Status::Unchecked,
            })
            .collect();
        let data = crate::table::of(library, &crate::table::values(built, &status));
        let mut shell = booted(built, Some(&data));
        let mut ui = model::ui(built, &status);
        let mut log: Vec<Key> = Vec::new();
        let mut stand_in: Option<String> = None;
        let mut kinds = std::collections::BTreeSet::new();
        let mut seed = 0x9E37_79B9_u32;
        let mut keys = WALK.to_vec();
        keys.extend([DOWN; 7]);
        keys.extend([F3, CLEAR, UP, F4, CLEAR]);
        for _ in 0..160 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            keys.push(stream[seed as usize % stream.len()]);
        }
        for (step, &key) in keys.iter().enumerate() {
            let at = format!("{} stages, step {step}", built.stages.len());
            if let Some(title) = stand_in.clone() {
                shell.tap(key);
                if pops_a_notice(Some(key_of(key))) {
                    stand_in = None;
                    shell.shows(&at, &model::frame(&ui));
                    assert_eq!(shell.depth(), 1, "{at}: popped");
                } else {
                    shell.shows(&at, &model::unavailable(&title));
                    assert_eq!(shell.depth(), 2, "{at}: stays");
                }
            } else {
                if key == CLEAR && shell.depth() == 1 {
                    continue;
                }
                if press(&mut shell, &mut ui, built, key, &at) {
                    stand_in = state_title(built, &ui);
                    ui = model::ui(built, &status);
                    for &k in &log {
                        ui.press(k);
                    }
                } else {
                    log.push(key_of(key));
                }
            }
            kinds.insert(shell.top_kind());
        }
        let all: std::collections::BTreeSet<u8> =
            [KIND_LIBRARIES, KIND_NOTICE, KIND_ABOUT, KIND_STAGE].into();
        assert_eq!(
            kinds,
            all,
            "{} stages: every kind was shown",
            built.stages.len()
        );
    }
}

/// The title of the notice that stands in for the model's screen: the stage's name for its list,
/// Verify for the checking screen.
fn state_title(built: &Built, ui: &mcui::Ui) -> Option<String> {
    match ui.state() {
        mcui::State::List { stage, .. } => Some(built.stage_name(stage).to_string()),
        mcui::State::Checking { .. } => Some("Verify".to_string()),
        _ => None,
    }
}
