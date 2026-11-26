//! A mirror of the model's theorem screen from its layouts, which the rig cannot read from the model
//! itself: the folds are the model's fit plus the toggles the keys made, and `following` is the
//! model's `followed() == scroll`. It stands in for what `Theorem` keeps private and is compared with
//! the model's own state, so a mistake in it fails the tests that use it.

use mcui::canvas::{CONTENT_ROWS, STATEMENT_WIDTH, text_width};
use mcui::data::{Lib, Statement};
use mcui::fold::{Fit, Folds, fit, fit_one, layout};
use std::rc::Rc;

pub struct Mirror {
    statement: Rc<Statement>,
    folds: Vec<Folds>,
    indent: usize,
}

impl Mirror {
    /// The folds the model's refit starts from for assertion `i`.
    pub fn new(lib: &Lib, i: usize) -> Mirror {
        let statement = lib.statement(i);
        let hyps = statement.hyps.len();
        let indent = if hyps == 0 {
            0
        } else {
            text_width(&format!("{hyps}.")) + 2
        };
        let items: Vec<Fit> = statement
            .all()
            .into_iter()
            .enumerate()
            .map(|(s, expr)| Fit {
                expr,
                width: STATEMENT_WIDTH - if s == hyps { 0 } else { indent },
            })
            .collect();
        let mut folds = fit(&items, CONTENT_ROWS);
        if items.len() > CONTENT_ROWS {
            let last = items.len() - 1;
            folds[last] = fit_one(items[last].expr, items[last].width, CONTENT_ROWS);
        }
        Mirror {
            statement,
            folds,
            indent,
        }
    }

    /// The fold of `node` of statement `s` flipped, as Enter and F1 do.
    pub fn toggle(&mut self, s: usize, node: usize) -> bool {
        let statement = Rc::clone(&self.statement);
        self.folds[s].toggle(statement.at(s), node)
    }

    pub fn folded(&self) -> usize {
        self.folds.iter().map(Folds::count).sum()
    }

    fn width(&self, s: usize) -> usize {
        STATEMENT_WIDTH
            - if s == self.statement.hyps.len() {
                0
            } else {
                self.indent
            }
    }

    /// The first row of each statement and the rows of all of them.
    pub fn starts(&self) -> (Vec<usize>, usize) {
        let mut row = 0;
        let starts = (0..self.folds.len())
            .map(|s| {
                let at = row;
                row += layout(self.statement.at(s), &self.folds[s], self.width(s)).rows();
                at
            })
            .collect();
        (starts, row)
    }

    /// The first and the one past the last row the cursor's node is drawn on, counted from the first
    /// row of all.
    pub fn span(&self, s: usize, node: usize) -> (usize, usize) {
        let (starts, _) = self.starts();
        let expr = self.statement.at(s);
        let rects = layout(expr, &self.folds[s], self.width(s)).cursor(expr, node);
        (
            starts[s] + rects.first().unwrap().row,
            starts[s] + rects.last().unwrap().row + 1,
        )
    }

    /// The model's `followed()` for a cursor and a scroll.
    pub fn followed(&self, s: usize, node: usize, scroll: usize) -> usize {
        let (_, rows) = self.starts();
        let (top, bottom) = self.span(s, node);
        let want = if top < scroll {
            top
        } else if bottom > scroll + CONTENT_ROWS {
            (bottom - CONTENT_ROWS).min(top)
        } else {
            scroll
        };
        want.min(rows.saturating_sub(CONTENT_ROWS))
    }

    /// Whether the cursor is where the model's follow would put the view.
    pub fn following(&self, s: usize, node: usize, scroll: usize) -> bool {
        self.followed(s, node, scroll) == scroll
    }

    /// Whether a fold above the node hides it.
    pub fn hides(&self, s: usize, node: usize) -> bool {
        let expr = self.statement.at(s);
        let mut at = expr.parent(node);
        while let Some(parent) = at {
            if self.folds[s].is_folded(parent) {
                return true;
            }
            at = expr.parent(parent);
        }
        false
    }

    /// Whether the node's rows are all outside the window of six rows from `scroll`.
    pub fn out_of_window(&self, s: usize, node: usize, scroll: usize) -> bool {
        let (top, bottom) = self.span(s, node);
        bottom <= scroll || top >= scroll + CONTENT_ROWS
    }
}
