use super::{Database, Kind, Proof, ProofBody, ProofError, StmtId, Sym, dv_pair};

/// High-water marks of one proof check, in the units a fixed-memory kernel has to budget.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub steps: usize,
    pub stack_depth: usize,
    pub stack_syms: usize,
    pub saved_syms: usize,
}

/// Checks one theorem at a time against a parsed database, reusing its buffers between proofs.
pub struct Verifier<'a> {
    db: &'a Database,
    arena: Vec<Sym>,
    starts: Vec<usize>,
    saved: Vec<Vec<Sym>>,
    subst: Vec<(usize, usize)>,
    scratch: Vec<Sym>,
    usage: Usage,
}

impl<'a> Verifier<'a> {
    pub fn new(db: &'a Database) -> Self {
        Verifier {
            db,
            arena: Vec::new(),
            starts: Vec::new(),
            saved: Vec::new(),
            subst: vec![(0, 0); db.syms.len()],
            scratch: Vec::new(),
            usage: Usage::default(),
        }
    }

    /// Measurements of the most recent `verify` call, up to the point where it stopped.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    fn pushed(&mut self) {
        self.usage.steps += 1;
        self.usage.stack_depth = self.usage.stack_depth.max(self.starts.len());
        self.usage.stack_syms = self.usage.stack_syms.max(self.arena.len());
    }

    pub fn verify(&mut self, id: StmtId) -> Result<(), ProofError> {
        let db = self.db;
        let stmt = db.stmt(id);
        self.usage = Usage::default();
        let proof = match &stmt.proof {
            Some(Ok(proof)) => proof,
            Some(Err(e)) => return Err(*e),
            None => return Err(ProofError::NotATheorem),
        };
        self.arena.clear();
        self.starts.clear();
        self.saved.clear();
        match &proof.body {
            ProofBody::Normal(steps) => {
                for &step in steps {
                    self.apply(step, proof)?;
                }
            }
            ProofBody::Compressed { labels, letters } => {
                self.run_compressed(&stmt.frame.hyps, labels, letters, proof)?;
            }
        }
        if self.starts.len() != 1 {
            return Err(ProofError::StackNotSingleton);
        }
        if self.arena != stmt.expr {
            return Err(ProofError::WrongConclusion);
        }
        Ok(())
    }

    fn run_compressed(
        &mut self,
        hyps: &[StmtId],
        labels: &[StmtId],
        body: &[u8],
        proof: &Proof,
    ) -> Result<(), ProofError> {
        let mut high: usize = 0;
        for &c in body {
            match c {
                b'A'..=b'T' => {
                    let index = high
                        .checked_mul(20)
                        .and_then(|n| n.checked_add((c - b'A') as usize))
                        .ok_or(ProofError::MalformedCompressed)?;
                    high = 0;
                    if index < hyps.len() {
                        self.apply(hyps[index], proof)?;
                    } else if index - hyps.len() < labels.len() {
                        self.apply(labels[index - hyps.len()], proof)?;
                    } else {
                        let saved = self
                            .saved
                            .get(index - hyps.len() - labels.len())
                            .ok_or(ProofError::BadBackReference)?;
                        self.starts.push(self.arena.len());
                        self.arena.extend_from_slice(saved);
                        self.pushed();
                    }
                }
                b'U'..=b'Y' => {
                    high = high
                        .checked_mul(5)
                        .and_then(|n| n.checked_add((c - b'U') as usize + 1))
                        .ok_or(ProofError::MalformedCompressed)?;
                }
                b'Z' => {
                    let Some(&top) = self.starts.last() else {
                        return Err(ProofError::MalformedCompressed);
                    };
                    if high != 0 {
                        return Err(ProofError::MalformedCompressed);
                    }
                    self.usage.saved_syms += self.arena.len() - top;
                    self.saved.push(self.arena[top..].to_vec());
                }
                b'?' => return Err(ProofError::Incomplete),
                _ => return Err(ProofError::MalformedCompressed),
            }
        }
        if high != 0 {
            return Err(ProofError::MalformedCompressed);
        }
        Ok(())
    }

    fn apply(&mut self, id: StmtId, proof: &Proof) -> Result<(), ProofError> {
        let db = self.db;
        let stmt = db.stmt(id);
        if matches!(stmt.kind, Kind::Float | Kind::Essential) {
            self.starts.push(self.arena.len());
            self.arena.extend_from_slice(&stmt.expr);
            self.pushed();
            return Ok(());
        }
        let hyps = &stmt.frame.hyps;
        if self.starts.len() < hyps.len() {
            return Err(ProofError::StackUnderflow);
        }
        let base = self.starts.len() - hyps.len();
        for (i, &hyp) in hyps.iter().enumerate() {
            let start = self.starts[base + i];
            let end = self
                .starts
                .get(base + i + 1)
                .copied()
                .unwrap_or(self.arena.len());
            let hyp = db.stmt(hyp);
            if hyp.kind == Kind::Float {
                if self.arena[start] != hyp.expr[0] {
                    return Err(ProofError::TypecodeMismatch);
                }
                self.subst[hyp.expr[1] as usize] = (start + 1, end);
            } else {
                self.substitute(&hyp.expr);
                if self.scratch != self.arena[start..end] {
                    return Err(ProofError::HypothesisMismatch);
                }
            }
        }
        for &(x, y) in &stmt.frame.dv {
            let (xs, xe) = self.subst[x as usize];
            let (ys, ye) = self.subst[y as usize];
            for &a in &self.arena[xs..xe] {
                if !db.syms[a as usize].is_var {
                    continue;
                }
                for &b in &self.arena[ys..ye] {
                    if !db.syms[b as usize].is_var {
                        continue;
                    }
                    if a == b || proof.scope_dv.binary_search(&dv_pair(a, b)).is_err() {
                        return Err(ProofError::DisjointViolation);
                    }
                }
            }
        }
        self.substitute(&stmt.expr);
        let bottom = self.starts.get(base).copied().unwrap_or(self.arena.len());
        self.starts.truncate(base);
        self.arena.truncate(bottom);
        self.starts.push(bottom);
        self.arena.extend_from_slice(&self.scratch);
        self.pushed();
        Ok(())
    }

    fn substitute(&mut self, expr: &[Sym]) {
        self.scratch.clear();
        for &s in expr {
            if self.db.syms[s as usize].is_var {
                let (start, end) = self.subst[s as usize];
                self.scratch.extend_from_slice(&self.arena[start..end]);
            } else {
                self.scratch.push(s);
            }
        }
        let held = self.arena.len() + self.scratch.len();
        self.usage.stack_syms = self.usage.stack_syms.max(held);
    }
}
