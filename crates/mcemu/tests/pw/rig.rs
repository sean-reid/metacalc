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
pub const MORE: u8 = 45;
pub const NO_WALK: u8 = 46;
pub const PLACE: u8 = 47;
pub const NO_TABLE: u8 = 48;
pub const LINKS: u8 = 49;
pub const SUBGOALS: u8 = 50;
pub const NONE: u8 = 9;

pub const MORE_ENTRY: &str = "PW_MORE";
pub const FIND: &str = "PW_FIND";

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

/// The registers of a start: the number in HL, the typecode in A, the budget in BC (0 for no cut).
pub fn regs_with(n: u16, typecode: u8, budget: u16) -> Regs {
    Regs {
        hl: n,
        bc: budget,
        de: BLOCK,
        ix: 0x1357,
        iy: 0x2468,
        a: typecode,
    }
}

pub fn regs(n: u16, typecode: u8) -> Regs {
    regs_with(n, typecode, 0)
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

/// The find block's fields, little endian.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub goal: u8,
    pub by: u16,
    pub size: u32,
    pub depth: u8,
    pub parent: u32,
    pub index: u8,
    pub step: u16,
    pub begin: u16,
    pub subgoals: Vec<u8>,
}

pub const FIND_SIZE: usize = 47;

pub fn found(b: &[u8]) -> Found {
    let count = usize::from(b[11]);
    Found {
        goal: b[0],
        by: u16::from_le_bytes([b[1], b[2]]),
        size: u32::from_le_bytes([b[3], b[4], b[5], 0]),
        depth: b[6],
        parent: u32::from_le_bytes([b[7], b[8], b[9], 0]),
        index: b[10],
        step: u16::from_le_bytes([b[12], b[13]]),
        begin: u16::from_le_bytes([b[14], b[15]]),
        subgoals: b[16..16 + count].to_vec(),
    }
}

impl Rig {
    pub fn more(&mut self, budget: u16) -> Out {
        self.call(
            MORE_ENTRY,
            Regs {
                hl: 0x1111,
                bc: budget,
                de: BLOCK,
                ix: 0x1357,
                iy: 0x2468,
                a: 0x5A,
            },
        )
    }

    pub fn find_regs(place: u32) -> Regs {
        Regs {
            hl: place as u16,
            bc: 0x7777,
            de: BLOCK,
            ix: 0x1357,
            iy: 0x2468,
            a: (place >> 16) as u8,
        }
    }

    pub fn find(&mut self, place: u32) -> Out {
        self.call(FIND, Rig::find_regs(place))
    }

    pub fn seed_find(&mut self) {
        self.poke(BLOCK, &[MARK; FIND_SIZE]);
    }

    pub fn find_bytes(&mut self) -> Vec<u8> {
        self.peek(BLOCK, FIND_SIZE)
    }

    /// Starts a walk with `budget` and continues it until it stops going on. Returns the last
    /// status and how many times the walk said it had more.
    pub fn sliced(&mut self, n: u16, typecode: u8, budget: u16) -> (u8, usize) {
        let mut out = self.call(SHAPE, regs_with(n, typecode, budget));
        let mut mores = 0;
        while out.status() == MORE {
            mores += 1;
            out = self.more(budget);
            assert!(mores < 5_000_000, "a walk that never ends");
        }
        (out.status(), mores)
    }

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
