use crate::clip::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;
use mcui::canvas::{CONTENT_ROWS, STATEMENT_WIDTH, TEXT_WIDTH};
use mcui::expr::Expr;
use mcui::fold::{Folds, fit_one, layout};

fn index(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}")) as u16
}

/// A conclusion that no fold can shorten and that takes several lines at `width`.
fn unfoldable(model: &Model, bytes: &[u8], width: usize) -> (u16, Expr) {
    for n in 0..decoded(bytes).assertions.len() as u16 {
        if !model.within(n) {
            continue;
        }
        let expr = model.expr(n, 0xFF);
        let flat = (0..expr.len()).all(|node| !expr.foldable(node));
        if flat && layout(&expr, &Folds::none(&expr), width).rows() > 2 {
            return (n, expr);
        }
    }
    panic!("a conclusion with nothing to fold");
}

/// fm's frame for the selected statement, drawn on a cleared canvas at the first text row.
fn drawn(fm: &mut Fm, width: usize) -> Vec<u8> {
    fm.clear();
    assert_eq!(fm.draw(3, 0, width, (0, 8)).status(), 0);
    fm.frame()
}

#[test]
fn loading_the_statement_again_drops_the_clip() {
    let bytes = fixture::nodes_library();
    let (model, n) = (Model::new(&bytes), index(&bytes, "longest"));
    let expr = model.expr(n, 0xFF);
    let plain = layout(&expr, &Folds::none(&expr), STATEMENT_WIDTH);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    let out = fm.clip(1, STATEMENT_WIDTH);
    assert_eq!((out.status(), count(out)), (0, 1));
    assert_eq!(count(fm.rows(STATEMENT_WIDTH)), 1);
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(count(fm.rows(STATEMENT_WIDTH)), plain.rows());
    assert!(plain.rows() > 1);
    same_frame(
        "reloaded",
        &drawn(&mut fm, STATEMENT_WIDTH),
        &frame_of(&plain),
    );
    assert_eq!(fm.nodes().status(), 0);
    assert_eq!(fm.cursor(1).status(), 0, "the cursor works again");
    assert_eq!(fm.fold(1, 1).status(), 0, "and so does a fold");
}

#[test]
fn fitting_again_drops_the_clip_of_a_statement_with_nothing_to_fold() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let width = 30;
    let (n, expr) = unfoldable(&model, &bytes, width);
    let plain = layout(&expr, &Folds::none(&expr), width);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.fit(CONTENT_ROWS, width, width).status(), 0);
    let cut = plain.clipped(width, 2);
    assert!(plain.rows() > 2 && cut != plain);
    let first = fm.clip(2, width);
    assert_eq!((first.status(), count(first)), (0, cut.rows()));
    same_frame("clipped", &drawn(&mut fm, width), &frame_of(&cut));
    assert_eq!(fm.fit(CONTENT_ROWS, width, width).status(), 0);
    assert_eq!(count(fm.rows(width)), plain.rows());
    same_frame("refitted", &drawn(&mut fm, width), &frame_of(&plain));
    let again = fm.clip(2, width);
    assert_eq!(
        (again.status(), count(again)),
        (0, cut.rows()),
        "it clips again"
    );
}

#[test]
fn fitting_again_drops_the_clip_of_a_statement_that_folds() {
    let bytes = fixture::nodes_library();
    let (model, n) = (Model::new(&bytes), index(&bytes, "longest"));
    let expr = model.expr(n, 0xFF);
    let width = 30;
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.clip(1, width).status(), 0);
    assert_eq!(fm.fit(2, width, width).status(), 0);
    let want = layout(&expr, &fit_one(&expr, width, 2), width);
    assert_eq!(count(fm.rows(width)), want.rows());
    same_frame("refitted", &drawn(&mut fm, width), &frame_of(&want));
}

#[test]
fn a_clipped_statement_refuses_a_second_clip_a_cursor_and_a_fold() {
    let bytes = fixture::nodes_library();
    let (model, n) = (Model::new(&bytes), index(&bytes, "longest"));
    let expr = model.expr(n, 0xFF);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.nodes().status(), 0);
    assert_eq!(fm.cursor(2).status(), 0);
    assert_eq!(fm.clip(1, STATEMENT_WIDTH).status(), 0);
    let clipped = layout(&expr, &Folds::none(&expr), STATEMENT_WIDTH).clipped(STATEMENT_WIDTH, 1);
    same_frame(
        "the clip clears the cursor",
        &drawn(&mut fm, STATEMENT_WIDTH),
        &frame_of(&clipped),
    );
    let refused = code("FM_ERR_CLIPPED");
    assert_eq!(fm.clip(1, STATEMENT_WIDTH).status(), refused);
    assert_eq!(fm.clip(3, STATEMENT_WIDTH).status(), refused);
    assert_eq!(fm.cursor(1).status(), refused);
    assert_eq!(fm.fold(1, 1).status(), refused);
    assert_eq!(fm.fold(1, 3).status(), code("FM_ERR_BAD"));
    assert_eq!(fm.cursor(NO_CURSOR).status(), 0, "no cursor stays allowed");
    same_frame(
        "refusals draw nothing",
        &drawn(&mut fm, STATEMENT_WIDTH),
        &frame_of(&clipped),
    );
    assert_eq!(count(fm.rows(STATEMENT_WIDTH)), 1);
}

#[test]
fn selecting_another_statement_and_back_keeps_the_clip_of_the_first() {
    let bytes = fixture::nodes_library();
    let (model, n) = (Model::new(&bytes), index(&bytes, "many"));
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(n).status(), 0);
    let width = 60;
    assert_eq!(fm.select(0xFF).status(), 0);
    let out = fm.clip(1, width);
    assert_eq!((out.status(), count(out)), (0, 1));
    assert_eq!(fm.select(0).status(), 0);
    let hyp = fm.rows(width);
    let expr = model.expr(n, 0);
    let want = layout(&expr, &Folds::none(&expr), width);
    assert_eq!(count(hyp), want.rows(), "another statement is whole");
    assert_eq!(fm.select(0xFF).status(), 0);
    assert_eq!(count(fm.rows(width)), 1);
    assert_eq!(fm.clip(1, width).status(), code("FM_ERR_CLIPPED"));
}

#[test]
fn a_statement_that_fits_is_left_alone_and_costs_a_check() {
    let bytes = fixture::nodes_library();
    let n = index(&bytes, "ax-1");
    let mut fm = Fm::new(&bytes);
    let zero = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    assert_eq!(fm.statement(n, 0xFF).status(), 0);
    assert_eq!(fm.fit(CONTENT_ROWS, TEXT_WIDTH, TEXT_WIDTH).status(), 0);
    let before = fm.records();
    let out = fm.clip(CONTENT_ROWS, TEXT_WIDTH);
    let spent = out.tstates - zero;
    println!("fm_clip of a statement that fits, counted by fm_fit: {spent} T-states");
    assert_eq!(out.status(), 0);
    assert_eq!(fm.records(), before);
    assert!(spent < 1200, "{spent}");
    let wide = fm.clip(CONTENT_ROWS, 20);
    assert_eq!(wide.status(), 0);
    assert!(count(wide) <= CONTENT_ROWS);
    assert_eq!(fm.clip(1, 20).status(), 0);
    assert_eq!(fm.clip(1, 20).status(), code("FM_ERR_CLIPPED"));
}

#[test]
fn clipping_with_nothing_loaded_or_a_closed_fm_is_refused() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.clip(1, 94).status(), code("FM_ERR_INDEX"));
    let mut closed = Fm::bare(&bytes);
    assert_eq!(closed.clip(1, 94).status(), code("FM_ERR_NOT_OPEN"));
}
