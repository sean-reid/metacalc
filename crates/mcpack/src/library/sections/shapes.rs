use super::{Cursor, Entry, SHAPES, SectionError, Trees, Writer, within_chunk};
use crate::library::PackError;

/// Most parts a syntax axiom's expression can have after its typecode.
pub const SHAPE_CAPACITY: usize = 255;

/// The syntax axioms' parts in reading order, one byte each: 0 for a constant, k + 1 for the
/// child k of a tree node, in the table order of the trees section. Decoding checks the bytes
/// against the trees' arities; only the library check compares them with the axioms' records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shapes {
    pub axioms: Vec<Vec<u8>>,
}

pub(super) fn encode(w: &mut Writer, axioms: &[Vec<u8>]) -> Result<(), PackError> {
    let too_large = || PackError::SectionTooLarge;
    let table = 2 * (axioms.len() + 2);
    let total = table + axioms.iter().map(Vec::len).sum::<usize>();
    w.open_whole(SHAPES, total).map_err(|_| too_large())?;
    let count = u16::try_from(axioms.len()).map_err(|_| too_large())?;
    w.put(&count.to_le_bytes());
    let mut offset = table;
    for axiom in axioms {
        w.put(
            &u16::try_from(offset)
                .map_err(|_| too_large())?
                .to_le_bytes(),
        );
        offset += axiom.len();
    }
    w.put(
        &u16::try_from(offset)
            .map_err(|_| too_large())?
            .to_le_bytes(),
    );
    for axiom in axioms {
        w.put(axiom);
    }
    w.close();
    Ok(())
}

pub(super) fn decode(bytes: &[u8], entry: &Entry, trees: &Trees) -> Result<Shapes, SectionError> {
    let bad = SectionError::Shapes;
    let end = entry.offset + entry.length;
    let mut cur = Cursor::new(bytes, entry.offset, end);
    let count = trees.syntax.len();
    if cur.u16().ok_or(bad)? != count {
        return Err(bad);
    }
    let mut offsets = Vec::with_capacity(count + 1);
    for _ in 0..=count {
        offsets.push(cur.u16().ok_or(bad)?);
    }
    if offsets[0] != 2 * (count + 2)
        || offsets.windows(2).any(|w| w[0] >= w[1])
        || entry.offset + offsets[count] != end
    {
        return Err(bad);
    }
    let mut axioms = Vec::with_capacity(count);
    for (axiom, pair) in trees.syntax.iter().zip(offsets.windows(2)) {
        let at = entry.offset + pair[0];
        let len = pair[1] - pair[0];
        let parts = bytes.get(at..at + len).ok_or(bad)?;
        let mut seen = vec![false; axiom.arity];
        for &byte in parts.iter().filter(|&&b| b != 0) {
            match seen.get_mut(usize::from(byte) - 1) {
                Some(slot) if !*slot => *slot = true,
                _ => return Err(bad),
            }
        }
        if len > SHAPE_CAPACITY || seen.contains(&false) || !within_chunk(at, len) {
            return Err(bad);
        }
        axioms.push(parts.to_vec());
    }
    Ok(Shapes { axioms })
}
