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

#[test]
fn a_key_a_screen_ignores_changes_nothing_and_blits_nothing() {
    let on_start = [DOWN, UP, LEFT, F4, F5, LETTER];
    let on_about = [DOWN, F1, F2, F3, F4, F5, LETTER];
    let on_notice = [DOWN, F2, F3, F4, F5, LETTER];
    for (built, n) in builds() {
        let mut shell = booted(built, Enter::Real);
        let start = start_of(built);
        for (open, keys, want) in [
            (None, &on_start[..], start.clone()),
            (Some(F3), &on_about[..], model::about(n)),
            (Some(F1), &on_notice[..], model::unavailable("Theorems")),
            (Some(F2), &on_notice[..], model::unavailable("Verify")),
        ] {
            let mut ui = model::ui(n);
            if let Some(open) = open {
                shell.tap(open);
                ui.press(key_of(open));
            }
            let (blits, depth, frames) = (shell.blits(), shell.depth(), shell.frames());
            for &key in keys {
                shell.tap(key);
                assert_eq!(
                    shell.blits(),
                    blits,
                    "key {key:02X} on a screen of depth {depth}"
                );
                assert_eq!(shell.depth(), depth, "key {key:02X}");
                assert_eq!(shell.frames(), frames, "key {key:02X}");
                shell.shows(&format!("after {key:02X}"), &want);
                if open.is_some() {
                    let key = match key {
                        F1 => Key::F1,
                        F2 => Key::F2,
                        F3 => Key::F3,
                        F4 => Key::F4,
                        F5 => Key::F5,
                        DOWN => Key::Down,
                        _ => Key::Letter('A'),
                    };
                    let state = ui.state();
                    ui.press(key);
                    assert_eq!(
                        (ui.depth(), ui.state()),
                        (2, state),
                        "the model ignores {key:?}"
                    );
                }
            }
            if open.is_some() {
                shell.tap(CLEAR);
            }
        }
    }
}

#[test]
fn second_then_a_key_the_app_ignores_blits_nothing() {
    let mut shell = booted(plain(), Enter::Real);
    shell.tap(SECOND);
    shell.tap(ONE);
    assert_eq!((shell.blits(), shell.depth()), (1, 1));
}

#[test]
fn clear_on_the_start_screen_leaves() {
    for (built, _) in builds() {
        let mut shell = booted(built, Enter::Real);
        shell.leave(CLEAR);
        assert_regions_blank(&mut shell);
    }
}

fn assert_regions_blank(shell: &mut Shell) {
    let (stack, vars) = (shell.app.label("SCR_STACK"), shell.app.label("SCR_VARS"));
    let (stack_size, vars_size) = (
        shell.app.label("SCR_STACK_SIZE"),
        shell.app.label("SCR_VARS_SIZE"),
    );
    assert_eq!(
        shell.app.m.read_ram(stack, usize::from(stack_size)),
        [0x20; 128]
    );
    assert_eq!(shell.app.m.read_ram(vars, usize::from(vars_size)), [0; 108]);
}

#[test]
fn quit_leaves_from_depth_2_and_fills_the_two_regions_as_it_does_from_depth_1() {
    for (built, _) in builds() {
        for open in [None, Some(F3), Some(F1), Some(F2)] {
            let mut shell = booted(built, Enter::Real);
            if let Some(open) = open {
                shell.tap(open);
                assert_eq!(shell.depth(), 2);
            }
            shell.tap(SECOND);
            let (neighbour_low, neighbour_high) = {
                let stack = shell.app.label("SCR_STACK");
                (stack - 1, stack + 128)
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
