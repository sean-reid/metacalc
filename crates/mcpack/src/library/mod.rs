//! The binary library a calculator reads, and a host model of the kernel that checks it.
//!
//! All integers are little-endian. A library is a header, a symbol table, a directory and one
//! record per assertion. Nothing but the directory crosses a multiple of `CHUNK`, so every
//! record can be read through one mapped 16 KB flash page. The symbol table ends inside the
//! first chunk.
//!
//! ```text
//! header     "MCL1", crc32 of everything after it (u32), length (u32),
//!            constants (u8), variables (u8), assertions (u16), directory offset (u32)
//! symbols    constants then variables, a symbol's id being its position:
//!            typecode id for a variable or FFh for a constant, name length (u8), name
//! directory  one u32 offset per assertion, starting at a multiple of four
//! record     kind (0 axiom, 1 theorem), label length (u8), label,
//!            hypothesis count (u8), each 0 + variable or 1 + length (u8) + symbols,
//!            disjoint pair count (u16), pairs, conclusion length (u8), symbols,
//!            and for a theorem:
//!            pair count (u16) and the disjoint pairs that involve a dummy variable,
//!            reference count (u16), each 8000h + dummy variable or an assertion number,
//!            proof length (u16), proof
//! hypotheses a record's floats come first and name variables in strictly rising id
//! references in first-use order: a proof's first step naming an entry names entry 0, the next
//!            new one entry 1, and so on, every entry is named, no two entries are equal, and
//!            a dummy entry never names a variable with a float hypothesis of its own
//! proof      one number per step: below the hypothesis count a hypothesis, then a
//!            reference, then a saved step; a byte under F0h is the number itself,
//!            F0h..FEh carries eight more bits in the next byte, FFh saves the top of the stack
//! ```
//!
//! Sections follow the last record. The kernel reads nothing after it, so a library without them
//! is as valid as one with them, and the header's length and CRC cover them either way. A
//! reader that knows the length from the header finds the trailer in the last bytes of the file.
//!
//! ```text
//! section    starts at a multiple of four, after the symbol table, the directory and every
//!            record; no item of it crosses a multiple of CHUNK, so every
//!            item can be read through one mapped page; only an index of fixed-size entries
//!            may cross, and the padding that keeps an item whole is zero bytes
//! table      one row per section: id (u8), offset (u32), length (u32), the rows right
//!            before the trailer, in section order; a reader skips an id it does not know
//! trailer    section count (u8), format version (u8), "MCSX"
//!
//! titles     id 1: count (u16), one u32 file offset per title, then the titles: the number of
//!            the first assertion of the library under the heading (u16), depth (u8, 1 to 4,
//!            1 outermost), length (u8), ASCII text; in source order, assertion numbers never
//!            falling, only headings with an assertion under them or under a heading inside them
//! glyphs     id 2: fingerprint of the font table (u16), symbol count (u8), one u16 offset from
//!            the section's start per symbol, then per symbol: spacing flags (u8: bit 0 gap
//!            before, 1 gap after, 2 gap beside another var, 3 no gap after), glyph count (u8),
//!            the glyph codes of the main display font
//! trees      id 3: syntax axiom count (u16, at most 383), per axiom its assertion number (u16)
//!            and its arity (u8), in rising assertion number; a syntax axiom has only float
//!            hypotheses and its statement holds each of their variables once; then one u32
//!            file offset per assertion, then per assertion a tree for each essential
//!            hypothesis in record order and one for the conclusion; a tree is its
//!            length in bytes (u8, FEh for no tree, FFh then a u16) and its nodes in preorder:
//!            00h..7Eh is a syntax table index, 7Fh then n is index 127 + n, and the arity of
//!            the axiom says how many subtrees follow; 80h..FEh is the variable with symbol id
//!            constants + byte - 80h, FFh then n the one with 127 + n; a library whose
//!            database has no grammar has no trees section; identical trees are stored once
//!            per use, and storing them once would save about 17.6 KB in the ring library
//! shapes     id 4: syntax axiom count (u16, equal to the trees section's), one u16 offset from the
//!            section's start per axiom and one past the last, then per axiom, in the syntax
//!            table's order, its parts after the typecode in reading order, one byte each: 00h
//!            a constant, k + 1 the child k (0-based, in the order the tree lists children); an
//!            axiom has 1 to 255 parts, every child 0 to arity - 1 appears exactly once, and
//!            the count of non-zero bytes is the arity; the first offset is 2 * (count + 2),
//!            the offsets strictly rise, and the last is the section's length; a library has a
//!            shapes section exactly when it has a trees section; the decoder checks that each
//!            item lies inside one chunk, and the packer keeps the whole section inside one
//!            chunk, so no item is padded
//! kinds      id 5: assertion count (u16, equal to the library's), then one bit per assertion in
//!            assertion order, set for a theorem, assertion 0 in the low bit of the first byte;
//!            the length is 2 + the count over 8 rounded up and the bits past the last assertion
//!            are zero; it may cross a multiple of CHUNK, and the library check compares it
//!            with every record's kind byte
//! label order id 6, written before kinds: assertion count (u16, equal to the library's), then
//!            every assertion number once, two bytes each, sorted by label bytewise, so the
//!            length is 2 + 2 * count; it may cross a multiple of CHUNK between two entries, and
//!            the decoder refuses a count or length that does not fit, a number at or above the
//!            count, a repeat, and two neighbours out of label order
//! ```
//!
//! A record is only usable if its float hypotheses come before its essential ones and name
//! distinct variables that its essential hypotheses and conclusion use, each of its expressions
//! starts with a constant and uses no variable without a float hypothesis, every
//! pair in its first list names two different variables that both have one, and every pair in a
//! theorem's second list names two different variables of which at least one has none.

mod check;
mod export;
mod kernel;
mod labels;
mod numbering;
mod pack;
mod read;
mod sections;
mod stages;

pub use check::{CheckError, HOST_LIMITS, check};
pub use export::{Export, ExportError, export, export_accepted, export_within};
pub use kernel::{CACHE_IDS, Kernel, KernelError, Limits, WORK_LIMIT};
pub use pack::{PackError, pack, pack_kernel};
pub use read::{
    Assertion, Hypothesis, Library, ReadError, Reference, Step, Variable, decode, read,
};
pub use sections::{
    AXIOM_CAPACITY, ENTRY_LEN, Entry, GLYPHS, GlyphMap, Glyphs, KINDS, Kinds, LABEL_ORDER,
    LabelOrder, Node, Part, SHAPE_CAPACITY, SHAPES, SectionCheckError, SectionError, Sections,
    Shapes, SyntaxAxiom, TITLES, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION, TREES, Title, Tree,
    Trees, VARIABLE_CAPACITY, locate as locate_sections, titles_in_order, verify_sections,
};
pub use stages::{Stage, StagesError, parse as parse_stages, render as render_stages, stages};

pub const MAGIC: &[u8; 4] = b"MCL1";
pub const CHUNK: usize = 1 << 14;
pub const HEADER_LEN: usize = 20;
pub(crate) const DUMMY: u16 = 0x8000;
pub(crate) const WIDE: u8 = 0xF0;
pub(crate) const SAVE: u8 = 0xFF;

/// IEEE CRC-32, bit by bit as the calculator computes it.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}
