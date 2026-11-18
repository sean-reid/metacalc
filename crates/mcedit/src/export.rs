use crate::error::Error;
use crate::session::{GoalId, GoalState, Session};
use crate::term::{Head, TermId};
use crate::vars::active_float;
use mcpack::mm::{Database, Kind, StmtId, Sym};

const WIDTH: usize = 79;

/// A step of an uncompressed proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Stmt(StmtId),
    /// The floating hypothesis of the dummy variable at this index of `Finished::dummies`.
    Dummy(usize),
}

/// A variable the proof uses that is not in the theorem's statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dummy {
    pub var: Sym,
    pub typecode: Sym,
    /// The label of its floating hypothesis.
    pub label: String,
    /// The floating hypothesis is new: no active one exists at the theorem.
    pub declared: bool,
}

/// A finished proof, ready to be written as Metamath.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    pub theorem: StmtId,
    pub proof: Vec<Label>,
    pub dummies: Vec<Dummy>,
    /// Every disjoint pair the theorem's block needs, sorted.
    pub dv: Vec<(Sym, Sym)>,
    /// The pairs among them the theorem does not already declare.
    pub added_dv: Vec<(Sym, Sym)>,
}

impl Session<'_> {
    /// The proof as an uncompressed label sequence, when nothing is open, unresolved or violated.
    pub fn export(&self) -> Result<Finished, Error> {
        self.check_terms()?;
        let status = self.status();
        if !(status.open.is_empty() && status.metas.is_empty() && status.violations.is_empty()) {
            return Err(Error::Unfinished {
                open: status.open.len(),
                metas: status.metas.len(),
                violations: status.violations.len(),
            });
        }
        let db = self.db();
        let mut proof = Vec::new();
        let mut dummies: Vec<Dummy> = Vec::new();
        self.emit_goal(0, &mut proof, &mut dummies)?;
        let mut taken: Vec<String> = Vec::new();
        for dummy in &mut dummies {
            match active_float(db, dummy.var, self.theorem) {
                Some(float) => dummy.label = db.stmt(float).label.clone(),
                None => {
                    let base = &db.stmt(self.theorem).label;
                    let mut n = 1;
                    let label = loop {
                        let label = format!("{base}.d{n}");
                        if db.find(&label).is_none() && !taken.contains(&label) {
                            break label;
                        }
                        n += 1;
                    };
                    taken.push(label.clone());
                    dummy.label = label;
                    dummy.declared = true;
                }
            }
        }
        let report = self.dv_report();
        let mut dv = db.stmt(self.theorem).frame.dv.clone();
        dv.extend(report.used);
        dv.sort_unstable();
        dv.dedup();
        Ok(Finished {
            theorem: self.theorem,
            proof,
            dummies,
            dv,
            added_dv: report.added,
        })
    }

    /// Every term the session holds must be one the store knows.
    fn check_terms(&self) -> Result<(), Error> {
        let known = |t| self.store.head(t).is_some();
        let goals = self.goals.iter().map(|g| g.term);
        let conditions = self.dvs.iter().flat_map(|c| [c.a, c.b]);
        match goals.chain(conditions).all(known) {
            true => Ok(()),
            false => Err(Error::Corrupt),
        }
    }

    fn emit_goal(
        &self,
        goal: GoalId,
        proof: &mut Vec<Label>,
        dummies: &mut Vec<Dummy>,
    ) -> Result<(), Error> {
        let db = self.db();
        match self.goals.get(goal as usize).ok_or(Error::Corrupt)?.state {
            GoalState::Closed { hyp } => proof.push(Label::Stmt(hyp)),
            GoalState::Applied {
                assertion,
                metas,
                first,
            } => {
                let (mut float, mut essential) = (0, 0);
                for &h in &db.stmt(assertion).frame.hyps {
                    if db.stmt(h).kind == Kind::Float {
                        let term = self.store.meta_term(metas + float).ok_or(Error::Corrupt)?;
                        self.emit_term(term, proof, dummies)?;
                        float += 1;
                    } else {
                        self.emit_goal(first + essential, proof, dummies)?;
                        essential += 1;
                    }
                }
                proof.push(Label::Stmt(assertion));
            }
            GoalState::Open => return Err(Error::Corrupt),
        }
        Ok(())
    }

    fn emit_term(
        &self,
        term: TermId,
        proof: &mut Vec<Label>,
        dummies: &mut Vec<Dummy>,
    ) -> Result<(), Error> {
        match self.store.head(term) {
            Some(Head::Var(var)) => match self.vars.mandatory_float(var) {
                Some(float) => proof.push(Label::Stmt(float)),
                None => {
                    let at = match dummies.iter().position(|d| d.var == var) {
                        Some(at) => at,
                        None => {
                            dummies.push(Dummy {
                                var,
                                typecode: self.vars.typecode(var).ok_or(Error::Corrupt)?,
                                label: String::new(),
                                declared: false,
                            });
                            dummies.len() - 1
                        }
                    };
                    proof.push(Label::Dummy(at));
                }
            },
            Some(Head::App(syntax)) => {
                for &kid in self.store.children(term) {
                    self.emit_term(kid, proof, dummies)?;
                }
                proof.push(Label::Stmt(syntax));
            }
            _ => return Err(Error::Corrupt),
        }
        Ok(())
    }
}

fn wrap(out: &mut String, indent: usize, tokens: &[String]) {
    let mut used = 0;
    for token in tokens {
        if used == 0 {
            out.push_str(&" ".repeat(indent));
            used = indent;
        } else if used + 1 + token.len() > WIDTH {
            out.push('\n');
            out.push_str(&" ".repeat(indent + 2));
            used = indent + 2;
        } else {
            out.push(' ');
            used += 1;
        }
        out.push_str(token);
        used += token.len();
    }
    out.push('\n');
}

impl Finished {
    fn name<'a>(&'a self, db: &'a Database, label: Label) -> &'a str {
        match label {
            Label::Stmt(id) => &db.stmt(id).label,
            Label::Dummy(i) => &self.dummies[i].label,
        }
    }

    fn sym(db: &Database, sym: Sym) -> String {
        db.syms[sym as usize].name.clone()
    }

    fn disjoint(&self, db: &Database, pairs: &[(Sym, Sym)], out: &mut String) {
        for &(a, b) in pairs {
            let tokens = [
                "$d".to_string(),
                Self::sym(db, a),
                Self::sym(db, b),
                "$.".into(),
            ];
            wrap(out, 2, &tokens);
        }
    }

    fn float(&self, db: &Database, dummy: &Dummy, out: &mut String) {
        let tokens = [
            dummy.label.clone(),
            "$f".into(),
            Self::sym(db, dummy.typecode),
            Self::sym(db, dummy.var),
            "$.".into(),
        ];
        wrap(out, 2, &tokens);
    }

    /// The `$p` statement with this proof.
    fn assertion(&self, db: &Database, out: &mut String) {
        let stmt = db.stmt(self.theorem);
        let mut tokens = vec![stmt.label.clone(), "$p".into()];
        tokens.extend(stmt.expr.iter().map(|&s| Self::sym(db, s)));
        tokens.push("$=".into());
        tokens.extend(self.proof.iter().map(|&l| self.name(db, l).to_string()));
        tokens.push("$.".into());
        wrap(out, 2, &tokens);
    }

    /// The statement to put where the theorem stood in its database. When the proof needs a new
    /// dummy hypothesis or a disjoint pair, a block holds those next to the statement.
    pub fn statement(&self, db: &Database) -> String {
        let new: Vec<&Dummy> = self.dummies.iter().filter(|d| d.declared).collect();
        let mut out = String::new();
        let wrapped = !new.is_empty() || !self.added_dv.is_empty();
        if wrapped {
            out.push_str("${\n");
            self.disjoint(db, &self.added_dv, &mut out);
            new.iter().for_each(|d| self.float(db, d, &mut out));
        }
        self.assertion(db, &mut out);
        if wrapped {
            out.push_str("$}\n");
        }
        out
    }

    /// The theorem as a block of its own: its disjoint pairs, hypotheses and dummy hypotheses,
    /// then the statement.
    pub fn text(&self, db: &Database) -> String {
        let mut out = String::from("${\n");
        self.disjoint(db, &self.dv, &mut out);
        for &h in &db.stmt(self.theorem).frame.hyps {
            let hyp = db.stmt(h);
            let mut tokens = vec![hyp.label.clone()];
            tokens.push(if hyp.kind == Kind::Float { "$f" } else { "$e" }.into());
            tokens.extend(hyp.expr.iter().map(|&s| Self::sym(db, s)));
            tokens.push("$.".into());
            wrap(&mut out, 2, &tokens);
        }
        self.dummies
            .iter()
            .for_each(|d| self.float(db, d, &mut out));
        self.assertion(db, &mut out);
        out.push_str("$}\n");
        out
    }
}
