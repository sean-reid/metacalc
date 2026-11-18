//! The proof editor behind the proving screen: a session restored from its saved bytes, one
//! operation at a time, and what the screen shows of it afterwards.

use crate::data::{Lib, PROVABLE};
use crate::expr::{Expr, Shape};
use mcedit::{
    AUTO_BUDGET, AUTO_DEPTH, Candidates, Catalog, Error, GoalId, GoalState, Head, Op, Session,
    TermId,
};
use mcpack::mm::StmtId;

/// Candidates listed for one goal.
const CANDIDATES: usize = 200;

pub struct Pick {
    pub assertion: StmtId,
    pub label: String,
    /// The goals applying it would leave.
    pub new_goals: usize,
}

pub enum Picks {
    /// The goal is still a bare unknown, so every assertion would fit.
    Unknown,
    List(Vec<Pick>),
}

pub struct Hyp {
    pub id: StmtId,
    pub expr: Expr,
}

/// One goal of the tree.
pub struct Node {
    pub goal: GoalId,
    pub depth: usize,
    /// Whether this goal is itself open.
    pub open: bool,
    /// Whether no goal at or below it is open.
    pub done: bool,
    pub expr: Expr,
}

/// What the screen shows of a session at one moment.
pub struct Work {
    pub saved: Vec<u8>,
    pub steps: usize,
    /// The goal on screen, none once nothing is open.
    pub goal: Option<GoalId>,
    pub open: usize,
    pub finished: bool,
    pub goal_expr: Option<Expr>,
    pub hyps: Vec<Hyp>,
    pub picks: Picks,
    pub tree: Vec<Node>,
}

pub struct Editor<'a> {
    session: Session<'a>,
    lead: Vec<String>,
}

fn refusal(error: &Error) -> &'static str {
    match error {
        Error::Unify(_) => "No match",
        Error::GoalNotOpen | Error::NoSuchGoal => "That goal is not open",
        _ => "Refused",
    }
}

impl<'a> Editor<'a> {
    /// A session on theorem `theorem` of `lib`: a new one, or the one `saved` holds.
    pub fn start(
        lib: &'a Lib,
        theorem: usize,
        saved: Option<&[u8]>,
    ) -> Result<Editor<'a>, &'static str> {
        if !lib.is_theorem(theorem) {
            return Err("an axiom has no proof");
        }
        let (db, grammar) = lib.database().ok_or("the library does not export")?;
        if lib
            .statement(theorem)
            .conclusion
            .lead
            .first()
            .map(String::as_str)
            != Some(PROVABLE)
        {
            return Err("it proves a syntax statement");
        }
        let id = lib.stmt_of(theorem).ok_or("the library does not export")?;
        let catalog = Catalog::new(db, grammar);
        let session = match saved {
            Some(bytes) => {
                Session::restore(catalog, bytes).map_err(|_| "the saved proof is damaged")?
            }
            None => Session::start_in(catalog, id).map_err(|_| "it proves a syntax statement")?,
        };
        if session.theorem() != id {
            return Err("the saved proof is for another theorem");
        }
        let lead = lib.statement(theorem).conclusion.lead.clone();
        Ok(Editor { session, lead })
    }

    /// Operations in force.
    pub fn steps(&self) -> usize {
        self.session.ops().len()
    }

    fn expr(&self, term: TermId) -> Expr {
        Expr::build(self.lead.clone(), &self.shape(term))
    }

    fn shape(&self, term: TermId) -> Shape {
        let (store, db) = (self.session.store(), self.session.db());
        match store.head(term) {
            Some(Head::Var(var)) => Shape::leaf(&db.syms[var as usize].name),
            Some(Head::Meta(meta)) => Shape::leaf(&format!("?{meta}")),
            Some(Head::App(syntax)) => {
                let stmt = db.stmt(syntax);
                let kids = store.children(term);
                let parts = stmt.expr[1..]
                    .iter()
                    .map(|&sym| {
                        let slot = db.syms[sym as usize]
                            .is_var
                            .then(|| {
                                stmt.frame
                                    .hyps
                                    .iter()
                                    .position(|&h| db.stmt(h).expr[1] == sym)
                            })
                            .flatten();
                        match slot {
                            Some(i) => self.shape(kids[i]),
                            None => Shape::tok(&db.syms[sym as usize].name),
                        }
                    })
                    .collect();
                Shape::node(parts)
            }
            None => Shape::leaf("?"),
        }
    }

    /// The goals in tree order with their depth.
    fn order(&self) -> Vec<(GoalId, usize)> {
        let mut out = Vec::new();
        let mut stack = vec![(0 as GoalId, 0usize)];
        while let Some((goal, depth)) = stack.pop() {
            out.push((goal, depth));
            stack.extend(self.session.children(goal).rev().map(|k| (k, depth + 1)));
        }
        out
    }

    fn is_open(&self, goal: GoalId) -> bool {
        self.session
            .goal(goal)
            .is_some_and(|g| matches!(g.state, GoalState::Open))
    }

    fn is_unknown(&self, goal: GoalId) -> bool {
        self.session
            .goal(goal)
            .is_some_and(|g| matches!(self.session.store().head(g.term), Some(Head::Meta(_))))
    }

    /// The open goal that follows `after` in tree order, wrapping at the end. A goal that is
    /// still a bare unknown waits behind the others, since nothing can be applied to it yet.
    fn following(&self, order: &[(GoalId, usize)], after: Option<GoalId>) -> Option<GoalId> {
        let at = after.and_then(|a| order.iter().position(|o| o.0 == a));
        let (tail, head) = match at {
            Some(at) => (&order[at + 1..], &order[..=at]),
            None => (order, &order[..0]),
        };
        let open: Vec<GoalId> = tail
            .iter()
            .chain(head)
            .map(|o| o.0)
            .filter(|&g| self.is_open(g))
            .collect();
        open.iter()
            .copied()
            .find(|&g| !self.is_unknown(g))
            .or(open.first().copied())
    }

    /// The open goal after `goal`, or the first open goal.
    pub fn next_after(&self, goal: Option<GoalId>) -> Option<GoalId> {
        self.following(&self.order(), goal)
    }

    pub fn apply(&mut self, goal: GoalId, assertion: StmtId) -> Result<(), &'static str> {
        let outcome = self
            .session
            .apply(goal, assertion)
            .map_err(|e| refusal(&e))?;
        self.reject_violations(outcome.violations.is_empty())
    }

    pub fn close(&mut self, goal: GoalId, hyp: StmtId) -> Result<(), &'static str> {
        let outcome = self.session.close(goal, hyp).map_err(|e| refusal(&e))?;
        self.reject_violations(outcome.violations.is_empty())
    }

    fn reject_violations(&mut self, clean: bool) -> Result<(), &'static str> {
        if clean {
            return Ok(());
        }
        let _ = self.session.undo();
        Err("It breaks a distinct variable rule")
    }

    pub fn auto(&mut self, goal: GoalId) -> Result<(), &'static str> {
        let term = self.session.goal(goal).ok_or("That goal is not open")?.term;
        if !self.session.store().is_ground(term) {
            return Err("The goal has unknowns");
        }
        let ops = self
            .session
            .auto(goal, AUTO_DEPTH, AUTO_BUDGET)
            .ok_or("Auto found nothing")?;
        self.session.play(&ops).map_err(|_| "Auto found nothing")
    }

    /// Undoes the last operation and returns the goal it reopened.
    pub fn undo(&mut self) -> Result<Option<GoalId>, &'static str> {
        match self.session.undo() {
            Ok(Op::Apply { goal, .. } | Op::Close { goal, .. }) => Ok(Some(goal)),
            Ok(_) => Ok(None),
            Err(_) => Err("Nothing to undo"),
        }
    }

    /// Everything the screen shows, with `goal` on screen when it is open and the first open
    /// goal otherwise.
    pub fn work(&mut self, goal: Option<GoalId>) -> Work {
        let order = self.order();
        let open: Vec<GoalId> = order
            .iter()
            .map(|o| o.0)
            .filter(|&g| self.is_open(g))
            .collect();
        let goal = goal
            .filter(|g| open.contains(g))
            .or_else(|| self.following(&order, None));
        let picks = match goal {
            Some(g) => match self.session.candidates(g, CANDIDATES) {
                Ok(Candidates::List(found)) => Picks::List(
                    found
                        .iter()
                        .map(|c| Pick {
                            assertion: c.assertion,
                            label: self.session.db().stmt(c.assertion).label.clone(),
                            new_goals: c.new_goals,
                        })
                        .collect(),
                ),
                _ => Picks::Unknown,
            },
            None => Picks::List(Vec::new()),
        };
        let mut has_open = vec![false; self.session.goal_count()];
        for &(g, _) in order.iter().rev() {
            has_open[g as usize] =
                self.is_open(g) || self.session.children(g).any(|k| has_open[k as usize]);
        }
        let tree = order
            .iter()
            .map(|&(g, depth)| Node {
                goal: g,
                depth,
                open: self.is_open(g),
                done: !has_open[g as usize],
                expr: self.expr(self.session.goal(g).expect("goals in the tree exist").term),
            })
            .collect();
        let hyps = self
            .session
            .hypotheses()
            .iter()
            .map(|&(id, term)| Hyp {
                id,
                expr: self.expr(term),
            })
            .collect();
        Work {
            saved: self.session.save(),
            steps: self.steps(),
            goal,
            open: open.len(),
            finished: open.is_empty() && self.session.export().is_ok(),
            goal_expr: goal.map(|g| self.expr(self.session.goal(g).expect("open goal").term)),
            hyps,
            picks,
            tree,
        }
    }
}
