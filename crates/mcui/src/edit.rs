//! The proof editor behind the proving screen: a session restored from its saved bytes, one
//! operation at a time, and what the screen shows of it afterwards.

use crate::data::{Lib, PROVABLE};
use crate::expr::{Expr, Shape};
use crate::proofs::SavedProof;
use mcedit::{
    AUTO_BUDGET, AUTO_DEPTH, Candidates, Error, GoalId, GoalState, Head, Op, Session, TermId,
};
use mcpack::mm::{Kind, StmtId};

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

/// How a goal of the tree stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// The goal itself is open.
    Open,
    /// Applied, with an open goal somewhere beneath it.
    Partial,
    /// Nothing at or below it is open.
    Done,
}

/// One goal of the tree.
pub struct Node {
    pub goal: GoalId,
    pub depth: usize,
    pub standing: Standing,
    pub expr: Expr,
}

/// What the screen shows of the goal on screen.
pub struct Snap {
    /// The goal on screen, none once nothing is open.
    pub goal: Option<GoalId>,
    pub open: usize,
    /// Nothing is open and no unknown is left.
    pub finished: bool,
    pub goal_expr: Option<Expr>,
    pub picks: Picks,
}

/// A live session on one theorem and the groups its operations were made in: each key press that
/// changes the session is one group, and undo takes back a whole group.
pub struct Editor {
    session: Session<'static>,
    lead: Vec<String>,
    groups: Vec<usize>,
    /// Operations replayed to rebuild the session when it was opened.
    replayed: usize,
}

fn refusal(error: &Error) -> &'static str {
    match error {
        Error::Unify(_) => "No match",
        Error::GoalNotOpen | Error::NoSuchGoal => "That goal is not open",
        _ => "Refused",
    }
}

impl Editor {
    /// A session on theorem `theorem` of `lib`: a new one, or the one `saved` holds.
    pub fn start(
        lib: &Lib,
        theorem: usize,
        saved: Option<&SavedProof>,
    ) -> Result<Editor, &'static str> {
        if !lib.is_theorem(theorem) {
            return Err("an axiom has no proof");
        }
        let catalog = lib.catalog().ok_or("the library does not export")?;
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
        let (session, groups) = match saved {
            Some(saved) => {
                let session = Session::restore(catalog, &saved.session).map_err(|e| match e {
                    Error::WrongLibrary => "the library has changed",
                    _ => "the saved proof is damaged",
                })?;
                let total = saved.groups.iter().try_fold(0usize, |sum, &g| {
                    (g > 0).then(|| sum.checked_add(g)).flatten()
                });
                if total != Some(session.ops().len()) {
                    return Err("the saved proof is damaged");
                }
                (session, saved.groups.clone())
            }
            None => (
                Session::start_in(catalog, id).map_err(|e| match e {
                    Error::TooLarge => "its statement is too large",
                    _ => "it proves a syntax statement",
                })?,
                Vec::new(),
            ),
        };
        if session.theorem() != id {
            return Err("the saved proof is for another theorem");
        }
        let lead = lib.statement(theorem).conclusion.lead.clone();
        let replayed = session.ops().len();
        Ok(Editor {
            session,
            lead,
            groups,
            replayed,
        })
    }

    /// Operations in force.
    pub fn steps(&self) -> usize {
        self.session.ops().len()
    }

    pub fn replayed(&self) -> usize {
        self.replayed
    }

    pub fn groups(&self) -> &[usize] {
        &self.groups
    }

    pub fn save(&self) -> Vec<u8> {
        self.session.save()
    }

    pub fn hyps(&self) -> Vec<Hyp> {
        self.session
            .hypotheses()
            .iter()
            .map(|&(id, term)| Hyp {
                id,
                expr: self.expr(term),
            })
            .collect()
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

    /// Runs `step` as one group: whatever it records, and the unknowns it leaves once the last
    /// goal is closed, undo together. A refused step leaves nothing behind.
    fn grouped(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        let before = self.session.ops().len();
        let done = step(self);
        if done.is_ok() {
            self.bind_unknowns();
        }
        if done.is_err() {
            while self.session.ops().len() > before {
                let _ = self.session.undo();
            }
            return done;
        }
        let made = self.session.ops().len() - before;
        if made > 0 {
            self.groups.push(made);
        }
        Ok(())
    }

    /// With no goal open, binds each unknown left to a variable: the theorem's own of its
    /// typecode in statement order, then the others in declaration order, the first one mcedit
    /// takes without breaking a distinct variable rule.
    fn bind_unknowns(&mut self) {
        if (0..self.session.goal_count() as GoalId).any(|g| self.is_open(g)) {
            return;
        }
        let db = self.session.db();
        let own: Vec<_> = db
            .stmt(self.session.theorem())
            .frame
            .hyps
            .iter()
            .map(|&h| db.stmt(h))
            .filter(|h| h.kind == Kind::Float)
            .map(|h| (h.expr[0], h.expr[1]))
            .collect();
        let unknowns: Vec<_> = self.session.store().unbound().collect();
        for meta in unknowns {
            let Some(typecode) = self.session.store().meta_typecode(meta) else {
                continue;
            };
            let mut vars: Vec<_> = own
                .iter()
                .filter(|p| p.0 == typecode)
                .map(|p| p.1)
                .collect();
            let others: Vec<_> = self.session.vars().of_typecode(typecode).collect();
            for var in others {
                if !vars.contains(&var) {
                    vars.push(var);
                }
            }
            for var in vars {
                match self.session.assign(meta, var) {
                    Ok(outcome) if outcome.violations.is_empty() => break,
                    Ok(_) => {
                        let _ = self.session.undo();
                    }
                    Err(_) => {}
                }
            }
        }
    }

    pub fn apply(&mut self, goal: GoalId, assertion: StmtId) -> Result<(), &'static str> {
        self.grouped(|me| {
            let outcome = me.session.apply(goal, assertion).map_err(|e| refusal(&e))?;
            me.reject_violations(outcome.violations.is_empty())
        })
    }

    pub fn close(&mut self, goal: GoalId, hyp: StmtId) -> Result<(), &'static str> {
        self.grouped(|me| {
            let outcome = me.session.close(goal, hyp).map_err(|e| refusal(&e))?;
            me.reject_violations(outcome.violations.is_empty())
        })
    }

    fn reject_violations(&mut self, clean: bool) -> Result<(), &'static str> {
        if clean {
            return Ok(());
        }
        let _ = self.session.undo();
        Err("It breaks a distinct variable rule")
    }

    pub fn auto(&mut self, goal: GoalId) -> Result<(), &'static str> {
        self.grouped(|me| {
            let term = me.session.goal(goal).ok_or("That goal is not open")?.term;
            if !me.session.store().is_ground(term) {
                return Err("The goal has unknowns");
            }
            let ops = me
                .session
                .auto(goal, AUTO_DEPTH, AUTO_BUDGET)
                .ok_or("Auto found nothing")?;
            me.session.play(&ops).map_err(|_| "Auto found nothing")
        })
    }

    /// Takes back the last group and returns the goal it started on.
    pub fn undo(&mut self) -> Result<Option<GoalId>, &'static str> {
        let size = self.groups.pop().ok_or("Nothing to undo")?;
        let at = self
            .session
            .ops()
            .len()
            .checked_sub(size)
            .filter(|_| size > 0);
        let first = *at
            .and_then(|at| self.session.ops().get(at))
            .ok_or("the saved proof is damaged")?;
        for _ in 0..size {
            let _ = self.session.undo();
        }
        Ok(match first {
            Op::Apply { goal, .. } | Op::Close { goal, .. } => Some(goal),
            _ => None,
        })
    }

    /// The goal on screen and what depends on it: `goal` when it is open, else the one after
    /// `after`, else the first open goal.
    pub fn snap(&mut self, goal: Option<GoalId>, after: Option<GoalId>) -> Snap {
        let order = self.order();
        let open = order.iter().filter(|o| self.is_open(o.0)).count();
        let goal = goal
            .filter(|&g| self.is_open(g))
            .or_else(|| self.following(&order, after));
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
        Snap {
            goal,
            open,
            finished: open == 0
                && self.session.store().unbound().next().is_none()
                && self.session.status().clean(),
            goal_expr: goal.map(|g| self.expr(self.session.goal(g).expect("open goal").term)),
            picks,
        }
    }

    /// The whole goal tree in order.
    pub fn tree(&self) -> Vec<Node> {
        let order = self.order();
        let mut has_open = vec![false; self.session.goal_count()];
        for &(g, _) in order.iter().rev() {
            has_open[g as usize] =
                self.is_open(g) || self.session.children(g).any(|k| has_open[k as usize]);
        }
        order
            .iter()
            .map(|&(g, depth)| Node {
                goal: g,
                depth,
                standing: match (self.is_open(g), has_open[g as usize]) {
                    (true, _) => Standing::Open,
                    (false, true) => Standing::Partial,
                    (false, false) => Standing::Done,
                },
                expr: self.expr(self.session.goal(g).expect("goals in the tree exist").term),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcedit::Catalog;
    use mcpack::mm::{self, Grammar};

    const SRC: &str = "
$c ( ) -> wff |- setvar Q $.
$v ph ps ch x $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
wi $a wff ( ph -> ps ) $.
${ $d x ph $. wq $a wff Q x ph $. $}
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  t.1 $e |- ph $.
  t.2 $e |- ( ph -> ps ) $.
  t $p |- ps $= ? $.
$}
";

    fn editor() -> Editor {
        let db: &'static mm::Database =
            Box::leak(Box::new(mm::parse(SRC.as_bytes().to_vec()).unwrap()));
        let grammar: &'static Grammar<'static> =
            Box::leak(Box::new(Grammar::new(db, &[("|-", "wff")]).unwrap()));
        let catalog = Catalog::new(db, grammar, 1);
        let session = Session::start_in(catalog, db.find("t").unwrap()).unwrap();
        Editor {
            session,
            lead: vec![PROVABLE.to_string()],
            groups: Vec::new(),
            replayed: 0,
        }
    }

    #[test]
    fn a_proof_that_cannot_be_checked_is_not_finished() {
        let mut editor = editor();
        let db = editor.session.db();
        let (ax_mp, min, maj) = (
            db.find("ax-mp").unwrap(),
            db.find("t.1").unwrap(),
            db.find("t.2").unwrap(),
        );
        editor.session.apply(0, ax_mp).unwrap();
        editor.session.close(1, min).unwrap();
        editor.session.close(2, maj).unwrap();
        assert!(editor.snap(None, None).finished);
        editor.session.corrupt_metas();
        assert!(!editor.snap(None, None).finished);
    }
}
