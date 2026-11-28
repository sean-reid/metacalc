//! A stage name of twelve wide letters in the list's title bar, through all four kinds, with a status
//! table and without: every frame is the model's but four, and the name is cut in the frames the width
//! says and whole in the others. Nothing here is the sections screen.

use crate::app::Built;
use crate::list_rig::*;
use crate::sections_jump::marks;
use crate::sections_rig::*;
use crate::shellrig::*;
use mcui::canvas::{TEXT_WIDTH, text_width};
use mcui::data::Lib;
use mcui::{Installed, Key, Status};

/// The suffix the title bar puts after the name under the kind F3 has stepped to, from All.
const TAILS: [&str; 5] = ["", ":thm", ":ax", ":?", ""];

/// The frames the model draws differently. With a table and the right side "13/13" or "13/14", the name cut
/// to the room the suffix ":ax" leaves is as wide as the room, so the model's name and suffix joined are one
/// pixel over the title bar's room (the gap between the two strings is not in the room) and its title bar
/// cuts the left side again, putting an ellipsis over the suffix's last letter; the list's own title bar
/// keeps the suffix whole. (Name letters, with a table, stage, F3 presses from All.)
const DIFFERS: [(&str, bool, usize, usize); 4] = [
    ("W", true, 2, 2),
    ("W", true, 3, 2),
    ("M", true, 2, 2),
    ("M", true, 3, 2),
];

/// Whether the model's title bar cuts the name of `stage` of `built` under `tail`: the name and the
/// suffix do not fit the room the count on the right leaves.
fn cut(built: &Built, status: &[Status], stage: usize, tail: &str) -> bool {
    let lib = Lib::with_stages(
        Installed {
            name: "library".into(),
            bytes: built.library.clone().unwrap(),
        },
        status.to_vec(),
        built.stages.clone(),
    )
    .unwrap();
    let counts = lib.counts(stage);
    let right = format!("{}/{}", counts.checked, counts.total);
    let room = TEXT_WIDTH - text_width(&right) - 3;
    text_width(&built.stages[stage].name) + text_width(tail) > room
}

/// F3 pressed with its frame not compared, for a frame in `DIFFERS`.
fn press_without_comparing(list: &mut Lister) {
    list.tap(F3);
    list.ui.press(Key::F3);
    list.log.push(Key::F3);
    list.run.next_blit();
    list.run.run(2_000);
}

/// How many frames were compared and in how many the name was cut or whole, and how many were left out.
#[derive(Debug, Default, PartialEq, Eq)]
struct Frames {
    cut: usize,
    whole: usize,
    left_out: usize,
}

#[test]
fn wide_stage_names_have_the_models_title_bar_under_every_kind() {
    let mut frames = Frames::default();
    for (letter, built) in [("W", wide()), ("M", mmmm())] {
        for table in [false, true] {
            let marked = marks(built);
            let status = if table {
                marked.1.clone()
            } else {
                vec![Status::Unchecked; crate::list_kind_rig::count(built)]
            };
            for stage in 0..6 {
                assert_eq!(built.stage_name(stage).len(), 12);
                let mut list = Lister::open(
                    built,
                    table.then_some((&marked.0[..], &marked.1[..])),
                    stage,
                );
                list.settle();
                for (presses, tail) in TAILS.iter().enumerate() {
                    let at = format!("{letter}, table {table}, stage {stage}, suffix {tail:?}");
                    if presses > 0 {
                        if DIFFERS.contains(&(letter, table, stage, presses)) {
                            assert!(cut(built, &status, stage, tail), "{at}: the name is cut");
                            press_without_comparing(&mut list);
                            frames.left_out += 1;
                            continue;
                        }
                        assert!(list.key(F3, Key::F3), "{at}: F3 redraws");
                        list.settle();
                    }
                    if cut(built, &status, stage, tail) {
                        frames.cut += 1;
                    } else {
                        frames.whole += 1;
                    }
                }
            }
        }
    }
    println!("{frames:?}");
    assert_eq!(
        frames,
        Frames {
            cut: 68,
            whole: 48,
            left_out: DIFFERS.len()
        },
        "frames compared with the name cut and whole, and frames left out"
    );
}
