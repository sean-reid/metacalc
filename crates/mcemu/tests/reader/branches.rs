use crate::damaged::*;
use crate::placements::layout;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{GLYPHS, TITLES, TREES, decode};

fn good() -> Vec<u8> {
    nested(6, &[1], &[(2, &[2]), (4, &[3])])
}

struct Where {
    index: usize,
    item: usize,
    end: usize,
    start: usize,
    count: u16,
}

fn at(bytes: &[u8], id: u8, n: usize) -> Where {
    let l = layout(bytes, id).unwrap();
    let library = decoded(bytes);
    let entry = library
        .sections
        .as_ref()
        .unwrap()
        .table
        .iter()
        .find(|e| e.id == id)
        .unwrap();
    Where {
        index: l.index.0 + l.entry * n,
        item: l.items[n].0,
        end: entry.offset + entry.length,
        start: entry.offset,
        count: library.assertions.len() as u16,
    }
}

/// The reader refuses `what` with status 3 where the host refuses the whole library, and the
/// library with nothing changed reads fine.
fn refused(
    what: &str,
    id: u8,
    n: usize,
    edit: &dyn Fn(&mut Vec<u8>, &Where),
    ask: &dyn Fn(&mut Rig) -> u8,
) {
    let base = good();
    let w = at(&base, id, n);
    let bytes = with(&base, |b| edit(b, &w));
    assert!(decode(&bytes).is_err(), "{what}: the host decodes it");
    let mut rig = Rig::with_library(&bytes);
    assert_eq!(rig.open(), ST_OK, "{what}: opens");
    assert_eq!(ask(&mut rig), ST_BAD, "{what}");
    let mut rig = Rig::with_library(&base);
    rig.open_ok();
    assert_eq!(ask(&mut rig), ST_OK, "{what}: unchanged");
}

fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn every_check_on_a_title_is_reached_by_a_change_the_decoder_also_refuses() {
    let ask = |rig: &mut Rig| rig.title(1).0;
    refused("depth 0", TITLES, 1, &|b, w| b[w.item + 2] = 0, &ask);
    refused("depth 5", TITLES, 1, &|b, w| b[w.item + 2] = 5, &ask);
    refused(
        "assertion past the count",
        TITLES,
        1,
        &|b, w| put16(b, w.item, w.count),
        &ask,
    );
    refused(
        "entry high byte",
        TITLES,
        1,
        &|b, w| b[w.index + 3] = 1,
        &ask,
    );
    refused(
        "item before its first",
        TITLES,
        1,
        &|b, w| put32(b, w.index, w.start as u32),
        &ask,
    );
    refused(
        "item at the section's end",
        TITLES,
        1,
        &|b, w| put32(b, w.index, w.end as u32),
        &ask,
    );
    refused(
        "text past the end",
        TITLES,
        1,
        &|b, w| b[w.item + 3] = 255,
        &ask,
    );
    refused(
        "text not ASCII",
        TITLES,
        1,
        &|b, w| b[w.item + 4] |= 0x80,
        &ask,
    );
}

#[test]
fn every_check_on_a_glyph_entry_is_reached_by_a_change_the_decoder_also_refuses() {
    let ask = |rig: &mut Rig| rig.glyph(1).0;
    refused(
        "flags over four bits",
        GLYPHS,
        1,
        &|b, w| b[w.item] = 0x10,
        &ask,
    );
    refused(
        "entry before the first item",
        GLYPHS,
        1,
        &|b, w| put16(b, w.index, 0),
        &ask,
    );
    refused(
        "entry at the section's end",
        GLYPHS,
        1,
        &|b, w| put16(b, w.index, (w.end - w.start) as u16),
        &ask,
    );
    refused(
        "codes past the end",
        GLYPHS,
        1,
        &|b, w| b[w.item + 1] = 255,
        &ask,
    );
}

#[test]
fn every_check_on_a_tree_and_a_syntax_entry_is_reached_by_a_change_the_decoder_also_refuses() {
    let number = (SYNTAX + 1) as u16;
    let ask = |rig: &mut Rig| rig.tree(number, 0xFF).status;
    let n = SYNTAX + 1;
    refused(
        "entry high byte",
        TREES,
        n,
        &|b, w| b[w.index + 3] = 1,
        &ask,
    );
    refused(
        "item before its first",
        TREES,
        n,
        &|b, w| put32(b, w.index, w.start as u32),
        &ask,
    );
    refused(
        "item at the section's end",
        TREES,
        n,
        &|b, w| put32(b, w.index, w.end as u32),
        &ask,
    );
    refused("zero length", TREES, n, &|b, w| b[w.item] = 0, &ask);
    refused(
        "long length of zero",
        TREES,
        n,
        &|b, w| {
            b[w.item] = 0xFF;
            put16(b, w.item + 1, 0);
        },
        &ask,
    );
    refused(
        "long length past the section",
        TREES,
        n,
        &|b, w| {
            b[w.item] = 0xFF;
            put16(b, w.item + 1, 0xFFF0);
        },
        &ask,
    );
    let base = good();
    let entry = decoded(&base)
        .sections
        .unwrap()
        .table
        .iter()
        .find(|e| e.id == TREES)
        .unwrap()
        .offset;
    let count = decoded(&base).assertions.len() as u16;
    let bytes = with(&base, |b| put16(b, entry + 2, count));
    assert!(decode(&bytes).is_err());
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    assert_eq!(
        rig.syntax(0).0,
        ST_BAD,
        "a syntax axiom past the assertions"
    );
    assert_eq!(rig.syntax(1).0, ST_OK);
}
