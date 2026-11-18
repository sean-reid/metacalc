use crate::catalog::Catalog;
use crate::dv::{DvCond, Violation};
use crate::error::Error;
use crate::term::{Mark, MetaId, Peak, Store, TermId};
use crate::vars::Vars;
use mcpack::mm::{Database, Grammar, Kind, Proof, ProofError, StmtId, Sym, Tree};
use std::ops::Range;
use std::rc::Rc;

pub type GoalId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalState {
    Open,
    /// Closed by an assertion. Its mandatory floating hypotheses stand for the metavariables from `metas`
    /// on, and its essential hypotheses became the goals from `first` on.
    Applied {
        assertion: StmtId,
        metas: MetaId,
        first: GoalId,
    },
    Closed {
        hyp: StmtId,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Goal {
    pub term: TermId,
    pub state: GoalState,
}

/// One user operation. A session is its list of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Apply { goal: GoalId, assertion: StmtId },
    Close { goal: GoalId, hyp: StmtId },
    Assign { meta: MetaId, var: Sym },
    Refine { meta: MetaId, syntax: StmtId },
}

/// What an operation changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub goals: Range<GoalId>,
    pub metas: Range<MetaId>,
    /// Conditions this operation completed and found broken.
    pub violations: Vec<Violation>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// Open goals in tree order.
    pub open: Vec<GoalId>,
    pub metas: Vec<MetaId>,
    pub violations: Vec<Violation>,
    /// Pairs the session added for dummy variables.
    pub added_dv: Vec<(Sym, Sym)>,
}

/// Sizes of the structures a session holds, in entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    pub terms: Peak,
    pub goals: usize,
    pub conditions: usize,
    pub ops: usize,
}

pub(crate) struct Undo {
    pub mark: Mark,
    pub metas: MetaId,
    pub goals: u32,
    pub conditions: u32,
    pub reopened: Option<GoalId>,
}

pub struct Session<'a> {
    pub(crate) cat: Rc<Catalog<'a>>,
    pub(crate) theorem: StmtId,
    pub(crate) provable: Sym,
    pub(crate) open_dv: bool,
    pub(crate) vars: Vars,
    pub(crate) store: Store<'a>,
    pub(crate) hyps: Vec<(StmtId, TermId)>,
    pub(crate) goals: Vec<Goal>,
    pub(crate) dvs: Vec<DvCond>,
    pub(crate) scope_dv: Vec<(Sym, Sym)>,
    pub(crate) ops: Vec<Op>,
    pub(crate) undo: Vec<Undo>,
    pub(crate) peak: Sizes,
    pub(crate) auto_nodes: usize,
}

impl<'a> Session<'a> {
    pub fn start(
        db: &'a Database,
        grammar: &'a Grammar<'a>,
        theorem: StmtId,
    ) -> Result<Self, Error> {
        Self::start_in(Catalog::new(db, grammar), theorem)
    }

    /// Starts a session that shares the parsed assertions of `catalog` with others.
    ///
    /// A theorem without a usable proof has no scope to read disjoint pairs from, so the session may
    /// add the pairs a dummy variable needs; any other theorem keeps to the pairs it declares.
    pub fn start_in(catalog: Rc<Catalog<'a>>, theorem: StmtId) -> Result<Self, Error> {
        let db = catalog.db;
        let stmt = db.stmts.get(theorem as usize).ok_or(Error::NotATheorem)?;
        if stmt.kind != Kind::Theorem {
            return Err(Error::NotATheorem);
        }
        let parsed = catalog.parsed(theorem).map_err(Error::Syntax)?;
        let (scope_dv, open_dv) = match &stmt.proof {
            Some(Ok(Proof { scope_dv, .. })) => (scope_dv.clone(), false),
            Some(Err(ProofError::NotATheorem)) | None => return Err(Error::NotATheorem),
            Some(Err(_)) => (stmt.frame.dv.clone(), true),
        };
        let vars = Vars::new(db, theorem);
        let store = Store::new(db, vars.types());
        let mut session = Session {
            cat: catalog,
            theorem,
            provable: stmt.expr[0],
            open_dv,
            vars,
            store,
            hyps: Vec::new(),
            goals: Vec::new(),
            dvs: Vec::new(),
            scope_dv,
            ops: Vec::new(),
            undo: Vec::new(),
            peak: Sizes::default(),
            auto_nodes: 0,
        };
        let essentials = stmt
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Essential);
        for (&hyp, tree) in essentials.zip(&parsed.hyps) {
            let term = session.build(tree, &[]);
            session.hyps.push((hyp, term));
        }
        let term = session.build(&parsed.conclusion, &[]);
        session.goals.push(Goal {
            term,
            state: GoalState::Open,
        });
        session.note_peak();
        Ok(session)
    }

    /// Lets the session add the disjoint pairs a dummy variable needs, or stops it.
    pub fn allow_new_disjoint(&mut self, allow: bool) {
        self.open_dv = allow;
    }

    pub fn db(&self) -> &'a Database {
        self.cat.db
    }

    pub fn theorem(&self) -> StmtId {
        self.theorem
    }

    /// The theorem's essential hypotheses with the terms they stand for.
    pub fn hypotheses(&self) -> &[(StmtId, TermId)] {
        &self.hyps
    }

    pub fn vars(&self) -> &Vars {
        &self.vars
    }

    pub fn store(&self) -> &Store<'a> {
        &self.store
    }

    pub fn goal(&self, goal: GoalId) -> Option<&Goal> {
        self.goals.get(goal as usize)
    }

    pub fn goal_count(&self) -> usize {
        self.goals.len()
    }

    /// The goals an applied goal produced, none for a goal that is not applied or does not exist.
    /// Goal ids are invalidated by `undo`.
    pub fn children(&self, goal: GoalId) -> Range<GoalId> {
        match self.goals.get(goal as usize).map(|g| g.state) {
            Some(GoalState::Applied {
                assertion, first, ..
            }) => first..first + self.essential_count(assertion) as GoalId,
            _ => 0..0,
        }
    }

    pub fn show(&self, term: TermId) -> Option<String> {
        self.store.show(term)
    }

    /// The goal as a provable statement in Metamath notation.
    pub fn show_goal(&self, goal: GoalId) -> Option<String> {
        let name = &self.db().syms[self.provable as usize].name;
        let term = self.goals.get(goal as usize)?.term;
        Some(format!("{name} {}", self.store.show(term)?))
    }

    /// The operations in force, oldest first.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn sizes(&self) -> Sizes {
        Sizes {
            terms: self.store.len(),
            goals: self.goals.len(),
            conditions: self.dvs.len(),
            ops: self.ops.len(),
        }
    }

    /// The high-water marks since the session started, rolled-back attempts included.
    pub fn peak(&self) -> Sizes {
        let mut peak = self.peak;
        peak.terms = self.store.peak();
        peak
    }

    pub fn status(&self) -> Status {
        let report = self.dv_report();
        let mut open = Vec::new();
        let mut pending = vec![0 as GoalId];
        while let Some(goal) = pending.pop() {
            match self.goals[goal as usize].state {
                GoalState::Open => open.push(goal),
                GoalState::Applied { .. } => pending.extend(self.children(goal).rev()),
                GoalState::Closed { .. } => {}
            }
        }
        Status {
            open,
            metas: self.store.unbound().collect(),
            violations: report.violations,
            added_dv: report.added,
        }
    }

    pub(crate) fn essential_count(&self, assertion: StmtId) -> usize {
        essential_count(self.db(), assertion)
    }

    /// Builds a term from a syntax tree; a variable in `subst` becomes its term and any other stays a variable.
    pub(crate) fn build(&mut self, tree: &Tree, subst: &[(Sym, TermId)]) -> TermId {
        let stmt = self.db().stmt(tree.stmt);
        if stmt.kind == Kind::Float {
            let var = stmt.expr[1];
            return match subst.iter().find(|p| p.0 == var) {
                Some(&(_, term)) => term,
                None => self.store.var(var),
            };
        }
        let kids: Vec<TermId> = tree.children.iter().map(|c| self.build(c, subst)).collect();
        self.store.app(tree.stmt, &kids)
    }

    pub(crate) fn note_peak(&mut self) {
        self.peak.goals = self.peak.goals.max(self.goals.len());
        self.peak.conditions = self.peak.conditions.max(self.dvs.len());
        self.peak.ops = self.peak.ops.max(self.ops.len());
    }

    /// Back to the state before the last operation.
    pub fn undo(&mut self) -> Result<Op, Error> {
        let undo = self.undo.pop().ok_or(Error::NothingToUndo)?;
        let op = self.ops.pop().ok_or(Error::NothingToUndo)?;
        self.rollback_to(undo);
        Ok(op)
    }

    pub(crate) fn rollback_to(&mut self, undo: Undo) {
        self.store.rollback(undo.mark);
        self.goals.truncate(undo.goals as usize);
        self.dvs.truncate(undo.conditions as usize);
        if let Some(goal) = undo.reopened {
            self.goals[goal as usize].state = GoalState::Open;
        }
    }
}

fn essential_count(db: &Database, assertion: StmtId) -> usize {
    db.stmt(assertion)
        .frame
        .hyps
        .iter()
        .filter(|&&h| db.stmt(h).kind == Kind::Essential)
        .count()
}
