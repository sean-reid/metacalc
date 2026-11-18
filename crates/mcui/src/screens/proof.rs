use super::{Ctx, Nav, Screen, State, reveal, step, theorem::Theorem};
use crate::canvas::{
    CONTENT_ROWS, Canvas, GUTTER, MARGIN, STATEMENT_WIDTH, TEXT_WIDTH, text_width,
};
use crate::data::{By, Goals, Lib};
use crate::fold::{Folds, Layout, fit_one, fitted, layout};
use crate::key::Key;
use crate::status::Mark;

/// Rows the goal may take before it is folded.
/// The labels of the five keys.
pub(crate) const KEYS: [&str; 5] = ["Fold", "Up", "Thm", "", ""];

const GOAL_ROWS: usize = 3;

/// One node of a theorem's goal tree: the goal, how it was proved, and its subgoals.
pub struct Proof {
    library: usize,
    goals: Goals,
    path: Vec<usize>,
    /// The subgoal the highlight is on.
    selected: usize,
    /// Whether the cursor is on the goal itself rather than a subgoal.
    on_goal: bool,
    full: bool,
    by: By,
    goal: Layout,
    subgoals: Vec<(Layout, usize)>,
}

fn number(n: usize) -> String {
    format!("{n}.")
}

impl Proof {
    pub fn open(ctx: &Ctx, library: usize, index: usize) -> Result<Screen, &'static str> {
        let goals = ctx.libs[library].goals(index)?;
        let mut proof = Proof {
            library,
            goals,
            path: Vec::new(),
            selected: 0,
            on_goal: false,
            full: false,
            by: By::Hypothesis(0),
            goal: Layout::empty(),
            subgoals: Vec::new(),
        };
        proof.refresh(ctx);
        Ok(Screen::Proof(proof))
    }

    fn lib<'a>(&self, ctx: &'a Ctx) -> &'a Lib {
        &ctx.libs[self.library]
    }

    /// Builds what the screen shows for the node the path points at.
    fn refresh(&mut self, ctx: &Ctx) {
        self.rebuild(ctx);
        self.selected = self.selected.min(self.subgoals.len().saturating_sub(1));
        self.on_goal = self.subgoals.is_empty();
    }

    /// Lays out the node again, leaving the cursor where it is.
    fn rebuild(&mut self, ctx: &Ctx) {
        let node = self.goals.node(&self.path);
        let view = self.lib(ctx).view(&self.goals, &node);
        self.by = view.by;
        let folds = if self.full {
            Folds::none(&view.goal)
        } else {
            fit_one(&view.goal, STATEMENT_WIDTH, GOAL_ROWS)
        };
        let goal = layout(&view.goal, &folds, STATEMENT_WIDTH);
        self.goal = if self.full {
            goal
        } else {
            goal.clipped(STATEMENT_WIDTH, GOAL_ROWS)
        };
        let indent = text_width(&number(view.subgoals.len())) + 2;
        self.subgoals = view
            .subgoals
            .iter()
            .map(|expr| (fitted(expr, TEXT_WIDTH - indent, 1), indent))
            .collect();
    }

    pub fn press(&mut self, ctx: &Ctx, key: Key) -> Nav {
        match key {
            Key::Up | Key::Down if self.subgoals.is_empty() => {}
            Key::Up | Key::Down => {
                if self.on_goal {
                    self.on_goal = key == Key::Up;
                } else if key == Key::Up && self.selected == 0 {
                    self.on_goal = true;
                } else {
                    self.selected = step(self.selected, self.subgoals.len(), key, 1);
                }
            }
            Key::Enter | Key::Right => {
                if !self.on_goal && self.selected < self.subgoals.len() {
                    self.path.push(self.selected);
                    self.selected = 0;
                    self.refresh(ctx);
                }
            }
            Key::Left | Key::F2 => {
                if let Some(came_from) = self.path.pop() {
                    self.refresh(ctx);
                    self.selected = came_from.min(self.subgoals.len().saturating_sub(1));
                    self.on_goal = self.subgoals.is_empty();
                }
            }
            Key::F1 => {
                self.full = !self.full;
                self.rebuild(ctx);
            }
            Key::F3 => {
                if let By::Assertion(i) = self.by {
                    return Nav::push(Theorem::open(ctx, self.library, i));
                }
            }
            Key::Clear => return Nav::Pop,
            _ => {}
        }
        Nav::Stay
    }

    /// The first display row shown: the view starts at the goal unless the highlight is below it.
    fn top(&self) -> usize {
        let highlighted = self.goal.rows() + 1 + self.selected;
        if self.on_goal {
            0
        } else {
            reveal(0, highlighted, CONTENT_ROWS)
        }
    }

    pub fn draw(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let lib = self.lib(ctx);
        let theorem = lib.label(self.goals.theorem);
        let place = format!(
            "{}/{}",
            capped(self.goals.position(&self.path)),
            capped(self.goals.total())
        );
        canvas.title(theorem, &place);
        let top = self.top() as isize;
        let window = 1..1 + CONTENT_ROWS as isize;
        let at = |display: usize| 1 + display as isize - top;
        canvas.layout(&self.goal, GUTTER, at(0), window.clone());
        if self.on_goal && window.contains(&at(0)) {
            canvas.marker(at(0) as usize);
        }
        let by_row = self.goal.rows();
        if window.contains(&at(by_row)) {
            let (left, mark) = match self.by {
                By::Hypothesis(n) => (format!("by hyp {n}"), None),
                By::Assertion(i) => (format!("by {}", lib.label(i)), Some(lib.mark(i))),
            };
            let right = mark.map_or(String::new(), |m: Mark| m.glyph().to_string());
            canvas.list_row(at(by_row) as usize, &left, &right, false);
        }
        for (k, (sub, indent)) in self.subgoals.iter().enumerate() {
            let row = at(by_row + 1 + k);
            if !window.contains(&row) {
                continue;
            }
            canvas.text(MARGIN, row as usize, &number(k + 1));
            canvas.layout(sub, MARGIN + indent, row, row..row + 1);
            if k == self.selected && !self.on_goal {
                canvas.invert_row(row as usize);
            }
        }
        canvas.soft_keys(KEYS);
    }

    pub fn state(&self, ctx: &Ctx) -> State {
        State::Proof {
            label: self.lib(ctx).label(self.goals.theorem).to_string(),
            path: self.path.clone(),
            selected: self.selected,
            on_goal: self.on_goal,
        }
    }
}

/// A count as the title shows it: whole up to five digits, then `99999+`.
fn capped(n: u64) -> String {
    if n > 99_999 {
        "99999+".to_string()
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::capped;

    #[test]
    fn counts_show_whole_up_to_five_digits() {
        assert_eq!(capped(7), "7");
        assert_eq!(capped(99_999), "99999");
        assert_eq!(capped(100_000), "99999+");
        assert_eq!(capped(u64::MAX), "99999+");
    }
}
