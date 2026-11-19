use crate::errors::Refusal;
use crate::fixture;
use crate::rig::*;
use crate::session::allowed_for;
use crate::support::*;
use mcpack::library::SHAPES;

/// A refused call built by damaging the packed bytes after the host has packed them, which the
/// host decoder would refuse: the machine as it stands before the call, and what the call must
/// leave of fm's state.
pub struct Damaged {
    pub refusal: Refusal,
    pub kind: Kind,
}

#[derive(Clone, Copy)]
pub enum Kind {
    /// fm_open: fm ends closed.
    Open,
    /// fm_nodes: the statement of this many tokens stays loaded and no table is built.
    Nodes(usize),
}

fn word(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn get(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn assertion(bytes: &[u8], label: &str) -> u16 {
    let library = decoded(bytes);
    library
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap() as u16
}

/// Where the shapes section starts and how many axioms it holds.
fn shapes(bytes: &[u8]) -> (usize, usize) {
    let table = decoded(bytes).sections.unwrap().table;
    let row = table.iter().find(|e| e.id == SHAPES).unwrap();
    (row.offset, get(bytes, row.offset))
}

/// The start of axiom `i`'s shape and one past its end in the library file.
fn item(bytes: &[u8], i: usize) -> (usize, usize) {
    let (start, _) = shapes(bytes);
    let at = start + 2 + 2 * i;
    (start + get(bytes, at), start + get(bytes, at + 2))
}

fn shape_with(bytes: &[u8], parts: &[u8]) -> usize {
    let (_, count) = shapes(bytes);
    (0..count)
        .find(|&i| {
            let (from, to) = item(bytes, i);
            bytes[from..to] == *parts
        })
        .unwrap()
}

fn open_case(
    what: &'static str,
    status: &str,
    damage: impl FnOnce(&mut Vec<u8>),
    prepare: impl FnOnce(&mut Fm),
) -> Damaged {
    let good = fixture::nodes_library();
    let mut fm = Fm::opened_primed(&good);
    assert_eq!(fm.statement(assertion(&good, "ax-1"), 0xFF).status(), 0);
    let mut bytes = good;
    damage(&mut bytes);
    fm.rig.load(&bytes);
    prepare(&mut fm);
    Damaged {
        refusal: Refusal {
            what,
            fm,
            target: "FM_OPEN",
            regs: Regs::default(),
            status: code(status),
        },
        kind: Kind::Open,
    }
}

fn nodes_case(
    what: &'static str,
    status: &str,
    base: &[u8],
    at: (&str, u8),
    damage: impl FnOnce(&mut Vec<u8>, usize),
    shapes_first: bool,
) -> Damaged {
    let n = assertion(base, at.0);
    let mut fm = Fm::opened_primed(base);
    let (offset, tokens) = {
        let out = fm.statement(n, at.1);
        assert_eq!(out.status(), 0, "{what}");
        (fm.rig.tree(n, at.1).offset as usize, count(out))
    };
    let mut bytes = base.to_vec();
    damage(&mut bytes, offset);
    if shapes_first {
        fm = Fm::primed(base);
        fm.rig.load(&bytes);
        assert_eq!(fm.open().status(), 0, "{what}: the damaged shapes open");
        assert_eq!(fm.statement(n, at.1).status(), 0, "{what}");
    } else {
        fm.rig.load(&bytes);
    }
    Damaged {
        refusal: Refusal {
            what,
            fm,
            target: "FM_NODES",
            regs: Regs::default(),
            status: code(status),
        },
        kind: Kind::Nodes(tokens),
    }
}

/// Turns the first `pairs` escaped operators of a tree into two one-byte ones each, which adds a
/// node a pair.
fn split(b: &mut [u8], at: usize, pairs: usize) {
    for i in (at..at + 2 * pairs).step_by(2) {
        assert_eq!(b[i], 0x7F);
        b[i] = 0;
        b[i + 1] = 0;
    }
}

/// Clears `bit` of the reader's flags for the sections it found, as a reader that had opened a
/// library without that section would hold them. The reader refuses such a library at open, so fm
/// meets it only through the state.
fn clear_have(fm: &mut Fm, bit: u8) {
    let at = fm.rig.label("LS_HAVE");
    let have = fm.rig.peek(at, 1)[0];
    fm.rig.poke(at, &[have & !bit]);
}

/// The deep statement with the library one byte short of its tree's end: lib_tree's own read is
/// whole and the refill of the last 252 bytes is not.
fn nodes_read_fault(nodes: &[u8]) -> Damaged {
    let mut case = nodes_case(
        "a refill the reader cannot read",
        "FM_ERR_READ",
        nodes,
        ("deep", 0xFF),
        |_, _| {},
        false,
    );
    let n = assertion(nodes, "deep");
    let end = case.refusal.fm.rig.tree(n, 0xFF).offset + 2 * fixture::CHAIN as u32 + 1;
    let at = case.refusal.fm.rig.label("LR_LENGTH");
    case.refusal.fm.rig.poke(at, &(end - 1).to_le_bytes()[..3]);
    case
}

pub fn cases() -> Vec<Damaged> {
    let nodes = fixture::nodes_library();
    let small = fixture::library();
    let table = |b: &[u8]| 2 * (shapes(b).1 + 2);
    let wi = shape_with(&nodes, &[0, 1, 0, 2, 0]);
    assert_eq!(shape_with(&small, &[0, 1]), 0, "wn is the first axiom");
    let mut out = vec![
        open_case(
            "a shapes count that is not the trees'",
            "FM_ERR_BAD",
            |b| {
                let (start, count) = shapes(b);
                word(b, start, count as u16 + 1);
            },
            |_| {},
        ),
        open_case(
            "a section too short for its offset table",
            "FM_ERR_BAD",
            |_| {},
            |fm| {
                let (_, count) = shapes(&nodes);
                let at = fm.rig.label("LS_SH") + 3;
                fm.rig.poke(at, &(2 * (count as u16 + 2) - 1).to_le_bytes());
            },
        ),
        open_case(
            "a section over 2048 bytes",
            "FM_ERR_TOO_BIG",
            |_| {},
            |fm| {
                let at = fm.rig.label("LS_SH") + 3;
                fm.rig.poke(at, &2049u16.to_le_bytes());
            },
        ),
        open_case(
            "a first item that starts inside the offset table",
            "FM_ERR_BAD",
            |b| {
                let (start, _) = shapes(b);
                let end = table(b) as u16;
                word(b, start + 2, end - 2);
            },
            |_| {},
        ),
        open_case(
            "a last item that ends past the section",
            "FM_ERR_BAD",
            |b| {
                let (start, count) = shapes(b);
                let end = item(b, count - 1).1 - start;
                word(b, start + 2 + 2 * count, end as u16 + 1);
            },
            |_| {},
        ),
        open_case(
            "an item with no parts",
            "FM_ERR_BAD",
            |b| {
                let (start, _) = shapes(b);
                let first = get(b, start + 2);
                word(b, start + 4, first as u16);
            },
            |_| {},
        ),
        open_case(
            "an item of 257 parts",
            "FM_ERR_BAD",
            |b| {
                let (start, _) = shapes(b);
                let first = get(b, start + 2);
                word(b, start + 4, first as u16 + 257);
            },
            |_| {},
        ),
        open_case(
            "a child past the axiom's arity",
            "FM_ERR_BAD",
            |b| {
                let (from, _) = item(b, wi);
                b[from + 3] = 3;
            },
            |_| {},
        ),
        open_case(
            "a child repeated",
            "FM_ERR_BAD",
            |b| {
                let (from, _) = item(b, wi);
                b[from + 3] = 1;
            },
            |_| {},
        ),
        open_case(
            "a child missing",
            "FM_ERR_BAD",
            |b| {
                let (from, _) = item(b, wi);
                b[from + 3] = 0;
            },
            |_| {},
        ),
    ];
    out.extend([
        nodes_case(
            "a tree that runs past its length",
            "FM_ERR_TREE",
            &small,
            ("ax-one", 0xFF),
            |b, at| b[at] = 0,
            false,
        ),
        nodes_case(
            "a tree with bytes after its root",
            "FM_ERR_TREE",
            &small,
            ("ax-3", 0xFF),
            |b, at| b[at] = 0x80,
            false,
        ),
        nodes_case(
            "a syntax index past the table",
            "FM_ERR_TREE",
            &small,
            ("ax-one", 0xFF),
            |b, at| b[at] = 0x7E,
            false,
        ),
        nodes_case(
            "a variable that is not the token at its place",
            "FM_ERR_TREE",
            &small,
            ("ax-one", 0xFF),
            |b, at| b[at] = 0x81,
            false,
        ),
        nodes_case(
            "a shape with a constant too many",
            "FM_ERR_TREE",
            &small,
            ("ax-3", 0xFF),
            |b, _| {
                let (start, _) = shapes(b);
                let end = get(b, start + 4);
                word(b, start + 4, end as u16 + 1);
            },
            true,
        ),
        nodes_case(
            "a shape with a constant too few",
            "FM_ERR_TREE",
            &small,
            ("ax-3", 0xFF),
            |b, _| {
                let (start, _) = shapes(b);
                let first = get(b, start + 2);
                word(b, start + 2, first as u16 + 1);
            },
            true,
        ),
        nodes_case(
            "a tree of 256 nodes",
            "FM_ERR_LIMIT",
            &nodes,
            ("deep", 0xFF),
            |b, at| split(b, at, 2),
            false,
        ),
        nodes_case(
            "a tree of 255 nodes, which the limit lets through and the token total refuses",
            "FM_ERR_TREE",
            &nodes,
            ("deep", 0xFF),
            |b, at| split(b, at, 1),
            false,
        ),
    ]);
    out.push(nodes_read_fault(&nodes));
    out.push(open_case(
        "a shapes section the reader cannot read to its end",
        "FM_ERR_READ",
        |_| {},
        |fm| {
            let (_, offset, length) = fm.rig.shapes();
            let end = offset + u32::from(length) - 1;
            fm.rig
                .poke(fm.rig.label("LR_LENGTH"), &end.to_le_bytes()[..3]);
        },
    ));
    out.push(open_case(
        "trees without shapes",
        "FM_ERR_BAD",
        |_| {},
        |fm| clear_have(fm, 16),
    ));
    out.push(open_case(
        "shapes without trees",
        "FM_ERR_BAD",
        |_| {},
        |fm| clear_have(fm, 4),
    ));
    out
}

#[test]
fn every_damaged_library_is_refused_with_its_status_and_leaves_what_the_header_says() {
    let (mut opened, mut nodes) = (0, 0);
    for mut case in cases() {
        let what = case.refusal.what;
        let shapes = case.refusal.fm.memory("FM_SECOND", 0x800);
        let out = case
            .refusal
            .fm
            .rig
            .call(case.refusal.target, case.refusal.regs);
        assert_eq!(out.status(), case.refusal.status, "{what}");
        assert_eq!(out.bc >> 8, 0, "{what}: nothing counted");
        let fm = &mut case.refusal.fm;
        match case.kind {
            Kind::Open => {
                opened += 1;
                assert_eq!(
                    fm.rows(94).status(),
                    code("FM_ERR_NOT_OPEN"),
                    "{what}: closed"
                );
                assert_eq!(
                    fm.statement(0, 0xFF).status(),
                    code("FM_ERR_NOT_OPEN"),
                    "{what}"
                );
            }
            Kind::Nodes(tokens) => {
                nodes += 1;
                assert_eq!(fm.memory("FV_NODES", 1), [0], "{what}: no table built");
                let rows = fm.rows(94);
                assert_eq!(
                    (rows.status(), count(rows) > 0),
                    (0, true),
                    "{what}: loaded"
                );
                assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0, "{what}");
                assert_eq!(usize::from(fm.memory("FV_COUNT", 1)[0]), tokens, "{what}");
            }
        }
        let nodes = matches!(case.kind, Kind::Nodes(_));
        if nodes {
            assert_eq!(
                fm.memory("FM_SECOND", 0x800),
                shapes,
                "{what}: the shapes copy is read only"
            );
        }
        let found = crate::guard::stray(&mut fm.rig, &allowed_for(nodes, true));
        assert!(found.is_empty(), "{what}: stray writes at {found:x?}");
    }
    assert_eq!((opened, nodes), (13, 9));
}
