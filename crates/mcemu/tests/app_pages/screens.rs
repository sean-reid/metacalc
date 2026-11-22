//! The shell's three screen kinds on the real app: the start screen, Notice and About, against
//! the frames and the key handling of the host model, and the exits from depth 1 and depth 2.

use crate::app::*;
use crate::shellrig::*;
use mcui::{Key, State};

/// The two builds the stub runs: no library, and the fixture library.
fn builds() -> [(&'static Built, usize); 2] {
    [(plain(), 0), (with_library(), 1)]
}

fn start_of(built: &Built) -> Vec<u8> {
    model::start(built.library.as_deref())
}

fn booted(built: &'static Built, enter: Enter) -> Shell {
    let mut shell = Shell::boot(built, enter);
    let want = start_of(built);
    shell.shows("the start screen", &want);
    assert_eq!((shell.depth(), shell.blits()), (1, 1));
    shell
}

#[test]
fn about_and_each_notice_are_the_frames_of_the_model() {
    for (built, n) in builds() {
        let mut shell = booted(built, Enter::Real);
        let screens = [
            (F3, "About", model::about(n)),
            (F1, "Theorems", model::unavailable("Theorems")),
            (ENTER, "Theorems", model::unavailable("Theorems")),
            (F2, "Verify", model::unavailable("Verify")),
        ];
        for (open, name, want) in screens {
            shell.tap(open);
            shell.shows(name, &want);
            assert_eq!(shell.lcd_bytes(), want, "{name}: the LCD");
            assert_eq!(shell.depth(), 2);
            shell.tap(CLEAR);
            let start = start_of(built);
            shell.shows("the start screen again", &start);
            assert_eq!(shell.lcd_bytes(), start);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Shown {
    Start,
    About,
    Theorems,
    Verify,
}

fn key_of(scan: u8) -> Key {
    match scan {
        F1 => Key::F1,
        F2 => Key::F2,
        F3 => Key::F3,
        ENTER => Key::Enter,
        CLEAR => Key::Clear,
        _ => panic!("no key for scancode {scan:02X}"),
    }
}

/// The kind of screen each of the model's states is in the app's frames.
fn kind_of(state: &State) -> u8 {
    match state {
        State::Libraries { .. } => KIND_START,
        State::Notice { .. } => KIND_NOTICE,
        State::About => KIND_ABOUT,
        other => panic!("the model is on {other:?}"),
    }
}

#[test]
fn the_keys_open_and_close_screens_as_the_model_does() {
    use Shown::*;
    let walks: [&[(u8, Shown)]; 8] = [
        &[(F3, About), (CLEAR, Start)],
        &[(F3, About), (ENTER, Start)],
        &[(F1, Theorems), (F1, Start)],
        &[(F2, Verify), (CLEAR, Start)],
        &[(ENTER, Theorems), (CLEAR, Start)],
        &[(F1, Theorems), (ENTER, Start)],
        &[(F2, Verify), (F1, Start)],
        &[
            (F3, About),
            (ENTER, Start),
            (F1, Theorems),
            (CLEAR, Start),
            (F2, Verify),
            (ENTER, Start),
        ],
    ];
    for (built, n) in builds() {
        let mut shell = booted(built, Enter::Real);
        for walk in walks {
            let mut ui = model::ui(n);
            for &(scan, shown) in walk {
                let blits = shell.blits();
                shell.tap(scan);
                ui.press(key_of(scan));
                let want = match shown {
                    Start => start_of(built),
                    About => model::about(n),
                    Theorems => model::unavailable("Theorems"),
                    Verify => model::unavailable("Verify"),
                };
                let at = format!("{n} libraries, {walk:?}, after {scan:02X}");
                shell.shows(&at, &want);
                assert_eq!(shell.depth(), ui.depth(), "{at}: the depth");
                if ui.depth() > 1 {
                    assert_eq!(shell.top_kind(), kind_of(&ui.state()), "{at}: the screen");
                }
                assert_eq!(shell.blits(), blits + 1, "{at}: one frame");
            }
            assert_eq!(shell.depth(), 1);
        }
    }
}

/// One key the app can produce, the scancodes that make it (the last is the key itself), and the
/// model's key for it, if the model has one.
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

/// What each key does on the start screen, which the model does not have (its first screen is the
/// libraries list): the table in screens.inc's header. F1 and ENTER open the notice of the
/// theorems, F2 the notice of the verifier, F3 About, and CLEAR leaves; the rest stay.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Start {
    Stay,
    About,
    Theorems,
    Verify,
    Leaves,
}

fn on_start(probe: &Probe) -> Start {
    match probe.name {
        "f3" => Start::About,
        "f1" | "enter" => Start::Theorems,
        "f2" => Start::Verify,
        "clear" => Start::Leaves,
        _ => Start::Stay,
    }
}

fn opens(shown: Start, n: usize) -> (Vec<u8>, u8) {
    match shown {
        Start::About => (model::about(n), KIND_ABOUT),
        Start::Theorems => (model::unavailable("Theorems"), KIND_NOTICE),
        Start::Verify => (model::unavailable("Verify"), KIND_NOTICE),
        _ => unreachable!(),
    }
}

#[test]
fn every_key_the_app_can_produce_does_on_each_screen_what_the_model_does() {
    for (built, n) in builds() {
        let start = start_of(built);
        let mut shell = booted(built, Enter::Real);
        // The start screen, from the table; CLEAR is run apart, since it ends the app.
        for probe in &PROBES {
            let want = on_start(probe);
            if want == Start::Leaves {
                continue;
            }
            let at = format!("{n} libraries, {} on the start screen", probe.name);
            let blits = shell.blits();
            for &scan in &probe.scans[..probe.scans.len() - 1] {
                shell.tap_modifier(scan);
            }
            shell.tap(*probe.scans.last().unwrap());
            if want == Start::Stay {
                assert_eq!((shell.depth(), shell.blits()), (1, blits), "{at}");
                shell.shows(&at, &start);
            } else {
                let (frame, kind) = opens(want, n);
                assert_eq!((shell.depth(), shell.blits()), (2, blits + 1), "{at}");
                assert_eq!(shell.top_kind(), kind, "{at}");
                shell.shows(&at, &frame);
                shell.tap(CLEAR);
            }
        }
        // About and both notices, from the model.
        for (open, scan, shown) in [
            (Key::F3, F3, Start::About),
            (Key::F1, F1, Start::Theorems),
            (Key::F2, F2, Start::Verify),
        ] {
            for probe in &PROBES {
                let at = format!("{n} libraries, {} on {shown:?}", probe.name);
                let mut ui = model::ui(n);
                ui.press(open);
                let state = ui.state();
                if let Some(key) = probe.model {
                    ui.press(key);
                }
                let popped = ui.depth() == 1;
                if !popped {
                    assert_eq!(
                        (ui.depth(), ui.state()),
                        (2, state),
                        "{at}: the model stays"
                    );
                }
                shell.tap(scan);
                let (frame, _) = opens(shown, n);
                shell.shows(&at, &frame);
                let blits = shell.blits();
                for &s in &probe.scans[..probe.scans.len() - 1] {
                    shell.tap_modifier(s);
                }
                shell.tap(*probe.scans.last().unwrap());
                if popped {
                    assert_eq!((shell.depth(), shell.blits()), (1, blits + 1), "{at}");
                    shell.shows(&at, &start);
                } else {
                    assert_eq!((shell.depth(), shell.blits()), (2, blits), "{at}");
                    shell.shows(&at, &frame);
                    shell.tap(CLEAR);
                }
                assert_eq!(shell.depth(), 1, "{at}");
            }
        }
    }
}

#[test]
fn second_then_a_key_the_app_ignores_blits_nothing() {
    let mut shell = booted(plain(), Enter::Real);
    shell.tap_modifier(SECOND);
    shell.tap(ONE);
    assert_eq!((shell.blits(), shell.depth()), (1, 1));
}

#[test]
fn clear_on_the_start_screen_leaves() {
    for (built, _) in builds() {
        let mut shell = booted(built, Enter::Real);
        shell.dirty_dead();
        shell.leave(CLEAR);
        assert_regions_blank(&mut shell);
    }
}

fn assert_regions_blank(shell: &mut Shell) {
    shell.assert_regions_put_away();
}

#[test]
fn quit_leaves_from_depth_2_and_fills_the_two_regions_as_it_does_from_depth_1() {
    for (built, _) in builds() {
        for open in [None, Some(F3), Some(F1), Some(F2)] {
            let mut shell = booted(built, Enter::Real);
            if let Some(open) = open {
                shell.tap(open);
            }
            assert_eq!(shell.depth(), if open.is_some() { 2 } else { 1 });
            shell.tap_modifier(SECOND);
            shell.dirty_dead();
            let (neighbour_low, neighbour_high) = {
                let stack = shell.app.label("SCR_STACK");
                let home = shell.app.label("HOME_STATE");
                (stack - 1, home + shell.app.label("HOME_STATE_SIZE"))
            };
            shell.app.m.load_ram(neighbour_low, &[0xE1]);
            shell.app.m.load_ram(neighbour_high, &[0x3B]);
            shell.leave(MODE);
            assert_regions_blank(&mut shell);
            assert_eq!(shell.app.m.read_ram(neighbour_low, 1), [0xE1]);
            assert_eq!(shell.app.m.read_ram(neighbour_high, 1), [0x3B]);
        }
    }
}

#[test]
fn a_failing_enter_pushes_the_error_notice_with_the_status_in_hex() {
    for (built, _) in builds() {
        for status in [0x5A, 0xC3, 0x0F] {
            for open in [F1, ENTER, F2] {
                let mut shell = booted(built, Enter::FailNotices(status));
                let first = shell.frames();
                shell.tap(open);
                let at = format!("{status:02X} after {open:02X}");
                shell.shows(&at, &model::error(status));
                assert_eq!(shell.lcd_bytes(), model::error(status), "{at}: the LCD");
                let mut error = vec![0u8; 16];
                error[..3].copy_from_slice(&[KIND_NOTICE, NOTICE_ERROR, status]);
                let want = [first[0].clone(), error];
                assert_eq!(
                    shell.frames(),
                    want,
                    "{at}: the stack the failed push left, and the error"
                );
                assert_eq!(shell.blits(), 2, "{at}");
                shell.tap(CLEAR);
                let start = start_of(built);
                shell.shows(&format!("{at}, back"), &start);
                assert_eq!(shell.frames(), first);
            }
        }
    }
}

#[test]
fn the_version_is_the_one_the_build_gives() {
    let built = versioned("100.10.5");
    let mut shell = booted(built, Enter::Real);
    shell.tap(F3);
    let model_about = model::about(0);
    let mut line = mcui::canvas::Canvas::default();
    line.text_centered(2, "metacalc 100.10.5");
    let line = crate::ui_support::pack(&line.into_bitmap());
    let mut want = model_about.clone();
    want[16 * 12..24 * 12].copy_from_slice(&line[16 * 12..24 * 12]);
    assert_ne!(
        want, model_about,
        "the model's version is not the one built"
    );
    shell.shows("About with version 100.10.5", &want);
}

#[test]
fn clear_on_the_error_notice_leaves_when_the_screen_under_it_fails_again() {
    let mut shell = booted(plain(), Enter::FailAboutWhenFlagged(0x77));
    shell.tap(F3);
    shell.flag(true);
    shell.power_down();
    shell.tap_lost(F4);
    shell.shows("the error over About", &model::error(0x77));
    assert_eq!(shell.depth(), 3);
    shell.dirty_dead();
    shell.leave(CLEAR);
    shell.assert_regions_put_away();
}

#[test]
fn a_refresh_of_the_error_notice_that_fails_leaves() {
    let mut shell = booted(plain(), Enter::FailNoticeOnce(0x41));
    shell.flag(true);
    shell.tap(F1);
    shell.shows("the error notice", &model::error(0x41));
    assert_eq!(shell.depth(), 2);
    shell.flag(true);
    shell.power_down();
    shell.dirty_dead();
    shell.leave(F4);
    shell.assert_regions_put_away();
}

#[test]
fn the_app_leaves_when_the_error_notices_own_push_fails() {
    for (built, _) in builds() {
        for open in [F1, F2] {
            let mut shell = booted(built, Enter::FailAll(0x5A));
            shell.dirty_dead();
            shell.leave(open);
            shell.assert_regions_put_away();
        }
    }
}
