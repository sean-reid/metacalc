use crate::dv::DvCond;
use crate::error::Error;
use crate::session::{GoalId, GoalState, Op, Outcome, Session, Undo};
use crate::term::{MetaId, TermId};
use mcpack::mm::{Kind, Parsed, StmtId, Sym};
use std::rc::Rc;

impl Session<'_> {
    /// Runs one recorded operation.
    pub fn run(&mut self, op: Op) -> Result<Outcome, Error> {
        match op {
            Op::Apply { goal, assertion } => self.apply(goal, assertion),
            Op::Close { goal, hyp } => self.close(goal, hyp),
            Op::Assign { meta, var } => self.assign(meta, var),
            Op::Refine { meta, syntax } => self.refine(meta, syntax),
        }
    }

    /// Runs operations in order and stops at the first that fails, leaving the earlier ones in force.
    pub fn play(&mut self, ops: &[Op]) -> Result<(), Error> {
        ops.iter().try_for_each(|&op| self.run(op).map(drop))
    }

    pub(crate) fn open_goal(&self, goal: GoalId) -> Result<(), Error> {
        match self.goals.get(goal as usize).map(|g| g.state) {
            None => Err(Error::NoSuchGoal),
            Some(GoalState::Open) => Ok(()),
            Some(_) => Err(Error::GoalNotOpen),
        }
    }

    /// An assertion of the provable typecode that comes before the theorem.
    pub(crate) fn usable(&self, assertion: StmtId) -> Result<Rc<Parsed>, Error> {
        let db = self.db();
        let stmt = db
            .stmts
            .get(assertion as usize)
            .ok_or(Error::NotAnAssertion)?;
        if !stmt.kind.is_assertion() || stmt.expr[0] != self.provable {
            return Err(Error::NotAnAssertion);
        }
        if assertion >= self.theorem {
            return Err(Error::Forward);
        }
        self.cat.parsed(assertion).map_err(Error::Syntax)
    }

    /// Renames the assertion's variables to fresh metavariables and unifies its conclusion with
    /// the goal; each essential hypothesis becomes a new goal.
    pub fn apply(&mut self, goal: GoalId, assertion: StmtId) -> Result<Outcome, Error> {
        self.open_goal(goal)?;
        let parsed = self.usable(assertion)?;
        let before = self.broken();
        let undo = self.checkpoint(Some(goal));
        if let Err(e) = self.apply_unchecked(goal, assertion, &parsed) {
            self.rollback_to(undo);
            return Err(e);
        }
        Ok(self.commit(Op::Apply { goal, assertion }, undo, &before))
    }

    /// One fresh metavariable per mandatory floating hypothesis, in frame order.
    pub(crate) fn rename(&mut self, assertion: StmtId) -> Vec<(Sym, TermId)> {
        let db = self.db();
        let mut subst = Vec::new();
        for &h in &db.stmt(assertion).frame.hyps {
            let float = db.stmt(h);
            if float.kind == Kind::Float {
                subst.push((float.expr[1], self.store.new_meta(float.expr[0])));
            }
        }
        subst
    }

    fn apply_unchecked(
        &mut self,
        goal: GoalId,
        assertion: StmtId,
        parsed: &Parsed,
    ) -> Result<(), Error> {
        let db = self.db();
        let stmt = db.stmt(assertion);
        let base = self.store.meta_count() as MetaId;
        let subst = self.rename(assertion);
        let conclusion = self.build(&parsed.conclusion, &subst);
        let target = self.goals[goal as usize].term;
        self.store.unify(target, conclusion).map_err(Error::Unify)?;
        let first = self.goals.len() as GoalId;
        for tree in &parsed.hyps {
            let term = self.build(tree, &subst);
            self.goals.push(crate::session::Goal {
                term,
                state: GoalState::Open,
            });
        }
        for &(x, y) in &stmt.frame.dv {
            let term_of = |v: Sym| subst.iter().find(|p| p.0 == v).map(|p| p.1);
            if let (Some(a), Some(b)) = (term_of(x), term_of(y)) {
                self.dvs.push(DvCond { assertion, a, b });
            }
        }
        self.goals[goal as usize].state = GoalState::Applied {
            assertion,
            metas: base,
            first,
        };
        Ok(())
    }

    /// Closes a goal with one of the theorem's essential hypotheses.
    pub fn close(&mut self, goal: GoalId, hyp: StmtId) -> Result<Outcome, Error> {
        self.open_goal(goal)?;
        let &(_, term) = self
            .hyps
            .iter()
            .find(|h| h.0 == hyp)
            .ok_or(Error::NotAHypothesis)?;
        let before = self.broken();
        let undo = self.checkpoint(Some(goal));
        let target = self.goals[goal as usize].term;
        if let Err(e) = self.store.unify(target, term) {
            self.rollback_to(undo);
            return Err(Error::Unify(e));
        }
        self.goals[goal as usize].state = GoalState::Closed { hyp };
        Ok(self.commit(Op::Close { goal, hyp }, undo, &before))
    }

    fn unbound(&self, meta: MetaId) -> Result<(), Error> {
        if meta as usize >= self.store.meta_count() {
            return Err(Error::NoSuchMeta);
        }
        match self.store.is_unbound(meta) {
            true => Ok(()),
            false => Err(Error::AlreadyBound),
        }
    }

    /// Instantiates a metavariable with a variable the database declares for its typecode.
    pub fn assign(&mut self, meta: MetaId, var: Sym) -> Result<Outcome, Error> {
        self.unbound(meta)?;
        let typecode = self.store.meta_typecode(meta).ok_or(Error::NoSuchMeta)?;
        if !self.vars.of_typecode(typecode).any(|v| v == var) {
            return Err(Error::NotAVariable);
        }
        let before = self.broken();
        let undo = self.checkpoint(None);
        let term = self.store.var(var);
        if let Err(e) = self.store.bind(meta, term) {
            self.rollback_to(undo);
            return Err(Error::Unify(e));
        }
        Ok(self.commit(Op::Assign { meta, var }, undo, &before))
    }

    /// Instantiates a metavariable with a syntax axiom applied to new metavariables.
    pub fn refine(&mut self, meta: MetaId, syntax: StmtId) -> Result<Outcome, Error> {
        self.unbound(meta)?;
        let db = self.db();
        let stmt = db.stmts.get(syntax as usize).ok_or(Error::NotSyntax)?;
        let typecode = self.store.meta_typecode(meta).ok_or(Error::NoSuchMeta)?;
        if stmt.kind != Kind::Axiom || stmt.expr[0] != typecode {
            return Err(Error::NotSyntax);
        }
        if syntax >= self.theorem {
            return Err(Error::Forward);
        }
        let before = self.broken();
        let undo = self.checkpoint(None);
        let kids: Vec<u32> = stmt
            .frame
            .hyps
            .iter()
            .map(|&h| self.store.new_meta(db.stmt(h).expr[0]))
            .collect();
        let term = self.store.app(syntax, &kids);
        let target = self.store.meta_term(meta).ok_or(Error::NoSuchMeta)?;
        if let Err(e) = self.store.unify(target, term) {
            self.rollback_to(undo);
            return Err(Error::Unify(e));
        }
        Ok(self.commit(Op::Refine { meta, syntax }, undo, &before))
    }

    fn checkpoint(&self, reopened: Option<GoalId>) -> Undo {
        Undo {
            mark: self.store.mark(),
            metas: self.store.meta_count() as MetaId,
            goals: self.goals.len() as u32,
            conditions: self.dvs.len() as u32,
            reopened,
        }
    }

    fn commit(&mut self, op: Op, undo: Undo, before: &[bool]) -> Outcome {
        let outcome = Outcome {
            goals: undo.goals..self.goals.len() as GoalId,
            metas: undo.metas..self.store.meta_count() as MetaId,
            violations: self.newly_broken(before),
        };
        self.ops.push(op);
        self.undo.push(undo);
        self.note_peak();
        outcome
    }
}
