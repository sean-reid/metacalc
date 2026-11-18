use crate::error::Error;
use crate::export::Step;
use crate::session::Session;
use crate::term::{Head, TermId};
use mcpack::mm::{Kind, StmtId, Sym};
use std::collections::HashMap;
use std::rc::Rc;

/// A disjoint pair of an applied assertion with the terms substituted for its two variables.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DvCond {
    pub assertion: StmtId,
    pub a: TermId,
    pub b: TermId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViolationKind {
    /// A variable occurs in both terms.
    Shared,
    /// The pair is not declared disjoint for the theorem.
    Undeclared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

/// What was broken before an operation: stored conditions by position, and one entry per built
/// condition violation, repeats included.
pub(crate) struct Broken {
    stored: Vec<bool>,
    built: HashMap<Violation, usize>,
}

/// What a condition check needs of a term: whether it is free of metavariables and its variables.
pub(crate) struct Info {
    ground: bool,
    vars: Vec<Sym>,
}

pub(crate) type Cache = HashMap<TermId, Rc<Info>>;

pub(crate) fn pair(a: Sym, b: Sym) -> (Sym, Sym) {
    if a < b { (a, b) } else { (b, a) }
}

impl Session<'_> {
    /// The disjoint pairs of every syntax axiom inside a term the exported proof builds, one set
    /// per occurrence; none when no syntax axiom declares a pair.
    fn built_conditions(&self) -> Result<Vec<DvCond>, Error> {
        let mut found = Vec::new();
        if !self.cat.syntax_dv() {
            return Ok(found);
        }
        self.walks.set(self.walks.get() + 1);
        let mut seen = HashMap::new();
        self.walk_proof(0, &mut |step| {
            if let Step::Value(term) = step {
                self.syntax_conditions(term, &mut found, &mut seen);
            }
            Ok(())
        })?;
        Ok(found)
    }

    /// Adds the conditions under `term`; a term already visited repeats what it produced.
    fn syntax_conditions(
        &self,
        term: TermId,
        out: &mut Vec<DvCond>,
        seen: &mut HashMap<TermId, (usize, usize)>,
    ) {
        let term = self.store.walk(term);
        let Some(Head::App(syntax)) = self.store.head(term) else {
            return;
        };
        if let Some(&(from, to)) = seen.get(&term) {
            out.extend_from_within(from..to);
            return;
        }
        self.visits.set(self.visits.get() + 1);
        let start = out.len();
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
            self.syntax_conditions(kid, out, seen);
        }
        seen.insert(term, (start, out.len()));
    }

    /// Whether `term` is free of metavariables and which variables it holds, computed once per
    /// term in a cache.
    fn info(&self, term: TermId, cache: &mut Cache) -> Rc<Info> {
        let term = self.store.walk(term);
        if let Some(found) = cache.get(&term) {
            return found.clone();
        }
        self.work.set(self.work.get() + 1);
        let info = match self.store.head(term) {
            Some(Head::Var(v)) => Info {
                ground: true,
                vars: vec![v],
            },
            Some(Head::App(_)) => {
                let mut info = Info {
                    ground: true,
                    vars: Vec::new(),
                };
                for &kid in self.store.children(term) {
                    let kid = self.info(kid, cache);
                    info.ground &= kid.ground;
                    self.work.set(self.work.get() + kid.vars.len());
                    for &v in &kid.vars {
                        if !info.vars.contains(&v) {
                            info.vars.push(v);
                        }
                    }
                }
                info
            }
            _ => Info {
                ground: false,
                vars: Vec::new(),
            },
        };
        let info = Rc::new(info);
        cache.insert(term, info.clone());
        info
    }

    /// Checks every condition whose two terms are free of metavariables.
    pub(crate) fn dv_report(&self) -> Result<Report, Error> {
        let built = self.built_conditions()?;
        Ok(self.report_of(self.dvs.iter().chain(&built)))
    }

    /// The report for the stored conditions alone.
    pub(crate) fn stored_report(&self) -> Report {
        self.report_of(self.dvs.iter())
    }

    fn report_of<'c>(&self, conds: impl Iterator<Item = &'c DvCond>) -> Report {
        let mut report = Report::default();
        let mut cache = Cache::new();
        for cond in conds {
            self.check_condition(cond, &mut cache, &mut report);
        }
        for list in [&mut report.added, &mut report.used] {
            list.sort_unstable();
            list.dedup();
        }
        report
    }

    /// The conditions that are broken now: which stored ones, and the violations of the built ones.
    pub(crate) fn broken(&self) -> Result<Broken, Error> {
        let mut cache = Cache::new();
        let mut one = |cond: &DvCond| {
            let mut one = Report::default();
            self.check_condition(cond, &mut cache, &mut one);
            one.violations
        };
        let stored = self.dvs.iter().map(|c| !one(c).is_empty()).collect();
        let mut built = HashMap::new();
        for cond in &self.built_conditions()? {
            for v in one(cond) {
                *built.entry(v).or_insert(0) += 1;
            }
        }
        Ok(Broken { stored, built })
    }

    /// The violations of the conditions that are broken now and were not in `before`. A built
    /// violation is new when it occurs more often now than before: it has no identity beyond its
    /// pair of variables, and a second copy elsewhere in the proof is a new breach.
    pub(crate) fn newly_broken(&self, before: &Broken) -> Result<Vec<Violation>, Error> {
        let mut found = Vec::new();
        let mut cache = Cache::new();
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
                self.check_condition(cond, &mut cache, &mut one);
                add(one.violations);
            }
        }
        let mut unmatched = before.built.clone();
        let mut fresh = Vec::new();
        for cond in &self.built_conditions()? {
            let mut one = Report::default();
            self.check_condition(cond, &mut cache, &mut one);
            for v in one.violations {
                match unmatched.get_mut(&v) {
                    Some(left) if *left > 0 => *left -= 1,
                    _ => fresh.push(v),
                }
            }
        }
        add(fresh);
        Ok(found)
    }

    fn check_condition(&self, cond: &DvCond, cache: &mut Cache, report: &mut Report) {
        let (xs, ys) = (self.info(cond.a, cache), self.info(cond.b, cache));
        if !xs.ground || !ys.ground {
            return;
        }
        self.work
            .set(self.work.get() + 1 + xs.vars.len() * ys.vars.len());
        for &x in &xs.vars {
            for &y in &ys.vars {
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
