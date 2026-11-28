//! The theorem screen on assertions built to sit on a boundary: 9 and 10 hypotheses, and a refit
//! that takes the rows under six.

use crate::app::*;
use crate::shellrig::*;
use crate::theorem_keys::{built_of, cursor, index_of, scroll};
use crate::theorem_return::next_node;
use crate::theorem_rig::*;
use mcui::Key;
use mcui::canvas::{STATEMENT_WIDTH, text_width};
use mcui::fold::{Folds, layout};

#[test]
fn nine_and_ten_hypotheses_indent_as_the_model_does_and_ten_is_drawn_in_a_scrolled_window() {
    let built = built_of("theorem-hyps", &[("hyps9", 9), ("hyps10", 10)]);
    for (label, hyps) in [("hyps9", 9usize), ("hyps10", 10)] {
        let n = index_of(built, label);
        let mut walker = Walker::at(built, None, 0, n);
        let want = text_width(&format!("{hyps}.")) + 2;
        assert_eq!(
            usize::from(walker.var("TV_INDENT")),
            want,
            "{label}: the indent"
        );
        assert!(walker.var16("TV_TOTAL") > 6, "{label}: the window scrolls");
        for _ in 0..hyps {
            assert!(walker.press(LEFT, Key::Left));
        }
        for _ in 0..hyps - 1 {
            assert!(walker.press(RIGHT, Key::Right));
        }
        let (s, _) = cursor(&walker);
        assert_eq!(s, hyps - 1, "{label}: the last hypothesis, number {hyps}");
        assert!(scroll(&walker) > 0, "{label}: scrolled to it");
    }
    let (nine, ten) = (text_width("9.") + 2, text_width("10.") + 2);
    assert!(ten > nine, "the indent differs at 10: {nine} and {ten}");
}

/// Assertions of a conclusion nested `depth` deep.
fn nested(depth: usize) -> String {
    let open = "( ph -> ".repeat(depth);
    let close = ") ".repeat(depth);
    format!("nest{depth} $a |- {open}ph {close}$.\n")
}

/// Assertions whose conclusion is a balanced tree of conjunctions `depth` deep.
fn balanced(depth: usize) -> String {
    fn tree(depth: usize) -> String {
        match depth {
            0 => "ph".to_string(),
            d => format!("( {} /\\ {} )", tree(d - 1), tree(d - 1)),
        }
    }
    format!("bal{depth} $a |- {} $.\n", tree(depth))
}

#[test]
fn a_refit_that_takes_the_rows_under_six_clamps_a_scrolled_view_to_the_top() {
    let src: String = (6..40).map(nested).chain((3..=5).map(balanced)).collect();
    let library = crate::fixture::custom(&src);
    let built = around("theorem-nested", &library);
    let lib = lib_of(built);
    let n = (0..lib.len())
        .find(|&i| {
            let f = facts(&lib, i);
            let expr = &lib.statement(i).conclusion;
            f.folded && f.rows < 6 && layout(expr, &Folds::none(expr), STATEMENT_WIDTH).rows() >= 8
        })
        .expect("a nested assertion the fit folds to under six rows");
    let f = facts(&lib, n);
    assert!(f.rows < 6 && f.folded);
    let mut walker = Walker::at(built, None, 0, n);
    assert_eq!(walker.var16("TV_TOTAL"), f.rows);
    let mut guard = 0;
    while walker.mirror().starts().1 < 8 || scroll(&walker) == 0 {
        guard += 1;
        assert!(
            guard < 400,
            "the keys unfold the statement and scroll to the cursor"
        );
        assert!(next_node(&mut walker), "a node is left to go to");
    }
    assert!(
        scroll(&walker) > 0 && walker.var16("TV_TOTAL") >= 8,
        "the precondition"
    );
    walker.excursion(F3, Key::F3);
    assert_eq!(
        walker.var16("TV_TOTAL"),
        f.rows,
        "the refit's rows are under six"
    );
    assert_eq!(scroll(&walker), 0, "the view is at the top");
}
