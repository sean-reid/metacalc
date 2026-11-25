//! One session, one key after another, through every kind of screen, the checking screen included:
//! the screen and the model are compared after each key.

use crate::builds::*;
use crate::keys::{opens_a_check, settled_table};
use crate::shellrig::*;
use mcui::State;

const WALK: [u8; 34] = [
    F3, CLEAR, ENTER, F1, F4, CLEAR, DOWN, F3, ENTER, F1, CLEAR, DOWN, F4, F1, UP, F3, CLEAR, DOWN,
    DOWN, F3, CLEAR, F1, ENTER, F4, CLEAR, DOWN, F3, CLEAR, UP, UP, UP, UP, F3, CLEAR,
];

/// `WALK` with F2 where it has F4: on a stage row F2 opens the checking screen.
const WALK_CHECKING: [u8; 34] = [
    F3, CLEAR, ENTER, F1, F2, CLEAR, DOWN, F3, ENTER, F1, CLEAR, DOWN, F2, F1, UP, F3, CLEAR, DOWN,
    DOWN, F3, CLEAR, F1, ENTER, F2, CLEAR, DOWN, F3, CLEAR, UP, UP, UP, UP, F3, CLEAR,
];

/// One session, one key after another, the screen and the model compared after each: a fixed walk
/// through every kind of screen, then a pseudo-random stream of keys, never CLEAR on the Libraries
/// screen, which leaves. A stage's list is left with CLEAR as soon as it has opened and been
/// compared; its own keys are the list tests'. With `checking` the walk and the stream press F2 where they
/// press F4 otherwise, on a table with every assertion settled so that a check has nothing to
/// verify, and the model follows the checking screen as it does the others.
fn long_walk(checking: bool) {
    let f = if checking { F2 } else { F4 };
    let walk = if checking { WALK_CHECKING } else { WALK };
    let stream = [
        UP, DOWN, F1, f, F3, ENTER, CLEAR, F4, F5, LEFT, RIGHT, DOWN, F3, F1,
    ];
    for built in [three_stages(), six_stages()] {
        let (data, status) = if checking {
            settled_table(built)
        } else {
            let library = built.library.as_ref().unwrap();
            let n = usize::from(crate::table::header_of(library).1);
            let status: Vec<mcui::Status> = (0..n)
                .map(|i| match i % 3 {
                    0 => mcui::Status::Checked,
                    _ => mcui::Status::Unchecked,
                })
                .collect();
            let table = crate::table::values(built, &status);
            (crate::table::of(library, &table), status)
        };
        let mut shell = booted(built, Some(&data));
        let mut ui = model::ui(built, &status);
        let mut kinds = std::collections::BTreeSet::new();
        let mut seed = 0x9E37_79B9_u32;
        let mut keys = walk.to_vec();
        keys.extend([DOWN; 7]);
        keys.extend([F3, CLEAR, UP, f, CLEAR]);
        for _ in 0..160 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            keys.push(stream[seed as usize % stream.len()]);
        }
        for (step, &key) in keys.iter().enumerate() {
            let at = format!("{} stages, step {step}", built.stages.len());
            let on_stage = matches!(
                ui.state(),
                State::Libraries { cursor, .. } if cursor < built.stages.len()
            );
            if checking && key == F2 && on_stage {
                opens_a_check(&mut shell, &mut ui, &at);
            } else {
                if key == CLEAR && shell.depth() == 1 {
                    continue;
                }
                if press(&mut shell, &mut ui, built, key, &at) {
                    kinds.insert(shell.top_kind());
                    press(&mut shell, &mut ui, built, CLEAR, &at);
                }
            }
            kinds.insert(shell.top_kind());
        }
        let mut all: std::collections::BTreeSet<u8> = [
            KIND_LIBRARIES,
            KIND_NOTICE,
            KIND_ABOUT,
            KIND_STAGE,
            KIND_LIST,
        ]
        .into();
        if checking {
            all.insert(KIND_CHECKING);
        }
        assert_eq!(
            kinds,
            all,
            "{} stages: every kind was shown",
            built.stages.len()
        );
    }
}

#[test]
fn a_long_walk_through_every_kind_of_screen_in_one_session_is_the_models_at_every_step() {
    long_walk(false);
}

/// The walk through the checking screen: F2 on a stage opens it, and the keys after it, F1 and
/// CLEAR among them, do what the model's do, with kind 5 reached.
#[test]
fn a_long_walk_that_opens_the_checking_screen_is_the_models_at_every_step() {
    long_walk(true);
}
