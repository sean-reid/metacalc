//! A stage name of twelve wide letters in the list's title bar, through all four kinds, with a status
//! table and without: every frame is the model's, and the name is cut in the frames the width
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

/// How many frames were compared, in how many the name was cut and in how many whole.
#[derive(Debug, Default, PartialEq, Eq)]
struct Frames {
    cut: usize,
    whole: usize,
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
        Frames { cut: 72, whole: 48 },
        "frames compared with the name cut and whole"
    );
}
