//! The shell's four screen kinds on the real app: the Libraries screen, Notice, About and the
//! stage About, against the frames of the host model, and the exits from depth 1 and depth 2. The
//! keys of every row are in keys.rs.

use crate::app::*;
use crate::builds::*;
use crate::shellrig::*;

fn shown(built: &'static Built, enter: Enter) -> Shell {
    let mut shell = Shell::boot(built, enter);
    shell.shows("the Libraries screen", &model::libraries(built));
    assert_eq!((shell.depth(), shell.blits()), (1, 1));
    shell
}

#[test]
fn the_libraries_screen_is_the_frame_the_models_canvas_draws() {
    for built in builds() {
        shown(built, Enter::Real);
    }
}

#[test]
fn a_stage_row_and_the_stage_about_are_the_models_whatever_the_name_and_the_title() {
    let library = crate::fixture::library();
    let count = crate::table::header_of(&library).1 as usize;
    let stages: Vec<mcui::Stage> = ["WWWWWWWWWWWW", "i", "mmmmm", "x-y.z_1"]
        .iter()
        .enumerate()
        .map(|(i, name)| mcui::Stage {
            name: (*name).into(),
            title: "W".repeat(48),
            end: (i + 1) * count / 4,
        })
        .collect();
    let built = staged_with("pages-names", &library, &stages);
    let mut shell = shown(built, Enter::Real);
    let mut ui = model::ui(built, &[]);
    for row in 0..4 {
        press(&mut shell, &mut ui, built, DOWN, &format!("row {row}"));
    }
    for row in (0..4).rev() {
        press(&mut shell, &mut ui, built, UP, &format!("row {row}"));
        press(
            &mut shell,
            &mut ui,
            built,
            F3,
            &format!("the About of row {row}"),
        );
        press(
            &mut shell,
            &mut ui,
            built,
            CLEAR,
            &format!("back to row {row}"),
        );
        let at = format!("the list of row {row}");
        press(&mut shell, &mut ui, built, F1, &at);
        press(&mut shell, &mut ui, built, CLEAR, &at);
    }
}

#[test]
fn the_screen_of_a_library_with_no_assertion_has_one_empty_stage() {
    let library = crate::support::pack_all(&crate::support::parse(
        "$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n",
    ));
    assert!(
        mcpack::library::decode(&library)
            .unwrap()
            .assertions
            .is_empty()
    );
    shown(around("empty", &library), Enter::Real);
}

#[test]
fn about_and_each_notice_are_the_frames_of_the_model_on_every_row() {
    for built in builds() {
        for row in 0..rows(built) {
            for open in [F3, F1, ENTER, F2] {
                let mut shell = shown(built, Enter::Real);
                let mut ui = model::ui(built, &[]);
                let at = format!("{} stages, row {row}", built.stages.len());
                for _ in 0..row {
                    press(&mut shell, &mut ui, built, DOWN, &at);
                }
                press(&mut shell, &mut ui, built, open, &at);
                assert_eq!(shell.depth(), 2, "{at}: opened");
                press(&mut shell, &mut ui, built, CLEAR, &at);
                assert_eq!(shell.depth(), 1, "{at}: closed");
            }
        }
    }
}

#[test]
fn second_then_a_key_the_app_ignores_blits_nothing() {
    let mut shell = shown(plain(), Enter::Real);
    shell.tap_modifier(SECOND);
    shell.tap(ONE);
    assert_eq!((shell.blits(), shell.depth()), (1, 1));
}

#[test]
fn clear_on_the_libraries_screen_leaves() {
    for built in builds() {
        let mut shell = shown(built, Enter::Real);
        shell.dirty_dead();
        shell.leave(CLEAR);
        shell.assert_regions_put_away();
    }
}

#[test]
fn quit_leaves_from_depth_2_and_fills_the_two_regions_as_it_does_from_depth_1() {
    for built in [plain(), three_stages()] {
        for open in [None, Some(F3), Some(F1), Some(F2)] {
            let mut shell = shown(built, Enter::Real);
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
            shell.assert_regions_put_away();
            assert_eq!(shell.app.m.read_ram(neighbour_low, 1), [0xE1]);
            assert_eq!(shell.app.m.read_ram(neighbour_high, 1), [0x3B]);
        }
    }
}

#[test]
fn a_failing_enter_pushes_the_error_notice_with_the_status_in_hex() {
    for built in [plain(), with_library()] {
        for status in [0x5A, 0xC3, 0x0F] {
            for open in [F1, ENTER, F2] {
                let mut shell = shown(built, Enter::FailNotices(status));
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
                shell.shows(&format!("{at}, back"), &model::libraries(built));
                assert_eq!(shell.frames(), first);
            }
        }
    }
}

#[test]
fn the_version_is_the_one_the_build_gives() {
    let built = versioned("100.10.5");
    let mut shell = shown(built, Enter::Real);
    shell.tap(F3);
    let model_about = model::about(built);
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
    let mut shell = shown(plain(), Enter::FailAboutWhenFlagged(0x77));
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
    let mut shell = shown(plain(), Enter::FailNoticeOnce(0x41));
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
    for built in [plain(), with_library()] {
        for open in [F1, F2] {
            let mut shell = shown(built, Enter::FailAll(0x5A));
            shell.dirty_dead();
            shell.leave(open);
            shell.assert_regions_put_away();
        }
    }
}

#[test]
fn the_libraries_enter_runs_once_for_each_opening_and_never_for_a_key_it_ignores() {
    let mut shell = shown(three_stages(), Enter::Real);
    assert_eq!(shell.enters(KIND_LIBRARIES), 1);
    for key in [F4, F5, LEFT, RIGHT, DOWN, UP] {
        shell.tap(key);
    }
    assert_eq!(
        shell.enters(KIND_LIBRARIES),
        1,
        "a cursor move builds nothing again"
    );
    shell.tap(F3);
    shell.tap(CLEAR);
    assert_eq!(
        shell.enters(KIND_LIBRARIES),
        2,
        "a screen that is uncovered is entered again"
    );
}
