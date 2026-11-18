use crate::term::Clash;
use mcpack::mm::SyntaxError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The statement is not a theorem with a provable conclusion the grammar reads.
    NotATheorem,
    Syntax(SyntaxError),
    NoSuchGoal,
    GoalNotOpen,
    /// The statement is not an axiom or theorem of the provable typecode.
    NotAnAssertion,
    /// The assertion does not come before the theorem being proved.
    Forward,
    NotAHypothesis,
    Unify(Clash),
    NoSuchMeta,
    AlreadyBound,
    /// The variable is not declared for the metavariable's typecode.
    NotAVariable,
    /// The statement is not a syntax axiom of the metavariable's typecode.
    NotSyntax,
    NothingToUndo,
    /// The step would make a term larger or deeper than the editor allows.
    TooLarge,
    /// The proof is unfinished: goals still open, metavariables unresolved or conditions violated.
    Unfinished {
        open: usize,
        metas: usize,
        violations: usize,
    },
    /// Saved bytes that are not a session of this version, or session state that breaks an invariant.
    Corrupt,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unfinished {
                open,
                metas,
                violations,
            } => write!(
                f,
                "unfinished: {open} open goals, {metas} metavariables, {violations} violations"
            ),
            other => write!(f, "{other:?}"),
        }
    }
}

impl std::error::Error for Error {}
