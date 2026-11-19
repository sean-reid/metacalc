//! Builds a library from a manifest.

use crate::manifest::Manifest;
use crate::mm::{self, Database, Grammar, GrammarError, Kind, StmtId, SyntaxError, Tree};

#[derive(Debug)]
pub enum BuildError {
    UnknownLabel(String),
    /// No axiom or theorem comes before the boundary label.
    Empty(String),
    /// A kept statement needs a syntax axiom the cut drops: statement, axiom, boundary label.
    SyntaxAfterCut(String, String, String),
    Grammar(GrammarError),
    Syntax(String, SyntaxError),
}

impl BuildError {
    /// Whether the manifest, rather than the database's contents, is at fault.
    pub fn is_bad_input(&self) -> bool {
        matches!(
            self,
            BuildError::UnknownLabel(_) | BuildError::Empty(_) | BuildError::SyntaxAfterCut(..)
        )
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::UnknownLabel(label) => {
                write!(
                    f,
                    "no statement is labelled {}",
                    mm::shown(label.as_bytes())
                )
            }
            BuildError::Empty(before) => write!(f, "no assertion comes before {before}"),
            BuildError::SyntaxAfterCut(statement, axiom, before) => write!(
                f,
                "{statement} needs the syntax axiom {axiom}, which the cut before = {before} drops; move the cut past {axiom} or drop {statement}"
            ),
            BuildError::Grammar(e) => {
                write!(f, "the syntax axioms do not form a usable grammar: {e}")
            }
            BuildError::Syntax(label, e) => write!(f, "{}: {e:?}", mm::shown(label.as_bytes())),
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
        if let Some((statement, axiom)) = dropped_syntax(db, &grammar, &kept) {
            let label = |id| db.stmt(id).label.clone();
            return Err(BuildError::SyntaxAfterCut(
                label(statement),
                label(axiom),
                label(before),
            ));
        }
    }
    Ok(kept)
}

/// The first kept assertion whose parse tree uses a syntax axiom outside `kept`, with that axiom.
fn dropped_syntax(db: &Database, grammar: &Grammar, kept: &[StmtId]) -> Option<(StmtId, StmtId)> {
    for &id in kept {
        if !matches!(db.stmt(id).kind, Kind::Axiom | Kind::Theorem) {
            continue;
        }
        let parsed = grammar.parse(id).ok()?;
        let mut trees: Vec<&Tree> = parsed.hyps.iter().map(|tree| &**tree).collect();
        trees.push(&parsed.conclusion);
        while let Some(tree) = trees.pop() {
            if db.stmt(tree.stmt).kind != Kind::Float && kept.binary_search(&tree.stmt).is_err() {
                return Some((id, tree.stmt));
            }
            trees.extend(tree.children.iter().map(|child| &**child));
        }
    }
    None
}
