mod support;

use mcui::Key::*;
use mcui::{State, Ui};
use support::{open, press, ui};

fn at(ui: &Ui) -> (usize, usize, usize, usize, bool) {
    match ui.state() {
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

fn long() -> Ui {
    let mut ui = support::ui_over(support::long_bytes(), &[], &[]);
    open(&mut ui, "axlong");
    ui
}

#[test]
fn a_fold_made_by_hand_is_gone_after_why_closes() {
    let mut ui = ui();
    open(&mut ui, "a2i");
    press(&mut ui, &[Down, Right, Enter]);
    assert_eq!(at(&ui).3, 1);
    press(&mut ui, &[F3, Clear]);
    let mut fresh = support::ui();
    open(&mut fresh, "a2i");
    press(&mut fresh, &[Down, Right]);
    assert_eq!(ui.state(), fresh.state());
    assert!(ui.frame() == fresh.frame());
    assert_eq!(at(&ui).3, 0);
}

#[test]
fn a_fold_made_by_hand_is_gone_after_dist_closes() {
    let mut ui = ui();
    open(&mut ui, "a2i");
    press(&mut ui, &[Down, Right, Enter, F2, Clear]);
    let mut fresh = support::ui();
    open(&mut fresh, "a2i");
    press(&mut fresh, &[Down, Right]);
    assert_eq!(ui.state(), fresh.state());
    assert!(ui.frame() == fresh.frame());
}

#[test]
fn folds_made_by_hand_stay_while_the_screen_is_on_top() {
    let mut ui = ui();
    open(&mut ui, "a2i");
    press(&mut ui, &[Down, Right, Enter]);
    let folded = ui.frame();
    press(&mut ui, &[Left, Right]);
    assert_eq!(at(&ui).3, 1);
    assert!(ui.frame() == folded);
}

#[test]
fn a_theorem_closed_over_trys_proving_screen_is_fitted_again() {
    let mut ui = ui();
    open(&mut ui, "a2i");
    press(&mut ui, &[Down, Right, Enter]);
    press(&mut ui, &[F4, Clear]);
    assert_eq!(at(&ui).3, 0);
}

/// Opens the long rule and unfolds, by hand, the first node its fit folded in the first
/// hypothesis. The cursor is left on that node.
fn long_unfolded() -> Ui {
    let mut ui = long();
    press(&mut ui, &[Left; 7]);
    press(&mut ui, &[Down, Right, Down, Right, Down, Right, Enter]);
    assert_eq!(at(&ui), (0, 6, 0, 41, false));
    ui
}

#[test]
fn a_cursor_inside_a_fold_the_refit_makes_keeps_its_node() {
    let mut ui = long_unfolded();
    ui.press(Down);
    assert_eq!(at(&ui), (0, 7, 0, 41, false));
    press(&mut ui, &[F2, Clear]);
    assert_eq!(at(&ui), (0, 7, 0, 42, false));
    let mut fresh = long();
    press(&mut fresh, &[Left; 7]);
    press(&mut fresh, &[Down, Right, Down, Right, Down, Right]);
    assert_eq!(at(&fresh), (0, 6, 0, 42, true));
    assert!(ui.frame() == fresh.frame(), "drawn on the folded ancestor");
}

#[test]
fn a_scroll_beyond_the_refit_layout_is_cut_back() {
    let mut ui = long_unfolded();
    while at(&ui).1 != 0 {
        ui.press(Up);
    }
    press(&mut ui, &[Right; 7]);
    let (statement, _, tall, _, _) = at(&ui);
    assert_eq!(statement, 7);
    press(&mut ui, &[F2, Clear]);
    let fresh = long();
    let (_, _, scroll, _, _) = at(&ui);
    assert!(scroll < tall, "{scroll} against {tall}");
    assert_eq!(ui.state(), fresh.state());
    assert!(ui.frame() == fresh.frame());
}

#[test]
fn a_scroll_inside_the_refit_layout_is_kept() {
    let mut ui = long();
    press(&mut ui, &[Left, Left, Left, Left, Left, Left, Left]);
    press(&mut ui, &[Right, Right, Right]);
    let before = at(&ui);
    press(&mut ui, &[F2, Clear]);
    assert_eq!(at(&ui), before);
}

#[test]
fn a_pop_at_full_depth_fits_the_top_theorem_again() {
    let mut ui = support::ui_over(support::hops_bytes(), &[], &[]);
    open(&mut ui, "t12");
    for _ in 0..11 {
        press(&mut ui, &[F3, F3]);
    }
    assert_eq!(ui.depth(), mcui::MAX_DEPTH);
    ui.press(Enter);
    assert_eq!(at(&ui).3, 1);
    ui.press(F3);
    assert_eq!(ui.depth(), mcui::MAX_DEPTH);
    ui.press(Clear);
    assert_eq!(ui.depth(), mcui::MAX_DEPTH - 1);
    assert_eq!(at(&ui).3, 0);
}

#[test]
fn a_theorem_closed_over_a_notice_is_fitted_again() {
    let mut ui = support::rejected_ui();
    open(&mut ui, "a1i");
    ui.press(Enter);
    assert_eq!(at(&ui).3, 1);
    ui.press(F3);
    assert!(matches!(ui.state(), State::Notice { .. }));
    ui.press(Clear);
    let mut fresh = support::rejected_ui();
    open(&mut fresh, "a1i");
    assert_eq!(ui.state(), fresh.state());
    assert!(ui.frame() == fresh.frame());
}

#[test]
fn the_cursor_is_in_view_after_a_refit_that_shortens_the_layout() {
    let mut ui = long();
    press(&mut ui, &[Left; 7]);
    for _ in 0..7 {
        press(&mut ui, &[Down, Right, Down, Right, Down, Right, Enter]);
        while at(&ui).1 != 0 {
            ui.press(Up);
        }
        ui.press(Right);
    }
    press(&mut ui, &[Left; 4]);
    assert_eq!(at(&ui).2, 6);
    press(&mut ui, &[F2, Clear]);
    let mut fresh = long();
    press(&mut fresh, &[Left; 4]);
    assert_eq!(at(&fresh).2, 3);
    assert_eq!(ui.state(), fresh.state());
    assert!(ui.frame() == fresh.frame());
}

#[test]
fn a_scroll_the_user_made_on_a_flat_statement_survives() {
    let mut ui = support::ui_over(support::flat_bytes(40), &[], &[]);
    open(&mut ui, "axflat");
    press(&mut ui, &[Down; 3]);
    assert_eq!(at(&ui).2, 3);
    let (state, frame) = (ui.state(), ui.frame());
    press(&mut ui, &[F2, Clear]);
    assert_eq!(ui.state(), state);
    assert!(ui.frame() == frame);
}
