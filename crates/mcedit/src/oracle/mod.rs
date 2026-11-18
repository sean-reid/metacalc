//! A check of the model against real proofs: each proof the reference verifier accepts is
//! rebuilt through the public operations, exported, written back into the database text and
//! verified again.

mod prooftree;
mod replay;
mod splice;

pub use prooftree::{ProofNode, proof_tree};
pub use replay::{
    Failure, Replayed, Report, replay, replay_as_new, replay_database, replay_proof,
    spliced_database, verify_spliced,
};
pub use splice::{SpliceError, close_scopes, splice, splice_at};
