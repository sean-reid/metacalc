mod support;

use mcui::Key::*;
use mcui::{Kind, State};
use support::{open, press, typed, ui};

fn list(state: State) -> (Option<String>, usize, usize, String, Kind, usize) {
    match state {
        State::List {
            label,
            cursor,
            top,
            find,
            kind,
            len,
            ..
        } => (label, cursor, top, find, kind, len),
        other => panic!("not a list: {other:?}"),
    }
}

fn theorem(state: State) -> (usize, usize, usize, usize, bool) {
    match state {
        State::Theorem {
            statement,
            node,
            scroll,
            folded,
            hidden,
            ..
        } => (statement, node, scroll, folded, hidden),
        other => panic!("not a theorem: {other:?}"),
    }
}

#[test]
fn the_libraries_screen_opens_a_library_and_keeps_its_place() {
    let mut ui = ui();
    assert_eq!(ui.state(), State::Libraries { cursor: 0, top: 0 });
    press(&mut ui, &[Up]);
    assert_eq!(ui.state(), State::Libraries { cursor: 0, top: 0 });
    press(&mut ui, &[Down, Down, Down]);
    assert_eq!(ui.state(), State::Libraries { cursor: 1, top: 0 });
    press(&mut ui, &[Up, Enter]);
    assert_eq!(ui.depth(), 2);
    press(&mut ui, &[Clear]);
    assert_eq!(ui.state(), State::Libraries { cursor: 0, top: 0 });
    press(&mut ui, &[Clear, Clear]);
    assert_eq!(ui.depth(), 1);
}

#[test]
fn the_list_scrolls_by_rows_and_stops_at_both_ends() {
    let mut ui = ui();
    press(&mut ui, &[Enter, Up]);
    let (label, cursor, top, _, _, len) = list(ui.state());
    assert_eq!((label.as_deref(), cursor, top), (Some("wi"), 0, 0));
    assert_eq!(len, 13);
    press(&mut ui, &[Down, Down, Down, Down]);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (4, 0));
    ui.press(Down);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (5, 1));
    for _ in 0..30 {
        ui.press(Down);
    }
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (12, 8));
    assert_eq!(list(ui.state()).0.as_deref(), Some("ax-many"));
    for _ in 0..30 {
        ui.press(Up);
    }
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (0, 0));
}

#[test]
fn left_and_right_page_the_list() {
    let mut ui = ui();
    press(&mut ui, &[Enter, Right]);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (5, 1));
    press(&mut ui, &[Right, Right, Right]);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (12, 8));
    press(&mut ui, &[Left]);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (7, 7));
    press(&mut ui, &[Left, Left]);
    assert_eq!((list(ui.state()).1, list(ui.state()).2), (0, 0));
}

#[test]
fn find_jumps_to_the_first_label_with_the_prefix_as_letters_are_typed() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "a");
    assert_eq!(list(ui.state()).0.as_deref(), Some("ax-1"));
    typed(&mut ui, "x-m");
    assert_eq!(list(ui.state()).0.as_deref(), Some("ax-mp"));
    typed(&mut ui, "a");
    let (label, _, _, find, _, _) = list(ui.state());
    assert_eq!(
        (label.as_deref(), find.as_str()),
        (Some("ax-many"), "ax-ma")
    );
    typed(&mut ui, "ny");
    assert_eq!(list(ui.state()).0.as_deref(), Some("ax-many"));
    let (_, cursor, top, ..) = list(ui.state());
    assert_eq!((cursor, top), (12, 8));
}

#[test]
fn a_prefix_that_matches_nothing_leaves_the_cursor_and_clear_removes_the_prefix_first() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "sy");
    assert_eq!(list(ui.state()).0.as_deref(), Some("syl"));
    let before = ui.frame();
    typed(&mut ui, "z");
    assert_eq!(list(ui.state()).0.as_deref(), Some("syl"));
    assert!(ui.frame() != before);
    ui.press(Clear);
    let (label, _, _, find, _, _) = list(ui.state());
    assert_eq!((label.as_deref(), find.as_str()), (Some("syl"), ""));
    assert_eq!(ui.depth(), 2);
    ui.press(Clear);
    assert_eq!(ui.depth(), 1);
}

#[test]
fn a_second_find_starts_over() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "syl");
    press(&mut ui, &[F2]);
    typed(&mut ui, "id");
    assert_eq!(list(ui.state()).0.as_deref(), Some("id"));
    assert_eq!(list(ui.state()).3, "id");
}

#[test]
fn kind_cycles_through_all_theorems_axioms_and_unchecked() {
    let mut ui = ui();
    press(&mut ui, &[Enter]);
    let kinds = [
        (Kind::All, 13),
        (Kind::Theorems, 6),
        (Kind::Axioms, 7),
        (Kind::Unchecked, 2),
        (Kind::All, 13),
    ];
    for (i, (kind, len)) in kinds.into_iter().enumerate() {
        if i > 0 {
            ui.press(F3);
        }
        let state = list(ui.state());
        assert_eq!((state.4, state.5), (kind, len), "step {i}");
    }
}

#[test]
fn changing_the_kind_keeps_the_cursor_on_the_same_or_the_next_assertion() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "mpd");
    press(&mut ui, &[F3]);
    assert_eq!(list(ui.state()).0.as_deref(), Some("mpd"));
    press(&mut ui, &[F3]);
    assert_eq!(list(ui.state()).0.as_deref(), Some("ax-many"));
    press(&mut ui, &[F3]);
    assert_eq!(
        list(ui.state()).0.as_deref(),
        Some("id"),
        "nothing unchecked follows, so the last"
    );
    press(&mut ui, &[F3, F3]);
    assert_eq!(list(ui.state()).0.as_deref(), Some("id"));
}

#[test]
fn clear_from_a_theorem_returns_to_the_same_row_of_the_same_list() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F3, F3]);
    press(&mut ui, &[Down, Down]);
    let before = list(ui.state());
    ui.press(Enter);
    assert!(matches!(ui.state(), State::Theorem { .. }));
    ui.press(Clear);
    assert_eq!(list(ui.state()), before);
}

#[test]
fn the_cursor_starts_on_the_conclusion_and_moves_over_the_tree() {
    let mut ui = ui();
    open(&mut ui, "syl");
    assert_eq!(theorem(ui.state()), (2, 0, 0, 0, false));
    press(&mut ui, &[Up]);
    assert_eq!(theorem(ui.state()).1, 0);
    ui.press(Down);
    assert_eq!(theorem(ui.state()).1, 1);
    ui.press(Right);
    assert_eq!(theorem(ui.state()).1, 2);
    ui.press(Right);
    assert_eq!(theorem(ui.state()).1, 2);
    ui.press(Left);
    assert_eq!(theorem(ui.state()).1, 1);
    ui.press(Left);
    assert_eq!(theorem(ui.state()).1, 1);
    ui.press(Down);
    assert_eq!(theorem(ui.state()).1, 1);
}

#[test]
fn down_enters_the_first_child_and_up_leaves_to_the_parent() {
    let mut ui = ui();
    open(&mut ui, "ax-2");
    let node = |ui: &mcui::Ui| theorem(ui.state()).1;
    press(&mut ui, &[Down]);
    assert_eq!(node(&ui), 1);
    press(&mut ui, &[Down]);
    assert_eq!(node(&ui), 2);
    press(&mut ui, &[Right, Down]);
    assert_eq!(node(&ui), 4);
    press(&mut ui, &[Right, Right]);
    assert_eq!(node(&ui), 5);
    press(&mut ui, &[Up]);
    assert_eq!(node(&ui), 3);
    press(&mut ui, &[Up]);
    assert_eq!(node(&ui), 1);
    press(&mut ui, &[Right]);
    assert_eq!(node(&ui), 6);
    press(&mut ui, &[Up, Up]);
    assert_eq!(node(&ui), 0);
}

#[test]
fn left_and_right_at_a_root_move_between_the_statements() {
    let mut ui = ui();
    open(&mut ui, "syl");
    ui.press(Right);
    assert_eq!(theorem(ui.state()).0, 2);
    press(&mut ui, &[Left]);
    assert_eq!(theorem(ui.state()).0, 1);
    press(&mut ui, &[Left]);
    assert_eq!(theorem(ui.state()).0, 0);
    press(&mut ui, &[Left]);
    assert_eq!(theorem(ui.state()).0, 0);
    press(&mut ui, &[Right, Right]);
    assert_eq!(theorem(ui.state()).0, 2);
}

#[test]
fn enter_and_fold_toggle_the_subexpression_under_the_cursor() {
    let mut ui = ui();
    open(&mut ui, "ax-2");
    press(&mut ui, &[Down, Right]);
    let open_frame = ui.frame();
    assert_eq!(theorem(ui.state()).3, 0);
    ui.press(Enter);
    assert_eq!(theorem(ui.state()), (0, 6, 0, 1, true));
    assert!(ui.frame() != open_frame);
    ui.press(Down);
    assert_eq!(
        theorem(ui.state()).1,
        6,
        "a folded subexpression cannot be entered"
    );
    ui.press(F1);
    assert_eq!(theorem(ui.state()), (0, 6, 0, 0, false));
    assert!(ui.frame() == open_frame);
    ui.press(Down);
    assert_eq!(theorem(ui.state()).1, 7);
}

#[test]
fn a_leaf_does_not_fold() {
    let mut ui = ui();
    open(&mut ui, "ax-1");
    press(&mut ui, &[Down, Enter]);
    assert_eq!(theorem(ui.state()).3, 0);
}

#[test]
fn the_view_scrolls_to_follow_the_cursor_over_statements_taller_than_the_screen() {
    let mut ui = ui();
    open(&mut ui, "ax-many");
    let (statement, _, scroll, _, _) = theorem(ui.state());
    assert_eq!((statement, scroll), (8, 3));
    for want in [
        (7, 3),
        (6, 3),
        (5, 3),
        (4, 3),
        (3, 3),
        (2, 2),
        (1, 1),
        (0, 0),
    ] {
        ui.press(Left);
        let (statement, _, scroll, _, _) = theorem(ui.state());
        assert_eq!((statement, scroll), want);
    }
    for want in [
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 1),
        (7, 2),
        (8, 3),
    ] {
        ui.press(Right);
        let (statement, _, scroll, _, _) = theorem(ui.state());
        assert_eq!((statement, scroll), want);
    }
}

#[test]
fn proof_nodes_descend_and_ascend_with_the_selection_restored() {
    let mut ui = ui();
    open(&mut ui, "syl");
    ui.press(F3);
    let at = |ui: &mcui::Ui| match ui.state() {
        State::Proof { path, selected, .. } => (path, selected),
        other => panic!("not a proof: {other:?}"),
    };
    assert_eq!(at(&ui), (vec![], 0));
    press(&mut ui, &[Up]);
    assert_eq!(at(&ui), (vec![], 0));
    press(&mut ui, &[Down, Down]);
    assert_eq!(at(&ui), (vec![], 1));
    ui.press(Enter);
    assert_eq!(at(&ui), (vec![1], 0));
    ui.press(Enter);
    assert_eq!(at(&ui), (vec![1, 0], 0));
    ui.press(Enter);
    assert_eq!(at(&ui), (vec![1, 0], 0), "a hypothesis has no subgoals");
    ui.press(F2);
    assert_eq!(at(&ui), (vec![1], 0));
    ui.press(Left);
    assert_eq!(at(&ui), (vec![], 1));
    ui.press(Left);
    assert_eq!(at(&ui), (vec![], 1));
}

#[test]
fn thm_opens_the_cited_theorem_and_clear_comes_back_to_the_same_node() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Down, Enter, F3]);
    match ui.state() {
        State::Theorem { label, .. } => assert_eq!(label, "a1i"),
        other => panic!("not a theorem: {other:?}"),
    }
    ui.press(Clear);
    match ui.state() {
        State::Proof { path, .. } => assert_eq!(path, vec![1]),
        other => panic!("not a proof: {other:?}"),
    }
    ui.press(Down);
    ui.press(Enter);
    ui.press(F3);
    assert!(
        matches!(ui.state(), State::Proof { .. }),
        "a hypothesis cites no theorem"
    );
}

#[test]
fn an_axiom_has_no_proof_to_read() {
    let mut ui = ui();
    open(&mut ui, "ax-2");
    ui.press(F3);
    assert_eq!(
        ui.state(),
        State::Notice {
            title: "Proof".into()
        }
    );
    ui.press(Clear);
    assert!(matches!(ui.state(), State::Theorem { .. }));
}

#[test]
fn keys_without_a_function_on_a_screen_change_nothing() {
    let mut ui = ui();
    let before = ui.frame();
    press(&mut ui, &[Left, Right, F4, F5, Letter('a'), Clear]);
    assert_eq!(ui.depth(), 1);
    assert!(ui.frame() == before);
}
