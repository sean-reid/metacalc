//! The sections after the last record and the trailer that locates them.

use super::{Assertion, CHUNK, PackError};
use crate::mm::{Database, Kind, StmtId, Sym};
use std::collections::HashMap;

mod frame;
mod glyphs;
mod kinds;
mod shapes;
mod titles;
mod trees;
mod truth;

pub(crate) use frame::Writer;
use frame::table;
pub use frame::table as locate;
pub use frame::{Entry, SectionError};
pub use glyphs::{GlyphMap, Glyphs};
pub use kinds::Kinds;
pub use shapes::{SHAPE_CAPACITY, Shapes};
pub use titles::Title;
pub(super) use titles::select as select_titles;
pub use trees::{AXIOM_CAPACITY, Node, Part, SyntaxAxiom, Tree, Trees, VARIABLE_CAPACITY};
pub use truth::{SectionCheckError, titles_in_order, verify_sections};

pub const TRAILER_MAGIC: &[u8; 4] = b"MCSX";
pub const TRAILER_VERSION: u8 = 1;
pub const TRAILER_LEN: usize = 6;
pub const ENTRY_LEN: usize = 9;

pub const TITLES: u8 = 1;
pub const GLYPHS: u8 = 2;
pub const TREES: u8 = 3;
pub const SHAPES: u8 = 4;
pub const KINDS: u8 = 5;

/// What a library carries after its records.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sections {
    /// Every section of the table, including those no reader here understands.
    pub table: Vec<Entry>,
    pub titles: Option<Vec<Title>>,
    pub glyphs: Option<GlyphMap>,
    pub trees: Option<Trees>,
    pub shapes: Option<Shapes>,
    pub kinds: Option<Kinds>,
}

/// A library's own parts, which the sections are read against.
pub(crate) struct Ctx<'a> {
    pub assertions: &'a [Assertion],
    /// Every symbol's name by id, constants first.
    pub names: &'a [String],
    pub constants: usize,
    /// Where the symbol table, the directory and the last record have all ended.
    pub records_end: usize,
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

pub(crate) fn read(bytes: &[u8], ctx: &Ctx) -> Result<Option<Sections>, SectionError> {
    let Some(table) = table(bytes)? else {
        return Ok(None);
    };
    if table
        .iter()
        .any(|entry| entry.offset < ctx.records_end || entry.offset % 4 != 0)
    {
        return Err(SectionError::Table);
    }
    let find = |id: u8| table.iter().find(|entry| entry.id == id);
    let titles = find(TITLES)
        .map(|entry| titles::decode(bytes, entry, ctx.assertions.len()))
        .transpose()?;
    let glyphs = find(GLYPHS)
        .map(|entry| glyphs::decode(bytes, entry, ctx.names.len()))
        .transpose()?;
    let trees = find(TREES)
        .map(|entry| trees::decode(bytes, entry, ctx))
        .transpose()?;
    let shapes = match (&trees, find(SHAPES)) {
        (Some(trees), Some(entry)) => Some(shapes::decode(bytes, entry, trees)?),
        (None, None) => None,
        _ => return Err(SectionError::Shapes),
    };
    let kinds = find(KINDS)
        .map(|entry| kinds::decode(bytes, entry, ctx.assertions.len()))
        .transpose()?;
    Ok(Some(Sections {
        table,
        titles,
        glyphs,
        trees,
        shapes,
        kinds,
    }))
}

/// What the packer knows about a library's statements, for the tree section.
pub(super) struct TreeInput<'a> {
    pub db: &'a Database,
    pub assertions: &'a [StmtId],
    pub local_sym: &'a HashMap<Sym, u8>,
    pub local_stmt: &'a HashMap<StmtId, u16>,
    pub constants: usize,
}

/// Appends the sections of a library whose records `out` holds, and closes it with the table
/// and trailer.
pub(super) fn append(
    out: Vec<u8>,
    titles: &[Title],
    glyphs: &GlyphMap,
    input: &TreeInput,
) -> Result<Vec<u8>, PackError> {
    let mut writer = Writer::new(out);
    titles::encode(&mut writer, titles)?;
    glyphs::encode(&mut writer, glyphs)?;
    let trees = trees::prepare(
        input.db,
        input.assertions,
        input.local_sym,
        input.local_stmt,
        input.constants,
    )?;
    if let Some(trees) = &trees {
        trees::encode(&mut writer, trees)?;
        shapes::encode(&mut writer, trees.shapes())?;
    }
    let theorems: Vec<bool> = input
        .assertions
        .iter()
        .map(|&id| input.db.stmt(id).kind == Kind::Theorem)
        .collect();
    kinds::encode(&mut writer, &theorems)?;
    writer.finish().map_err(|_| PackError::SectionTooLarge)
}
