use crate::export::Step;
use crate::session::Session;
use crate::term::{Head, TermId};
use mcpack::mm::{Kind, StmtId, Sym};

/// A disjoint pair of an applied assertion with the terms substituted for its two variables.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DvCond {
    pub assertion: StmtId,
    pub a: TermId,
    pub b: TermId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolationKind {
    /// A variable occurs in both terms.
    Shared,
    /// The pair is not declared disjoint for the theorem.
    Undeclared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Violation {
    pub assertion: StmtId,
    /// The offending variables, lower symbol first.
    pub x: Sym,
    pub y: Sym,
    pub kind: ViolationKind,
}

#[derive(Default)]
pub(crate) struct Report {
    pub violations: Vec<Violation>,
    /// Pairs involving a dummy variable that the proof needs and the theorem does not declare.
    pub added: Vec<(Sym, Sym)>,
    /// Every pair of variables a ground condition relates, sorted.
    pub used: Vec<(Sym, Sym)>,
}

/// What was broken before an operation: stored conditions by position, built ones by violation.
pub(crate) struct Broken {
    stored: Vec<bool>,
    built: Vec<Violation>,
}

pub(crate) fn pair(a: Sym, b: Sym) -> (Sym, Sym) {
    if a < b { (a, b) } else { (b, a) }
}

impl Session<'_> {
    /// The disjoint pairs of every syntax axiom inside a term the exported proof builds.
    fn built_conditions(&self) -> Vec<DvCond> {
        let mut found = Vec::new();
        let _ = self.walk_proof(0, &mut |step| {
            if let Step::Value(term) = step {
                self.syntax_conditions(term, &mut found);
            }
            Ok(())
        });
        found
    }

    fn syntax_conditions(&self, term: TermId, out: &mut Vec<DvCond>) {
        let Some(Head::App(syntax)) = self.store.head(term) else {
            return;
        };
        let db = self.db();
        let frame = &db.stmt(syntax).frame;
        let kids = self.store.children(term);
        let kid_of = |v: Sym| {
            let floats = frame
                .hyps
                .iter()
                .filter(|&&h| db.stmt(h).kind == Kind::Float);
            let at = floats.map(|&h| db.stmt(h).expr[1]).position(|x| x == v)?;
            kids.get(at).copied()
        };
        for &(x, y) in &frame.dv {
            if let (Some(a), Some(b)) = (kid_of(x), kid_of(y)) {
                out.push(DvCond {
                    assertion: syntax,
                    a,
                    b,
                });
            }
        }
        for &kid in kids {
            self.syntax_conditions(kid, out);
        }
    }

    /// Checks every condition whose two terms are free of metavariables.
    pub(crate) fn dv_report(&self) -> Report {
        let mut report = Report {
            violations: Vec::new(),
            added: Vec::new(),
            used: Vec::new(),
        };
        for cond in self.dvs.iter().chain(&self.built_conditions()) {
            self.check_condition(cond, &mut report);
        }
        for list in [&mut report.added, &mut report.used] {
            list.sort_unstable();
            list.dedup();
        }
        report
    }

    /// Whether each condition is broken now.
    pub(crate) fn broken(&self) -> Broken {
        let one = |cond: &DvCond| {
            let mut one = Report::default();
            self.check_condition(cond, &mut one);
            one.violations
        };
        Broken {
            stored: self.dvs.iter().map(|c| !one(c).is_empty()).collect(),
            built: self.built_conditions().iter().flat_map(one).collect(),
        }
    }

    /// The violations of the conditions that are broken now and were not in `before`.
    pub(crate) fn newly_broken(&self, before: &Broken) -> Vec<Violation> {
        let mut found = Vec::new();
        let mut add = |violations: Vec<Violation>| {
            for v in violations {
                if !found.contains(&v) {
                    found.push(v);
                }
            }
        };
        for (at, cond) in self.dvs.iter().enumerate() {
            if before.stored.get(at) != Some(&true) {
                let mut one = Report::default();
                self.check_condition(cond, &mut one);
                add(one.violations);
            }
        }
        let mut now = Report::default();
        for cond in &self.built_conditions() {
            self.check_condition(cond, &mut now);
        }
        add(now
            .violations
            .into_iter()
            .filter(|v| !before.built.contains(v))
            .collect());
        found
    }

    fn check_condition(&self, cond: &DvCond, report: &mut Report) {
        if !self.store.is_ground(cond.a) || !self.store.is_ground(cond.b) {
            return;
        }
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        self.store.variables(cond.a, &mut xs);
        self.store.variables(cond.b, &mut ys);
        for &x in &xs {
            for &y in &ys {
                let (lo, hi) = pair(x, y);
                let mut fail = |kind| {
                    let v = Violation {
                        assertion: cond.assertion,
                        x: lo,
                        y: hi,
                        kind,
                    };
                    if !report.violations.contains(&v) {
                        report.violations.push(v);
                    }
                };
                if x == y {
                    fail(ViolationKind::Shared);
                    continue;
                }
                report.used.push((lo, hi));
                if self.scope_dv.binary_search(&(lo, hi)).is_ok() {
                    continue;
                }
                let dummy = !self.vars.is_mandatory(x) || !self.vars.is_mandatory(y);
                if self.open_dv && dummy {
                    report.added.push((lo, hi));
                } else {
                    fail(ViolationKind::Undeclared);
                }
            }
        }
    }
}
