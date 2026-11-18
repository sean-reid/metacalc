use crate::error::Error;
use crate::session::{GoalId, Op, Session};
use crate::term::{Head, TermId};
use mcpack::mm::{Kind, StmtId, Tree};

/// Applications of assertions `auto` may nest.
pub const AUTO_DEPTH: usize = 2;
/// Assertions `auto` tries before it gives up.
pub const AUTO_BUDGET: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub assertion: StmtId,
    /// The goals applying it would leave.
    pub new_goals: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Candidates {
    /// The goal is a bare metavariable and every assertion fits it.
    Unconstrained,
    /// Assertions before the theorem, in database order, whose conclusion unifies with the goal.
    List(Vec<Candidate>),
}

impl Session<'_> {
    fn may_fit(&self, goal: TermId, tree: &Tree) -> bool {
        if self.db().stmt(tree.stmt).kind == Kind::Float {
            return true;
        }
        match self.store.head(goal) {
            Some(Head::App(syntax)) => syntax == tree.stmt,
            Some(Head::Var(_)) => false,
            _ => true,
        }
    }

    pub fn candidates(&mut self, goal: GoalId, limit: usize) -> Result<Candidates, Error> {
        self.open_goal(goal)?;
        let target = self.goals[goal as usize].term;
        if matches!(self.store.head(target), Some(Head::Meta(_))) {
            return Ok(Candidates::Unconstrained);
        }
        let mut found = Vec::new();
        for assertion in 0..self.theorem {
            if found.len() >= limit {
                break;
            }
            let Ok(parsed) = self.usable(assertion) else {
                continue;
            };
            if !self.may_fit(target, &parsed.conclusion) {
                continue;
            }
            let mark = self.store.mark();
            let subst = self.rename(assertion);
            let conclusion = self.build(&parsed.conclusion, &subst);
            if self.store.unify(target, conclusion).is_ok() {
                found.push(Candidate {
                    assertion,
                    new_goals: parsed.hyps.len(),
                });
            }
            self.store.rollback(mark);
        }
        Ok(Candidates::List(found))
    }

    /// Nodes the last `auto` call tried.
    pub fn auto_nodes(&self) -> usize {
        self.auto_nodes
    }

    /// Searches depth first for the operations that close a goal without metavariables, using the
    /// theorem's hypotheses and candidate assertions in database order. Returns nothing when the
    /// search ends or spends `budget` assertion attempts. The session is left as it was.
    pub fn auto(&mut self, goal: GoalId, depth: usize, budget: usize) -> Option<Vec<Op>> {
        self.auto_nodes = 0;
        self.open_goal(goal).ok()?;
        if !self.store.is_ground(self.goals[goal as usize].term) {
            return None;
        }
        let base = self.ops.len();
        let found = self.solve(goal, depth, budget);
        let steps = self.ops[base..].to_vec();
        while self.ops.len() > base {
            let _ = self.undo();
        }
        found.then_some(steps)
    }

    fn solve(&mut self, goal: GoalId, depth: usize, budget: usize) -> bool {
        let hyps: Vec<StmtId> = self.hyps.iter().map(|h| h.0).collect();
        if hyps.into_iter().any(|h| self.close(goal, h).is_ok()) {
            return true;
        }
        if depth == 0 {
            return false;
        }
        let base = self.ops.len();
        for assertion in 0..self.theorem {
            if self.auto_nodes >= budget {
                return false;
            }
            let Ok(parsed) = self.usable(assertion) else {
                continue;
            };
            if !self.may_fit(self.goals[goal as usize].term, &parsed.conclusion) {
                continue;
            }
            self.auto_nodes += 1;
            let Ok(outcome) = self.apply(goal, assertion) else {
                continue;
            };
            let ground = outcome
                .goals
                .clone()
                .all(|g| self.store.is_ground(self.goals[g as usize].term));
            if ground
                && outcome.violations.is_empty()
                && outcome
                    .goals
                    .clone()
                    .all(|g| self.solve(g, depth - 1, budget))
            {
                return true;
            }
            while self.ops.len() > base {
                let _ = self.undo();
            }
        }
        false
    }
}
