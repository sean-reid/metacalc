use mcpack::library::{Assertion, Reference};

/// What the calculator knows about one assertion, as its status table records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Checked,
    Unchecked,
    Failed,
}

/// What a screen shows beside an assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Checked,
    Unchecked,
    Failed,
    /// Checked, but its proof uses an assertion that is not.
    Resting,
}

impl Mark {
    /// The character of interface text that draws the mark.
    pub fn glyph(self) -> char {
        match self {
            Mark::Checked => mcglyph::CHECK,
            Mark::Unchecked => '?',
            Mark::Failed => mcglyph::CROSS,
            Mark::Resting => '~',
        }
    }
}

/// The mark of every assertion. A proof only uses earlier assertions, so one pass settles them;
/// one that names itself or a later assertion cannot be right and shows as failed.
pub fn marks(assertions: &[Assertion], status: &[Status]) -> Vec<Mark> {
    let mut marks: Vec<Mark> = Vec::with_capacity(assertions.len());
    for (at, (assertion, status)) in assertions.iter().zip(status).enumerate() {
        let forward = assertion
            .references
            .iter()
            .any(|r| matches!(r, Reference::Assertion(i) if *i >= at));
        marks.push(match status {
            _ if forward => Mark::Failed,
            Status::Unchecked => Mark::Unchecked,
            Status::Failed => Mark::Failed,
            Status::Checked => {
                let rests = assertion.references.iter().any(|r| match r {
                    Reference::Assertion(i) => marks.get(*i).is_some_and(|&m| m != Mark::Checked),
                    _ => false,
                });
                if rests { Mark::Resting } else { Mark::Checked }
            }
        });
    }
    marks
}
