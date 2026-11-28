//! A stage name of twelve wide letters in the list's title bar, through all four kinds, with a status
//! table and without: every frame is the model's, and the name is cut in the frames the width
//! says and whole in the others. Nothing here is the sections screen.

use crate::app::Built;
use crate::list_rig::*;
use crate::sections_jump::marks;
use crate::sections_rig::*;
use crate::shellrig::*;
use mcui::canvas::{TEXT_WIDTH, text_width, truncate};
use mcui::data::Lib;
use mcui::{Installed, Key, Status};

/// The suffix the title bar puts after the name under the kind F3 has stepped to, from All.
const TAILS: [&str; 5] = ["", ":thm", ":ax", ":?", ""];

/// The room the title bar leaves the name of `stage` of `built` under `tail`: what the count on the
/// right leaves, less the suffix.
fn name_room(built: &Built, status: &[Status], stage: usize, tail: &str) -> usize {
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
    (TEXT_WIDTH - text_width(&right) - 3).saturating_sub(text_width(tail))
}

/// Whether the model's title bar cuts the name of `stage` of `built` under `tail`.
fn cut(built: &Built, status: &[Status], stage: usize, tail: &str) -> bool {
    text_width(&built.stages[stage].name) > name_room(built, status, stage, tail)
}

/// Whether the cut name is as wide as its room, so the name and suffix joined run one pixel past it.
fn fills(built: &Built, status: &[Status], stage: usize, tail: &str) -> bool {
    let room = name_room(built, status, stage, tail);
    cut(built, status, stage, tail)
        && text_width(&truncate(&built.stages[stage].name, room)) == room
}

/// How many frames were compared, in how many the name was cut and in how many whole.
#[derive(Debug, Default, PartialEq, Eq)]
struct Frames {
    cut: usize,
    whole: usize,
}

#[test]
fn wide_stage_names_have_the_models_title_bar_under_every_kind() {
    let mut frames = Frames::default();
    let mut filling = Vec::new();
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
                        assert!(list.key(F3, Key::F3), "{at}: F3 redraws");
                        list.settle();
                    }
                    if fills(built, &status, stage, tail) {
                        filling.push((letter, table, stage, *tail));
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
        Frames { cut: 72, whole: 48 },
        "frames compared with the name cut and whole"
    );
    assert_eq!(
        filling,
        [
            ("W", true, 2, ":ax"),
            ("W", true, 3, ":ax"),
            ("M", true, 2, ":ax"),
            ("M", true, 3, ":ax")
        ],
        "frames where the cut name fills its room"
    );
}
