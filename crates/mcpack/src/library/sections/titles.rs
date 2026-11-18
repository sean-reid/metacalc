use super::{Entry, SectionError, TITLES, Writer};
use crate::library::PackError;
use crate::mm::{Database, StmtId};

/// A set.mm section title and the first assertion of the library under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub assertion: usize,
    /// 1 to 4, 1 being the outermost.
    pub depth: u8,
    pub text: String,
}

/// The titles of `db` that have an assertion of `assertions` under them, with every ancestor of
/// such a title, in source order. `assertions` lists the library's statements in database order.
pub fn select(db: &Database, assertions: &[StmtId]) -> Vec<Title> {
    let headings = &db.headings;
    let first_at = |before: StmtId| assertions.partition_point(|&id| id < before);
    let mut keep: Vec<bool> = headings
        .iter()
        .enumerate()
        .map(|(k, heading)| {
            let end = headings.get(k + 1).map_or(StmtId::MAX, |next| next.before);
            assertions
                .get(first_at(heading.before))
                .is_some_and(|&id| id < end)
        })
        .collect();
    let mut chain: Vec<usize> = Vec::new();
    let mut parent: Vec<Option<usize>> = Vec::with_capacity(headings.len());
    for (k, heading) in headings.iter().enumerate() {
        while chain
            .last()
            .is_some_and(|&top| headings[top].depth >= heading.depth)
        {
            chain.pop();
        }
        parent.push(chain.last().copied());
        chain.push(k);
    }
    for k in (0..headings.len()).rev() {
        if let (true, Some(up)) = (keep[k], parent[k]) {
            keep[up] = true;
        }
    }
    headings
        .iter()
        .zip(keep)
        .filter(|&(_, keep)| keep)
        .map(|(heading, _)| Title {
            assertion: first_at(heading.before),
            depth: heading.depth,
            text: heading.title.clone(),
        })
        .collect()
}

pub(super) fn encode(w: &mut Writer, titles: &[Title]) -> Result<(), PackError> {
    let too_large = |title: &Title| PackError::TitleTooLong(title.text.clone());
    let count = u16::try_from(titles.len()).map_err(|_| PackError::SectionTooLarge)?;
    w.open(TITLES);
    w.put(&count.to_le_bytes());
    let index = w.len();
    w.put(&vec![0; 4 * titles.len()]);
    for (i, title) in titles.iter().enumerate() {
        let number = u16::try_from(title.assertion).map_err(|_| PackError::SectionTooLarge)?;
        let len = u8::try_from(title.text.len()).map_err(|_| too_large(title))?;
        let mut item = number.to_le_bytes().to_vec();
        item.extend([title.depth, len]);
        item.extend_from_slice(title.text.as_bytes());
        let offset = w.item(&item).map_err(|_| PackError::SectionTooLarge)?;
        w.patch(index + 4 * i, &offset.to_le_bytes());
    }
    w.close();
    Ok(())
}

pub(super) fn decode(
    bytes: &[u8],
    entry: &Entry,
    assertions: usize,
) -> Result<Vec<Title>, SectionError> {
    let bad = SectionError::Titles;
    let end = entry.offset + entry.length;
    let mut cur = super::Cursor::new(bytes, entry.offset, end);
    let count = cur.u16().ok_or(bad)?;
    let first_item = cur.pos + 4 * count;
    let mut titles = Vec::with_capacity(count);
    for _ in 0..count {
        let at = cur.u32().ok_or(bad)?;
        if at < first_item {
            return Err(bad);
        }
        let mut item = super::Cursor::new(bytes, at, end);
        let assertion = item.u16().ok_or(bad)?;
        let depth = item.u8().ok_or(bad)? as u8;
        let len = item.u8().ok_or(bad)?;
        let text = std::str::from_utf8(item.take(len).ok_or(bad)?).map_err(|_| bad)?;
        let whole = super::within_chunk(at, item.pos - at);
        if assertion >= assertions || !(1..=4).contains(&depth) || !text.is_ascii() || !whole {
            return Err(bad);
        }
        titles.push(Title {
            assertion,
            depth,
            text: text.to_string(),
        });
    }
    Ok(titles)
}
