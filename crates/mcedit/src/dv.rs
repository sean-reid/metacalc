use crate::session::Session;
use crate::term::TermId;
use mcpack::mm::{StmtId, Sym};

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

pub(crate) fn pair(a: Sym, b: Sym) -> (Sym, Sym) {
    if a < b { (a, b) } else { (b, a) }
}

impl Session<'_> {
    /// Checks every condition whose two terms are free of metavariables.
    pub(crate) fn dv_report(&self) -> Report {
        let mut report = Report {
            violations: Vec::new(),
            added: Vec::new(),
            used: Vec::new(),
        };
        for cond in &self.dvs {
            self.check_condition(cond, &mut report);
        }
        for list in [&mut report.added, &mut report.used] {
            list.sort_unstable();
            list.dedup();
        }
        report
    }

    /// Whether each condition is broken now, in condition order.
    pub(crate) fn broken(&self) -> Vec<bool> {
        self.dvs
            .iter()
            .map(|cond| {
                let mut one = Report::default();
                self.check_condition(cond, &mut one);
                !one.violations.is_empty()
            })
            .collect()
    }

    /// The violations of the conditions that are broken now and were not in `before`.
    pub(crate) fn newly_broken(&self, before: &[bool]) -> Vec<Violation> {
        let mut found = Vec::new();
        for (at, cond) in self.dvs.iter().enumerate() {
            if before.get(at) == Some(&true) {
                continue;
            }
            let mut one = Report::default();
            self.check_condition(cond, &mut one);
            for v in one.violations {
                if !found.contains(&v) {
                    found.push(v);
                }
            }
        }
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
