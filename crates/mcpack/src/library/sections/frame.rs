use super::{CHUNK, ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION};
use crate::library::HEADER_LEN;

/// One row of the section table: the section's id and the file range it occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub id: u8,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionError {
    /// The last bytes carry the trailer magic but not a trailer this reader knows.
    Trailer,
    /// The section table is cut short, repeats an id, or points outside the file, into itself,
    /// into the records or off a multiple of four.
    Table,
    Titles,
    Glyphs,
    Trees,
    Shapes,
}

impl std::fmt::Display for SectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SectionError::Trailer => "the trailer is damaged or of an unknown version",
            SectionError::Table => "the section table is damaged",
            SectionError::Titles => "the titles section is damaged",
            SectionError::Glyphs => "the glyph map is damaged",
            SectionError::Trees => "the parse tree section is damaged",
            SectionError::Shapes => {
                "the shapes section is damaged or does not match the parse trees"
            }
        })
    }
}

impl std::error::Error for SectionError {}

/// Locates the section table: `None` when the file has no trailer.
pub fn table(bytes: &[u8]) -> Result<Option<Vec<Entry>>, SectionError> {
    let Some(start) = bytes.len().checked_sub(TRAILER_LEN) else {
        return Ok(None);
    };
    let trailer = &bytes[start..];
    if &trailer[2..] != TRAILER_MAGIC {
        return Ok(None);
    }
    if trailer[1] != TRAILER_VERSION {
        return Err(SectionError::Trailer);
    }
    let count = trailer[0] as usize;
    let first = start
        .checked_sub(count * ENTRY_LEN)
        .filter(|&first| first >= HEADER_LEN)
        .ok_or(SectionError::Table)?;
    let mut entries: Vec<Entry> = Vec::with_capacity(count);
    for row in bytes[first..start].as_chunks::<ENTRY_LEN>().0 {
        let word = |at: usize| u32::from_le_bytes([row[at], row[at + 1], row[at + 2], row[at + 3]]);
        let (offset, length) = (word(1) as usize, word(5) as usize);
        let end = offset.checked_add(length).ok_or(SectionError::Table)?;
        if offset < HEADER_LEN || end > first || entries.iter().any(|e| e.id == row[0]) {
            return Err(SectionError::Table);
        }
        entries.push(Entry {
            id: row[0],
            offset,
            length,
        });
    }
    let mut ranges: Vec<(usize, usize)> = entries.iter().map(|e| (e.offset, e.length)).collect();
    ranges.sort_unstable();
    if ranges.windows(2).any(|w| w[0].0 + w[0].1 > w[1].0) {
        return Err(SectionError::Table);
    }
    Ok(Some(entries))
}

/// Appends sections to a packed library's records: each starts at a multiple of four, and the
/// table and trailer close the file.
pub(crate) struct Writer {
    out: Vec<u8>,
    entries: Vec<Entry>,
    open: Option<(u8, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TooLarge;

impl Writer {
    pub fn new(out: Vec<u8>) -> Writer {
        Writer {
            out,
            entries: Vec::new(),
            open: None,
        }
    }

    pub fn len(&self) -> usize {
        self.out.len()
    }

    /// Opens a section of `len` bytes that starts at a chunk boundary when it would cross one
    /// otherwise.
    pub fn open_whole(&mut self, id: u8, len: usize) -> Result<(), TooLarge> {
        if len > CHUNK {
            return Err(TooLarge);
        }
        let start = self.out.len().next_multiple_of(4);
        if start % CHUNK + len > CHUNK {
            self.out.resize(start.next_multiple_of(CHUNK), 0);
        }
        self.open(id);
        Ok(())
    }

    pub fn open(&mut self, id: u8) {
        debug_assert!(self.open.is_none());
        self.out.resize(self.out.len().next_multiple_of(4), 0);
        self.open = Some((id, self.out.len()));
    }

    pub fn close(&mut self) {
        let (id, offset) = self.open.take().expect("a section is open");
        self.entries.push(Entry {
            id,
            offset,
            length: self.out.len() - offset,
        });
    }

    /// Appends bytes that may cross a boundary, such as an index of fixed-size entries.
    pub fn put(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    pub fn patch(&mut self, at: usize, bytes: &[u8]) {
        self.out[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// Appends an item that stays inside one 16 KB chunk of the file, padding first when it
    /// would cross a boundary, and returns its offset.
    pub fn item(&mut self, bytes: &[u8]) -> Result<u32, TooLarge> {
        if bytes.len() > CHUNK {
            return Err(TooLarge);
        }
        if self.out.len() % CHUNK + bytes.len() > CHUNK {
            self.out.resize(self.out.len().next_multiple_of(CHUNK), 0);
        }
        let offset = u32::try_from(self.out.len()).map_err(|_| TooLarge)?;
        self.out.extend_from_slice(bytes);
        Ok(offset)
    }

    pub fn finish(mut self) -> Result<Vec<u8>, TooLarge> {
        for entry in &self.entries {
            self.out.push(entry.id);
            for word in [entry.offset, entry.length] {
                let word = u32::try_from(word).map_err(|_| TooLarge)?;
                self.out.extend(word.to_le_bytes());
            }
        }
        let count = u8::try_from(self.entries.len()).map_err(|_| TooLarge)?;
        self.out.extend([count, TRAILER_VERSION]);
        self.out.extend(TRAILER_MAGIC);
        u32::try_from(self.out.len()).map_err(|_| TooLarge)?;
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened_at(before: usize, len: usize) -> Result<usize, TooLarge> {
        let mut w = Writer::new(vec![1; before]);
        w.open_whole(9, len)?;
        Ok(w.len())
    }

    #[test]
    fn a_whole_section_starts_where_it_stays_inside_one_chunk() {
        assert_eq!(opened_at(0, CHUNK), Ok(0));
        assert_eq!(opened_at(4, CHUNK), Ok(CHUNK));
        assert_eq!(opened_at(0, CHUNK - 2), Ok(0));
        assert_eq!(opened_at(4, CHUNK - 2), Ok(CHUNK));
        assert_eq!(opened_at(4, CHUNK - 4), Ok(4));
        assert_eq!(opened_at(8, CHUNK - 4), Ok(CHUNK));
        assert_eq!(opened_at(CHUNK + 2, 100), Ok(CHUNK + 4));
        assert_eq!(opened_at(2 * CHUNK - 100, 100), Ok(2 * CHUNK - 100));
        assert_eq!(opened_at(2 * CHUNK - 100, 101), Ok(2 * CHUNK));
        assert_eq!(opened_at(10, CHUNK + 1), Err(TooLarge));
    }
}
