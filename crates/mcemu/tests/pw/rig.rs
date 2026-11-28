use crate::proofs::*;
use crate::support::*;

/// The reader and the walk in one image, which fits the page the tests run from.
pub const PW: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/pw_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/pw_test.lab")),
    org: 0x4000,
};

pub const SHAPE: &str = "PW_SHAPE";

/// Where the caller keeps the walk's result block, in bank B.
pub const BLOCK: u16 = 0xA200;
pub const MARK: u8 = 0xE5;

/// The statuses of the walk's header, written out here and not read from the image.
pub const UNDERFLOW: u8 = 32;
pub const BAD_REF: u8 = 33;
pub const FORWARD: u8 = 34;
pub const SAVE_EMPTY: u8 = 35;
pub const NO_ENTRY: u8 = 36;
pub const MANY_ENTRIES: u8 = 37;
pub const GOALS: u8 = 38;
pub const STACK: u8 = 39;
pub const SAVED: u8 = 40;
pub const CUT: u8 = 41;
pub const SYNTAX: u8 = 42;
pub const TOTAL: u8 = 43;
pub const REFS: u8 = 44;
pub const NONE: u8 = 9;

/// What the walk writes to the caller's block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub steps: u16,
    pub goals: u8,
    pub links: u16,
    pub total: u32,
}

impl Counts {
    pub fn of(shape: Shape) -> Counts {
        Counts {
            steps: shape.steps as u16,
            goals: shape.goals as u8,
            links: shape.links as u16,
            total: shape.total as u32,
        }
    }
}

pub fn regs(n: u16, typecode: u8) -> Regs {
    Regs {
        hl: n,
        bc: u16::from(typecode) << 8 | 0x77,
        de: BLOCK,
        ix: 0x1357,
        iy: 0x2468,
        a: 0x5A,
    }
}

/// The id of the provable typecode, `|-`, read from the symbol table the format lays out: after
/// the 20 byte header, one entry for each constant and variable, a typecode byte, a length and the
/// name.
pub fn provable(bytes: &[u8]) -> Option<u8> {
    let symbols = usize::from(bytes[12]) + usize::from(bytes[13]);
    let mut at = 20;
    for id in 0..symbols {
        let len = usize::from(bytes[at + 1]);
        if bytes[at] == 0xFF && &bytes[at + 2..at + 2 + len] == b"|-" {
            return Some(id as u8);
        }
        at += 2 + len;
    }
    None
}

impl Rig {
    pub fn seed_result(&mut self) {
        self.poke(BLOCK, &[MARK; 8]);
    }

    pub fn result(&mut self) -> Counts {
        let b = self.peek(BLOCK, 8);
        Counts {
            steps: u16::from_le_bytes([b[0], b[1]]),
            goals: b[2],
            links: u16::from_le_bytes([b[3], b[4]]),
            total: u32::from_le_bytes([b[5], b[6], b[7], 0]),
        }
    }

    pub fn result_bytes(&mut self) -> Vec<u8> {
        self.peek(BLOCK, 8)
    }

    pub fn shape(&mut self, n: u16, typecode: u8) -> Out {
        self.call(SHAPE, regs(n, typecode))
    }
}

/// A rig on the walk's image with `bytes` loaded and the reader open.
pub fn opened(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_image(PW);
    rig.load(bytes);
    rig.open_ok();
    rig
}
