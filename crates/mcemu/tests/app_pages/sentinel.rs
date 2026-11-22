//! The sentinel: what the shell does when the OS has overwritten saveSScreen while it waited for a
//! key, and what it does when the screen is intact.

use crate::app::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;

fn start_of(built: &Built) -> Vec<u8> {
    model::start(built.library.as_deref())
}

fn booted(built: &'static Built, enter: Enter) -> Shell {
    Shell::boot(built, enter)
}

#[test]
fn the_sentinel_is_the_complement_of_the_frames_byte_after_every_blit() {
    let mut shell = booted(plain(), Enter::Real);
    let (sentinel, frame) = shell.sentinel();
    for open in [None, Some(F3), Some(CLEAR), Some(F1), Some(F1), Some(F2)] {
        if let Some(open) = open {
            shell.tap(open);
        }
        let (s, f) = (
            shell.app.m.read_ram(sentinel, 1)[0],
            shell.app.m.read_ram(frame, 1)[0],
        );
        assert_eq!(s, !f, "after {open:?}");
    }
}

#[test]
fn an_intact_sentinel_runs_no_refresh_and_no_blit_for_an_ignored_key() {
    for built in [plain(), with_library()] {
        let mut shell = booted(built, Enter::Real);
        for open in [None, Some(F3), Some(CLEAR), Some(F1)] {
            if let Some(open) = open {
                shell.tap(open);
            }
            let before = (
                shell.enters(KIND_START),
                shell.enters(KIND_NOTICE),
                shell.enters(KIND_ABOUT),
                shell.blits(),
            );
            shell.tap(F4);
            shell.tap(LETTER);
            let after = (
                shell.enters(KIND_START),
                shell.enters(KIND_NOTICE),
                shell.enters(KIND_ABOUT),
                shell.blits(),
            );
            assert_eq!(after, before, "after {open:?}");
        }
    }
}

/// After the power down the next key, which the screen ignores, makes the top screen's enter run
/// once, and its frame is drawn and blitted once.
fn refreshed(built: &'static Built, enter: Enter, open: Option<u8>, kind: u8, want: &[u8]) {
    let mut shell = booted(built, enter);
    if let Some(open) = open {
        shell.tap(open);
    }
    shell.shows("before the power down", want);
    let (enters, blits) = (shell.enters(kind), shell.blits());
    shell.power_down();
    shell.tap(F4);
    assert_eq!(
        shell.enters(kind),
        enters + 1,
        "the top screen was entered once"
    );
    assert_eq!(shell.blits(), blits + 1, "and drawn once");
    shell.shows("after the power down and a key", want);
    let (sentinel, frame) = shell.sentinel();
    let (s, f) = (
        shell.app.m.read_ram(sentinel, 1)[0],
        shell.app.m.read_ram(frame, 1)[0],
    );
    assert_eq!(s, !f, "the sentinel is stored again");
    shell.tap(F4);
    assert_eq!(
        shell.enters(kind),
        enters + 1,
        "the next key runs no refresh"
    );
    assert_eq!(shell.blits(), blits + 1);
}

#[test]
fn a_power_down_while_about_is_open_refreshes_it_on_the_next_key() {
    refreshed(plain(), Enter::Real, Some(F3), KIND_ABOUT, &model::about(0));
    refreshed(
        with_library(),
        Enter::Real,
        Some(F3),
        KIND_ABOUT,
        &model::about(1),
    );
}

#[test]
fn a_power_down_while_a_notice_is_open_refreshes_it_on_the_next_key() {
    refreshed(
        plain(),
        Enter::Real,
        Some(F1),
        KIND_NOTICE,
        &model::unavailable("Theorems"),
    );
}

#[test]
fn a_power_down_on_the_start_screen_refreshes_it_on_the_next_key() {
    for built in [plain(), with_library()] {
        refreshed(built, Enter::Real, None, KIND_START, &start_of(built));
    }
}

#[test]
fn a_power_down_destroys_the_error_notices_line_and_the_next_key_builds_it_again() {
    for status in [0x5A, 0xC3] {
        let mut shell = booted(plain(), Enter::FailNotices(status));
        shell.tap(F1);
        shell.shows("the error notice", &model::error(status));
        let buf = shell.app.label("APP_BUF");
        let line = shell.app.m.read_ram(buf, 9);
        shell.power_down();
        assert_ne!(
            shell.app.m.read_ram(buf, 9),
            line,
            "the power down wrote over the line"
        );
        let blits = shell.blits();
        shell.tap(F4);
        assert_eq!(shell.blits(), blits + 1);
        assert_eq!(
            shell.app.m.read_ram(buf, 9),
            line,
            "the line is built again"
        );
        shell.shows("after the power down", &model::error(status));
    }
}

#[test]
fn a_frame_byte_equal_to_a_fixed_value_does_not_hide_a_power_down() {
    let mut shell = booted(plain(), Enter::Real);
    let (sentinel, frame) = shell.sentinel();
    for byte in [0x00, 0xFF, 0xA5, 0x5A, 0x55, 0xAA, 0x01, 0x80] {
        let (enters, blits) = (shell.enters(KIND_START), shell.blits());
        shell.app.m.load_ram(frame, &[byte]);
        shell.app.m.load_ram(sentinel, &[byte]);
        shell.tap(F4);
        assert_eq!(
            shell.enters(KIND_START),
            enters + 1,
            "frame byte {byte:02X} copied over the sentinel"
        );
        assert_eq!(shell.blits(), blits + 1);
        shell.app.m.load_ram(frame, &[byte]);
        shell.app.m.load_ram(sentinel, &[!byte]);
        shell.tap(F4);
        assert_eq!(
            shell.enters(KIND_START),
            enters + 1,
            "frame byte {byte:02X}, sentinel intact"
        );
        assert_eq!(shell.blits(), blits + 1);
    }
}

#[test]
fn a_refresh_that_fails_after_a_power_down_pushes_the_error_notice() {
    let mut shell = booted(plain(), Enter::FailAboutWhenFlagged(0x77));
    shell.tap(F3);
    shell.shows("About", &model::about(0));
    let about = shell.frames();
    shell.flag(true);
    shell.power_down();
    shell.tap_lost(F4);
    shell.flag(false);
    shell.shows("the error over About", &model::error(0x77));
    let frames = shell.frames();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[..2], about);
    assert_eq!(frames[2][..3], [KIND_NOTICE, NOTICE_ERROR, 0x77]);
    shell.tap(CLEAR);
    shell.shows("About again", &model::about(0));
    assert_eq!(shell.frames(), about);
    same_frame("the LCD", &shell.lcd_bytes(), &model::about(0));
}

#[test]
fn a_notice_number_past_the_tables_draws_the_error_notice() {
    let mut shell = booted(plain(), Enter::Real);
    shell.tap(F1);
    let at = shell.app.label("SCR_STACK") + 16 + 1;
    shell.app.m.load_ram(at, &[9]);
    let blits = shell.blits();
    shell.power_down();
    shell.tap(F4);
    assert_eq!(shell.blits(), blits + 1);
    shell.shows("a notice numbered 9", &model::error(0));
}
