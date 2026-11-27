use crate::damaged::{offset_of, row_of, with};
use crate::findrig::*;
use crate::isolation::pattern;
use crate::secrig::*;
use crate::support::*;
use mcpack::library::LABEL_ORDER;

/// Bytes of stack the header says the deepest exit of the three calls uses below its return address.
pub const STACK_DEPTH: usize = 10;
pub const COUNT: u16 = 40;

pub fn labels() -> Vec<String> {
    (0..COUNT).map(|i| format!("ax-{i}")).collect()
}

pub fn bits() -> Vec<bool> {
    pseudo(usize::from(COUNT), 5)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Stage {
    Closed,
    NoSection,
    Open,
    /// The library ends inside the label order section.
    CutOrder,
    /// The kinds bitmap is where no byte can be read, the label order section is whole.
    CutKinds,
}

pub const STAGES: [Stage; 5] = [
    Stage::Closed,
    Stage::NoSection,
    Stage::Open,
    Stage::CutOrder,
    Stage::CutKinds,
];

pub fn staged_at(stage: Stage, junk: Option<u8>, ports: Option<(u8, u8, bool)>) -> Rig {
    let good = library(&labels(), Some(&bits()));
    let bytes = if stage == Stage::NoSection {
        with(&good, |b| b[row_of(&good, LABEL_ORDER)] = 77)
    } else {
        good.clone()
    };
    let mut rig = Rig::new();
    if let Some(flip) = junk {
        for page in 0..8u8 {
            for (at, byte) in rig.m.ram_page(page).iter_mut().enumerate() {
                *byte = pattern(page, at) ^ flip;
            }
        }
    }
    if let Some((port5, port7, ei)) = ports {
        rig.m.out_port(5, port5);
        rig.m.out_port(7, port7);
        rig.ei = ei;
    }
    rig.load(&bytes);
    if stage != Stage::Closed {
        rig.open_ok();
    }
    match stage {
        Stage::CutOrder => {
            let at = offset_of(&good, LABEL_ORDER) + 5;
            rig.poke(rig.label("LR_LENGTH"), &at.to_le_bytes()[..3]);
        }
        Stage::CutKinds => {
            let end = rig.state_word("LR_LENGTH", 3);
            rig.poke(rig.label("LS_KD"), &end.to_le_bytes()[..3]);
        }
        _ => {}
    }
    rig
}

pub struct Exit {
    pub name: &'static str,
    pub target: &'static str,
    pub regs: Regs,
    pub status: u8,
    /// Prepares the caller's memory.
    pub pokes: Vec<(u16, Vec<u8>)>,
}

fn range_exit(name: &'static str, prefix: &[u8], len: u8, at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: RANGE,
        regs: range_regs(at, len),
        status,
        pokes: if at < 0xC000 {
            vec![(at, prefix.to_vec())]
        } else {
            Vec::new()
        },
    }
}

fn walk_exit(name: &'static str, b: [u8; 9], at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: WALK,
        regs: walk_regs(at),
        status,
        pokes: vec![(BLOCK_AT, b.to_vec())],
    }
}

fn copy_exit(name: &'static str, first: u16, count: u16, at: u16, status: u8) -> Exit {
    Exit {
        name,
        target: COPY,
        regs: copy_regs(first, count, at),
        status,
        pokes: vec![(COPY_AT, vec![MARK; 0x100])],
    }
}

pub fn exits(stage: Stage) -> Vec<Exit> {
    let whole = |kind| block(0, COUNT, 0, COUNT, kind);
    match stage {
        Stage::Closed => vec![
            range_exit("closed range", b"ax-1", 4, PREFIX_AT, ST_CLOSED),
            walk_exit("closed walk", whole(EVERY), BLOCK_AT, ST_CLOSED),
            copy_exit("closed copy", 0, 5, COPY_AT, ST_CLOSED),
        ],
        Stage::NoSection => vec![
            range_exit("none range", b"ax-1", 4, PREFIX_AT, ST_NONE),
            walk_exit("none walk", whole(EVERY), BLOCK_AT, ST_NONE),
            copy_exit("none copy", 0, 5, COPY_AT, ST_NONE),
        ],
        Stage::CutOrder => vec![
            range_exit("cut range", b"ax-1", 4, PREFIX_AT, ST_BAD),
            walk_exit("cut walk", whole(EVERY), BLOCK_AT, ST_BAD),
            copy_exit("cut copy", 0, 5, COPY_AT, ST_BAD),
        ],
        Stage::CutKinds => vec![walk_exit(
            "cut kinds walk",
            whole(THEOREM),
            BLOCK_AT,
            ST_BAD,
        )],
        Stage::Open => vec![
            range_exit("range match", b"ax-1", 4, PREFIX_AT, ST_OK),
            range_exit("range none after", b"q", 1, PREFIX_AT, ST_OK),
            range_exit("range none before", b",", 1, PREFIX_AT, ST_OK),
            range_exit("range length 0", b"a", 0, PREFIX_AT, ST_ARG),
            range_exit("range length 9", b"ax-1ax-1a", 9, PREFIX_AT, ST_ARG),
            range_exit("range pointer", b"ax-1", 4, 0xC000, ST_ARG),
            range_exit(
                "range pointer at the end of bank B",
                b"ax-1",
                4,
                0xBFFC,
                ST_OK,
            ),
            walk_exit("walk every", whole(EVERY), BLOCK_AT, ST_OK),
            walk_exit("walk theorem", whole(THEOREM), BLOCK_AT, ST_OK),
            walk_exit(
                "walk axiom none",
                block(0, 1, 0, COUNT, AXIOM),
                BLOCK_AT,
                ST_OK,
            ),
            walk_exit(
                "walk empty range",
                block(3, 0, 0, COUNT, THEOREM),
                BLOCK_AT,
                ST_OK,
            ),
            walk_exit("walk bad kind", whole(2), BLOCK_AT, ST_ARG),
            walk_exit("walk bad block", whole(EVERY), 0xC000, ST_ARG),
            walk_exit(
                "walk bad range",
                block(0, COUNT + 1, 0, COUNT, EVERY),
                BLOCK_AT,
                ST_INDEX,
            ),
            walk_exit(
                "walk bad bound",
                block(0, COUNT, 0, COUNT + 1, EVERY),
                BLOCK_AT,
                ST_INDEX,
            ),
            walk_exit(
                "walk bad start",
                block(0, COUNT, 5, 4, EVERY),
                BLOCK_AT,
                ST_INDEX,
            ),
            copy_exit("copy run", 0, 5, COPY_AT, ST_OK),
            copy_exit("copy long run", 0, COUNT, COPY_AT, ST_OK),
            copy_exit("copy no entries", 3, 0, 0xC000, ST_OK),
            copy_exit("copy bad buffer", 0, 5, 0xC000, ST_ARG),
            copy_exit("copy bad range", 0, COUNT + 1, COPY_AT, ST_INDEX),
        ],
    }
}

pub fn run(rig: &mut Rig, e: &Exit) -> Out {
    for (at, bytes) in &e.pokes {
        rig.poke(*at, bytes);
    }
    rig.call(e.target, e.regs)
}
