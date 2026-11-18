//! The sections after the last record and the trailer that locates them.

use super::{CHUNK, HEADER_LEN, PackError};

mod frame;
mod glyphs;
mod titles;
mod truth;

pub(crate) use frame::Writer;
pub(super) use frame::table;
pub use frame::{Entry, SectionError};
pub use glyphs::{GlyphMap, Glyphs};
pub use titles::Title;
pub(super) use titles::select as select_titles;
pub use truth::{SectionCheckError, verify_sections};

pub const TRAILER_MAGIC: &[u8; 4] = b"MCSX";
pub const TRAILER_VERSION: u8 = 1;
pub const TRAILER_LEN: usize = 6;
pub const ENTRY_LEN: usize = 9;

pub const TITLES: u8 = 1;
pub const GLYPHS: u8 = 2;

/// What a library carries after its records.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sections {
    /// Every section of the table, including those no reader here understands.
    pub table: Vec<Entry>,
    pub titles: Option<Vec<Title>>,
    pub glyphs: Option<GlyphMap>,
}

/// The sizes of a library's own parts that the sections must agree with.
pub(crate) struct Counts {
    pub assertions: usize,
    pub symbols: usize,
}

/// A bounded reader over a file range, which every section decoder reads through.
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8], pos: usize, end: usize) -> Self {
        Cursor { bytes, pos, end }
    }

    pub fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len).filter(|&end| end <= self.end)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    pub fn u8(&mut self) -> Option<usize> {
        Some(self.take(1)?[0] as usize)
    }

    pub fn u16(&mut self) -> Option<usize> {
        let b = self.take(2)?;
        Some(u16::from_le_bytes([b[0], b[1]]) as usize)
    }

    pub fn u32(&mut self) -> Option<usize> {
        let b = self.take(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }
}

/// Whether the `len` bytes at `at` sit inside one chunk of the file.
pub(crate) fn within_chunk(at: usize, len: usize) -> bool {
    at / CHUNK == (at + len.max(1) - 1) / CHUNK
}

/// The sections of a library's bytes, or `None` when it has no trailer.
pub fn read(bytes: &[u8]) -> Result<Option<Sections>, SectionError> {
    let header = bytes.get(..HEADER_LEN).ok_or(SectionError::Trailer)?;
    read_with(
        bytes,
        &Counts {
            assertions: u16::from_le_bytes([header[14], header[15]]) as usize,
            symbols: header[12] as usize + header[13] as usize,
        },
    )
}

pub(crate) fn read_with(bytes: &[u8], counts: &Counts) -> Result<Option<Sections>, SectionError> {
    let Some(table) = table(bytes)? else {
        return Ok(None);
    };
    let find = |id: u8| table.iter().find(|entry| entry.id == id);
    let titles = find(TITLES)
        .map(|entry| titles::decode(bytes, entry, counts.assertions))
        .transpose()?;
    let glyphs = find(GLYPHS)
        .map(|entry| glyphs::decode(bytes, entry, counts))
        .transpose()?;
    Ok(Some(Sections {
        table,
        titles,
        glyphs,
    }))
}

/// Appends the sections of a library whose records `out` holds, and closes it with the table
/// and trailer.
pub(super) fn append(
    out: Vec<u8>,
    titles: &[Title],
    glyphs: &GlyphMap,
) -> Result<Vec<u8>, PackError> {
    let mut writer = Writer::new(out);
    titles::encode(&mut writer, titles)?;
    glyphs::encode(&mut writer, glyphs)?;
    writer.finish().map_err(|_| PackError::SectionTooLarge)
}
