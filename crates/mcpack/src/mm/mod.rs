//! Metamath database model, parser and reference verifier.

mod closure;
mod grammar;
mod headings;
mod include;
mod outline;
mod parse;
mod shown;
mod verify;

pub use closure::closure;
pub use grammar::{Grammar, GrammarError, Parsed, SyntaxError, Tree};
pub use outline::{Node, OutlineError, outline};
pub use parse::{ParseError, parse, parse_at, parse_at_capped};
pub use verify::{Event, Usage, Verifier};

pub type Sym = u32;
pub type StmtId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Float,
    Essential,
    Axiom,
    Theorem,
}

impl Kind {
    pub fn is_assertion(self) -> bool {
        matches!(self, Kind::Axiom | Kind::Theorem)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofError {
    NotATheorem,
    Incomplete,
    UnknownLabel,
    InactiveHypothesis,
    MandatoryHypothesisInLabelList,
    MalformedCompressed,
    BadBackReference,
    StackUnderflow,
    TypecodeMismatch,
    HypothesisMismatch,
    DisjointViolation,
    StackNotSingleton,
    WrongConclusion,
}

pub struct Symbol {
    pub name: String,
    pub is_var: bool,
}

/// Mandatory hypotheses in database order and mandatory disjoint pairs, each pair ordered low-high and the list sorted.
#[derive(Default)]
pub struct Frame {
    pub hyps: Vec<StmtId>,
    pub dv: Vec<(Sym, Sym)>,
}

pub enum ProofBody {
    Normal(Vec<StmtId>),
    Compressed {
        labels: Vec<StmtId>,
        letters: Vec<u8>,
    },
}

pub struct Proof {
    pub body: ProofBody,
}

pub struct Stmt {
    pub label: String,
    pub kind: Kind,
    pub expr: Vec<Sym>,
    pub frame: Frame,
    pub proof: Option<Result<Proof, ProofError>>,
    /// For a theorem, every disjoint pair active at it, mandatory or not, sorted. Empty for any other statement.
    pub scope_dv: Vec<(Sym, Sym)>,
    /// For a hypothesis, the number of statements parsed when its scope closed, or `StmtId::MAX` while it is still open. A hypothesis is active at statement `s` when it precedes `s` and `scope_end > s`.
    pub scope_end: StmtId,
}

/// A section title of the source: a comment between two rules of one of the four kinds set.mm uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// 1 for the `####` rule, 2 for `#*#*`, 3 for `=-=-` and 4 for `-.-.`.
    pub depth: u8,
    pub title: String,
    /// The number of statements before the comment, which is the id of the first statement under it.
    pub before: StmtId,
}

pub struct Database {
    pub syms: Vec<Symbol>,
    pub stmts: Vec<Stmt>,
    /// Each `$v` activation of a variable: the variable, the number of statements before it and the number when its block closed (`StmtId::MAX` while open).
    pub var_scopes: Vec<(Sym, StmtId, StmtId)>,
    /// The section titles in source order.
    pub headings: Vec<Heading>,
}

impl Database {
    pub fn stmt(&self, id: StmtId) -> &Stmt {
        &self.stmts[id as usize]
    }

    pub fn theorems(&self) -> impl Iterator<Item = StmtId> + '_ {
        (0..self.stmts.len() as StmtId).filter(|&i| self.stmt(i).kind == Kind::Theorem)
    }

    /// Whether the variable's `$v` is in force at statement `at`.
    pub fn var_active(&self, var: Sym, at: StmtId) -> bool {
        self.var_scopes
            .iter()
            .any(|&(v, from, to)| v == var && from <= at && at < to)
    }

    pub fn find(&self, label: &str) -> Option<StmtId> {
        self.stmts
            .iter()
            .position(|s| s.label == label)
            .map(|i| i as StmtId)
    }
}

pub(crate) fn dv_pair(a: Sym, b: Sym) -> (Sym, Sym) {
    if a < b { (a, b) } else { (b, a) }
}
