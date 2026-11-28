//! The sections screen walked by row and by page over every title of every stage of the rich library,
//! each frame compared with the model's and each count with the model's.

use crate::list_rig::*;
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::Walker;
use mcui::Key;

/// The titles each stage with titles has, from the packed library and written by hand.
const COUNTS: [usize; 5] = [3, 6, 7, 33, 49];

/// A walk's keys, how many moved and how many stayed.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    moves: usize,
    stays: usize,
}

/// Presses `scan` `times` times on the sections screen, every frame compared, and counts the keys that
/// drew and the ones that did not.
fn press(list: &mut Lister, scan: u8, key: Key, times: usize, tally: &mut Tally) {
    for _ in 0..times {
        if list.sections_key(scan, key) {
            tally.moves += 1;
        } else {
            tally.stays += 1;
        }
    }
}

/// The sections screen of `stage` opened from the list's first assertion.
fn opened(stage: usize) -> Walker {
    let mut walker = Walker::on_list(rich(), None, stage);
    walker.list.open_sections();
    walker
}

#[test]
fn a_walk_by_row_over_every_title_of_each_stage_has_the_models_frames_and_counts() {
    let built = rich();
    assert_eq!(&title_counts(built)[1..], &COUNTS);
    reset_lcd_checks();
    for (n, len) in COUNTS.into_iter().enumerate() {
        let mut walker = opened(n + 1);
        let list = &mut walker.list;
        let at = list.sections();
        assert_eq!((at.cursor, at.len), (0, len), "stage {}", n + 1);
        assert!(len > 0);
        let mut tally = Tally::default();
        press(list, DOWN, Key::Down, len - 1, &mut tally);
        assert_eq!(list.sections().cursor, len - 1, "stage {}: the last", n + 1);
        press(list, DOWN, Key::Down, 1, &mut tally);
        press(list, UP, Key::Up, len - 1, &mut tally);
        assert_eq!(list.sections().cursor, 0, "stage {}: the first", n + 1);
        press(list, UP, Key::Up, 1, &mut tally);
        assert_eq!(
            tally,
            Tally {
                moves: 2 * (len - 1),
                stays: 2
            },
            "stage {}",
            n + 1
        );
    }
    assert_lcd_checks("the row walk", [KIND_SECTIONS], |_| 190);
}

#[test]
fn a_walk_by_page_from_either_end_and_from_between_has_the_models_frames() {
    let mut pages = 0;
    reset_lcd_checks();
    for (n, len) in COUNTS.into_iter().enumerate() {
        for start in [0, 3] {
            if start >= len {
                continue;
            }
            let mut walker = opened(n + 1);
            let list = &mut walker.list;
            list.goto_title(start);
            let mut tally = Tally::default();
            let forward = (len - 1 - start).div_ceil(6);
            press(list, RIGHT, Key::Right, forward, &mut tally);
            assert_eq!(list.sections().cursor, len - 1, "stage {}: the last", n + 1);
            press(list, RIGHT, Key::Right, 1, &mut tally);
            let back = (len - 1).div_ceil(6);
            press(list, LEFT, Key::Left, back, &mut tally);
            assert_eq!(list.sections().cursor, 0, "stage {}: the first", n + 1);
            press(list, LEFT, Key::Left, 1, &mut tally);
            assert_eq!(
                tally,
                Tally {
                    moves: forward + back,
                    stays: 2
                },
                "stage {} from {start}",
                n + 1
            );
            pages += forward + back;
        }
    }
    assert_eq!(pages, 65, "the pages walked");
    assert_lcd_checks("the page walk", [KIND_SECTIONS], |_| 98);
}

/// Every title of each stage of the shipped list walked down and up by `scan`, `step` titles a key, with
/// every frame compared and the count of titles the model's.
fn walk_shipped(scan_down: u8, key_down: Key, scan_up: u8, key_up: Key, step: usize) {
    let built = crate::built::shipped_staged();
    let counts = title_counts(built);
    assert_eq!(counts.len(), 3);
    println!("titles in the shipped stages: {counts:?}");
    for (stage, &len) in counts.iter().enumerate() {
        assert!(len > 0, "stage {stage} has titles");
        let mut walker = Walker::on_list(built, None, stage);
        let list = &mut walker.list;
        assert_eq!(list.open_sections().len, len, "stage {stage}: the count");
        list.goto_title(0);
        let mut tally = Tally::default();
        let keys = (len - 1).div_ceil(step);
        press(list, scan_down, key_down, keys, &mut tally);
        assert_eq!(list.sections().cursor, len - 1, "stage {stage}: the last");
        press(list, scan_down, key_down, 1, &mut tally);
        press(list, scan_up, key_up, keys, &mut tally);
        assert_eq!(list.sections().cursor, 0, "stage {stage}: the first");
        press(list, scan_up, key_up, 1, &mut tally);
        let want = Tally {
            moves: 2 * keys,
            stays: 2,
        };
        assert_eq!(tally, want, "stage {stage}: {len} titles");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn a_walk_by_row_over_every_title_of_the_shipped_stages_has_the_models_frames_and_counts() {
    walk_shipped(DOWN, Key::Down, UP, Key::Up, 1);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn a_walk_by_page_over_every_title_of_the_shipped_stages_has_the_models_frames_and_counts() {
    walk_shipped(RIGHT, Key::Right, LEFT, Key::Left, 6);
}
