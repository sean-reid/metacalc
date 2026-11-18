//! Metamath database model, parser and reference verifier.

mod closure;
mod grammar;
mod parse;
mod verify;

pub use closure::closure;
pub use grammar::{Grammar, GrammarError, Parsed, SyntaxError, Tree};
pub use parse::{ParseError, parse};
pub use verify::{Usage, Verifier};

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
    /// Every disjoint pair active at the theorem, mandatory or not, sorted.
    pub scope_dv: Vec<(Sym, Sym)>,
}

pub struct Stmt {
    pub label: String,
    pub kind: Kind,
    pub expr: Vec<Sym>,
    pub frame: Frame,
    pub proof: Option<Result<Proof, ProofError>>,
}

pub struct Database {
    pub syms: Vec<Symbol>,
    pub stmts: Vec<Stmt>,
}

impl Database {
    pub fn stmt(&self, id: StmtId) -> &Stmt {
        &self.stmts[id as usize]
    }

    pub fn theorems(&self) -> impl Iterator<Item = StmtId> + '_ {
        (0..self.stmts.len() as StmtId).filter(|&i| self.stmt(i).kind == Kind::Theorem)
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
