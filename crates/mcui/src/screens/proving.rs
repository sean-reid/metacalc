use super::{Ctx, Nav, Screen, State, View, notice::Notice, reveal, step};
use crate::canvas::{
    CONTENT_ROWS, Canvas, GUTTER, MARGIN, STATEMENT_WIDTH, TEXT_WIDTH, text_width,
};
use crate::edit::{Editor, Picks, Work};
use crate::fold::{Layout, fitted};
use crate::key::Key;
use crate::proofs::SavedProof;
use crate::status::Mark;
use mcedit::GoalId;

/// The words on the five keys; the font has no room for Aply, Auto, Undo and Tree in a key.
const KEYS: [&str; 5] = ["Apl", "Hyp", "Aut", "Und", "Tre"];

/// Rows the current goal may take before it is folded.
const GOAL_ROWS: usize = 2;
/// Rows the finished proof's statement may take.
const STATEMENT_ROWS: usize = 4;
/// Pixels an outline row steps in for each level of the tree.
const INDENT: usize = 3;
const MAX_DEPTH: usize = 8;

#[derive(Default, Clone, Copy)]
struct Cursor {
    at: usize,
    top: usize,
}

impl Cursor {
    fn go(&mut self, len: usize, key: Key, rows: usize) {
        self.at = step(self.at, len, key, rows);
        self.top = reveal(self.top, self.at, rows);
    }

    fn to(at: usize, rows: usize) -> Cursor {
        Cursor {
            at,
            top: reveal(0, at, rows),
        }
    }
}

/// A theorem being proved backwards: the goal on screen, the hypotheses, the candidates for the
/// goal, and the whole goal tree as an outline.
pub struct Proving {
    library: usize,
    theorem: usize,
    work: Work,
    view: View,
    goal: Layout,
    statement: Layout,
    hyps: Vec<Layout>,
    picks: Cursor,
    hyp: Cursor,
    tree: Cursor,
}

fn number(n: usize) -> String {
    format!("{n}.")
}

impl Proving {
    /// Opens the proof of theorem `index`, from the saved form the interface holds for it or
    /// from the bare statement.
    pub fn open(ctx: &Ctx, library: usize, index: usize) -> Screen {
        let saved = ctx.proofs.find(library, index);
        let lib = &ctx.libs[library];
        let started = Editor::start(lib, index, saved.map(|s| s.session.as_slice()));
        let work = started.map(|mut editor| editor.work(saved.and_then(|s| s.goal)));
        match work {
            Ok(work) => {
                let mut proving = Proving {
                    library,
                    theorem: index,
                    work,
                    view: View::Candidates,
                    goal: Layout::empty(),
                    statement: Layout::empty(),
                    hyps: Vec::new(),
                    picks: Cursor::default(),
                    hyp: Cursor::default(),
                    tree: Cursor::default(),
                };
                proving.refresh(ctx);
                Screen::Proving(proving)
            }
            Err(why) => Notice::screen("Prove", &[why]),
        }
    }

    /// The saved form of what is on screen.
    pub fn saved(&self) -> SavedProof {
        SavedProof {
            library: self.library,
            theorem: self.theorem,
            session: self.work.saved.clone(),
            goal: self.work.goal,
            steps: self.work.steps,
        }
    }

    /// Lays out what the session now shows.
    fn refresh(&mut self, ctx: &Ctx) {
        self.goal = match &self.work.goal_expr {
            Some(expr) => fitted(expr, STATEMENT_WIDTH, GOAL_ROWS),
            None => Layout::empty(),
        };
        let lib = &ctx.libs[self.library];
        self.statement = fitted(
            &lib.statement(self.theorem).conclusion,
            STATEMENT_WIDTH,
            STATEMENT_ROWS,
        );
        let indent = text_width(&number(self.work.hyps.len())) + 2;
        self.hyps = self
            .work
            .hyps
            .iter()
            .map(|h| fitted(&h.expr, TEXT_WIDTH - indent, 1))
            .collect();
    }

    fn goal_rows(&self) -> usize {
        self.goal.rows().max(1)
    }

    /// Rows under the hypotheses line, where candidates or hypotheses are listed.
    fn list_rows(&self) -> usize {
        CONTENT_ROWS - self.goal_rows() - 1
    }

    fn at_tree_row(&self) -> usize {
        self.work
            .tree
            .iter()
            .position(|n| Some(n.goal) == self.work.goal)
            .unwrap_or(0)
    }

    /// Replaces the session's state with what `step` leaves, or shows why it was refused.
    fn run(
        &mut self,
        ctx: &Ctx,
        title: &'static str,
        step: impl FnOnce(&mut Editor) -> Result<Option<GoalId>, &'static str>,
    ) -> Nav {
        let lib = &ctx.libs[self.library];
        let done =
            Editor::start(lib, self.theorem, Some(&self.work.saved)).and_then(|mut editor| {
                let goal = step(&mut editor)?;
                Ok(editor.work(goal))
            });
        match done {
            Ok(work) => {
                self.work = work;
                self.picks = Cursor::default();
                self.hyp = Cursor::default();
                if self.view == View::Hyps {
                    self.view = View::Candidates;
                }
                self.refresh(ctx);
                self.tree = Cursor::to(self.at_tree_row(), CONTENT_ROWS);
                Nav::Stay
            }
            Err(why) => Nav::push(Notice::screen(title, &[why])),
        }
    }

    fn apply(&mut self, ctx: &Ctx) -> Nav {
        let Some(goal) = self.work.goal else {
            return Nav::Stay;
        };
        let pick = match &self.work.picks {
            Picks::Unknown => return Nav::push(Notice::screen("Apply", &["Not known yet"])),
            Picks::List(list) => list.get(self.picks.at).map(|p| p.assertion),
        };
        let Some(assertion) = pick else {
            return Nav::push(Notice::screen("Apply", &["No candidate"]));
        };
        self.run(ctx, "Apply", |editor| {
            editor.apply(goal, assertion)?;
            Ok(editor.next_after(Some(goal)))
        })
    }

    fn close(&mut self, ctx: &Ctx) -> Nav {
        let (Some(goal), Some(hyp)) = (self.work.goal, self.work.hyps.get(self.hyp.at)) else {
            return Nav::Stay;
        };
        let id = hyp.id;
        self.run(ctx, "Hyp", |editor| {
            editor.close(goal, id)?;
            Ok(editor.next_after(Some(goal)))
        })
    }

    fn auto(&mut self, ctx: &Ctx) -> Nav {
        let Some(goal) = self.work.goal else {
            return Nav::Stay;
        };
        self.run(ctx, "Auto", |editor| {
            editor.auto(goal)?;
            Ok(editor.next_after(Some(goal)))
        })
    }

    fn undo(&mut self, ctx: &Ctx) -> Nav {
        self.run(ctx, "Undo", |editor| editor.undo())
    }

    fn jump(&mut self, ctx: &Ctx) -> Nav {
        let Some(node) = self.work.tree.get(self.tree.at).filter(|n| n.open) else {
            return Nav::Stay;
        };
        let goal = node.goal;
        let nav = self.run(ctx, "Tree", |_| Ok(Some(goal)));
        self.view = View::Candidates;
        nav
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        if key == Key::Clear {
            return match self.view {
                View::Candidates => Nav::Pop,
                _ => {
                    self.view = View::Candidates;
                    Nav::Stay
                }
            };
        }
        match (self.view, key) {
            (View::Tree, Key::Up | Key::Down | Key::Left | Key::Right) => {
                self.tree.go(self.work.tree.len(), key, CONTENT_ROWS);
            }
            (View::Tree, Key::Enter | Key::F1) => return self.jump(ctx),
            (View::Tree, Key::F2 | Key::F3) => {}
            (View::Hyps, Key::Up | Key::Down | Key::Left | Key::Right) => {
                let rows = self.list_rows();
                self.hyp.go(self.work.hyps.len(), key, rows);
            }
            (View::Hyps, Key::Enter | Key::F1) => return self.close(ctx),
            (View::Candidates, Key::Up | Key::Down | Key::Left | Key::Right) => {
                let rows = self.list_rows();
                let len = match &self.work.picks {
                    Picks::List(list) => list.len(),
                    Picks::Unknown => 0,
                };
                self.picks.go(len, key, rows);
            }
            (View::Candidates, Key::Enter | Key::F1) => return self.apply(ctx),
            (_, Key::F2) if self.work.goal.is_none() => {}
            (_, Key::F2) => {
                self.view = if self.view == View::Hyps {
                    View::Candidates
                } else {
                    View::Hyps
                };
            }
            (_, Key::F3) => return self.auto(ctx),
            (_, Key::F4) => return self.undo(ctx),
            (View::Tree, Key::F5) => self.view = View::Candidates,
            (_, Key::F5) => {
                self.view = View::Tree;
                self.tree = Cursor::to(self.at_tree_row(), CONTENT_ROWS);
            }
            _ => {}
        }
        Nav::Stay
    }

    fn draw_finished(&self, canvas: &mut Canvas) {
        let window = 1..1 + CONTENT_ROWS as isize;
        if self.work.finished {
            canvas.text_centered(1, "Proof finished");
            canvas.layout(&self.statement, GUTTER, 2, window);
        } else {
            canvas.text_centered(1, "No open goals");
            canvas.text_centered(2, "Unknowns remain");
        }
    }

    fn draw_tree(&self, canvas: &mut Canvas) {
        let mark = text_width("?").max(text_width(&mcglyph::CHECK.to_string())) + 2;
        for row in 0..CONTENT_ROWS.min(self.work.tree.len().saturating_sub(self.tree.top)) {
            let at = self.tree.top + row;
            let node = &self.work.tree[at];
            let indent = MARGIN + INDENT * node.depth.min(MAX_DEPTH);
            let glyph = if node.done {
                Mark::Checked
            } else {
                Mark::Unchecked
            };
            canvas.text(indent, 1 + row, &glyph.glyph().to_string());
            let width = TEXT_WIDTH - (indent - MARGIN) - mark;
            let line = fitted(&node.expr, width, 1);
            let at_row = 1 + row as isize;
            canvas.layout(&line, indent + mark, at_row, at_row..at_row + 1);
            if at == self.tree.at {
                canvas.invert_row(1 + row);
            }
        }
    }

    fn draw_lists(&self, canvas: &mut Canvas) {
        let goal_rows = self.goal_rows();
        let window = 1..1 + CONTENT_ROWS as isize;
        canvas.layout(&self.goal, GUTTER, 1, window);
        let hyps_row = 1 + goal_rows;
        canvas.text(MARGIN, hyps_row, &format!("hyps: {}", self.work.hyps.len()));
        let rows = self.list_rows();
        match self.view {
            View::Hyps => {
                let indent = text_width(&number(self.work.hyps.len())) + 2;
                if self.work.hyps.is_empty() {
                    canvas.text_centered(hyps_row + 1 + rows / 2, "none");
                }
                let shown = rows.min(self.hyps.len().saturating_sub(self.hyp.top));
                for row in 0..shown {
                    let at = self.hyp.top + row;
                    let y = hyps_row + 1 + row;
                    canvas.text(MARGIN, y, &number(at + 1));
                    let (line, y) = (&self.hyps[at], y as isize);
                    canvas.layout(line, MARGIN + indent, y, y..y + 1);
                    if at == self.hyp.at {
                        canvas.invert_row(y as usize);
                    }
                }
            }
            _ => match &self.work.picks {
                Picks::Unknown => canvas.text_centered(hyps_row + 1 + rows / 2, "not known yet"),
                Picks::List(list) if list.is_empty() => {
                    canvas.text_centered(hyps_row + 1 + rows / 2, "no candidates");
                }
                Picks::List(list) => {
                    let shown = rows.min(list.len().saturating_sub(self.picks.top));
                    for row in 0..shown {
                        let at = self.picks.top + row;
                        let pick = &list[at];
                        canvas.list_row(
                            hyps_row + 1 + row,
                            &pick.label,
                            &format!("+{}", pick.new_goals),
                            at == self.picks.at,
                        );
                    }
                }
            },
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = &ctx.libs[self.library];
        canvas.title(lib.label(self.theorem), &format!("{} open", self.work.open));
        match (self.view, self.work.goal) {
            (View::Tree, _) => self.draw_tree(canvas),
            (_, None) => self.draw_finished(canvas),
            _ => self.draw_lists(canvas),
        }
        let [apply, _, _, undo, tree] = KEYS;
        canvas.soft_keys(match (self.view, self.work.goal) {
            (View::Tree, _) => [apply, "", "", undo, tree],
            (_, None) => ["", "", "", undo, tree],
            _ => KEYS,
        });
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::Proving {
            label: ctx.libs[self.library].label(self.theorem).to_string(),
            open: self.work.open,
            goal: self.work.goal,
            finished: self.work.finished,
            view: self.view,
            selected: match self.view {
                View::Candidates => self.picks.at,
                View::Hyps => self.hyp.at,
                View::Tree => self.tree.at,
            },
            candidate: match (&self.work.picks, self.view) {
                (Picks::List(list), View::Candidates) => {
                    list.get(self.picks.at).map(|p| p.label.clone())
                }
                _ => None,
            },
            steps: self.work.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KEYS;
    use crate::canvas::{soft_key_room, text_width};

    #[test]
    fn every_key_label_fits_its_key() {
        for (i, label) in KEYS.iter().enumerate() {
            assert!(text_width(label) <= soft_key_room(i), "{label}");
        }
    }
}
