use crate::support::*;
use mcpack::library::{GLYPHS, SHAPES, TITLES, TREES};

pub const ST_TRAILER: u8 = 6;
pub const ST_TABLE: u8 = 7;
pub const ST_SECTION: u8 = 8;
pub const ST_NONE: u8 = 9;
pub const ST_GLYPHS: u8 = 10;

pub const HAVE_TITLES: u8 = 1;
pub const HAVE_GLYPHS: u8 = 2;
pub const HAVE_TREES: u8 = 4;
pub const FONT_MATCHES: u8 = 8;
pub const HAVE_SHAPES: u8 = 16;
pub const HAVE_KINDS: u8 = 32;

/// The foreign-font image keeps a fingerprint the packer never writes.
pub const FOREIGN: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/reader_foreign.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/reader_foreign.lab")),
    org: 0x4000,
};

pub struct Tree {
    pub status: u8,
    pub len: u16,
    pub offset: u32,
    pub bytes: Vec<u8>,
}

fn word(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .rev()
        .fold(0, |acc, &b| (acc << 8) | u32::from(b))
}

impl Rig {
    pub fn sections(&mut self) -> (u8, u8) {
        let out = self.call("LIB_SECTIONS", Regs::default());
        (out.status(), (out.bc >> 8) as u8)
    }

    pub fn title_count(&mut self) -> (u8, u16) {
        let out = self.call("LIB_TITLE_COUNT", Regs::default());
        (out.status(), out.hl)
    }

    /// (status, assertion, depth, text).
    pub fn title(&mut self, n: u16) -> (u8, u16, u8, Vec<u8>) {
        let regs = Regs {
            hl: n,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_TITLE", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), out.hl, out.bc as u8, self.peek(BUF, len))
    }

    pub fn title_for(&mut self, k: u16) -> (u8, u16) {
        let regs = Regs {
            hl: k,
            ..Regs::default()
        };
        let out = self.call("LIB_TITLE_FOR", regs);
        (out.status(), out.hl)
    }

    pub fn glyph_map(&mut self) -> (u8, u16) {
        let out = self.call("LIB_GLYPH_MAP", Regs::default());
        (out.status(), out.hl)
    }

    /// (status, offset, length) of the glyph section.
    pub fn glyphs(&mut self) -> (u8, u32, u16) {
        let out = self.call("LIB_GLYPHS", Regs::default());
        let offset = (u32::from(out.de as u8) << 16) | u32::from(out.hl);
        (out.status(), offset, out.bc)
    }

    /// (status, flags, codes).
    pub fn glyph(&mut self, id: u8) -> (u8, u8, Vec<u8>) {
        let regs = Regs {
            a: id,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_GLYPH", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), out.bc as u8, self.peek(BUF, len))
    }

    pub fn syntax_count(&mut self) -> (u8, u16) {
        let out = self.call("LIB_SYNTAX_COUNT", Regs::default());
        (out.status(), out.hl)
    }

    /// (status, assertion, arity).
    pub fn syntax(&mut self, i: u16) -> (u8, u16, u8) {
        let regs = Regs {
            hl: i,
            ..Regs::default()
        };
        let out = self.call("LIB_SYNTAX", regs);
        (out.status(), out.hl, (out.bc >> 8) as u8)
    }

    /// One call of lib_arities with a buffer of `size` bytes at `buf`: (status, the count it
    /// says it wrote, the 512 bytes from `buf`, fewer at the top of memory).
    pub fn arities(&mut self, buf: u16, size: u16) -> (u8, u16, Vec<u8>) {
        let regs = Regs {
            de: buf,
            bc: size,
            ..Regs::default()
        };
        let out = self.call("LIB_ARITIES", regs);
        (
            out.status(),
            out.hl,
            self.peek(buf, 512.min(0x1_0000 - usize::from(buf))),
        )
    }

    /// (status, offset, length).
    pub fn shapes(&mut self) -> (u8, u32, u16) {
        let out = self.call("LIB_SHAPES", Regs::default());
        (
            out.status(),
            (u32::from(out.de & 0xFF) << 16) | u32::from(out.hl),
            out.bc,
        )
    }

    /// (status, the part bytes).
    pub fn shape(&mut self, i: u16) -> (u8, Vec<u8>) {
        let regs = Regs {
            hl: i,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_SHAPE", regs);
        let len = usize::from(out.bc >> 8);
        (out.status(), self.peek(BUF, len))
    }

    /// Statement `k` of assertion `n`, FFh for the conclusion. `bytes` holds what the call copied.
    pub fn tree(&mut self, n: u16, k: u8) -> Tree {
        let regs = Regs {
            hl: n,
            bc: u16::from(k) << 8,
            de: BUF,
            ..Regs::default()
        };
        let out = self.call("LIB_TREE", regs);
        let len = out.bc;
        let bytes = if out.status() == ST_OK {
            self.peek(BUF, usize::from(len.min(255)))
        } else {
            Vec::new()
        };
        Tree {
            status: out.status(),
            len,
            offset: (u32::from(out.de) << 16) | u32::from(out.hl),
            bytes,
        }
    }

    /// The ls_ fields: (have, [(offset, length, first item); 4], title count, fingerprint,
    /// syntax count).
    pub fn section_state(&mut self) -> SectionState {
        let have = self.state_byte("LS_HAVE");
        let mut rows = [(0, 0, 0); 4];
        for (row, name) in rows.iter_mut().zip(["LS_TI", "LS_GL", "LS_TR"]) {
            let b = self.named(name, 9);
            *row = (word(&b[0..3]), word(&b[3..6]), word(&b[6..9]));
        }
        let b = self.named("LS_SH", 5);
        rows[3] = (word(&b[0..3]), word(&b[3..5]), 0);
        SectionState {
            have,
            rows,
            titles: word(&self.named("LS_TI_N", 2)) as u16,
            fingerprint: word(&self.named("LS_GL_FP", 2)) as u16,
            syntax: word(&self.named("LS_TR_N", 2)) as u16,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SectionState {
    pub have: u8,
    pub rows: [(u32, u32, u32); 4],
    pub titles: u16,
    pub fingerprint: u16,
    pub syntax: u16,
}

pub fn ids() -> [u8; 4] {
    [TITLES, GLYPHS, TREES, SHAPES]
}

/// The bit of `ls_have` that says the section with `id` is there.
pub fn have_bit(id: u8) -> u8 {
    if id == SHAPES {
        HAVE_SHAPES
    } else {
        1 << (id - 1)
    }
}
