use mcpack::mm::{Database, Event, Kind, ProofError, StmtId, Tree, Verifier};
use std::rc::Rc;

/// A provable step of a verified proof: the assertion or hypothesis applied, the syntax trees
/// substituted for its mandatory floating hypotheses and the proofs of its essential ones.
#[derive(Debug)]
pub struct ProofNode {
    pub label: StmtId,
    pub subst: Vec<Rc<Tree>>,
    pub children: Vec<Rc<ProofNode>>,
}

enum Entry {
    Syntax(Rc<Tree>),
    Provable(Rc<ProofNode>),
}

impl Entry {
    fn dup(&self) -> Entry {
        match self {
            Entry::Syntax(t) => Entry::Syntax(t.clone()),
            Entry::Provable(n) => Entry::Provable(n.clone()),
        }
    }
}

/// Reads the proof of a theorem off the verifier's trace.
pub fn proof_tree(db: &Database, theorem: StmtId) -> Result<Rc<ProofNode>, ProofError> {
    let provable = db.stmt(theorem).expr[0];
    let events = Verifier::new(db).trace(theorem)?;
    let mut stack: Vec<Entry> = Vec::new();
    let mut saved: Vec<Entry> = Vec::new();
    for event in events {
        match event {
            Event::Applied {
                label, consumed, ..
            } => {
                let stmt = db.stmt(label);
                let at = stack.len() - consumed;
                let popped: Vec<Entry> = stack.drain(at..).collect();
                let (mut subst, mut children) = (Vec::new(), Vec::new());
                for entry in popped {
                    match entry {
                        Entry::Syntax(tree) => subst.push(tree),
                        Entry::Provable(node) => children.push(node),
                    }
                }
                stack.push(match stmt.kind {
                    Kind::Float => Entry::Syntax(Rc::new(Tree {
                        stmt: label,
                        children: Vec::new(),
                    })),
                    _ if stmt.expr[0] == provable => Entry::Provable(Rc::new(ProofNode {
                        label,
                        subst,
                        children,
                    })),
                    _ => Entry::Syntax(Rc::new(Tree {
                        stmt: label,
                        children: subst,
                    })),
                });
            }
            Event::Saved => {
                let top = stack.last().map(Entry::dup);
                saved.extend(top);
            }
            Event::Reused(i) => stack.push(saved[i].dup()),
        }
    }
    match stack.pop() {
        Some(Entry::Provable(node)) => Ok(node),
        _ => Err(ProofError::WrongConclusion),
    }
}
