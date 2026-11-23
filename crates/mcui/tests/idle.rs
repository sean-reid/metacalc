//! The list draws its preview row only when no key is waiting.

mod support;

use mcglyph::Bitmap;
use mcui::Key::{self, *};
use mcui::canvas::{Canvas, TEXT_WIDTH};
use mcui::data::{Installed, Lib};
use mcui::fold::fitted;
use mcui::{CheckStep, Stage, State, Status, Statuses, Ui};
use std::collections::{HashSet, VecDeque};
use support::{
    db, index_of, logic_bytes, pack_all, press, sectioned, statuses, ui_over, unchecked_ui,
};

fn preview_rows() -> std::ops::Range<usize> {
    48..56
}

fn raw(ui: &mut Ui, keys: &[Key]) {
    keys.iter().for_each(|&k| ui.press(k));
}

fn list(ui: &Ui) -> (Option<String>, bool) {
    match ui.state() {
        State::List { label, pending, .. } => (label, pending),
        other => panic!("not a list: {other:?}"),
    }
}

fn pending(ui: &Ui) -> bool {
    list(ui).1
}

fn label(ui: &Ui) -> String {
    list(ui).0.expect("a row is highlighted")
}

fn preview_blank(frame: &Bitmap) -> bool {
    preview_rows().all(|y| (0..96).all(|x| !frame.get(x, y)))
}

fn same_outside_preview(a: &Bitmap, b: &Bitmap) -> bool {
    (0..64)
        .filter(|y| !preview_rows().contains(y))
        .all(|y| (0..96).all(|x| a.get(x, y) == b.get(x, y)))
}

fn same_preview(a: &Bitmap, b: &Bitmap) -> bool {
    preview_rows().all(|y| (0..96).all(|x| a.get(x, y) == b.get(x, y)))
}

/// The preview row of `label` as the model computed it before the row waited for idle: the
/// conclusion fitted to one row, drawn on an empty canvas.
fn direct_preview(label: &str) -> Bitmap {
    let bytes = logic_bytes();
    let at = index_of(&bytes, label);
    let table = statuses(&bytes, &[], &[]);
    let lib = Lib::new(
        Installed {
            name: "logic".into(),
            bytes,
        },
        table,
    )
    .expect("the library loads");
    let layout = fitted(&lib.statement(at).conclusion, TEXT_WIDTH, 1);
    let mut canvas = Canvas::default();
    canvas.layout(&layout, 1, 6, 6..7);
    canvas.into_bitmap()
}

/// A list opened on the logic library and settled, the cursor on `keys`' last stop.
fn settled_list(keys: &[Key]) -> Ui {
    let mut ui = support::ui();
    ui.press_settled(Enter);
    press(&mut ui, keys);
    assert!(!pending(&ui));
    ui
}

#[test]
fn opening_the_list_leaves_the_preview_pending_and_blank() {
    let mut ui = support::ui();
    ui.press(Enter);
    assert!(pending(&ui));
    assert!(preview_blank(&ui.frame()));
    assert_eq!(ui.previews_computed(), 0);
}

#[test]
fn every_key_that_moves_the_cursor_leaves_the_preview_pending_and_blank() {
    let keys: [(&str, Key, &[Key]); 6] = [
        ("down", Down, &[]),
        ("up", Up, &[Down, Down]),
        ("right", Right, &[]),
        ("left", Left, &[Right, Down]),
        ("kind", F3, &[Down]),
        ("a jumping letter", Letter('i'), &[]),
    ];
    for (what, key, before) in keys {
        let mut ui = settled_list(before);
        let (from, settled) = (label(&ui), ui.frame());
        assert!(
            !preview_blank(&settled),
            "{what}: the start shows a preview"
        );
        ui.press(key);
        assert_ne!(label(&ui), from, "{what}: the cursor moved");
        assert!(pending(&ui), "{what}");
        let blank = ui.frame();
        assert!(preview_blank(&blank), "{what}: the row is blank");
        let mut other = settled_list(before);
        other.press_settled(key);
        assert!(
            same_outside_preview(&blank, &other.frame()),
            "{what}: only the preview row differs"
        );
    }
}

#[test]
fn landing_from_the_sections_screen_leaves_the_preview_pending_and_blank() {
    let mut ui = ui_over(pack_all(&db(&sectioned())).unwrap(), &[], &[]);
    ui.press_settled(Enter);
    ui.press_settled(F5);
    raw(&mut ui, &[Down, Down, Enter]);
    assert!(pending(&ui));
    assert!(preview_blank(&ui.frame()));
    assert!(ui.idle());
    assert!(!preview_blank(&ui.frame()));
}

#[test]
fn idle_settles_the_preview_and_says_whether_the_row_changed() {
    for key in [Down, Right, F3, Letter('i')] {
        let mut ui = settled_list(&[]);
        ui.press(key);
        let blank = ui.frame();
        assert!(ui.idle(), "{key:?}");
        assert!(!pending(&ui));
        let frame = ui.frame();
        assert!(!preview_blank(&frame));
        assert!(same_outside_preview(&blank, &frame));
        assert!(!ui.idle(), "a second idle has nothing to do");
        assert_eq!(ui.frame(), frame);
    }
}

#[test]
fn the_settled_preview_is_the_conclusion_fitted_to_one_row() {
    let mut ui = support::ui();
    ui.press(Enter);
    for _ in 0..13 {
        ui.press(Down);
        ui.idle();
        let want = direct_preview(&label(&ui));
        assert!(!preview_blank(&want));
        assert!(same_preview(&ui.frame(), &want), "{}", label(&ui));
    }
}

#[test]
fn keys_that_leave_the_cursor_where_it_is_keep_a_settled_preview() {
    let mut ui = settled_list(&[]);
    let first = ui.frame();
    for key in [Up, Left, F2] {
        ui.press(key);
        assert!(!pending(&ui), "{key:?} at the top");
        assert!(same_preview(&ui.frame(), &first), "{key:?}");
    }
    let mut ui = settled_list(&[Right, Right, Right]);
    let last = label(&ui);
    ui.press(Right);
    assert_eq!(label(&ui), last);
    assert!(!pending(&ui));
    ui.press(Down);
    assert!(!pending(&ui));
    assert!(!preview_blank(&ui.frame()));
}

#[test]
fn a_jump_that_misses_keeps_a_settled_preview() {
    let mut ui = settled_list(&[Down]);
    let was = ui.frame();
    ui.press(Letter('z'));
    assert!(!pending(&ui));
    assert!(same_preview(&ui.frame(), &was));
    assert!(!preview_blank(&ui.frame()));
    assert!(!ui.idle());
}

#[test]
fn a_kind_that_keeps_the_cursor_on_its_assertion_keeps_a_settled_preview() {
    let mut ui = settled_list(&[Letter('i')]);
    assert_eq!(label(&ui), "id");
    let frame = ui.frame();
    ui.press(F3);
    assert_eq!(label(&ui), "id");
    assert!(!pending(&ui));
    assert!(same_preview(&ui.frame(), &frame));
}

#[test]
fn an_empty_filtered_list_settles_at_idle_with_no_frame_change() {
    let mut ui = ui_over(logic_bytes(), &[], &[]);
    ui.press_settled(Enter);
    press(&mut ui, &[F3, F3, F3]);
    assert!(matches!(
        ui.state(),
        State::List {
            len: 0,
            kind: mcui::Kind::Unchecked,
            ..
        }
    ));
    ui.press(Down);
    let frame = ui.frame();
    assert!(!ui.idle());
    assert!(!pending(&ui));
    assert_eq!(ui.frame(), frame);
    assert!(preview_blank(&frame));
}

#[test]
fn coming_back_to_the_list_from_a_screen_above_keeps_a_settled_preview() {
    let mut ui = settled_list(&[Down]);
    let frame = ui.frame();
    ui.press(Enter);
    assert_eq!(ui.depth(), 3);
    ui.press(Clear);
    assert_eq!(ui.depth(), 2);
    assert!(!pending(&ui));
    assert_eq!(ui.frame(), frame);
}

#[test]
fn several_moves_without_idle_compute_no_preview() {
    let mut ui = settled_list(&[]);
    let base = ui.previews_computed();
    raw(&mut ui, &[Down, Down, Right, Up, F3, Down, Letter('i')]);
    assert_eq!(ui.previews_computed(), base);
    assert!(pending(&ui));
    let at = label(&ui);
    assert!(ui.idle());
    assert_eq!(ui.previews_computed(), base + 1);
    assert!(same_preview(&ui.frame(), &direct_preview(&at)));
    assert!(!ui.idle());
    assert_eq!(ui.previews_computed(), base + 1);
}

#[test]
fn idle_does_nothing_on_a_screen_that_is_not_a_list() {
    let keys = [Enter, Clear, Down, F1, F2, F3, F4, F5, Right];
    let mut seen: HashSet<String> = HashSet::new();
    let mut frontier: VecDeque<Vec<Key>> = VecDeque::from([vec![]]);
    let mut expanded = HashSet::new();
    let bytes = pack_all(&db(&sectioned())).unwrap();
    let total = mcpack::library::read(&bytes).unwrap().len();
    let make = || {
        let stages = [total / 2, total]
            .map(|end| Stage {
                name: format!("stage {end}"),
                title: String::new(),
                end,
            })
            .to_vec();
        let table = vec![Status::Unchecked; total];
        Ui::with_stages(
            vec![Installed {
                name: "logic".into(),
                bytes: bytes.clone(),
            }],
            vec![stages],
            Statuses {
                libraries: vec![table],
                my_proofs: 2,
            },
        )
        .expect("the library loads")
    };
    while let Some(path) = frontier.pop_front() {
        let mut ui = make();
        raw_settled(&mut ui, &path);
        let state = ui.state();
        let name = format!("{state:?}");
        if !expanded.insert(name.clone()) || expanded.len() > 300 {
            continue;
        }
        let kind = name.split(' ').next().unwrap().to_string();
        seen.insert(kind.clone());
        if kind != "List" {
            let (frame, depth) = (ui.frame(), ui.depth());
            assert!(!ui.idle(), "{name}");
            assert_eq!(ui.state(), state, "{name}");
            assert_eq!(ui.frame(), frame, "{name}");
            assert_eq!(ui.depth(), depth, "{name}");
        }
        for key in keys {
            let mut next = path.clone();
            next.push(key);
            frontier.push_back(next);
        }
    }
    for screen in [
        "Libraries",
        "List",
        "Theorem",
        "Dist",
        "Proof",
        "Notice",
        "About",
        "StageAbout",
        "Checking",
        "Proving",
        "Sections",
    ] {
        assert!(
            seen.contains(screen),
            "the walk never reached {screen}: {seen:?}"
        );
    }
}

fn raw_settled(ui: &mut Ui, keys: &[Key]) {
    keys.iter().for_each(|&k| ui.press_settled(k));
}

#[test]
fn the_check_job_and_idle_do_not_disturb_each_other() {
    let mut ui = unchecked_ui(logic_bytes());
    ui.press(F2);
    assert!(matches!(ui.state(), State::Checking { .. }));
    let mut steps = 0;
    loop {
        let frame = ui.frame();
        assert!(!ui.idle());
        assert_eq!(ui.frame(), frame);
        match ui.step_check() {
            CheckStep::Checked | CheckStep::Failed => steps += 1,
            CheckStep::Finished => break,
            CheckStep::Idle => panic!("the checking screen closed"),
        }
    }
    assert_eq!(steps, 13);
    ui.press(F1);
    ui.press(Enter);
    assert!(pending(&ui));
    assert_eq!(ui.step_check(), CheckStep::Idle);
    assert!(ui.idle());
    assert!(!pending(&ui));
    assert_eq!(ui.step_check(), CheckStep::Idle);
    assert!(!ui.idle());
}
