use super::{Cursor, Entry, LABEL_ORDER, SectionError, Writer};
use crate::library::{Assertion, PackError};

/// The assertion numbers sorted by label, bytewise, preceded by the assertion count as a u16, two
/// bytes each, little endian. Labels are unique, so the order is total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelOrder {
    pub numbers: Vec<u16>,
}

/// Bytes of the section for `count` assertions.
pub fn section_len(count: usize) -> usize {
    2 + 2 * count
}

/// The numbers of `labels` in label order.
pub(super) fn order_of<'a>(labels: impl Iterator<Item = &'a str>) -> Vec<usize> {
    let labels: Vec<&str> = labels.collect();
    let mut numbers: Vec<usize> = (0..labels.len()).collect();
    numbers.sort_by(|&a, &b| labels[a].as_bytes().cmp(labels[b].as_bytes()));
    numbers
}

pub(super) fn encode(w: &mut Writer, labels: &[&str]) -> Result<(), PackError> {
    let count = u16::try_from(labels.len()).map_err(|_| PackError::SectionTooLarge)?;
    w.open(LABEL_ORDER);
    w.put(&count.to_le_bytes());
    for n in order_of(labels.iter().copied()) {
        w.put(&(n as u16).to_le_bytes());
    }
    w.close();
    Ok(())
}

pub(super) fn decode(
    bytes: &[u8],
    entry: &Entry,
    assertions: &[Assertion],
) -> Result<LabelOrder, SectionError> {
    let bad = SectionError::LabelOrder;
    let count = assertions.len();
    let mut cur = Cursor::new(bytes, entry.offset, entry.offset + entry.length);
    if cur.u16().ok_or(bad)? != count || entry.length != section_len(count) {
        return Err(bad);
    }
    let mut numbers = Vec::with_capacity(count);
    for _ in 0..count {
        let n = cur.u16().ok_or(bad)?;
        if n >= count {
            return Err(bad);
        }
        numbers.push(n as u16);
    }
    let label = |n: u16| assertions[n as usize].label.as_bytes();
    // Labels strictly rise, so no number repeats.
    if numbers.windows(2).any(|w| label(w[0]) >= label(w[1])) {
        return Err(bad);
    }
    Ok(LabelOrder { numbers })
}
