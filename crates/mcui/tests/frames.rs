//! Whole frames, pixel for pixel, against text art checked in under tests/frames. Set
//! MCUI_BLESS=1 to write the current frames instead of comparing.

mod support;

use mcui::Key::*;
use mcui::Ui;
use support::{check_frame, extra_ui, open, press, typed, ui};

fn check(ui: &Ui, name: &str) {
    check_frame(&ui.frame(), name);
}

#[test]
fn libraries() {
    check(&ui(), "libraries");
}

#[test]
fn list() {
    let mut ui = ui();
    press(&mut ui, &[Enter, Down, Down]);
    check(&ui, "list");
}

#[test]
fn list_find_with_no_match() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "synx");
    check(&ui, "list-find-miss");
}

#[test]
fn theorem_with_hypotheses() {
    let mut ui = ui();
    open(&mut ui, "syl");
    check(&ui, "theorem");
}

#[test]
fn theorem_folded_to_fit_with_the_cursor_below_the_root() {
    let mut ui = ui();
    open(&mut ui, "ax-many");
    press(&mut ui, &[Down, Right]);
    check(&ui, "theorem-scrolled");
}

#[test]
fn theorem_after_folding_under_the_cursor() {
    let mut ui = ui();
    open(&mut ui, "ax-2");
    press(&mut ui, &[Down, Right, Enter]);
    check(&ui, "theorem-folded");
}

#[test]
fn distinct_variables() {
    let mut ui = ui();
    open(&mut ui, "alid");
    press(&mut ui, &[F2]);
    check(&ui, "dist");
}

#[test]
fn proof_root() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3]);
    check(&ui, "proof-root");
}

#[test]
fn proof_subgoal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Down, Enter]);
    check(&ui, "proof-subgoal");
}

#[test]
fn not_available_yet() {
    let mut ui = ui();
    press(&mut ui, &[Down, Enter]);
    check(&ui, "unavailable");
}

#[test]
fn about() {
    let mut ui = ui();
    press(&mut ui, &[F3]);
    let frame = ui.frame();
    let version = format!("metacalc {}", env!("CARGO_PKG_VERSION"));
    let line = mcglyph::Font::builtin().text(&version);
    let x0 = (96 - line.width()) / 2;
    let mut blanked = frame.clone();
    for y in 16..24 {
        for x in 0..96 {
            assert_eq!(
                frame.get(x, y),
                x >= x0 && y >= 17 && line.get(x - x0, y - 17),
                "{x},{y}"
            );
            blanked.set(x, y, false);
        }
    }
    check_frame(&blanked, "about");
}

#[test]
fn theorem_with_the_cursor_on_a_hypothesis_root() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[Left]);
    check(&ui, "theorem-hypothesis");
}

#[test]
fn proof_with_the_cursor_on_the_goal() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Up]);
    check(&ui, "proof-goal");
}

#[test]
fn the_library_list_says_when_statements_are_flat() {
    let mut ui = support::ui_over(
        {
            let mut bytes = support::logic_bytes();
            bytes[4] ^= 0xFF;
            bytes
        },
        &[],
        &[],
    );
    press(&mut ui, &[Enter]);
    check(&ui, "list-flat");
}

#[test]
fn a_notice_wraps_at_word_boundaries() {
    let mut broken = support::logic_bytes();
    broken[4] ^= 0xFF;
    let mut ui = support::ui_over(broken, &[], &[]);
    open(&mut ui, "syl");
    press(&mut ui, &[F3]);
    check(&ui, "notice-no-export");
}

#[test]
fn a_syntax_theorem_has_no_goal_tree() {
    let mut ui = extra_ui();
    open(&mut ui, "wtwice");
    press(&mut ui, &[F3]);
    check(&ui, "notice-syntax");
}

#[test]
fn a_long_find_prefix_shows_its_tail() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "abcdefghijklmnopqrstuvwxyzabcd");
    check(&ui, "list-find-long");
}

#[test]
fn list_rows_carry_every_mark() {
    let mut ui = ui();
    press(&mut ui, &[Enter, F2]);
    typed(&mut ui, "a1i");
    check(&ui, "list-marks");
}

#[test]
fn a_kind_that_matches_nothing_leaves_an_empty_list() {
    let mut ui = support::ui_with(&[], &[]);
    press(&mut ui, &[Enter, F3, F3, F3]);
    check(&ui, "list-empty");
}

#[test]
fn no_libraries() {
    let ui = Ui::new(Vec::new(), mcui::Statuses::default()).unwrap();
    check(&ui, "no-libraries");
}

#[test]
fn an_empty_library() {
    let mut ui = empty_ui();
    check(&ui, "empty-library");
    press(&mut ui, &[Enter]);
    check(&ui, "empty-library-list");
}

fn empty_ui() -> Ui {
    let db = support::db(support::LOGIC);
    let bytes = mcpack::library::pack(&db, &[]).expect("an empty library packs");
    support::ui_over(bytes, &[], &[])
}

#[test]
fn a_theorem_whose_root_is_folded() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[Enter]);
    check(&ui, "theorem-root-folded");
}

#[test]
fn many_distinct_variable_pairs_scroll() {
    let mut ui = extra_ui();
    open(&mut ui, "ax-dvs");
    press(&mut ui, &[F2]);
    check(&ui, "dist-top");
    press(
        &mut ui,
        &[Down, Down, Down, Down, Down, Down, Down, Down, Down, Down],
    );
    check(&ui, "dist-scrolled");
}

#[test]
fn a_proof_node_with_more_than_six_subgoals() {
    let mut ui = extra_ui();
    open(&mut ui, "many2");
    press(&mut ui, &[F3]);
    check(&ui, "proof-many-top");
    press(&mut ui, &[Down, Down, Down, Down, Down, Down, Down]);
    check(&ui, "proof-many-bottom");
}

#[test]
fn hide_on_the_proof_screen() {
    let mut ui = ui();
    open(&mut ui, "syl");
    press(&mut ui, &[F3, Down, F1]);
    check(&ui, "proof-hide");
}

#[test]
fn a_hypothesis_leaf_keeps_its_cursor_on_the_goal() {
    let mut ui = extra_ui();
    open(&mut ui, "foo");
    press(&mut ui, &[F3, Down, Enter]);
    check(&ui, "proof-hypothesis-root");
}

#[test]
fn distinct_variables_of_an_assertion_with_none() {
    let mut ui = ui();
    open(&mut ui, "ax-1");
    press(&mut ui, &[F2]);
    check(&ui, "dist-none");
}

#[test]
fn a_long_name_gives_way_to_a_kind_suffix_and_wide_counts() {
    let mut bytes = support::big_bytes();
    bytes[4] ^= 0xFF;
    let table = support::statuses(&bytes, &["ax7"], &[]);
    let mut ui = Ui::new(
        vec![mcui::Installed {
            name: "an-extremely-long-library-name".into(),
            bytes,
        }],
        mcui::Statuses {
            libraries: vec![table],
            my_proofs: 0,
        },
    )
    .unwrap();
    press(&mut ui, &[Enter, F3, F3]);
    let frame = ui.frame();
    let right = "flat 1199/1200";
    let room = 96 - 2 - mcglyph::Font::builtin().text_width(right) - 3;
    let name = mcui::canvas::truncate(
        "an-extremely-long-library-name",
        room - mcglyph::Font::builtin().text_width(":ax"),
    );
    assert!(name.ends_with(mcglyph::ELLIPSIS));
    let left = format!("{name}:ax");
    assert!(support::title_is(&frame, &left, right));
    check_frame(&frame, "list-long-name");
}
