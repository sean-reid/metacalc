//! What the walk's table must hold, worked out on the host from the decoded library and the
//! stored proof's bytes, and the table as the Z80 leaves it.

use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::{Hypothesis, Library, Reference, Step};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The address, in bank B, of the step that proves the goal.
    pub step: u16,
    /// The address of the first step of the first entry that step takes, else its own.
    pub begin: u16,
    /// 8000h + the hypothesis's number among the essential ones (0 for a float), or the
    /// assertion's number.
    pub by: u16,
    pub subgoals: Vec<u8>,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub rows: Vec<Row>,
    pub steps: u16,
    pub links: u16,
    pub total: u32,
}

#[derive(Clone, Copy)]
struct Entry {
    goal: Option<usize>,
    start: u16,
}

fn provable(first: &str) -> bool {
    first == "|-"
}

/// The table of theorem `n`, by the rules of the header applied to the library's decoded
/// records: `None` when the proof ends on a syntax statement.
pub fn mirror(library: &Library, bytes: &[u8], n: usize) -> Option<Table> {
    let a = &library.assertions[n];
    let l = layout(bytes, n);
    let mut at = 0x8000 | (l.proof & 0x3FFF) as u16;
    let mut ranks = Vec::new();
    let mut essentials = 0u16;
    for h in &a.hypotheses {
        match h {
            Hypothesis::Float { typecode, .. } => ranks.push((provable(typecode), 0)),
            Hypothesis::Essential(syms) => {
                essentials += 1;
                ranks.push((provable(&syms[0]), essentials));
            }
        }
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut stack: Vec<Entry> = Vec::new();
    let mut saved: Vec<Option<usize>> = Vec::new();
    let mut steps = 0u16;
    for step in &a.proof {
        let here = at;
        match *step {
            Step::Save => {
                saved.push(stack.last().expect("a save has an entry").goal);
                at += 1;
            }
            Step::Number(k) => {
                at += if k < 0xF0 { 1 } else { 2 };
                steps += 1;
                let hyps = a.hypotheses.len();
                let refs = a.references.len();
                if k < hyps {
                    let (goal, rank) = ranks[k];
                    let mut entry = Entry {
                        goal: None,
                        start: here,
                    };
                    if goal {
                        rows.push(Row {
                            step: here,
                            begin: here,
                            by: 0x8000 | rank as u16,
                            subgoals: vec![],
                            size: 1,
                        });
                        entry.goal = Some(rows.len() - 1);
                    }
                    stack.push(entry);
                } else if k < hyps + refs {
                    let (take, goal, by) = match &a.references[k - hyps] {
                        Reference::Assertion(i) => {
                            let r = &library.assertions[*i];
                            (r.hypotheses.len(), provable(&r.conclusion[0]), *i as u16)
                        }
                        Reference::Dummy(v) => (0, provable(&v.typecode), 0x8000),
                        Reference::Invalid(_) => panic!("an invalid dummy entry"),
                    };
                    let from = stack.len() - take;
                    let taken: Vec<Entry> = stack.drain(from..).collect();
                    let begin = taken.first().map_or(here, |e| e.start);
                    let mut entry = Entry {
                        goal: None,
                        start: begin,
                    };
                    if goal {
                        let subgoals: Vec<u8> = taken
                            .iter()
                            .filter_map(|e| e.goal.map(|g| g as u8))
                            .collect();
                        let size = 1 + subgoals
                            .iter()
                            .map(|&g| rows[usize::from(g)].size)
                            .sum::<u32>();
                        rows.push(Row {
                            step: here,
                            begin,
                            by,
                            subgoals,
                            size,
                        });
                        entry.goal = Some(rows.len() - 1);
                    }
                    stack.push(entry);
                } else {
                    stack.push(Entry {
                        goal: saved[k - hyps - refs],
                        start: here,
                    });
                }
            }
        }
    }
    assert_eq!(stack.len(), 1, "a proof that leaves one entry");
    stack[0].goal?;
    let root = &rows[rows.len() - 1];
    assert_eq!(stack[0].goal, Some(rows.len() - 1));
    Some(Table {
        steps,
        links: rows.iter().map(|r| r.subgoals.len() as u16).sum(),
        total: root.size,
        rows,
    })
}

/// A byte of the spare page by its address in bank C.
pub struct Spare(pub Vec<u8>);

impl Spare {
    pub fn of(rig: &mut Rig) -> Spare {
        Spare(rig.m.ram_page(SPARE_PAGE).to_vec())
    }

    pub fn at(&self, addr: u16) -> usize {
        usize::from(addr - 0xC000)
    }

    pub fn u8(&self, addr: u16) -> u8 {
        self.0[self.at(addr)]
    }

    pub fn u16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.u8(addr), self.u8(addr + 1)])
    }

    pub fn u24(&self, addr: u16) -> u32 {
        u32::from(self.u16(addr)) | u32::from(self.u8(addr + 2)) << 16
    }
}

/// The table the Z80 holds on the spare page after a finished walk.
pub fn read_table(rig: &mut Rig) -> Table {
    let s = Spare::of(rig);
    let l = |name: &str| rig.label(name);
    let goals = usize::from(s.u8(l("PV_GOALS")));
    let links = s.u16(l("PV_LINKS"));
    let starts: Vec<u16> = (0..goals)
        .map(|g| s.u16(l("PW_LSTART") + 2 * g as u16))
        .collect();
    let rows = (0..goals)
        .map(|g| {
            let from = usize::from(starts[g]);
            let to = if g + 1 < goals {
                usize::from(starts[g + 1])
            } else {
                usize::from(links)
            };
            let at = l("PW_LINKS");
            Row {
                step: s.u16(l("PW_STEP") + 2 * g as u16),
                begin: s.u16(l("PW_BEGIN") + 2 * g as u16),
                by: s.u16(l("PW_BY") + 2 * g as u16),
                subgoals: (from..to).map(|i| s.u8(at + i as u16)).collect(),
                size: s.u24(l("PW_SIZES") + 3 * g as u16),
            }
        })
        .collect();
    Table {
        rows,
        steps: s.u16(l("PV_STEPS")),
        links,
        total: s.u24(l("PV_TOTAL")),
    }
}
