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

/// The mark of assertion `at` given the marks of the ones before it. A proof only uses earlier
/// assertions; one that names itself or a later assertion cannot be right and shows as failed.
fn mark_of(assertions: &[Assertion], status: &[Status], marks: &[Mark], at: usize) -> Mark {
    let assertion = &assertions[at];
    let forward = assertion
        .references
        .iter()
        .any(|r| matches!(r, Reference::Assertion(i) if *i >= at));
    match status[at] {
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
    }
}

/// The mark of every assertion, settled in one pass.
pub fn marks(assertions: &[Assertion], status: &[Status]) -> Vec<Mark> {
    let mut marks: Vec<Mark> = Vec::with_capacity(assertions.len());
    for at in 0..assertions.len() {
        let mark = mark_of(assertions, status, &marks, at);
        marks.push(mark);
    }
    marks
}

/// Settles `marks` again after the status of assertion `changed` moved: its own mark, then only
/// the assertions that use one whose mark changed.
pub fn update(assertions: &[Assertion], status: &[Status], marks: &mut [Mark], changed: usize) {
    let mut moved = vec![false; assertions.len()];
    for at in changed..assertions.len() {
        let uses_moved = assertions[at]
            .references
            .iter()
            .any(|r| matches!(r, Reference::Assertion(i) if moved[*i]));
        if at != changed && !uses_moved {
            continue;
        }
        let mark = mark_of(assertions, status, marks, at);
        moved[at] = mark != marks[at];
        marks[at] = mark;
    }
}
