//! The binary library a calculator reads, and a host model of the kernel that checks it.
//!
//! All integers are little-endian. A library is a header, a symbol table, a directory and one
//! record per assertion. Nothing but the directory crosses a multiple of `CHUNK`, so every
//! record can be read through one mapped 16 KB flash page.
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
//! section    starts at a multiple of four; no item of it crosses a multiple of CHUNK, so every
//!            item can be read through one mapped page; only an index of fixed-size entries
//!            may cross, and the padding that keeps an item whole is zero bytes
//! table      one row per section: id (u8), offset (u32), length (u32), the rows right
//!            before the trailer, in section order; a reader skips an id it does not know
//! trailer    section count (u8), format version (u8), "MCSX"
//! ```
//!
//! A record is only usable if its float hypotheses name distinct variables, each of its
//! expressions starts with a constant and uses no variable without a float hypothesis, every
//! pair in its first list names two different variables that both have one, and every pair in a
//! theorem's second list names two different variables of which at least one has none.

mod check;
mod export;
mod kernel;
mod pack;
mod read;
mod sections;

pub use check::{CheckError, check};
pub use export::{ExportError, export};
pub use kernel::{Kernel, KernelError, Limits, WORK_LIMIT};
pub use pack::{PackError, pack, pack_kernel};
pub use read::{
    Assertion, Hypothesis, Library, ReadError, Reference, Step, Variable, decode, read,
};
pub use sections::{
    ENTRY_LEN, Entry, SectionError, Sections, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION,
};

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
