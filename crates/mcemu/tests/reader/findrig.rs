//! The caller's side of the three find calls, and a mirror of their rules written from the header
//! of lib/lib.z80 and nothing else.

use crate::damaged::{offset_of, with};
use crate::secrig::*;
use crate::support::*;
use mcpack::library::KINDS;

pub const RANGE: &str = "LIB_FIND_RANGE";
pub const WALK: &str = "LIB_FIND_WALK";
pub const COPY: &str = "LIB_FIND_COPY";
pub const AXIOM: u8 = 0;
pub const THEOREM: u8 = 1;
pub const EVERY: u8 = 0xFF;
pub const RUN: usize = 127;

/// Where the caller keeps the prefix, the walk's block and the copy's buffer, in bank B.
pub const PREFIX_AT: u16 = BUF;
pub const BLOCK_AT: u16 = BUF + 0x20;
pub const COPY_AT: u16 = BUF + 0x100;
pub const MARK: u8 = 0xE5;

/// Labels that put each case where it is needed: prefixes of other labels, labels that differ only
/// in the eighth and only in the ninth character, labels of 1 to 10 characters, '-', '.', '_',
/// digits and both cases.
pub const PURPOSE: [&str; 54] = [
    "a",
    "ab",
    "abc",
    "abcd",
    "abcde",
    "abcdef",
    "abcdefg",
    "abcdefgh",
    "abcdefghi",
    "abcdefghij",
    "abcdefg1",
    "abcdefg2",
    "abcdefgh1",
    "abcdefgh2",
    "A",
    "Ab",
    "AB",
    "a-b",
    "a.b",
    "a_b",
    "a1",
    "a2",
    "a10",
    "a-",
    "-",
    "_",
    ".",
    "0",
    "9",
    "1x",
    "z",
    "zz",
    "Zz",
    "ax-1",
    "ax-2",
    "ax-mp",
    "ax-ext",
    "axext1",
    "axext2",
    "syl",
    "syl2anc",
    "sylbi",
    "sylib",
    "sylan",
    "sylanb",
    "mp2",
    "mp2b",
    "mpd",
    "ab-c",
    "ab.c",
    "ab_c",
    "ab0c",
    "Z",
    "m",
];

/// A library of axioms with these labels in this order, the kinds bitmap overwritten by `bits`
/// when given (true for a theorem), so the reader sees a pattern the records need not agree with.
pub fn library(labels: &[String], bits: Option<&[bool]>) -> Vec<u8> {
    let mut src = String::from("$c wff |- $.\n$v p $.\nwp $f wff p $.\n");
    for label in labels {
        src += &format!("{label} $a |- p $.\n");
    }
    let bytes = pack_all(&parse(&src));
    let Some(bits) = bits else { return bytes };
    assert_eq!(bits.len(), labels.len());
    let at = offset_of(&bytes, KINDS) as usize + 2;
    with(&bytes, |b| {
        b[at..at + bits.len().div_ceil(8)].fill(0);
        for (n, _) in bits.iter().enumerate().filter(|(_, t)| **t) {
            b[at + n / 8] |= 1 << (n % 8);
        }
    })
}

pub fn strings(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|l| l.to_string()).collect()
}

pub fn pseudo(n: usize, seed: u32) -> Vec<bool> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            x >> 29 & 1 == 1
        })
        .collect()
}

pub fn opened(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    rig
}

/// The rules of the three calls over a library's labels in assertion order.
pub struct Model {
    pub labels: Vec<Vec<u8>>,
    /// The assertion numbers sorted by label, which is the section.
    pub order: Vec<u16>,
    pub bits: Option<Vec<bool>>,
}

impl Model {
    pub fn new(labels: &[String], bits: Option<Vec<bool>>) -> Model {
        let labels: Vec<Vec<u8>> = labels.iter().map(|l| l.as_bytes().to_vec()).collect();
        let mut order: Vec<u16> = (0..labels.len() as u16).collect();
        order.sort_by(|&a, &b| labels[usize::from(a)].cmp(&labels[usize::from(b)]));
        Model {
            labels,
            order,
            bits,
        }
    }

    pub fn len(&self) -> u16 {
        self.labels.len() as u16
    }

    fn label_at(&self, index: usize) -> &[u8] {
        &self.labels[usize::from(self.order[index])]
    }

    /// (first index, count) for a prefix of 1 to 8 bytes: where the first label that starts with it
    /// is and how many follow, or where it would sort when none does.
    pub fn range(&self, prefix: &[u8]) -> Result<(u16, u16), u8> {
        if prefix.is_empty() || prefix.len() > 8 {
            return Err(ST_ARG);
        }
        let first = (0..self.order.len())
            .take_while(|&i| self.label_at(i) < prefix)
            .count();
        let count = (first..self.order.len())
            .take_while(|&i| self.label_at(i).starts_with(prefix))
            .count();
        Ok((first as u16, count as u16))
    }

    /// The answer of the walk and how many bitmap bytes it reads, which is the number of candidates
    /// inside start..bound and below the best so far under a kind of 0 or 1.
    pub fn walk(
        &self,
        (first, count): (u16, u16),
        start: u16,
        bound: u16,
        kind: u8,
    ) -> Result<(u16, usize), u8> {
        if kind != EVERY && kind > THEOREM {
            return Err(ST_ARG);
        }
        if kind != EVERY && self.bits.is_none() {
            return Err(ST_NONE);
        }
        if usize::from(first) + usize::from(count) > self.order.len()
            || bound > self.len()
            || start > bound
        {
            return Err(ST_INDEX);
        }
        let (mut best, mut reads) = (bound, 0);
        for &n in &self.order[usize::from(first)..usize::from(first + count)] {
            if n < start || n >= best {
                continue;
            }
            if kind == EVERY {
                best = n;
                continue;
            }
            reads += 1;
            if self.bits.as_ref().unwrap()[usize::from(n)] == (kind == THEOREM) {
                best = n;
            }
        }
        Ok((best, reads))
    }

    /// The entries the copy leaves in the buffer, two bytes each.
    pub fn copy(&self, first: u16, count: u16) -> Result<Vec<u8>, u8> {
        let taken = usize::from(count).min(RUN);
        if usize::from(first) + usize::from(count) > self.order.len() {
            return Err(ST_INDEX);
        }
        let first = usize::from(first);
        Ok(self.order[first..first + taken]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect())
    }
}

/// Registers a bare `ret` fails on: A and the flags hold junk, every register holds a mark.
fn seeded(bc: u16, de: u16, hl: u16) -> Regs {
    Regs {
        a: 0x55,
        bc,
        de,
        hl,
        ix: 0x1357,
        iy: 0x2468,
    }
}

pub fn range_regs(prefix_at: u16, len: u8) -> Regs {
    seeded(u16::from(len) << 8 | 0x33, prefix_at, 0xFEED)
}

/// (status, first, count) of one range call over `prefix`, which is poked at `PREFIX_AT` unless
/// the test points somewhere else.
pub fn ask_range(rig: &mut Rig, prefix: &[u8]) -> Out {
    rig.poke(PREFIX_AT, prefix);
    rig.call(RANGE, range_regs(PREFIX_AT, prefix.len() as u8))
}

pub fn block(first: u16, count: u16, start: u16, bound: u16, kind: u8) -> [u8; 9] {
    let mut b = [0u8; 9];
    b[0..2].copy_from_slice(&first.to_le_bytes());
    b[2..4].copy_from_slice(&count.to_le_bytes());
    b[4..6].copy_from_slice(&start.to_le_bytes());
    b[6..8].copy_from_slice(&bound.to_le_bytes());
    b[8] = kind;
    b
}

pub fn walk_regs(at: u16) -> Regs {
    seeded(0x7777, 0x6666, at)
}

pub fn ask_walk(
    rig: &mut Rig,
    (first, count): (u16, u16),
    start: u16,
    bound: u16,
    kind: u8,
) -> Out {
    rig.poke(BLOCK_AT, &block(first, count, start, bound, kind));
    rig.call(WALK, walk_regs(BLOCK_AT))
}

pub fn copy_regs(first: u16, count: u16, buf: u16) -> Regs {
    seeded(count, buf, first)
}

pub fn ask_copy(rig: &mut Rig, first: u16, count: u16) -> (Out, Vec<u8>) {
    rig.poke(COPY_AT, &[MARK; 0x100]);
    let out = rig.call(COPY, copy_regs(first, count, COPY_AT));
    (out, rig.peek(COPY_AT, 0x100))
}

/// The ranges a prefix sweep finds in `model`: every prefix of 1 to 8 bytes of every label, each
/// with its last byte one lower and one higher.
pub fn sweep_prefixes(model: &Model) -> Vec<Vec<u8>> {
    let mut found: Vec<Vec<u8>> = Vec::new();
    for label in &model.labels {
        for p in 1..=label.len().min(8) {
            let exact = label[..p].to_vec();
            for delta in [0i16, -1, 1] {
                let mut prefix = exact.clone();
                let last = prefix.len() - 1;
                prefix[last] = (i16::from(prefix[last]) + delta) as u8;
                if !found.contains(&prefix) {
                    found.push(prefix);
                }
            }
        }
    }
    found
}

impl Rig {
    /// A little-endian word or 24-bit value of the state block, by label.
    pub fn state_word(&mut self, name: &str, len: usize) -> u32 {
        self.named(name, len)
            .iter()
            .rev()
            .fold(0, |acc, &b| (acc << 8) | u32::from(b))
    }
}
