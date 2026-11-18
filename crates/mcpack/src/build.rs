//! Builds a library from a manifest.

use crate::manifest::Manifest;
use crate::mm::{self, Database, Grammar, GrammarError, StmtId, SyntaxError};

#[derive(Debug)]
pub enum BuildError {
    UnknownLabel(String),
    /// No axiom or theorem comes before the boundary label.
    Empty(String),
    Grammar(GrammarError),
    Syntax(String, SyntaxError),
}

impl BuildError {
    /// Whether the manifest, rather than the database's contents, is at fault.
    pub fn is_bad_input(&self) -> bool {
        matches!(self, BuildError::UnknownLabel(_) | BuildError::Empty(_))
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::UnknownLabel(label) => write!(f, "no statement is labelled {label}"),
            BuildError::Empty(before) => write!(f, "no assertion comes before {before}"),
            BuildError::Grammar(e) => {
                write!(f, "the syntax axioms do not form a usable grammar: {e:?}")
            }
            BuildError::Syntax(label, e) => write!(f, "{label}: {e:?}"),
        }
    }
}

/// The statements a manifest keeps, in database order.
///
/// Dependencies point to earlier statements, so cutting the closure before a label keeps it
/// dependency-closed.
pub fn closure(db: &Database, manifest: &Manifest) -> Result<Vec<StmtId>, BuildError> {
    let find = |label: &String| {
        db.find(label)
            .ok_or_else(|| BuildError::UnknownLabel(label.clone()))
    };
    let roots = manifest
        .roots
        .iter()
        .map(find)
        .collect::<Result<Vec<_>, _>>()?;
    let before = manifest.before.as_ref().map(find).transpose()?;
    let grammar = Grammar::new(db, &[("|-", "wff")]).map_err(BuildError::Grammar)?;
    let mut kept = mm::closure(db, &grammar, &roots)
        .map_err(|(id, e)| BuildError::Syntax(db.stmt(id).label.clone(), e))?;
    if let Some(before) = before {
        kept.retain(|&id| id < before);
        if !kept.iter().any(|&id| db.stmt(id).kind.is_assertion()) {
            return Err(BuildError::Empty(db.stmt(before).label.clone()));
        }
    }
    Ok(kept)
}
