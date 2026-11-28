//! The sentinel: what the shell does when the OS has overwritten saveSScreen while it waited for a
//! key, and what it does when the screen is intact.

use crate::app::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;

fn libraries_of(built: &Built) -> Vec<u8> {
    model::libraries(built)
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

const KINDS: [u8; 6] = [
    crate::theorem_rig::KIND_THEOREM,
    KIND_LIBRARIES,
    KIND_NOTICE,
    KIND_ABOUT,
    KIND_STAGE,
    KIND_LIST,
];

/// Taps each of `ignored`, which the top screen does nothing with, and asserts that no screen was
/// entered and no frame blitted.
fn ignores(shell: &mut Shell, ignored: &[u8], at: &str) {
    let before = (KINDS.map(|k| shell.enters(k)), shell.blits());
    for &key in ignored {
        shell.tap(key);
    }
    let after = (KINDS.map(|k| shell.enters(k)), shell.blits());
    assert_eq!(after, before, "{at}");
}

#[test]
fn an_intact_sentinel_runs_no_refresh_and_no_blit_for_an_ignored_key() {
    for built in [plain(), with_library()] {
        let mut shell = booted(built, Enter::Real);
        let library = built.library.is_some();
        for (n, open) in [None, Some(F3), Some(CLEAR), Some(F1), Some(F4)]
            .into_iter()
            .enumerate()
        {
            if n == 4 && !library {
                continue;
            }
            if let Some(open) = open {
                shell.tap(open);
            }
            let at = format!("after {open:?}, number {n}");
            if library && n == 3 {
                assert_eq!(shell.top_kind(), KIND_LIST, "{at}: the list");
                shell.app.m.run_app(3_000_000);
                assert_ne!(crate::builds::list_preview(&mut shell), 0, "{at}: settled");
                ignores(&mut shell, &[F5], &at);
            } else {
                if library && n == 4 {
                    assert_eq!(shell.top_kind(), KIND_NOTICE, "{at}: the stage's notice");
                    assert_eq!(shell.depth(), 3, "{at}: over the list");
                }
                ignores(&mut shell, &[F4, LETTER], &at);
            }
        }
    }
}

#[test]
fn an_intact_sentinel_runs_no_refresh_and_no_blit_for_f5_and_a_letter_on_a_theorem() {
    let mut shell = booted(with_library(), Enter::Real);
    shell.tap(F1);
    shell.app.m.run_app(3_000_000);
    shell.tap(ENTER);
    assert_eq!(
        shell.top_kind(),
        crate::theorem_rig::KIND_THEOREM,
        "the theorem"
    );
    ignores(&mut shell, &[LETTER, F5, ONE], "on a theorem");
}

#[test]
fn an_intact_sentinel_runs_no_refresh_and_no_blit_for_the_keys_the_dist_screen_ignores() {
    let mut shell = booted(with_library(), Enter::Real);
    shell.tap(F1);
    shell.app.m.run_app(3_000_000);
    shell.tap(ENTER);
    shell.tap(F2);
    assert_eq!(
        shell.top_kind(),
        crate::dist_rig::KIND_DIST,
        "the Dist screen"
    );
    ignores(
        &mut shell,
        &[LETTER, F1, F3, F4, F5, ONE, ENTER, LEFT, RIGHT],
        "on the Dist screen",
    );
}

/// After the power down the next key, which the screen ignores, makes the top screen's enter run
/// once, and its frame is drawn and blitted once.
fn refreshed(built: &'static Built, enter: Enter, open: &[u8], kind: u8, want: &[u8]) {
    let mut shell = booted(built, enter);
    for &open in open {
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
    for built in [plain(), with_library()] {
        refreshed(built, Enter::Real, &[F3], KIND_ABOUT, &model::about(built));
    }
}

#[test]
fn a_power_down_while_a_notice_is_open_refreshes_it_on_the_next_key() {
    refreshed(
        plain(),
        Enter::Real,
        &[F1],
        KIND_NOTICE,
        &model::unavailable("My proofs"),
    );
}

#[test]
fn a_power_down_while_a_stages_notice_is_open_over_its_list_refreshes_it_on_the_next_key() {
    let built = with_library();
    refreshed(
        built,
        Enter::Real,
        &[F1, F4],
        KIND_NOTICE,
        &model::unavailable(built.stage_name(0)),
    );
}

#[test]
fn a_power_down_on_the_libraries_screen_refreshes_it_on_the_next_key() {
    for built in [plain(), with_library()] {
        refreshed(
            built,
            Enter::Real,
            &[],
            KIND_LIBRARIES,
            &libraries_of(built),
        );
    }
}

#[test]
fn a_power_down_destroys_the_error_notices_line_and_the_next_key_builds_it_again() {
    for status in [0x5A, 0xC3] {
        let mut shell = booted(plain(), Enter::FailNotices(status));
        shell.tap(F1);
        shell.shows("the error notice", &model::error(status));
        let buf = shell.app.label("NT_LINE");
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
        let (enters, blits) = (shell.enters(KIND_LIBRARIES), shell.blits());
        shell.app.m.load_ram(frame, &[byte]);
        shell.app.m.load_ram(sentinel, &[byte]);
        shell.tap(F4);
        assert_eq!(
            shell.enters(KIND_LIBRARIES),
            enters + 1,
            "frame byte {byte:02X} copied over the sentinel"
        );
        assert_eq!(shell.blits(), blits + 1);
        shell.app.m.load_ram(frame, &[byte]);
        shell.app.m.load_ram(sentinel, &[!byte]);
        shell.tap(F4);
        assert_eq!(
            shell.enters(KIND_LIBRARIES),
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
    shell.shows("About", &model::about(plain()));
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
    shell.shows("About again", &model::about(plain()));
    assert_eq!(shell.frames(), about);
    same_frame("the LCD", &shell.lcd_bytes(), &model::about(plain()));
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
