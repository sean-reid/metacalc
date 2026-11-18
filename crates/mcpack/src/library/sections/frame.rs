use super::{ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION};
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
    /// The section table is cut short, repeats an id, or points outside the file or into itself.
    Table,
    Titles,
    Glyphs,
    Trees,
}

impl std::fmt::Display for SectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SectionError::Trailer => "the trailer is damaged or of an unknown version",
            SectionError::Table => "the section table is damaged",
            SectionError::Titles => "the titles section is damaged",
            SectionError::Glyphs => "the glyph map is damaged",
            SectionError::Trees => "the parse tree section is damaged",
        })
    }
}

impl std::error::Error for SectionError {}

/// Locates the section table: `None` when the file has no trailer.
pub(crate) fn table(bytes: &[u8]) -> Result<Option<Vec<Entry>>, SectionError> {
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
    for row in bytes[first..start].chunks_exact(ENTRY_LEN) {
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

/// Closes a packed library with its section table and trailer.
pub(crate) struct Writer {
    out: Vec<u8>,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TooLarge;

impl Writer {
    pub fn new(out: Vec<u8>) -> Writer {
        Writer {
            out,
            entries: Vec::new(),
        }
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
