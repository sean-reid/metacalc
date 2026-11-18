//! The proof of a theorem as a backward goal tree.

use super::{Database, Event, Kind, ProofError, StmtId, Verifier};
use std::fmt::{self, Write};
use std::rc::Rc;

/// One provable statement in a proof and the steps that establish it.
#[derive(Debug, PartialEq, Eq)]
pub struct Node {
    /// The assertion or hypothesis applied to reach the statement.
    pub label: StmtId,
    /// The statement after substitution, typecode first.
    pub expr: Vec<super::Sym>,
    /// The proofs of the essential hypotheses of `label`, in order.
    pub children: Vec<Rc<Node>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutlineError {
    /// The statement is an axiom, a hypothesis, or not a theorem of the database.
    NotATheorem,
    /// The proof does not verify.
    Proof(ProofError),
    /// The theorem does not conclude a statement of the provable typecode.
    NotProvable,
}

impl fmt::Display for OutlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlineError::NotATheorem => f.write_str("it is not a theorem"),
            OutlineError::Proof(e) => write!(f, "its proof does not verify: {e:?}"),
            OutlineError::NotProvable => f.write_str("it does not conclude a provable statement"),
        }
    }
}

/// Builds the goal tree of a verified theorem. Only steps whose typecode is `provable` become
/// nodes. A step a compressed proof saved is shared by every place it is used.
pub fn outline(db: &Database, id: StmtId, provable: &str) -> Result<Rc<Node>, OutlineError> {
    if db.stmt(id).kind != Kind::Theorem {
        return Err(OutlineError::NotATheorem);
    }
    let events = Verifier::new(db).trace(id).map_err(OutlineError::Proof)?;
    let mut stack: Vec<Option<Rc<Node>>> = Vec::new();
    let mut saved: Vec<Option<Rc<Node>>> = Vec::new();
    for event in events {
        match event {
            Event::Applied {
                label,
                expr,
                consumed,
                essentials,
            } => {
                let at = stack.len() - consumed;
                let children: Vec<Rc<Node>> = stack.drain(at..).flatten().collect();
                debug_assert_eq!(children.len(), essentials);
                let is_provable = db.syms[expr[0] as usize].name == provable;
                stack.push(is_provable.then(|| {
                    Rc::new(Node {
                        label,
                        expr,
                        children,
                    })
                }));
            }
            Event::Saved => saved.push(stack.last().cloned().flatten()),
            Event::Reused(i) => stack.push(saved[i].clone()),
        }
    }
    stack.pop().flatten().ok_or(OutlineError::NotProvable)
}

impl Node {
    /// Writes the tree one node per line, two spaces of indent per level: the statement, then the
    /// label in brackets.
    pub fn render(&self, db: &Database) -> String {
        let mut out = String::new();
        self.write(db, 0, &mut out);
        out
    }

    fn write(&self, db: &Database, depth: usize, out: &mut String) {
        let words: Vec<&str> = self
            .expr
            .iter()
            .map(|&s| db.syms[s as usize].name.as_str())
            .collect();
        let _ = writeln!(
            out,
            "{:depth$}{}  [{}]",
            "",
            words.join(" "),
            db.stmt(self.label).label,
            depth = depth * 2
        );
        for child in &self.children {
            child.write(db, depth + 1, out);
        }
    }
}
