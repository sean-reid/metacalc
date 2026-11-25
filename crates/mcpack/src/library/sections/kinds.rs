use super::{Cursor, Entry, KINDS, SectionError, Writer};
use crate::library::PackError;

/// Which assertions are theorems, one bit each in assertion order, assertion 0 in the low bit of
/// the first byte, preceded by the assertion count as a u16. Bits past the last assertion are
/// zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kinds {
    pub theorems: Vec<bool>,
}

/// Bytes of the section for `count` assertions.
pub fn section_len(count: usize) -> usize {
    2 + count.div_ceil(8)
}

pub(super) fn encode(w: &mut Writer, theorems: &[bool]) -> Result<(), PackError> {
    let count = u16::try_from(theorems.len()).map_err(|_| PackError::SectionTooLarge)?;
    w.open(KINDS);
    w.put(&count.to_le_bytes());
    let mut bits = vec![0u8; theorems.len().div_ceil(8)];
    for (n, _) in theorems.iter().enumerate().filter(|(_, t)| **t) {
        bits[n / 8] |= 1 << (n % 8);
    }
    w.put(&bits);
    w.close();
    Ok(())
}

pub(super) fn decode(
    bytes: &[u8],
    entry: &Entry,
    assertions: usize,
) -> Result<Kinds, SectionError> {
    let bad = SectionError::Kinds;
    let mut cur = Cursor::new(bytes, entry.offset, entry.offset + entry.length);
    if cur.u16().ok_or(bad)? != assertions || entry.length != section_len(assertions) {
        return Err(bad);
    }
    let bits = cur.take(assertions.div_ceil(8)).ok_or(bad)?;
    if !assertions.is_multiple_of(8) && bits[bits.len() - 1] >> (assertions % 8) != 0 {
        return Err(bad);
    }
    Ok(Kinds {
        theorems: (0..assertions)
            .map(|n| bits[n / 8] >> (n % 8) & 1 == 1)
            .collect(),
    })
}
