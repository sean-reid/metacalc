use crate::damaged::reseal;
use crate::placements::{find, layout, symbolic, titled};
use crate::sections::*;
use crate::support::*;
use mcpack::library::{GLYPHS, TITLES, TREES, decode};

/// An item the packer padded to the start of a chunk: its number, its place and length, the room
/// the padding left before it, and where the index entry for it and the entry's base lie.
struct Placed {
    number: usize,
    at: usize,
    len: usize,
    room: usize,
    entry: usize,
    width: usize,
    base: usize,
}

fn placed(bytes: &[u8], id: u8) -> Option<Placed> {
    let layout = layout(bytes, id)?;
    let sections = decoded(bytes).sections?;
    let base = match id {
        GLYPHS => sections.table.iter().find(|e| e.id == id)?.offset,
        _ => 0,
    };
    let (number, &(at, len)) = layout
        .items
        .iter()
        .enumerate()
        .find(|&(_, &(at, len))| at % CHUNK == 0 && len > 1 && at > 0)?;
    let before = layout
        .items
        .iter()
        .filter(|&&(other, _)| other < at)
        .map(|&(other, size)| other + size)
        .max()?;
    (before < at).then_some(Placed {
        number,
        at,
        len,
        room: at - before,
        entry: layout.index.0 + number * layout.entry,
        width: layout.entry,
        base,
    })
}

/// The library with item `p` copied to start `back` bytes before the chunk it starts, and its
/// index entry pointing there. The packer never does this.
fn straddling(bytes: &[u8], p: &Placed, back: usize) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let item = bytes[p.at..p.at + p.len].to_vec();
    out[p.at - back..p.at - back + p.len].copy_from_slice(&item);
    let value = (p.at - back - p.base) as u32;
    out[p.entry..p.entry + p.width].copy_from_slice(&value.to_le_bytes()[..p.width]);
    reseal(&mut out);
    out
}

fn backs(p: &Placed) -> Vec<usize> {
    let far = p.room.min(p.len - 1);
    let mut all = vec![1, far];
    all.dedup();
    all
}

fn crossing(id: u8, what: &str, bytes: &[u8], read: &dyn Fn(&mut Rig, usize) -> Vec<u8>) {
    let p = placed(bytes, id).unwrap_or_else(|| panic!("{what}: no item starts a chunk"));
    let mut whole = opened(bytes);
    let want: Vec<u8> = read(&mut whole, p.number);
    assert_eq!(want[0], ST_OK, "{what}: the packed item reads");
    for back in backs(&p) {
        let moved = straddling(bytes, &p, back);
        assert!(decode(&moved).is_err(), "{what}: {back} bytes back");
        let mut rig = opened(&moved);
        let got = read(&mut rig, p.number);
        assert_eq!(
            got[0], ST_BAD,
            "{what}: {back} bytes back, item {}",
            p.number
        );
        for n in (0..layout(bytes, id).unwrap().items.len()).filter(|&n| n != p.number) {
            assert_eq!(read(&mut rig, n), read(&mut whole, n), "{what}: item {n}");
        }
    }
}

#[test]
fn a_title_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "title at a chunk start",
        100..400,
        &|n| titled(n, 1),
        &|b| placed(b, TITLES).is_some_and(|p| p.room > 1),
    );
    crossing(TITLES, "titles", &bytes, &|rig, n| {
        let (status, assertion, depth, text) = rig.title(n as u16);
        let mut out = vec![status];
        out.extend(assertion.to_le_bytes());
        out.push(depth);
        out.extend(text);
        out
    });
}

#[test]
fn a_glyph_entry_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "glyph entry at a chunk start",
        60..200,
        &|l| symbolic(l, 1),
        &|b| placed(b, GLYPHS).is_some_and(|p| p.room > 1),
    );
    crossing(GLYPHS, "glyphs", &bytes, &|rig, n| {
        let (status, flags, mut codes) = rig.glyph(n as u8);
        codes.insert(0, flags);
        codes.insert(0, status);
        codes
    });
}

#[test]
fn a_tree_item_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "tree item at a chunk start",
        100..600,
        &|n| titled(n, 0),
        &|b| placed(b, TREES).is_some_and(|p| p.room > 1),
    );
    crossing(TREES, "trees", &bytes, &|rig, n| {
        let mut out = Vec::new();
        for k in [0, 0xFF] {
            let tree = rig.tree(n as u16, k);
            out.push(tree.status);
            out.extend(tree.bytes);
        }
        out
    });
}
