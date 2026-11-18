//! The proofs in progress the interface holds, each as the saved form of its editing session.

/// One proof in progress: the theorem it proves, mcedit's saved session and the goal on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedProof {
    pub library: usize,
    /// The theorem's number in its library.
    pub theorem: usize,
    /// The versioned encoding `mcedit::Session::save` writes.
    pub session: Vec<u8>,
    /// The goal the proving screen was on, none once nothing is open.
    pub goal: Option<u32>,
    /// Operations in force.
    pub steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreError {
    NoSuchLibrary,
    NoSuchTheorem,
    /// The bytes are not a session of this version for that theorem.
    Unreadable(&'static str),
}

struct Held {
    proof: SavedProof,
    /// Whether the proofs count the status table gave already includes it.
    counted: bool,
}

/// The proofs in progress, and the count "My proofs" shows: the proofs the status table knew of
/// plus the ones begun here.
pub struct Proofs {
    known: usize,
    held: Vec<Held>,
}

impl Proofs {
    pub fn new(known: usize) -> Proofs {
        Proofs {
            known,
            held: Vec::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.known + self.held.iter().filter(|h| !h.counted).count()
    }

    pub fn find(&self, library: usize, theorem: usize) -> Option<&SavedProof> {
        self.held
            .iter()
            .map(|h| &h.proof)
            .find(|p| p.library == library && p.theorem == theorem)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SavedProof> {
        self.held.iter().map(|h| &h.proof)
    }

    fn position(&self, proof: &SavedProof) -> Option<usize> {
        self.held
            .iter()
            .position(|h| h.proof.library == proof.library && h.proof.theorem == proof.theorem)
    }

    /// Keeps `proof` as the saved form of its theorem. One with no operations left is dropped
    /// unless the status table counted it.
    pub fn store(&mut self, proof: SavedProof) {
        match (self.position(&proof), proof.steps) {
            (Some(at), _) if self.held[at].counted => self.held[at].proof = proof,
            (Some(at), 0) => {
                self.held.remove(at);
            }
            (Some(at), _) => self.held[at].proof = proof,
            (None, 0) => {}
            (None, _) => self.held.push(Held {
                proof,
                counted: false,
            }),
        }
    }

    /// Takes back a proof the status table already counts.
    pub fn restore(&mut self, proof: SavedProof) {
        match self.position(&proof) {
            Some(at) => {
                self.held[at] = Held {
                    proof,
                    counted: true,
                }
            }
            None => self.held.push(Held {
                proof,
                counted: true,
            }),
        }
    }
}
