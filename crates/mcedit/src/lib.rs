//! The logic of a proof editor: a backward goal tree over a Metamath database, built from
//! unification, with the disjoint-variable rules that make a finished tree a proof.

mod catalog;
mod codec;
mod dv;
mod error;
mod export;
mod ops;
pub mod oracle;
mod search;
mod session;
mod term;
mod vars;

pub use catalog::Catalog;
pub use dv::{Violation, ViolationKind};
pub use error::Error;
pub use export::{Dummy, Finished, Label};
pub use search::{AUTO_BUDGET, AUTO_DEPTH, Candidate, Candidates};
pub use session::{Goal, GoalId, GoalState, Op, Outcome, Session, Sizes, Status};
pub use term::{Clash, Head, MAX_TERM_DEPTH, MAX_TERM_NODES, MetaId, Peak, Piece, Store, TermId};
pub use vars::Vars;
