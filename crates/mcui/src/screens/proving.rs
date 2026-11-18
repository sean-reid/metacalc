use super::{Ctx, Nav, Screen, State, View, notice::Notice, reveal, step};
use crate::canvas::{
    CONTENT_ROWS, Canvas, GUTTER, MARGIN, STATEMENT_WIDTH, TEXT_WIDTH, text_width,
};
use crate::edit::{Editor, Hyp, Node, Picks, Snap, Standing};
use crate::fold::{Layout, fitted};
use crate::key::Key;
use crate::proofs::SavedProof;
use mcedit::GoalId;

/// The word on the first key of the outline when the highlighted goal can be jumped to.
const SEE: &str = "See";

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
    editor: Editor,
    snap: Snap,
    hyp_list: Vec<Hyp>,
    /// The outline, built while the tree is on screen.
    tree_list: Vec<Node>,
    view: View,
    goal: Layout,
    statement: Layout,
    hyps: Vec<Layout>,
    picks: Cursor,
    hyp: Cursor,
    tree: Cursor,
    saved: SavedProof,
}

fn number(n: usize) -> String {
    format!("{n}.")
}

impl Proving {
    /// Opens the proof of theorem `index`, from the saved form the interface holds for it or
    /// from the bare statement.
    pub fn open(ctx: &Ctx, library: usize, index: usize) -> Screen {
        let held = ctx.proofs.find(library, index);
        let lib = &ctx.libs[library];
        let mut editor = match Editor::start(lib, index, held) {
            Ok(editor) => editor,
            Err(why) => return Notice::screen("Prove", &[why]),
        };
        let snap = editor.snap(held.and_then(|s| s.goal), None);
        let hyp_list = editor.hyps();
        let indent = text_width(&number(hyp_list.len())) + 2;
        let hyps = hyp_list
            .iter()
            .map(|h| fitted(&h.expr, TEXT_WIDTH - indent, 1))
            .collect();
        let statement = fitted(
            &lib.statement(index).conclusion,
            STATEMENT_WIDTH,
            STATEMENT_ROWS,
        );
        let saved = SavedProof {
            library,
            crc: lib.crc(),
            theorem: index,
            session: editor.save(),
            goal: snap.goal,
            steps: editor.steps(),
            groups: editor.groups().to_vec(),
        };
        let mut proving = Proving {
            library,
            theorem: index,
            editor,
            snap,
            hyp_list,
            tree_list: Vec::new(),
            view: View::Candidates,
            goal: Layout::empty(),
            statement,
            hyps,
            picks: Cursor::default(),
            hyp: Cursor::default(),
            tree: Cursor::default(),
            saved,
        };
        proving.refresh();
        Screen::Proving(Box::new(proving))
    }

    /// The saved form of what is on screen.
    pub fn saved(&self) -> &SavedProof {
        &self.saved
    }

    /// Lays out the goal on screen.
    fn refresh(&mut self) {
        self.goal = match &self.snap.goal_expr {
            Some(expr) => fitted(expr, STATEMENT_WIDTH, GOAL_ROWS),
            None => Layout::empty(),
        };
    }

    fn goal_rows(&self) -> usize {
        self.goal.rows().max(1)
    }

    /// Rows under the hypotheses line, where candidates or hypotheses are listed.
    fn list_rows(&self) -> usize {
        CONTENT_ROWS - self.goal_rows() - 1
    }

    fn at_tree_row(&self) -> usize {
        self.tree_list
            .iter()
            .position(|n| Some(n.goal) == self.snap.goal)
            .unwrap_or(0)
    }

    fn open_tree(&mut self) {
        self.tree_list = self.editor.tree();
        self.tree = Cursor::to(self.at_tree_row(), CONTENT_ROWS);
    }

    /// Runs `step`, which names the goal to show or the one to move on from, and shows what the
    /// session now holds; a refusal shows why instead and leaves everything as it was.
    fn run(
        &mut self,
        title: &'static str,
        step: impl FnOnce(&mut Editor) -> Result<(Option<GoalId>, Option<GoalId>), &'static str>,
    ) -> Nav {
        let (goal, after) = match step(&mut self.editor) {
            Ok(moved) => moved,
            Err(why) => return Nav::push(Notice::screen(title, &[why])),
        };
        self.snap = self.editor.snap(goal, after);
        self.saved.session = self.editor.save();
        self.saved.goal = self.snap.goal;
        self.saved.steps = self.editor.steps();
        self.saved.groups = self.editor.groups().to_vec();
        self.picks = Cursor::default();
        self.hyp = Cursor::default();
        if self.view == View::Hyps {
            self.view = View::Candidates;
        }
        self.refresh();
        if self.view == View::Tree {
            self.open_tree();
        }
        Nav::Stay
    }

    fn apply(&mut self) -> Nav {
        let Some(goal) = self.snap.goal else {
            return Nav::Stay;
        };
        let pick = match &self.snap.picks {
            Picks::Unknown => return Nav::push(Notice::screen("Apply", &["Not known yet"])),
            Picks::List(list) => list.get(self.picks.at).map(|p| p.assertion),
        };
        let Some(assertion) = pick else {
            return Nav::push(Notice::screen("Apply", &["No candidate"]));
        };
        self.run("Apply", |editor| {
            editor.apply(goal, assertion)?;
            Ok((None, Some(goal)))
        })
    }

    fn close(&mut self) -> Nav {
        let (Some(goal), Some(hyp)) = (self.snap.goal, self.hyp_list.get(self.hyp.at)) else {
            return Nav::Stay;
        };
        let id = hyp.id;
        self.run("Hyp", |editor| {
            editor.close(goal, id)?;
            Ok((None, Some(goal)))
        })
    }

    fn auto(&mut self) -> Nav {
        let Some(goal) = self.snap.goal else {
            return Nav::Stay;
        };
        self.run("Auto", |editor| {
            editor.auto(goal)?;
            Ok((None, Some(goal)))
        })
    }

    fn undo(&mut self) -> Nav {
        self.run("Undo", |editor| Ok((editor.undo()?, None)))
    }

    fn jump(&mut self) -> Nav {
        let Some(node) = self
            .tree_list
            .get(self.tree.at)
            .filter(|n| n.standing == Standing::Open)
        else {
            return Nav::Stay;
        };
        let goal = node.goal;
        let nav = self.run("Tree", |_| Ok((Some(goal), None)));
        self.view = View::Candidates;
        nav
    }

    pub fn press(&mut self, _ctx: &Ctx, key: Key) -> Nav {
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
                self.tree.go(self.tree_list.len(), key, CONTENT_ROWS);
            }
            (View::Tree, Key::Enter | Key::F1) => return self.jump(),
            (View::Tree, Key::F2 | Key::F3) => {}
            (View::Hyps, Key::Up | Key::Down | Key::Left | Key::Right) => {
                let rows = self.list_rows();
                self.hyp.go(self.hyp_list.len(), key, rows);
            }
            (View::Hyps, Key::Enter | Key::F1) => return self.close(),
            (View::Candidates, Key::Up | Key::Down | Key::Left | Key::Right) => {
                let rows = self.list_rows();
                let len = match &self.snap.picks {
                    Picks::List(list) => list.len(),
                    Picks::Unknown => 0,
                };
                self.picks.go(len, key, rows);
            }
            (View::Candidates, Key::Enter | Key::F1) => return self.apply(),
            (_, Key::F2) if self.snap.goal.is_none() => {}
            (_, Key::F2) => {
                self.view = if self.view == View::Hyps {
                    View::Candidates
                } else {
                    View::Hyps
                };
            }
            (_, Key::F3) => return self.auto(),
            (_, Key::F4) => return self.undo(),
            (View::Tree, Key::F5) => self.view = View::Candidates,
            (_, Key::F5) => {
                self.view = View::Tree;
                self.open_tree();
            }
            _ => {}
        }
        Nav::Stay
    }

    fn draw_finished(&self, canvas: &mut Canvas) {
        let window = 1..1 + CONTENT_ROWS as isize;
        if self.snap.finished {
            canvas.text_centered(1, "Proof finished");
            canvas.layout(&self.statement, GUTTER, 2, window);
        } else {
            canvas.text_centered(1, "No open goals");
            canvas.text_centered(2, "Not valid yet");
        }
    }

    fn draw_tree(&self, canvas: &mut Canvas) {
        let mark = text_width("?").max(text_width(&mcglyph::CHECK.to_string())) + 2;
        for row in 0..CONTENT_ROWS.min(self.tree_list.len().saturating_sub(self.tree.top)) {
            let at = self.tree.top + row;
            let node = &self.tree_list[at];
            let indent = MARGIN + INDENT * node.depth.min(MAX_DEPTH);
            let glyph = match node.standing {
                Standing::Open => Some('?'),
                Standing::Done => Some(mcglyph::CHECK),
                Standing::Partial => None,
            };
            if let Some(glyph) = glyph {
                canvas.text(indent, 1 + row, &glyph.to_string());
            }
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
        canvas.text(MARGIN, hyps_row, &format!("hyps: {}", self.hyp_list.len()));
        let rows = self.list_rows();
        match self.view {
            View::Hyps => {
                let indent = text_width(&number(self.hyp_list.len())) + 2;
                if self.hyp_list.is_empty() {
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
            _ => match &self.snap.picks {
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
        canvas.title(lib.label(self.theorem), &format!("{} open", self.snap.open));
        match (self.view, self.snap.goal) {
            (View::Tree, _) => self.draw_tree(canvas),
            (_, None) => self.draw_finished(canvas),
            _ => self.draw_lists(canvas),
        }
        let [_, _, _, undo, tree] = KEYS;
        let see = match self.tree_list.get(self.tree.at) {
            Some(node) if node.standing == Standing::Open => SEE,
            _ => "",
        };
        canvas.soft_keys(match (self.view, self.snap.goal) {
            (View::Tree, _) => [see, "", "", undo, tree],
            (_, None) => ["", "", "", undo, tree],
            _ => KEYS,
        });
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::Proving {
            label: ctx.libs[self.library].label(self.theorem).to_string(),
            open: self.snap.open,
            goal: self.snap.goal,
            finished: self.snap.finished,
            view: self.view,
            selected: match self.view {
                View::Candidates => self.picks.at,
                View::Hyps => self.hyp.at,
                View::Tree => self.tree.at,
            },
            candidate: match (&self.snap.picks, self.view) {
                (Picks::List(list), View::Candidates) => {
                    list.get(self.picks.at).map(|p| p.label.clone())
                }
                _ => None,
            },
            steps: self.saved.steps,
            replayed: self.editor.replayed(),
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
        assert!(text_width(super::SEE) <= soft_key_room(0));
    }
}
