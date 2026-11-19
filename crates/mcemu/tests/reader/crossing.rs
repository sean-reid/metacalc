use crate::damaged::{get32, put32, reseal, row_of, rows};
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

fn placed(bytes: &[u8], id: u8, from: usize) -> Option<Placed> {
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
        .find(|&(_, &(at, len))| at % CHUNK == 0 && len > 1 && at >= from.max(1))?;
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

fn crossing(
    id: u8,
    what: &str,
    bytes: &[u8],
    from: usize,
    read: &dyn Fn(&mut Rig, usize) -> Vec<u8>,
) {
    let p = placed(bytes, id, from).unwrap_or_else(|| panic!("{what}: no item starts a chunk"));
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

fn title_read(rig: &mut Rig, n: usize) -> Vec<u8> {
    let (status, assertion, depth, text) = rig.title(n as u16);
    let mut out = vec![status];
    out.extend(assertion.to_le_bytes());
    out.push(depth);
    out.extend(text);
    out
}

fn glyph_read(rig: &mut Rig, n: usize) -> Vec<u8> {
    let (status, flags, mut codes) = rig.glyph(n as u8);
    codes.insert(0, flags);
    codes.insert(0, status);
    codes
}

fn tree_read(rig: &mut Rig, n: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for k in [0, 0xFF] {
        let tree = rig.tree(n as u16, k);
        out.push(tree.status);
        out.extend(tree.bytes);
    }
    out
}

/// Past 64 KB the third byte of an offset is not zero.
const HIGH: usize = 1 << 16;

#[test]
fn a_title_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "title at a chunk start",
        100..400,
        &|n| titled(n, 1),
        &|b| placed(b, TITLES, 0).is_some_and(|p| p.room > 1),
    );
    crossing(TITLES, "titles", &bytes, 0, &title_read);
}

#[test]
fn a_title_that_crosses_a_chunk_past_64_kb_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "title at a chunk start past 64 KB",
        3000..3200,
        &|n| titled(n, 0),
        &|b| placed(b, TITLES, HIGH).is_some_and(|p| p.room > 1),
    );
    crossing(TITLES, "titles", &bytes, HIGH, &title_read);
}

#[test]
fn a_glyph_entry_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "glyph entry at a chunk start",
        60..200,
        &|l| symbolic(l, 1),
        &|b| placed(b, GLYPHS, 0).is_some_and(|p| p.room > 1),
    );
    crossing(GLYPHS, "glyphs", &bytes, 0, &glyph_read);
}

#[test]
fn a_tree_item_that_crosses_a_chunk_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "tree item at a chunk start",
        100..600,
        &|n| titled(n, 0),
        &|b| placed(b, TREES, 0).is_some_and(|p| p.room > 1),
    );
    crossing(TREES, "trees", &bytes, 0, &tree_read);
}

#[test]
fn a_tree_item_that_crosses_a_chunk_past_64_kb_is_refused_as_the_decoder_refuses_it() {
    let bytes = find(
        "tree item at a chunk start past 64 KB",
        3000..3200,
        &|n| titled(n, 0),
        &|b| placed(b, TREES, HIGH).is_some_and(|p| p.room > 1),
    );
    crossing(TREES, "trees", &bytes, HIGH, &tree_read);
}

/// The start of each statement of the tree item at `at`, up to its end at `end`.
fn statements(bytes: &[u8], at: usize, end: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut p = at;
    while p < end {
        starts.push(p);
        p += match bytes[p] {
            0xFE => 1,
            0xFF => 3 + usize::from(u16::from_le_bytes([bytes[p + 1], bytes[p + 2]])),
            n => 1 + usize::from(n),
        };
    }
    assert_eq!(p, end);
    starts
}

#[test]
fn a_tree_item_whose_last_statement_runs_past_its_section_is_refused_for_every_statement() {
    let src = format!(
        "{GRAMMAR}ax-0 $a |- p $.\n${{\nh1 $e |- p $.\nh2 $e |- ( p -> q ) $.\nax-last $a |- q $.\n$}}\n"
    );
    let good = pack_with_sections(&src);
    let table = decoded(&good).sections.unwrap().table;
    let section = table.iter().find(|e| e.id == TREES).unwrap();
    let section_end = section.offset + section.length;
    let (at, len) = *layout(&good, TREES).unwrap().items.last().unwrap();
    assert_eq!(at + len, section_end, "the last item ends the section");
    let starts = statements(&good, at, at + len);
    assert!(starts.len() >= 2, "{starts:?}");
    let last = *starts.last().unwrap();
    assert!(good[last] < 0xFE, "the last statement is a tree");
    let mut bytes = good.clone();
    bytes[last] = 0xFD;
    reseal(&mut bytes);
    assert!(last + 1 + 0xFD > section_end);
    assert!(decode(&bytes).is_err(), "the decoder refuses");
    let n = (decoded(&good).assertions.len() - 1) as u16;
    let mut rig = opened(&bytes);
    for k in (0..starts.len() as u8 - 1).chain([0xFF]) {
        assert_eq!(rig.tree(n, k).status, ST_BAD, "statement {k}");
    }
    let mut rig = opened(&good);
    assert_eq!(
        rig.tree(n, 0xFF).status,
        ST_OK,
        "the unchanged library reads"
    );
}

/// Whether an item of the section ends on the last byte before a multiple of 64 KB.
fn ends_on_64_kb(bytes: &[u8], id: u8) -> bool {
    layout(bytes, id).is_some_and(|l| {
        l.items
            .iter()
            .any(|&(at, len)| (at + len) % HIGH == 0 && at + len > 0)
    })
}

#[test]
fn an_item_that_ends_on_the_last_byte_before_a_64_kb_multiple_reads_as_the_decoder_reads_it() {
    for (id, name, count) in [(TITLES, "titles", 3101), (TREES, "trees", 3441)] {
        let bytes = titled(count, 0);
        assert!(
            ends_on_64_kb(&bytes, id),
            "{name}: no item ends before a 64 KB multiple in titled({count}, 0)"
        );
        let mut rig = opened(&bytes);
        check_sections(&mut rig, &bytes, name, &ascending);
    }
}

/// The library with `extra` zero bytes after the trees section, which the section then covers.
fn with_longer_trees(good: &[u8], extra: usize) -> Vec<u8> {
    let row = row_of(good, TREES);
    let mut out = good.to_vec();
    let length = get32(&out, row + 5) + extra as u32;
    put32(&mut out, row + 5, length);
    let table = rows(good).1;
    out.splice(table..table, std::iter::repeat_n(0, extra));
    reseal(&mut out);
    out
}

#[test]
fn a_tree_item_of_64_kb_ending_in_the_same_quarter_of_the_next_64_kb_is_refused() {
    let base = titled(100, 0);
    let good = with_longer_trees(&base, 70000);
    let row = row_of(&good, TREES);
    let section_end = (get32(&good, row + 1) + get32(&good, row + 5)) as usize;
    let items = layout(&base, TREES).unwrap().items;
    let (n, at) = items
        .iter()
        .enumerate()
        .find_map(|(n, &(at, len))| {
            let single = statements(&good, at, at + len).len() == 1 && good[at] < 0xFE;
            (single && at + 65537 <= section_end).then_some((n, at))
        })
        .expect("a one statement item with 64 KB of section after it");
    let mut bytes = good.clone();
    bytes[at] = 0xFF;
    bytes[at + 1..at + 3].copy_from_slice(&65534u16.to_le_bytes());
    reseal(&mut bytes);
    assert_eq!(
        ((at + 65536) ^ at) & 0xC000,
        0,
        "the same quarter of its chunk run"
    );
    assert_ne!((at + 65536) >> 16, at >> 16);
    assert!(decode(&bytes).is_err(), "the decoder refuses");
    let mut rig = opened(&bytes);
    assert_eq!(rig.tree(n as u16, 0xFF).status, ST_BAD);
    let mut rig = opened(&good);
    assert_eq!(rig.tree(n as u16, 0xFF).status, ST_OK);
}
