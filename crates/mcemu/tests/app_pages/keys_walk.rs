//! One session, one key after another, through every kind of screen, the checking screen included:
//! the screen and the model are compared after each key.

use crate::app::*;
use crate::builds::*;
use crate::keys::{opens_a_check, pops_a_notice, settled_table};
use crate::shellrig::*;
use mcui::{Key, State};

use crate::list_kind_rig::code;
use crate::theorem_rig::KIND_THEOREM;

const WALK: [u8; 34] = [
    F3, CLEAR, ENTER, F1, F4, CLEAR, DOWN, F3, ENTER, F1, CLEAR, DOWN, F4, F1, UP, F3, CLEAR, DOWN,
    DOWN, F3, CLEAR, F1, ENTER, F4, CLEAR, DOWN, F3, CLEAR, UP, UP, UP, UP, F3, CLEAR,
];

/// `WALK` with F2 where it has F4: on a stage row F2 opens the checking screen.
const WALK_CHECKING: [u8; 34] = [
    F3, CLEAR, ENTER, F1, F2, CLEAR, DOWN, F3, ENTER, F1, CLEAR, DOWN, F2, F1, UP, F3, CLEAR, DOWN,
    DOWN, F3, CLEAR, F1, ENTER, F2, CLEAR, DOWN, F3, CLEAR, UP, UP, UP, UP, F3, CLEAR,
];

/// The keys the stream sent inside a list and inside a stage's notice in one build, 233 and 106 at the
/// least, and the keys inside a list by the list's kind (All, Theorems, Axioms, Unchecked) and the
/// theorem screens opened from one, printed by each run: 65, 67, 65 and 28 keys and 13, 15, 22 and 7
/// screens at the least. A build with no theorem opens none under Theorems.
const IN_LIST_FLOOR: usize = 225;
const IN_NOTICE_FLOOR: usize = 100;
const KEYS_UNDER_KIND_FLOOR: [usize; 4] = [65, 60, 60, 25];
const THEOREMS_UNDER_KIND_FLOOR: [usize; 4] = [12, 14, 20, 6];

/// Whether the open list shows no assertion, so that ENTER and F1 push the stage's notice as they do
/// on a stage with none.
fn shows_nothing(ui: &mcui::Ui) -> bool {
    matches!(ui.state(), State::List { len: 0, .. })
}

/// The model on Libraries again and taken through `log`, with no idle work: the list it was on, with
/// its preview pending.
fn replayed(built: &Built, status: &[mcui::Status], log: &[Key]) -> mcui::Ui {
    let mut ui = model::ui(built, status);
    for &key in log {
        ui.press(key);
    }
    ui
}

/// One session, one key after another, the screen and the model compared after each: a fixed walk
/// through every kind of screen, then a pseudo-random stream of keys, never CLEAR on the Libraries
/// screen, which leaves. In the fixed walk a stage's list is left with CLEAR as soon as it has
/// opened and been compared; in the stream the keys go on inside it: the arrows and CLEAR are the
/// model's, ENTER and F1 open a theorem screen when the list shows something, which CLEAR leaves at once, and otherwise, with F2 to F4, the stage's notice, which the model's list has no screen
/// for, so the notice's rule stands in for it as list_keys.rs does: the notice is compared with the
/// model's "Not available yet" and the list that comes back is the model's after the same keys with
/// no idle work. F5 does nothing on a stage with no section titles, which these builds are. With
/// `checking` the walk and the stream press F2 where they
/// press F4 otherwise, on a table with every assertion settled so that a check has nothing to
/// verify, and the model follows the checking screen as it does the others.
fn long_walk(checking: bool) {
    let f = if checking { F2 } else { F4 };
    let walk = if checking { WALK_CHECKING } else { WALK };
    let stream = [
        UP, DOWN, F1, f, F3, ENTER, CLEAR, F4, F5, LEFT, RIGHT, DOWN, F3, F1, ENTER, F3,
    ];
    let builds = [
        three_stages(),
        six_stages(),
        crate::list_rig::proofs_three(),
    ];
    for built in builds {
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
        let mut log: Vec<Key> = Vec::new();
        let mut notice: Option<String> = None;
        let (mut in_list, mut in_notice) = (0, 0);
        let mut under = [0usize; 4];
        let mut opened = [0usize; 4];
        let mut seed = 0x9E37_79B9_u32;
        let mut keys = walk.to_vec();
        keys.extend([DOWN; 7]);
        keys.extend([F3, CLEAR, UP, f, CLEAR]);
        for _ in 0..900 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            keys.push(stream[seed as usize % stream.len()]);
        }
        for (step, &key) in keys.iter().enumerate() {
            let at = format!("{} stages, step {step}", built.stages.len());
            let fixed = step < walk.len() + 12;
            let on_stage = matches!(
                ui.state(),
                State::Libraries { cursor, .. } if cursor < built.stages.len()
            );
            let in_a_list = matches!(ui.state(), State::List { .. });
            if let (None, State::List { kind, .. }) = (&notice, ui.state()) {
                under[usize::from(code(kind))] += 1;
            }
            if let Some(title) = notice.clone() {
                in_notice += 1;
                shell.tap(key);
                if pops_a_notice(Some(key_of(key))) {
                    notice = None;
                    ui = replayed(built, &status, &log);
                    shell.shows(&at, &model::frame(&ui));
                    assert_eq!(shell.depth(), 2, "{at}: popped to the list");
                } else {
                    shell.shows(&at, &model::unavailable(&title));
                    assert_eq!(shell.depth(), 3, "{at}: stays");
                }
            } else if in_a_list && !fixed && [ENTER, F1].contains(&key) && !shows_nothing(&ui) {
                in_list += 1;
                settle_list(&mut shell, &mut ui);
                let State::List { kind, .. } = ui.state() else {
                    unreachable!("a list is open");
                };
                log.push(key_of(key));
                press(&mut shell, &mut ui, built, key, &at);
                assert_eq!(shell.top_kind(), KIND_THEOREM, "{at}: the theorem screen");
                opened[usize::from(code(kind))] += 1;
                kinds.insert(shell.top_kind());
                shell.tap(CLEAR);
                log.push(Key::Clear);
                ui = replayed(built, &status, &log);
                shell.shows(&at, &model::frame(&ui));
                assert_eq!(shell.depth(), 2, "{at}: back on the list");
            } else if in_a_list
                && !fixed
                && ([F2, F4, F5].contains(&key) || [ENTER, F1].contains(&key))
            {
                in_list += 1;
                settle_list(&mut shell, &mut ui);
                let (blits, frame) = (shell.blits(), shell.frames()[1].clone());
                let title = built.stage_name(usize::from(frame[1])).to_string();
                shell.tap(key);
                if key == F5 {
                    assert_eq!(
                        (shell.depth(), shell.blits()),
                        (2, blits),
                        "{at}: F5, no sections"
                    );
                    assert_eq!(shell.frames()[1], frame, "{at}");
                } else {
                    shell.shows(&at, &model::unavailable(&title));
                    assert_eq!(shell.depth(), 3, "{at}: the stage's notice");
                    notice = Some(title);
                }
            } else if checking && key == F2 && on_stage {
                opens_a_check(&mut shell, &mut ui, &at);
                log.push(Key::F2);
            } else {
                if key == CLEAR && shell.depth() == 1 {
                    continue;
                }
                in_list += usize::from(in_a_list);
                log.push(key_of(key));
                if press(&mut shell, &mut ui, built, key, &at) && fixed {
                    kinds.insert(shell.top_kind());
                    log.push(Key::Clear);
                    press(&mut shell, &mut ui, built, CLEAR, &at);
                }
            }
            kinds.insert(shell.top_kind());
        }
        println!(
            "{} stages: {in_list} keys inside a list, {in_notice} inside a stage's notice",
            built.stages.len()
        );
        println!(
            "{} stages: keys inside a list by kind {under:?}, theorem screens opened by kind {opened:?}",
            built.stages.len()
        );
        let theorems = crate::support::decoded(built.library.as_ref().unwrap())
            .assertions
            .iter()
            .any(|a| a.theorem);
        for k in 0..4 {
            assert!(
                under[k] >= KEYS_UNDER_KIND_FLOOR[k],
                "{under:?} keys inside a list by kind"
            );
            if k == 1 && !theorems {
                assert_eq!(opened[k], 0, "no theorem to open under Theorems");
            } else {
                assert!(
                    opened[k] >= THEOREMS_UNDER_KIND_FLOOR[k],
                    "{opened:?} theorem screens opened by kind"
                );
            }
        }
        assert!(in_list >= IN_LIST_FLOOR, "{in_list} keys inside a list");
        assert!(
            in_notice >= IN_NOTICE_FLOOR,
            "{in_notice} keys inside a notice"
        );
        let mut all: std::collections::BTreeSet<u8> = [
            KIND_LIBRARIES,
            KIND_NOTICE,
            KIND_ABOUT,
            KIND_STAGE,
            KIND_LIST,
            KIND_THEOREM,
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
