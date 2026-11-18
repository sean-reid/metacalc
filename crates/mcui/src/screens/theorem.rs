use super::{Ctx, Nav, Screen, State, dist::Dist, notice::Notice, proof::Proof};
use crate::canvas::{CONTENT_ROWS, Canvas, GUTTER, STATEMENT_WIDTH, text_width};
use crate::data::Statement;
use crate::fold::{Fit, Folds, Layout, fit, layout};
use crate::key::Key;
use std::rc::Rc;

pub struct Theorem {
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
    hyp_indent: usize,
}

impl Theorem {
    pub fn open(ctx: &Ctx, library: usize, index: usize) -> Screen {
        let statement = ctx.libs[library].statement(index);
        let hyp_indent = if statement.hyps.is_empty() {
            0
        } else {
            text_width(&format!("{}.", statement.hyps.len())) + 2
        };
        let mut screen = Theorem {
            library,
            index,
            folds: Vec::new(),
            cursor: (statement.hyps.len(), 0),
            scroll: 0,
            layouts: Vec::new(),
            starts: Vec::new(),
            hyp_indent,
            statement,
        };
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
        screen.arrange();
        Screen::Theorem(screen)
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

    /// Lays the statements out under each other and moves the view to the cursor.
    fn arrange(&mut self) {
        let count = self.statement.hyps.len() + 1;
        self.layouts = (0..count)
            .map(|s| layout(self.statement.at(s), &self.folds[s], self.width(s)))
            .collect();
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
        let (s, node) = self.cursor;
        let rects = self.layouts[s].cursor(self.statement.at(s), node);
        let top = self.starts[s] + rects.first().map_or(0, |r| r.row);
        let bottom = self.starts[s] + rects.last().map_or(0, |r| r.row) + 1;
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + CONTENT_ROWS {
            self.scroll = (bottom - CONTENT_ROWS).min(top);
        }
        self.scroll = self.scroll.min(row.saturating_sub(CONTENT_ROWS));
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        let (s, node) = self.cursor;
        let statement = Rc::clone(&self.statement);
        let expr = statement.at(s);
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
                }
            }
            Key::Up => {
                if let Some(parent) = expr.parent(node) {
                    self.cursor.1 = parent;
                }
            }
            Key::Enter | Key::F1 => {
                self.folds[s].toggle(expr, node);
            }
            Key::F2 => return Nav::push(Dist::open(self.library, self.index)),
            Key::F3 => {
                return Nav::push(match Proof::open(ctx, self.library, self.index) {
                    Ok(screen) => screen,
                    Err(why) => Notice::screen("Proof", &[why]),
                });
            }
            Key::F4 => return Nav::push(Notice::unavailable("Prove")),
            Key::Clear => return Nav::Pop,
            _ => {}
        }
        self.arrange();
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
        canvas.soft_keys(["Fold", "Dist", "Why", "Try", ""]);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
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
