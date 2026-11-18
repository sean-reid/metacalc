//! The sections after the last record and the trailer that locates them.

mod frame;

pub(crate) use frame::Writer;
pub(super) use frame::table;
pub use frame::{Entry, SectionError};

pub const TRAILER_MAGIC: &[u8; 4] = b"MCSX";
pub const TRAILER_VERSION: u8 = 1;
pub const TRAILER_LEN: usize = 6;
pub const ENTRY_LEN: usize = 9;

/// What a library carries after its records.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sections {
    /// Every section of the table, including those no reader here understands.
    pub table: Vec<Entry>,
}

/// The sections of a library's bytes, or `None` when it has no trailer.
pub fn read(bytes: &[u8]) -> Result<Option<Sections>, SectionError> {
    Ok(table(bytes)?.map(|table| Sections { table }))
}
