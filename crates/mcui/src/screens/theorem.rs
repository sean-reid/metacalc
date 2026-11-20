use super::{
    Ctx, Nav, Screen, State,
    dist::Dist,
    notice::{self, Notice},
    proof::Proof,
    proving::Proving,
};
use crate::canvas::{CONTENT_ROWS, Canvas, GUTTER, STATEMENT_WIDTH, text_width};
use crate::caps;
use crate::data::Statement;
use crate::fold::{Fit, Folds, Layout, fit, fit_one, layout};
use crate::key::Key;
use std::rc::Rc;

/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Fold", "Dist", "Why", "Try", ""];

/// The keys that work on a theorem past a cap.
const REFUSED_KEYS: [&str; 5] = ["", "Dist", "", "Try", ""];

/// The sentence that stands in for the statements of an assertion past a cap.
const TOO_LARGE: &str = "This theorem is too large to show on the calculator";

pub struct Theorem {
    /// The assertion is past a cap: nothing is laid out and the statements are not drawn.
    too_large: bool,
    library: usize,
    index: usize,
    statement: Rc<Statement>,
    folds: Vec<Folds>,
    /// The statement the cursor is in (the conclusion is last) and the subexpression.
    cursor: (usize, usize),
    scroll: usize,
    layouts: Vec<Layout>,
    /// First row of each statement.
    starts: Vec<usize>,
    /// Rows of all the statements together.
    rows: usize,
    hyp_indent: usize,
}

impl Theorem {
    pub fn open(ctx: &Ctx, library: usize, index: usize) -> Screen {
        Screen::Theorem(Theorem::fitted(ctx, library, index))
    }

    /// Fits the folds afresh after the screen above closed; the cursor stays. The scroll stays
    /// as far as the new layout holds it, and follows the cursor unless the user had scrolled
    /// the cursor out of view.
    pub(super) fn refit(&mut self, ctx: &Ctx) {
        if self.too_large {
            return;
        }
        let (cursor, scroll) = (self.cursor, self.scroll);
        let following = self.followed() == scroll;
        *self = Theorem::fitted(ctx, self.library, self.index);
        let s = cursor.0.min(self.layouts.len() - 1);
        let node = cursor.1.min(self.statement.at(s).len() - 1);
        self.cursor = (s, node);
        self.scroll = scroll;
        if following {
            self.follow();
        } else {
            self.scroll = self.scroll.min(self.rows.saturating_sub(CONTENT_ROWS));
        }
    }

    fn fitted(ctx: &Ctx, library: usize, index: usize) -> Theorem {
        let statement = ctx.libs[library].statement(index);
        let hyp_indent = if statement.hyps.is_empty() {
            0
        } else {
            text_width(&format!("{}.", statement.hyps.len())) + 2
        };
        let mut screen = Theorem {
            too_large: !caps::within(&statement.all()),
            library,
            index,
            folds: Vec::new(),
            cursor: (statement.hyps.len(), 0),
            scroll: 0,
            layouts: Vec::new(),
            starts: Vec::new(),
            rows: 0,
            hyp_indent,
            statement,
        };
        if screen.too_large {
            return screen;
        }
        let exprs = screen.statement.all();
        let items: Vec<Fit> = exprs
            .iter()
            .enumerate()
            .map(|(s, &expr)| Fit {
                expr,
                width: screen.width(s),
            })
            .collect();
        screen.folds = fit(&items, CONTENT_ROWS);
        if items.len() > CONTENT_ROWS {
            let last = items.len() - 1;
            screen.folds[last] = fit_one(items[last].expr, items[last].width, CONTENT_ROWS);
        }
        screen.layouts = (0..items.len()).map(|s| screen.laid_out(s)).collect();
        screen.place();
        screen.follow();
        screen
    }

    fn is_conclusion(&self, statement: usize) -> bool {
        statement == self.statement.hyps.len()
    }

    fn indent(&self, statement: usize) -> usize {
        if self.is_conclusion(statement) {
            0
        } else {
            self.hyp_indent
        }
    }

    fn width(&self, statement: usize) -> usize {
        STATEMENT_WIDTH - self.indent(statement)
    }

    fn laid_out(&self, statement: usize) -> Layout {
        layout(
            self.statement.at(statement),
            &self.folds[statement],
            self.width(statement),
        )
    }

    /// Finds the first row of each statement.
    fn place(&mut self) {
        let mut row = 0;
        self.starts = self
            .layouts
            .iter()
            .map(|l| {
                let at = row;
                row += l.rows();
                at
            })
            .collect();
        self.rows = row;
    }

    /// Moves the view to the cursor.
    fn follow(&mut self) {
        self.scroll = self.followed();
    }

    /// The scroll that shows the cursor, moving the view as little as possible.
    fn followed(&self) -> usize {
        let (s, node) = self.cursor;
        let rects = self.layouts[s].cursor(self.statement.at(s), node);
        let top = self.starts[s] + rects.first().map_or(0, |r| r.row);
        let bottom = self.starts[s] + rects.last().map_or(0, |r| r.row) + 1;
        let scroll = if top < self.scroll {
            top
        } else if bottom > self.scroll + CONTENT_ROWS {
            (bottom - CONTENT_ROWS).min(top)
        } else {
            self.scroll
        };
        scroll.min(self.rows.saturating_sub(CONTENT_ROWS))
    }

    fn proof(&self, ctx: &Ctx) -> Nav {
        Nav::push(match Proof::open(ctx, self.library, self.index) {
            Ok(screen) => screen,
            Err(why) => Notice::screen("Proof", &[why]),
        })
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        if self.too_large {
            return match key {
                Key::F2 => Nav::push(Dist::open(self.library, self.index)),
                Key::F4 => Nav::push(Proving::open(ctx, self.library, self.index)),
                Key::Clear => Nav::Pop,
                _ => Nav::Stay,
            };
        }
        let (s, node) = self.cursor;
        let statement = Rc::clone(&self.statement);
        let expr = statement.at(s);
        let mut follow = true;
        match key {
            Key::Left | Key::Right => {
                let forward = key == Key::Right;
                if let Some(next) = expr.sibling(node, forward) {
                    self.cursor.1 = next;
                } else if node == 0 {
                    let next = if forward {
                        (s + 1 < self.layouts.len()).then_some(s + 1)
                    } else {
                        s.checked_sub(1)
                    };
                    if let Some(next) = next {
                        self.cursor = (next, 0);
                    }
                }
            }
            Key::Down => {
                if !self.folds[s].is_folded(node)
                    && let Some(&first) = expr.children(node).first()
                {
                    self.cursor.1 = first;
                } else if expr.len() == 1 {
                    follow = false;
                    if self.scroll + CONTENT_ROWS < self.rows {
                        self.scroll += 1;
                    }
                }
            }
            Key::Up => {
                if let Some(parent) = expr.parent(node) {
                    self.cursor.1 = parent;
                } else if expr.len() == 1 {
                    follow = false;
                    self.scroll = self.scroll.saturating_sub(1);
                }
            }
            Key::Enter | Key::F1 => {
                if self.folds[s].toggle(expr, node) {
                    self.layouts[s] = self.laid_out(s);
                    self.place();
                }
            }
            Key::F2 => return Nav::push(Dist::open(self.library, self.index)),
            Key::F3 => return self.proof(ctx),
            Key::F4 => return Nav::push(Proving::open(ctx, self.library, self.index)),
            Key::Clear => return Nav::Pop,
            _ => {}
        }
        if follow {
            self.follow();
        }
        Nav::Stay
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        let kind = if lib.is_theorem(self.index) {
            "thm"
        } else {
            "ax"
        };
        let status = format!("{} {kind}", lib.mark(self.index).glyph());
        canvas.title(lib.label(self.index), &status);
        if self.too_large {
            notice::body(canvas, &[TOO_LARGE]);
            canvas.soft_keys(REFUSED_KEYS);
            return;
        }
        let window = 1..1 + CONTENT_ROWS as isize;
        for (s, layout) in self.layouts.iter().enumerate() {
            let row = 1 + self.starts[s] as isize - self.scroll as isize;
            let x = GUTTER + self.indent(s);
            if !self.is_conclusion(s) && window.contains(&row) {
                canvas.text(GUTTER, row as usize, &format!("{}.", s + 1));
            }
            canvas.layout(layout, x, row, window.clone());
        }
        let (s, node) = self.cursor;
        let row = 1 + self.starts[s] as isize - self.scroll as isize;
        let rects = self.layouts[s].cursor(self.statement.at(s), node);
        if node == 0 {
            if window.contains(&row) {
                canvas.marker(row as usize);
            }
        } else {
            canvas.cursor(&rects, GUTTER + self.indent(s), row, window);
        }
        canvas.soft_keys(KEYS);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        if self.too_large {
            return State::Theorem {
                library: self.library,
                label: ctx.libs[self.library].label(self.index).to_string(),
                statement: self.cursor.0,
                node: 0,
                scroll: 0,
                folded: 0,
                hidden: false,
            };
        }
        State::Theorem {
            library: self.library,
            label: ctx.libs[self.library].label(self.index).to_string(),
            statement: self.cursor.0,
            node: self.cursor.1,
            scroll: self.scroll,
            folded: self.folds.iter().map(Folds::count).sum(),
            hidden: self.folds[self.cursor.0].is_folded(self.cursor.1),
        }
    }
}
