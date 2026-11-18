//! Size and memory measurements of a dependency closure.

use crate::mm::{Database, Kind, ProofBody, ProofError, StmtId, Usage, Verifier};
use std::fmt;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Segment {
    pub name: String,
    pub axioms: usize,
    pub theorems: usize,
    pub floats: usize,
    pub essentials: usize,
    pub statement_syms: usize,
    pub proof_refs: usize,
    pub proof_len: usize,
    pub steps: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Peak {
    pub value: usize,
    pub label: String,
}

impl Peak {
    fn raise(&mut self, value: usize, label: &str) {
        if value > self.value {
            self.value = value;
            self.label = label.to_string();
        }
    }
}

#[derive(Debug, Default)]
pub struct Measure {
    pub segments: Vec<Segment>,
    pub constants: usize,
    pub variables: usize,
    pub longest_statement: Peak,
    pub longest_proof: Peak,
    pub most_steps: Peak,
    pub deepest_stack: Peak,
    pub largest_stack: Peak,
    pub largest_saved: Peak,
    pub failures: Vec<(String, ProofError)>,
}

/// Measures `closure`, a list of statements in database order, checking every proof in it.
///
/// Each id in `splits` starts a new segment, so the rows separate the stages of a database.
pub fn measure(db: &Database, closure: &[StmtId], splits: &[StmtId]) -> Measure {
    let mut splits = splits.to_vec();
    splits.sort_unstable();
    splits.dedup();
    let mut segments = vec![Segment {
        name: "start".into(),
        ..Segment::default()
    }];
    segments.extend(splits.iter().map(|&id| Segment {
        name: db.stmt(id).label.clone(),
        ..Segment::default()
    }));

    let mut out = Measure::default();
    let mut used = vec![false; db.syms.len()];
    let mut verifier = Verifier::new(db);
    for &id in closure {
        let stmt = db.stmt(id);
        let segment = &mut segments[splits.partition_point(|&split| split <= id)];
        match stmt.kind {
            Kind::Float => segment.floats += 1,
            Kind::Essential => segment.essentials += 1,
            Kind::Axiom => segment.axioms += 1,
            Kind::Theorem => segment.theorems += 1,
        }
        segment.statement_syms += stmt.expr.len();
        out.longest_statement.raise(stmt.expr.len(), &stmt.label);
        for &sym in &stmt.expr {
            used[sym as usize] = true;
        }
        if stmt.kind != Kind::Theorem {
            continue;
        }
        if let Some(Ok(proof)) = &stmt.proof {
            let (refs, len) = match &proof.body {
                ProofBody::Normal(steps) => (0, steps.len()),
                ProofBody::Compressed { labels, letters } => (labels.len(), letters.len()),
            };
            segment.proof_refs += refs;
            segment.proof_len += len;
            out.longest_proof.raise(len, &stmt.label);
        }
        if let Err(e) = verifier.verify(id) {
            out.failures.push((stmt.label.clone(), e));
        }
        let Usage {
            steps,
            stack_depth,
            stack_syms,
            saved_syms,
        } = verifier.usage();
        segment.steps += steps;
        out.most_steps.raise(steps, &stmt.label);
        out.deepest_stack.raise(stack_depth, &stmt.label);
        out.largest_stack.raise(stack_syms, &stmt.label);
        out.largest_saved.raise(saved_syms, &stmt.label);
    }
    for (sym, _) in used.iter().enumerate().filter(|&(_, &used)| used) {
        if db.syms[sym].is_var {
            out.variables += 1;
        } else {
            out.constants += 1;
        }
    }
    out.segments = segments;
    out
}

impl Measure {
    pub fn total(&self) -> Segment {
        let mut total = Segment {
            name: "total".into(),
            ..Segment::default()
        };
        for s in &self.segments {
            total.axioms += s.axioms;
            total.theorems += s.theorems;
            total.floats += s.floats;
            total.essentials += s.essentials;
            total.statement_syms += s.statement_syms;
            total.proof_refs += s.proof_refs;
            total.proof_len += s.proof_len;
            total.steps += s.steps;
        }
        total
    }
}

impl fmt::Display for Measure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self
            .segments
            .iter()
            .map(|s| s.name.len())
            .max()
            .unwrap_or(0)
            .max(5);
        writeln!(
            f,
            "{:<width$} {:>7} {:>8} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "from",
            "axioms",
            "theorems",
            "floats",
            "essentials",
            "stmt syms",
            "proof refs",
            "proof len",
            "steps"
        )?;
        let total = self.total();
        let rows = self
            .segments
            .iter()
            .chain((self.segments.len() > 1).then_some(&total));
        for s in rows {
            writeln!(
                f,
                "{:<width$} {:>7} {:>8} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10}",
                s.name,
                s.axioms,
                s.theorems,
                s.floats,
                s.essentials,
                s.statement_syms,
                s.proof_refs,
                s.proof_len,
                s.steps
            )?;
        }
        writeln!(
            f,
            "symbols: {} constants, {} variables",
            self.constants, self.variables
        )?;
        for (name, peak) in [
            ("longest statement, symbols", &self.longest_statement),
            ("longest proof", &self.longest_proof),
            ("most proof steps", &self.most_steps),
            ("deepest stack, entries", &self.deepest_stack),
            ("largest stack, symbols", &self.largest_stack),
            ("largest saved set, symbols", &self.largest_saved),
        ] {
            writeln!(f, "{name}: {} ({})", peak.value, peak.label)?;
        }
        Ok(())
    }
}
