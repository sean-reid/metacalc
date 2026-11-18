use mcpack::library::{Assertion, Reference};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

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

/// The status and mark of every assertion, and the totals the screens read. A check changes one
/// status, and only the assertions that use one whose mark moved are looked at again.
pub struct Ledger {
    status: Vec<Status>,
    marks: Vec<Mark>,
    /// The assertions that use each one, later ones only: `users[starts[i]..starts[i + 1]]`.
    starts: Vec<u32>,
    users: Vec<u32>,
    checked: usize,
    failed: usize,
    settled: usize,
    failed_checks: usize,
    /// Every status before this one is settled.
    next: usize,
}

impl Ledger {
    pub fn new(assertions: &[Assertion], status: Vec<Status>) -> Ledger {
        let n = assertions.len();
        let backward = |at: usize| {
            assertions[at]
                .references
                .iter()
                .filter_map(move |r| match r {
                    Reference::Assertion(i) if *i < at => Some(*i),
                    _ => None,
                })
        };
        let mut starts = vec![0u32; n + 1];
        for at in 0..n {
            for i in backward(at) {
                starts[i + 1] += 1;
            }
        }
        for i in 0..n {
            starts[i + 1] += starts[i];
        }
        let mut users = vec![0u32; starts[n] as usize];
        let mut fill = starts.clone();
        for at in 0..n {
            for i in backward(at) {
                users[fill[i] as usize] = at as u32;
                fill[i] += 1;
            }
        }
        let marks = marks(assertions, &status);
        let count = |keep: &dyn Fn(usize) -> bool| (0..n).filter(|&i| keep(i)).count();
        Ledger {
            checked: count(&|i| counts_as_checked(marks[i])),
            failed: count(&|i| marks[i] == Mark::Failed),
            settled: count(&|i| status[i] != Status::Unchecked),
            failed_checks: count(&|i| status[i] == Status::Failed),
            next: status
                .iter()
                .position(|&s| s == Status::Unchecked)
                .unwrap_or(n),
            status,
            marks,
            starts,
            users,
        }
    }

    pub fn len(&self) -> usize {
        self.status.len()
    }

    pub fn status(&self, i: usize) -> Status {
        self.status[i]
    }

    pub fn mark(&self, i: usize) -> Mark {
        self.marks[i]
    }

    /// Assertions marked checked, whether or not they rest on others.
    pub fn checked(&self) -> usize {
        self.checked
    }

    /// Assertions marked failed.
    pub fn failed(&self) -> usize {
        self.failed
    }

    /// Assertions whose status a check has settled, either way.
    pub fn settled(&self) -> usize {
        self.settled
    }

    /// Assertions whose recorded status is a failed check.
    pub fn failed_checks(&self) -> usize {
        self.failed_checks
    }

    /// The first assertion whose status is still unchecked.
    pub fn next_unchecked(&self) -> Option<usize> {
        (self.next < self.len()).then_some(self.next)
    }

    /// Records the status of assertion `changed` and settles the marks it moves.
    pub fn set(&mut self, assertions: &[Assertion], changed: usize, status: Status) {
        let was = self.status[changed];
        self.settled = self.settled + usize::from(status != Status::Unchecked)
            - usize::from(was != Status::Unchecked);
        self.failed_checks = self.failed_checks + usize::from(status == Status::Failed)
            - usize::from(was == Status::Failed);
        self.status[changed] = status;
        if status == Status::Unchecked {
            self.next = self.next.min(changed);
        } else {
            while self.next < self.len() && self.status[self.next] != Status::Unchecked {
                self.next += 1;
            }
        }
        let mut pending = BinaryHeap::from([Reverse(changed as u32)]);
        let mut last = None;
        while let Some(Reverse(at)) = pending.pop() {
            if last == Some(at) {
                continue;
            }
            last = Some(at);
            let at = at as usize;
            let mark = mark_of(assertions, &self.status, &self.marks, at);
            let old = std::mem::replace(&mut self.marks[at], mark);
            if mark == old {
                continue;
            }
            self.checked = self.checked + usize::from(counts_as_checked(mark))
                - usize::from(counts_as_checked(old));
            self.failed =
                self.failed + usize::from(mark == Mark::Failed) - usize::from(old == Mark::Failed);
            let range = self.starts[at] as usize..self.starts[at + 1] as usize;
            pending.extend(self.users[range].iter().map(|&u| Reverse(u)));
        }
    }
}

fn counts_as_checked(mark: Mark) -> bool {
    matches!(mark, Mark::Checked | Mark::Resting)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assertion(uses: &[usize]) -> Assertion {
        Assertion {
            theorem: true,
            label: String::new(),
            hypotheses: Vec::new(),
            disjoint: Vec::new(),
            conclusion: Vec::new(),
            extra_disjoint: Vec::new(),
            references: uses.iter().map(|&i| Reference::Assertion(i)).collect(),
            proof: Vec::new(),
        }
    }

    #[test]
    fn marks_and_totals_after_any_run_of_changes_equal_a_fresh_pass_over_the_table() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut roll = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let n = 80;
        let assertions: Vec<Assertion> = (0..n)
            .map(|at| {
                let uses: Vec<usize> = (0..roll(5))
                    .map(|_| match roll(10) {
                        0 => at + roll(4),
                        1 => n + roll(5),
                        _ => roll(at.max(1)),
                    })
                    .collect();
                assertion(&uses)
            })
            .collect();
        let mut seen = std::collections::HashSet::new();
        let assertions: Vec<Assertion> = assertions
            .into_iter()
            .map(|mut a| {
                seen.clear();
                a.references
                    .retain(|r| matches!(r, Reference::Assertion(i) if seen.insert(*i)));
                a
            })
            .collect();
        let all = [Status::Checked, Status::Unchecked, Status::Failed];
        let mut ledger = Ledger::new(&assertions, vec![Status::Unchecked; n]);
        let mut status = vec![Status::Unchecked; n];
        for step in 0..2000 {
            let (at, to) = (roll(n), all[roll(3)]);
            ledger.set(&assertions, at, to);
            status[at] = to;
            let fresh = Ledger::new(&assertions, status.clone());
            assert_eq!(ledger.marks, fresh.marks, "step {step}");
            assert_eq!(ledger.checked, fresh.checked, "step {step}");
            assert_eq!(ledger.failed, fresh.failed, "step {step}");
            assert_eq!(ledger.settled, fresh.settled, "step {step}");
            assert_eq!(ledger.failed_checks, fresh.failed_checks, "step {step}");
            assert_eq!(ledger.next, fresh.next, "step {step}");
        }
    }
}
