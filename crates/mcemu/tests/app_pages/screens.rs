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
    names_and_titles("pages-names", &crate::fixture::library(), false);
}

/// With more than 10,000 assertions every row's text is wide enough to cut a name, so the names
/// that sit on the edge of the room exist and the test fails when it finds none.
#[test]
fn a_stage_name_the_model_cuts_to_the_room_its_text_leaves_is_cut_the_same_on_a_wide_library() {
    names_and_titles("pages-names-wide", crate::numbers::wide(), true);
}

/// With ring's 3,326 assertions a row's text is wide enough to cut a name.
#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding ring.mclib>"]
fn a_stage_name_the_model_cuts_to_the_room_its_text_leaves_is_cut_the_same() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let ring = std::fs::read(format!("{dir}/ring.mclib")).expect("library reads");
    names_and_titles("pages-names-ring", &ring, true);
}

fn names_and_titles(build: &str, library: &[u8], edge_required: bool) {
    let library = library.to_vec();
    let count = crate::table::header_of(&library).1 as usize;
    let widest = (b'!'..=b'~')
        .map(char::from)
        .max_by_key(|c| mcui::canvas::text_width(&c.to_string()))
        .unwrap();
    let too_wide = widest.to_string().repeat(12);
    assert!(
        mcui::canvas::text_width(&too_wide) <= mcui::canvas::TEXT_WIDTH,
        "a name of twelve characters fits the title bar, which the notice and the stage About rely on"
    );
    // Every stage is under way, so each row ends in "checked/total". The fourth and fifth names sit
    // on the edge of the room their text leaves, one cut at it and not a pixel above, the other
    // cut a pixel below it and not at it, which a Z80 that is a pixel out either way gets wrong.
    let status: Vec<mcui::Status> = (0..count)
        .map(|i| {
            if i % 2 == 0 {
                mcui::Status::Checked
            } else {
                mcui::Status::Unchecked
            }
        })
        .collect();
    let stage = |i: usize, name: &str| mcui::Stage {
        name: name.into(),
        title: "W".repeat(48),
        end: (i + 1) * count / 6,
    };
    let placeholders: Vec<mcui::Stage> = (0..6).map(|i| stage(i, "a")).collect();
    let probe = crate::realos::libraries_ui(Some(&crate::realos::Shown {
        stages: placeholders,
        status: status.clone(),
        ..crate::realos::Shown::new(LIBRARY_NAME, &library)
    }));
    let rights: Vec<String> = probe
        .frame_calls()
        .1
        .into_iter()
        .filter_map(|call| match call {
            mcui::canvas::Call::ListRow { right, .. } => Some(right),
            _ => None,
        })
        .collect();
    let chars: Vec<char> = (b'!'..=b'~').map(char::from).collect();
    // A name whose cut differs between two limits a pixel apart, for the room row `i` leaves.
    let edge = |i: usize, below: bool| {
        let room = mcui::canvas::TEXT_WIDTH - 3 - mcui::canvas::text_width(&rights[i]);
        let (low, high) = if below {
            (room - 1, room)
        } else {
            (room, room + 1)
        };
        let found = chars
            .iter()
            .flat_map(|&a| chars.iter().map(move |&b| (a, b)))
            .flat_map(|(a, b)| (0..=12).flat_map(move |n| (1..=12 - n).map(move |m| (a, b, n, m))))
            .map(|(a, b, n, m)| format!("{}{}", a.to_string().repeat(n), b.to_string().repeat(m)))
            .find(|name| mcui::canvas::truncate(name, low) != mcui::canvas::truncate(name, high));
        assert!(
            found.is_some() || !edge_required,
            "no name is cut at {low} pixels and not at {high}: the text is {:?}",
            rights[i]
        );
        found.unwrap_or_else(|| "edge".into())
    };
    let (above, below) = (edge(3, false), edge(4, true));
    let names = [too_wide.as_str(), "i", "mmmmm", &above, &below, "x-y.z_1"];
    let stages: Vec<mcui::Stage> = names.iter().enumerate().map(|(i, n)| stage(i, n)).collect();
    let built = staged_with(build, &library, &stages);
    let data = crate::table::of(&library, &crate::table::values(built, &status));
    let mut shell = booted(built, Some(&data));
    shell.patience = if count > 10_000 { 4 } else { 1 };
    let mut ui = model::ui(built, &status);
    shell.shows("the Libraries screen", &model::frame(&ui));
    for row in 0..6 {
        press(&mut shell, &mut ui, built, DOWN, &format!("row {row}"));
    }
    for row in (0..6).rev() {
        let at = format!("row {row}");
        press(&mut shell, &mut ui, built, UP, &at);
        press(&mut shell, &mut ui, built, F3, &at);
        press(&mut shell, &mut ui, built, CLEAR, &at);
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
            let home = shell.app.label("HOME_STATE");
            shell.app.m.load_ram(home, &[0x44, 0x55]);
            shell.leave(MODE);
            shell.assert_regions_put_away();
            assert_eq!(shell.app.m.read_ram(home, 2), [0, 0], "cmdShadCur");
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
