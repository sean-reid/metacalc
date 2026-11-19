mod support;

use mcui::Key::*;
use mcui::{Kind, State};
use support::{check_frame, open, press, typed, ui};

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
        (Kind::Unchecked, 4),
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
    press(&mut ui, &[Clear]);
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
fn with_more_statements_than_rows_each_hypothesis_takes_a_row_and_the_conclusion_stays_open() {
    let mut ui = support::ui_over(support::long_bytes(), &[], &[]);
    open(&mut ui, "axlong");
    let (statement, node, scroll, folded, _) = theorem(ui.state());
    assert_eq!((statement, node), (7, 0));
    assert!(folded >= 7, "each hypothesis is folded to its row");
    assert!(scroll > 0, "the conclusion is in view");
    press(&mut ui, &[Down]);
    let (_, node, _, _, hidden) = theorem(ui.state());
    assert_ne!(node, 0);
    assert!(!hidden, "the conclusion shows its subexpressions");
    check_frame(&ui.frame(), "theorem-more-statements-than-rows");
    press(
        &mut ui,
        &[Up, Left, Left, Left, Left, Left, Left, Left, Left],
    );
    let (statement, _, scroll, _, _) = theorem(ui.state());
    assert_eq!((statement, scroll), (0, 0));
    check_frame(&ui.frame(), "theorem-more-statements-than-rows-top");
}

#[test]
fn a_flat_statement_over_six_rows_scrolls_a_row_at_a_time_with_up_and_down() {
    let mut ui = support::ui_over(support::flat_bytes(40), &[], &[]);
    open(&mut ui, "axflat");
    let source = support::nested(40);
    let tokens: Vec<&str> = std::iter::once("|-").chain(source.split(' ')).collect();
    let lines = support::font_lines(&tokens, 92);
    assert!(lines.len() > 6);
    let last = lines.len() - 6;
    let first_row_is_line = |ui: &mcui::Ui, line: usize| {
        let row: Vec<&str> = lines[line].iter().map(String::as_str).collect();
        support::tokens_at(&ui.frame(), 3, 1, &row)
    };
    let (_, node, top, _, _) = theorem(ui.state());
    assert_eq!((node, top), (0, 0));
    assert!(first_row_is_line(&ui, 0));
    check_frame(&ui.frame(), "theorem-flat-top");
    for line in 1..=last {
        ui.press(Down);
        assert_eq!(theorem(ui.state()).2, line);
        assert!(first_row_is_line(&ui, line), "after {line} presses of Down");
    }
    check_frame(&ui.frame(), "theorem-flat-end");
    ui.press(Down);
    assert_eq!(
        theorem(ui.state()).2,
        last,
        "the view stops at the last row"
    );
    assert!(first_row_is_line(&ui, last));
    for line in (0..last).rev() {
        ui.press(Up);
        assert_eq!(theorem(ui.state()).2, line);
        assert!(first_row_is_line(&ui, line), "after Up to line {line}");
    }
    ui.press(Up);
    assert_eq!(theorem(ui.state()).2, 0);
    assert_eq!(
        theorem(ui.state()).1,
        0,
        "the cursor stays on the statement"
    );
}

#[test]
fn up_and_down_on_a_tree_statement_leave_the_cursor_in_view() {
    let mut ui = support::ui_over(support::long_bytes(), &[], &[]);
    open(&mut ui, "axlong");
    let (statement, _, scroll, _, _) = theorem(ui.state());
    assert_eq!(statement, 7);
    assert!(scroll > 0, "the conclusion starts below the first rows");
    press(&mut ui, &[Left; 7]);
    assert_eq!(theorem(ui.state()).0, 0);
    assert_eq!(theorem(ui.state()).2, 0);
    loop {
        let before = theorem(ui.state());
        ui.press(Down);
        if theorem(ui.state()).1 == before.1 {
            break;
        }
    }
    let at_leaf = (theorem(ui.state()), ui.frame());
    assert_ne!(at_leaf.0.1, 0, "the cursor is inside the first hypothesis");
    for _ in 0..3 {
        ui.press(Down);
    }
    assert_eq!(
        theorem(ui.state()),
        at_leaf.0,
        "Down on a leaf scrolls nothing"
    );
    assert!(ui.frame() == at_leaf.1);
    press(&mut ui, &[Up; 8]);
    press(&mut ui, &[Right; 7]);
    let (statement, node, scroll, _, _) = theorem(ui.state());
    assert_eq!((statement, node), (7, 0));
    assert!(scroll > 0);
    let rooted = ui.frame();
    press(&mut ui, &[Up, Up]);
    assert_eq!(
        theorem(ui.state()).2,
        scroll,
        "Up on a root scrolls nothing"
    );
    assert!(ui.frame() == rooted);
}

#[test]
fn the_list_preview_of_a_flat_statement_over_one_row_ends_in_an_ellipsis() {
    let mut ui = support::ui_over(support::flat_bytes(40), &[], &[]);
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "axflat");
    let frame = ui.frame();
    assert!(support::tokens_at(
        &frame,
        1,
        6,
        &["|-", "(", "ph", "->", "("]
    ));
    assert!(support::ends_in_ellipsis(&frame, 6));
    check_frame(&frame, "list-flat-preview-cut");
}

fn screen_of(ui: &mcui::Ui) -> String {
    match ui.state() {
        State::Theorem { label, .. } => format!("theorem {label}"),
        State::Proof { label, .. } => format!("proof {label}"),
        State::List { .. } => "list".into(),
        State::Libraries { .. } => "libraries".into(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn hopping_between_proofs_and_theorems_keeps_the_newest_screens_and_clear_then_reaches_the_list() {
    let mut ui = support::ui_over(support::hops_bytes(), &[], &[]);
    open(&mut ui, "t12");
    assert_eq!(ui.depth(), 3);
    for hop in 0..11 {
        press(&mut ui, &[F3, F3]);
        assert!(ui.depth() <= mcui::MAX_DEPTH, "hop {hop}");
    }
    assert_eq!(ui.depth(), mcui::MAX_DEPTH);
    let mut back = Vec::new();
    while screen_of(&ui) != "list" {
        back.push(screen_of(&ui));
        ui.press(Clear);
    }
    assert_eq!(
        back,
        [
            "theorem t1",
            "proof t2",
            "theorem t2",
            "proof t3",
            "theorem t3",
            "proof t4",
        ]
    );
    assert_eq!(ui.depth(), 2);
    press(&mut ui, &[Clear, Clear]);
    assert_eq!(screen_of(&ui), "libraries");
}

#[test]
fn one_rejected_assertion_leaves_every_statement_a_tree_and_the_list_unflattened() {
    let sound = support::logic_bytes();
    let alid = support::index_of(&sound, "alid") as u16;
    let rejected = support::refreshed(support::patch_reference(&sound, "a1i", 0, alid));
    assert!(mcpack::library::export(&rejected).is_err());
    let mut ui = support::ui_over(rejected, &[], &[]);
    press(&mut ui, &[Enter]);
    assert!(support::title_is(&ui.frame(), "logic", "12/13"));
    ui.press(Clear);
    open(&mut ui, "syl");
    ui.press(Down);
    assert_ne!(
        theorem(ui.state()).1,
        0,
        "the conclusion has subexpressions"
    );
}

fn notice_title(ui: &mcui::Ui) -> String {
    match ui.state() {
        State::Notice { title } => title,
        other => panic!("not a notice: {other:?}"),
    }
}

#[test]
fn with_one_rejected_assertion_why_reads_the_proof_of_an_unrelated_theorem() {
    for label in ["a2i", "id", "alid", "syl"] {
        let mut ui = support::rejected_ui();
        open(&mut ui, label);
        ui.press(F3);
        assert!(
            matches!(ui.state(), State::Proof { label: ref l, .. } if l == label),
            "{label}"
        );
    }
}

#[test]
fn why_on_the_rejected_theorem_says_its_proof_failed() {
    let mut ui = support::rejected_ui();
    open(&mut ui, "a1i");
    ui.press(F3);
    assert_eq!(notice_title(&ui), "Proof");
    ui.press(Clear);
    assert!(matches!(ui.state(), State::Theorem { ref label, .. } if label == "a1i"));
}

#[test]
fn try_opens_on_an_unrelated_theorem_and_on_the_rejected_one() {
    for label in ["a2i", "syl", "a1i"] {
        let mut ui = support::rejected_ui();
        open(&mut ui, label);
        ui.press(F4);
        assert!(
            matches!(ui.state(), State::Proving { label: ref l, .. } if l == label),
            "{label}"
        );
    }
}

#[test]
fn the_list_title_and_marks_do_not_depend_on_the_rejection() {
    let (bytes, table) = support::rejected_a1i();
    let lib = mcui::data::Lib::new(
        mcui::data::Installed {
            name: "logic".into(),
            bytes,
        },
        table,
    )
    .unwrap();
    let mark = |label: &str| lib.mark((0..lib.len()).find(|&i| lib.label(i) == label).unwrap());
    assert_eq!(mark("a1i"), mcui::Mark::Failed);
    assert_eq!(mark("syl"), mcui::Mark::Resting);
    assert_eq!(mark("a2i"), mcui::Mark::Checked);
    let mut ui = support::rejected_ui();
    press(&mut ui, &[Enter]);
    assert!(support::title_is(&ui.frame(), "logic", "12/13"));
}

fn lib_with(bytes: Vec<u8>, table: Vec<mcui::Status>) -> mcui::data::Lib {
    let installed = mcui::data::Installed {
        name: "logic".into(),
        bytes,
    };
    mcui::data::Lib::new(installed, table).unwrap()
}

fn index_in(lib: &mcui::data::Lib, label: &str) -> usize {
    (0..lib.len()).find(|&i| lib.label(i) == label).unwrap()
}

#[test]
fn a_table_that_calls_a_rejected_theorem_checked_does_not_get_a_check_mark() {
    let (bytes, _) = support::rejected_a1i();
    let table = support::statuses(&bytes, &[], &[]);
    let lib = lib_with(bytes.clone(), table.clone());
    let a1i = index_in(&lib, "a1i");
    assert!(lib.unproved(a1i));
    assert_eq!(lib.mark(a1i), mcui::Mark::Failed);
    assert_eq!(lib.mark(index_in(&lib, "syl")), mcui::Mark::Resting);
    assert_eq!(lib.mark(index_in(&lib, "a2i")), mcui::Mark::Checked);
    let mut ui = support::ui_over(bytes, &[], &[]);
    open(&mut ui, "a1i");
    ui.press(F3);
    assert_eq!(notice_title(&ui), "Proof");
}

#[test]
fn the_installed_table_is_unchanged_by_a_demotion() {
    let (bytes, _) = support::rejected_a1i();
    let table = support::statuses(&bytes, &[], &[]);
    let lib = lib_with(bytes, table.clone());
    let a1i = index_in(&lib, "a1i");
    assert_eq!(lib.status(a1i), mcui::Status::Failed);
    for (i, &status) in table.iter().enumerate() {
        assert_eq!(lib.installed_status(i), status, "{}", lib.label(i));
    }
    assert_eq!(lib.installed_status(a1i), mcui::Status::Checked);
}

#[test]
fn a_table_that_calls_a_sound_theorem_failed_still_lets_why_read_its_proof() {
    let (bytes, _) = support::rejected_a1i();
    let table = support::statuses(&bytes, &[], &["a2i"]);
    let lib = lib_with(bytes.clone(), table);
    let a2i = index_in(&lib, "a2i");
    assert!(!lib.unproved(a2i));
    assert_eq!(lib.mark(a2i), mcui::Mark::Failed);
    let mut ui = support::ui_over(bytes, &[], &["a2i"]);
    open(&mut ui, "a2i");
    ui.press(F3);
    assert!(matches!(ui.state(), State::Proof { .. }));
}

#[test]
fn the_rejected_list_and_the_screens_name_the_same_theorems() {
    let (bytes, _) = support::rejected_a1i();
    let export = mcpack::library::export_accepted(&bytes).unwrap();
    let lib = lib_with(bytes.clone(), support::statuses(&bytes, &[], &[]));
    let unproved: Vec<&str> = (0..lib.len())
        .filter(|&i| lib.unproved(i))
        .map(|i| lib.label(i))
        .collect();
    let rejected: Vec<&str> = export.rejected.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(unproved, rejected);
    for i in 0..lib.len() {
        assert_eq!(lib.verify(i), !rejected.contains(&lib.label(i)));
    }
}

#[test]
fn every_key_a_notice_offers_closes_it() {
    for key in [F1, Clear, Enter] {
        let mut ui = ui();
        press(&mut ui, &[Down, Down, F2]);
        assert!(matches!(ui.state(), State::Notice { .. }));
        assert!(support::key_label_is(&ui.frame(), 0, "Back"));
        ui.press(key);
        assert_eq!(ui.depth(), 1, "{key:?}");
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

#[test]
fn up_from_the_first_subgoal_puts_the_cursor_on_the_goal_and_down_takes_it_back() {
    let mut ui = ui();
    open(&mut ui, "syl");
    ui.press(F3);
    let on_goal = |ui: &mcui::Ui| match ui.state() {
        State::Proof {
            on_goal, selected, ..
        } => (on_goal, selected),
        other => panic!("{other:?}"),
    };
    assert_eq!(on_goal(&ui), (false, 0));
    let subgoal = ui.frame();
    ui.press(Up);
    assert_eq!(on_goal(&ui), (true, 0));
    assert!(ui.frame() != subgoal);
    ui.press(Enter);
    assert_eq!(on_goal(&ui), (true, 0), "the goal cannot be entered");
    ui.press(Down);
    assert_eq!(on_goal(&ui), (false, 0));
    assert!(ui.frame() == subgoal);
}

#[test]
fn a_node_without_subgoals_keeps_the_cursor_on_its_goal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Enter]);
    match ui.state() {
        State::Proof { on_goal, .. } => assert!(on_goal),
        other => panic!("{other:?}"),
    }
}

fn proof(ui: &mcui::Ui) -> (Vec<usize>, usize, bool) {
    match ui.state() {
        State::Proof {
            path,
            selected,
            on_goal,
            ..
        } => (path, selected, on_goal),
        other => panic!("not a proof: {other:?}"),
    }
}

#[test]
fn arrows_and_enter_on_a_node_without_subgoals_change_nothing() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Enter]);
    assert_eq!(proof(&ui), (vec![0], 0, true));
    let frame = ui.frame();
    press(&mut ui, &[Down, Enter, Right, Up, Down, Down]);
    assert_eq!(proof(&ui), (vec![0], 0, true));
    assert!(ui.frame() == frame, "the goal keeps its cursor");
}

#[test]
fn a_proof_whose_root_is_a_hypothesis_has_nothing_to_enter() {
    let mut ui = support::extra_ui();
    open(&mut ui, "foo");
    press(&mut ui, &[F3]);
    assert_eq!(proof(&ui), (vec![], 0, true));
    let frame = ui.frame();
    press(&mut ui, &[Down, Enter, Right, Up, Down]);
    assert_eq!(proof(&ui), (vec![], 0, true));
    assert!(ui.frame() == frame);
    ui.press(F3);
    assert!(
        matches!(ui.state(), State::Proof { .. }),
        "a hypothesis cites no theorem"
    );
}

#[test]
fn hide_keeps_the_cursor_where_it_was() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Down]);
    assert_eq!(proof(&ui), (vec![], 1, false));
    ui.press(F1);
    assert_eq!(proof(&ui), (vec![], 1, false));
    press(&mut ui, &[Up, Up]);
    assert_eq!(proof(&ui), (vec![], 0, true));
    press(&mut ui, &[F1, F1]);
    assert_eq!(proof(&ui), (vec![], 0, true));
}

fn proof_scroll(ui: &mcui::Ui) -> (usize, bool) {
    match ui.state() {
        State::Proof {
            scroll, on_goal, ..
        } => (scroll, on_goal),
        other => panic!("not a proof: {other:?}"),
    }
}

#[test]
fn an_unfolded_goal_scrolls_a_row_at_a_time_before_the_cursor_reaches_the_subgoals() {
    let mut ui = support::ui_over(support::tall_goal_bytes(), &[], &[]);
    open(&mut ui, "tl");
    press(&mut ui, &[F3, F1]);
    let source = support::nested(30);
    let tokens: Vec<&str> = std::iter::once("|-").chain(source.split(' ')).collect();
    let lines = support::font_lines(&tokens, 92);
    let first_row_is_line = |ui: &mcui::Ui, line: usize| {
        let row: Vec<&str> = lines[line].iter().map(String::as_str).collect();
        support::tokens_at(&ui.frame(), 3, 1, &row)
    };
    let by_shown = |ui: &mcui::Ui, scroll: usize| {
        let row = (1 + lines.len()).saturating_sub(scroll);
        let shown = (1..=6).contains(&row);
        assert_eq!(
            shown && support::text_at(&ui.frame(), 1, row, "by ax-mp", false),
            shown
        );
        shown
    };
    let (bottom, on_goal) = proof_scroll(&ui);
    assert!(!on_goal && bottom >= 3, "the first subgoal is in view");
    assert!(first_row_is_line(&ui, bottom));
    assert!(by_shown(&ui, bottom));
    check_frame(&ui.frame(), "proof-tall-goal-opened");
    ui.press(Up);
    assert_eq!(proof_scroll(&ui), (bottom, true));
    for row in (0..bottom).rev() {
        ui.press(Up);
        assert_eq!(proof_scroll(&ui), (row, true));
        assert!(first_row_is_line(&ui, row), "scrolled up to {row}");
    }
    ui.press(Up);
    assert_eq!(proof_scroll(&ui), (0, true));
    assert!(!by_shown(&ui, 0), "the by line is below the first six rows");
    check_frame(&ui.frame(), "proof-tall-goal-top");
    let mut presses = 0;
    while proof_scroll(&ui).1 {
        let at = proof_scroll(&ui).0;
        let by_in_view = by_shown(&ui, at);
        ui.press(Down);
        presses += 1;
        if by_in_view {
            assert!(
                !proof_scroll(&ui).1,
                "the cursor moves on once the by line shows"
            );
        } else {
            assert_eq!(proof_scroll(&ui), (at + 1, true));
            assert!(
                first_row_is_line(&ui, at + 1),
                "scrolled down to {}",
                at + 1
            );
        }
        if presses == 2 {
            check_frame(&ui.frame(), "proof-tall-goal-middle");
        }
        assert!(presses < 40, "the cursor never left the goal");
    }
    let last = presses - 1;
    assert!(last >= 3, "the goal has more than six rows");
    assert!(
        last + 5 <= lines.len(),
        "one row earlier the by line is below the window"
    );
    assert_eq!(proof(&ui).1, 0);
    ui.press(Up);
    assert_eq!(proof_scroll(&ui), (last + 1, true));
    ui.press(Up);
    assert_eq!(proof_scroll(&ui), (last, true));
}

#[test]
fn a_goal_with_no_subgoals_scrolls_with_up_and_down_and_stops_at_its_last_row() {
    let mut ui = support::ui_over(support::tall_goal_bytes(), &[], &[]);
    open(&mut ui, "lh");
    press(&mut ui, &[F3, F1]);
    assert_eq!(proof_scroll(&ui), (0, true));
    let mut at = 0;
    for _ in 0..40 {
        ui.press(Down);
        at = proof_scroll(&ui).0;
    }
    assert!(at >= 3);
    ui.press(Down);
    assert_eq!(proof_scroll(&ui), (at, true));
    for _ in 0..40 {
        ui.press(Up);
    }
    assert_eq!(proof_scroll(&ui), (0, true));
}

#[test]
fn eight_subgoals_scroll_the_view_to_the_highlight() {
    let mut ui = support::extra_ui();
    open(&mut ui, "many2");
    ui.press(F3);
    let inverted_rows = |ui: &mcui::Ui| -> Vec<usize> {
        let frame = ui.frame();
        (1..7).filter(|&r| frame.get(0, 8 * r + 1)).collect()
    };
    assert_eq!(inverted_rows(&ui), [3]);
    for k in 1..8 {
        ui.press(Down);
        assert_eq!(proof(&ui), (vec![], k, false));
        assert_eq!(inverted_rows(&ui), [(3 + k).min(6)], "subgoal {k}");
    }
    ui.press(Down);
    assert_eq!(proof(&ui).1, 7);
    ui.press(Enter);
    assert_eq!(proof(&ui).0, vec![7]);
}

#[test]
fn a_long_find_prefix_leaves_every_later_frame_drawable() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    support::typed(&mut ui, "abcdefghijklmnopqrstuvwxyzabcd");
    let frame = ui.frame();
    let find = "abcdefghijklmnopqrstuvwxyzabcd";
    let right = format!(
        "{} {}",
        mcui::canvas::tail(find, mcui::canvas::PREFIX_WIDTH - 9),
        mcglyph::CROSS
    );
    let left = mcui::canvas::truncate(
        "logic",
        96 - 2 - mcglyph::Font::builtin().text_width(&right) - 3,
    );
    assert!(support::title_is(&frame, &left, &right));
    press(&mut ui, &[Down, F3, F3, F3, F3, F2]);
    let _ = ui.frame();
}

#[test]
fn the_no_match_cross_goes_when_the_cursor_moves_or_the_kind_changes() {
    for key in [Down, Up, Left, Right, F3] {
        let mut ui = ui();
        press(&mut ui, &[Enter, F2]);
        typed(&mut ui, "synx");
        assert!(support::title_is(
            &ui.frame(),
            "logic",
            &format!("synx {}", mcglyph::CROSS)
        ));
        ui.press(key);
        let right = "synx";
        if key == F3 {
            let cross = format!("{right} {}", mcglyph::CROSS);
            assert!(
                support::title_is(&ui.frame(), "logic:thm", &cross),
                "the prefix is searched for again"
            );
        } else {
            let mut fresh = self::ui();
            press(&mut fresh, &[Enter]);
            let (a, b) = (ui.frame(), fresh.frame());
            assert!(
                (0..8).all(|y| (0..96).all(|x| a.get(x, y) == b.get(x, y))),
                "{key:?} restores the title"
            );
        }
    }
}

#[test]
fn find_survives_a_kind_change_and_searches_the_filtered_list() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "sy");
    assert_eq!(list(ui.state()).0.as_deref(), Some("syl"));
    press(&mut ui, &[F3]);
    let (label, _, _, find, kind, _) = list(ui.state());
    assert_eq!(
        (label.as_deref(), find.as_str(), kind),
        (Some("syl"), "sy", Kind::Theorems)
    );
    press(&mut ui, &[F3]);
    let (label, _, _, find, kind, _) = list(ui.state());
    assert_eq!(
        (label.as_deref(), find.as_str(), kind),
        (Some("ax-many"), "sy", Kind::Axioms)
    );
    assert!(support::title_is(
        &ui.frame(),
        "logic:ax",
        &format!("sy {}", mcglyph::CROSS)
    ));
    typed(&mut ui, "l");
    let (label, _, _, _, _, _) = list(ui.state());
    assert_eq!(
        label.as_deref(),
        Some("ax-many"),
        "syl is not an axiom, so the cursor stays"
    );
    assert!(support::title_is(
        &ui.frame(),
        "logic:ax",
        &format!("syl {}", mcglyph::CROSS)
    ));
}

#[test]
fn a_kind_with_nothing_in_it_shows_an_empty_list_that_does_not_open() {
    let mut ui = support::ui_with(&[], &[]);
    press(&mut ui, &[Enter, F3, F3, F3]);
    let (label, cursor, top, _, kind, len) = list(ui.state());
    assert_eq!(
        (label, cursor, top, kind, len),
        (None, 0, 0, Kind::Unchecked, 0)
    );
    press(
        &mut ui,
        &[Enter, F1, Up, Down, Left, Right, F2, Letter('a')],
    );
    assert_eq!(ui.depth(), 2);
    ui.press(F3);
    assert_eq!(list(ui.state()).5, 13);
}

#[test]
fn my_proofs_opens_the_not_available_frame_and_verify_needs_a_library() {
    let mut ui = ui();
    press(&mut ui, &[Down, Enter]);
    assert_eq!(
        ui.state(),
        State::Notice {
            title: "My proofs".into()
        }
    );
    ui.press(Clear);
    assert_eq!(ui.state(), State::Libraries { cursor: 1, top: 0 });
    ui.press(F2);
    assert_eq!(
        ui.state(),
        State::Notice {
            title: "Verify".into()
        }
    );
}

#[test]
fn notice_and_about_leave_on_clear_or_enter_and_ignore_the_rest() {
    for open_with in [vec![Down, F2], vec![F3]] {
        for leave in [Clear, Enter] {
            let mut ui = ui();
            press(&mut ui, &open_with);
            let frame = ui.frame();
            press(
                &mut ui,
                &[Up, Down, Left, Right, F1, F2, F3, F4, F5, On, Letter('x')],
            );
            assert!(ui.frame() == frame);
            assert_eq!(ui.depth(), 2);
            ui.press(leave);
            assert_eq!(ui.depth(), 1);
        }
    }
}

#[test]
fn dist_scrolls_one_row_at_a_time_and_ignores_left_and_right() {
    let mut ui = support::extra_ui();
    open(&mut ui, "ax-dvs");
    ui.press(F2);
    let top = |ui: &mcui::Ui| match ui.state() {
        State::Dist { top, .. } => top,
        other => panic!("{other:?}"),
    };
    assert_eq!(top(&ui), 0);
    ui.press(Up);
    assert_eq!(top(&ui), 0);
    let first = ui.frame();
    press(&mut ui, &[Left, Right]);
    assert!(ui.frame() == first);
    ui.press(Down);
    assert_eq!(top(&ui), 1);
    assert!(ui.frame() != first);
    for _ in 0..30 {
        ui.press(Down);
    }
    let last = top(&ui);
    assert!(last >= 2);
    let bottom = ui.frame();
    ui.press(Down);
    assert_eq!(top(&ui), last);
    assert!(ui.frame() == bottom);
    ui.press(Up);
    assert_eq!(top(&ui), last - 1);
}

#[test]
fn clear_from_dist_returns_to_the_same_cursor_and_scroll() {
    let mut ui = ui();
    open(&mut ui, "ax-many");
    press(&mut ui, &[Left, Left]);
    let before = ui.state();
    let frame = ui.frame();
    ui.press(F2);
    assert!(matches!(ui.state(), State::Dist { .. }));
    ui.press(Clear);
    assert_eq!(ui.state(), before);
    assert!(ui.frame() == frame);
}

#[test]
fn a_folded_root_cannot_be_entered_and_unfolds_again() {
    let mut ui = ui();
    open(&mut ui, "syl");
    ui.press(Enter);
    assert_eq!(theorem(ui.state()), (2, 0, 0, 1, true));
    press(&mut ui, &[Down, Right]);
    assert_eq!(theorem(ui.state()).1, 0);
    ui.press(Enter);
    assert_eq!(theorem(ui.state()), (2, 0, 0, 0, false));
}

fn unchecked_labels(ui: &mut mcui::Ui) -> Vec<String> {
    press(ui, &[Enter, F3, F3, F3]);
    let (_, _, _, _, kind, len) = list(ui.state());
    assert_eq!(kind, Kind::Unchecked);
    press(
        ui,
        &[Up, Up, Up, Up, Up, Up, Up, Up, Up, Up, Up, Up, Up, Up],
    );
    let mut labels = Vec::new();
    for _ in 0..len {
        labels.push(list(ui.state()).0.unwrap());
        ui.press(Down);
    }
    labels
}

#[test]
fn the_unchecked_kind_follows_the_marks_so_it_lists_failed_and_resting_assertions() {
    let mut ui = ui();
    assert_eq!(unchecked_labels(&mut ui), ["a2i", "mpd", "syl", "id"]);
}

#[test]
fn an_assertion_failed_by_a_forward_reference_is_listed_as_unchecked() {
    let sound = support::logic_bytes();
    let lib = mcpack::library::read(&sound).unwrap();
    let at = |label: &str| lib.iter().position(|a| a.label == label).unwrap() as u16;
    let patched = support::refreshed(support::patch_reference(&sound, "a1i", 0, at("alid")));
    let mut ui = support::ui_over(patched, &[], &[]);
    assert_eq!(unchecked_labels(&mut ui), ["a1i", "syl"]);
}

#[test]
fn a_movement_key_after_a_miss_ends_the_find() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "sy");
    typed(&mut ui, "z");
    press(&mut ui, &[Down]);
    let (_, _, _, find, _, _) = list(ui.state());
    assert_eq!(find, "");
    typed(&mut ui, "a");
    let (label, _, _, find, _, _) = list(ui.state());
    assert_eq!((label.as_deref(), find.as_str()), (Some("ax-1"), "a"));
}

#[test]
fn a_movement_key_after_a_match_ends_the_find() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "ax-m");
    press(&mut ui, &[Up]);
    let (_, _, _, find, _, _) = list(ui.state());
    assert_eq!(find, "");
    typed(&mut ui, "i");
    let (label, _, _, find, _, _) = list(ui.state());
    assert_eq!((label.as_deref(), find.as_str()), (Some("id"), "i"));
}
