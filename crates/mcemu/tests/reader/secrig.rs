use crate::support::*;
use mcpack::library::{GLYPHS, TITLES, TREES};

pub const ST_TRAILER: u8 = 6;
pub const ST_TABLE: u8 = 7;
pub const ST_SECTION: u8 = 8;
pub const ST_NONE: u8 = 9;
pub const ST_GLYPHS: u8 = 10;

pub const HAVE_TITLES: u8 = 1;
pub const HAVE_GLYPHS: u8 = 2;
pub const HAVE_TREES: u8 = 4;
pub const FONT_MATCHES: u8 = 8;

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

    /// One call of lib_node on the bytes, which are placed at `at`: (status, kind, value, the
    /// pointer after, the count left).
    pub fn node(&mut self, bytes: &[u8], at: u16) -> (u8, u8, u16, u16, u8) {
        self.poke(at, bytes);
        let regs = Regs {
            hl: at,
            bc: bytes.len() as u16,
            ..Regs::default()
        };
        let out = self.call("LIB_NODE", regs);
        (
            out.status(),
            (out.bc >> 8) as u8,
            out.de,
            out.hl,
            out.bc as u8,
        )
    }

    /// Walks the whole tree in `bytes` through the iterator, refilling the 255 byte buffer as a
    /// caller would. Returns the nodes as (kind, value) and the status that ended the walk.
    pub fn walk(&mut self, bytes: &[u8]) -> (u8, Vec<(u8, u16)>) {
        let mut nodes = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let chunk = &bytes[at..bytes.len().min(at + 255)];
            self.poke(BUF, chunk);
            let regs = Regs {
                hl: BUF,
                bc: chunk.len() as u16,
                ..Regs::default()
            };
            let out = self.call("CALL_WALK", regs);
            let count = usize::from(self.peek(self.label("NODE_COUNT"), 1)[0]);
            let list = self.peek(self.label("NODES"), 3 * count);
            nodes.extend(
                list.chunks(3)
                    .map(|n| (n[0], u16::from_le_bytes([n[1], n[2]]))),
            );
            let used = usize::from(out.hl - BUF);
            at += used;
            match out.status() {
                ST_OK => {}
                ST_RANGE if used > 0 && at < bytes.len() => {}
                status => return (status, nodes),
            }
        }
        (ST_OK, nodes)
    }

    /// The ls_ fields: (have, [(offset, length, first item); 3], title count, fingerprint,
    /// syntax count).
    pub fn section_state(&mut self) -> SectionState {
        let have = self.state_byte("LS_HAVE");
        let mut rows = [(0, 0, 0); 3];
        for (row, name) in rows.iter_mut().zip(["LS_TI", "LS_GL", "LS_TR"]) {
            let b = self.named(name, 9);
            *row = (word(&b[0..3]), word(&b[3..6]), word(&b[6..9]));
        }
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
    pub rows: [(u32, u32, u32); 3],
    pub titles: u16,
    pub fingerprint: u16,
    pub syntax: u16,
}

pub fn ids() -> [u8; 3] {
    [TITLES, GLYPHS, TREES]
}
