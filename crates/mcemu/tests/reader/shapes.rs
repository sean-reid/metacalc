use crate::damaged::{get32, offset_of, put32, reseal, row_of, rows, with};
use crate::secrig::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{CHUNK, ENTRY_LEN, SHAPES, TRAILER_LEN, TREES, decode};

fn base() -> Vec<u8> {
    headed(60, 2, 0)
}

fn start(bytes: &[u8]) -> usize {
    offset_of(bytes, SHAPES) as usize
}

fn word(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

/// Open the damaged bytes and say what the decoder says of them.
fn opens(bytes: &[u8]) -> (Rig, u8, bool) {
    let mut rig = Rig::with_library(bytes);
    let status = rig.open();
    (rig, status, decode(bytes).is_ok())
}

#[test]
fn every_shape_is_the_decoders_and_the_section_is_where_the_table_says() {
    let bytes = base();
    let mut rig = opened(&bytes);
    check_sections(&mut rig, &bytes, "headed", &ascending);
    let shapes = decoded(&bytes).sections.unwrap().shapes.unwrap();
    assert_eq!(shapes.axioms.len(), 3);
    assert_eq!(rig.shape(0), (ST_OK, shapes.axioms[0].clone()));
    assert_eq!(rig.sections().1 & HAVE_SHAPES, HAVE_SHAPES);
}

#[test]
fn a_library_without_a_shapes_section_opens_as_before() {
    let bytes = base();
    let row = row_of(&bytes, SHAPES);
    let (_, first) = rows(&bytes);
    assert_eq!(row, first + 3 * ENTRY_LEN);
    let (mut rig, status, host) = opens(&with(&bytes, |b| {
        b[row] = 40;
        b[first + 2 * ENTRY_LEN] = 41;
    }));
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(rig.sections(), (ST_OK, HAVE_TITLES | HAVE_GLYPHS));
    assert_eq!(rig.shapes().0, ST_NONE);
    assert_eq!(rig.shape(0).0, ST_NONE);
}

#[test]
fn one_of_trees_and_shapes_without_the_other_is_refused_at_open_as_the_decoder_refuses_it() {
    let good = base();
    let (trees, shapes) = (row_of(&good, TREES), row_of(&good, SHAPES));
    for (what, bytes) in [
        ("no trees", with(&good, |b| b[trees] = 40)),
        ("no shapes", with(&good, |b| b[shapes] = 40)),
    ] {
        let (_, status, host) = opens(&bytes);
        assert_eq!((status, host), (ST_SECTION, false), "{what}");
    }
}

#[test]
fn a_count_that_is_not_the_trees_or_a_table_that_does_not_fit_is_refused_at_open() {
    let good = base();
    let at = start(&good);
    let length = row_of(&good, SHAPES) + 5;
    for (what, bytes) in [
        ("one more", with(&good, |b| word(b, at, 4))),
        ("one fewer", with(&good, |b| word(b, at, 2))),
        ("none", with(&good, |b| word(b, at, 0))),
        ("a section of 4 bytes", with(&good, |b| put32(b, length, 4))),
        ("a section of 9", with(&good, |b| put32(b, length, 9))),
        ("a section of 0", with(&good, |b| put32(b, length, 0))),
    ] {
        let (_, status, host) = opens(&bytes);
        assert_eq!((status, host), (ST_SECTION, false), "{what}");
    }
}

/// The offsets, the first part and the child bytes of `bytes`' three shapes: [10, 12, 17, 22]
/// then the items.
#[test]
fn a_shape_the_format_forbids_is_refused_by_lib_shape_as_by_the_decoder() {
    let good = base();
    let at = start(&good);
    assert_eq!(&good[at..at + 2], &[3, 0]);
    let item = |bytes: &[u8], i: usize| {
        let o = |k: usize| {
            usize::from(u16::from_le_bytes([
                bytes[at + 2 + 2 * k],
                bytes[at + 3 + 2 * k],
            ]))
        };
        (o(i), o(i + 1))
    };
    let (first, second) = (item(&good, 0), item(&good, 1));
    assert_eq!((first.1 - first.0, second.1 - second.0), (2, 5));
    let cases: Vec<(&str, Vec<u8>, usize)> = vec![
        (
            "a child past the arity",
            with(&good, |b| b[at + first.0 + 1] = 2),
            0,
        ),
        (
            "a child past 255",
            with(&good, |b| b[at + first.0 + 1] = 255),
            0,
        ),
        (
            "a missing child",
            with(&good, |b| b[at + first.0 + 1] = 0),
            0,
        ),
        (
            "a repeated child",
            with(&good, |b| b[at + second.0 + 3] = 1),
            1,
        ),
        (
            "a missing and a repeated child",
            with(&good, |b| b[at + second.0 + 1] = 2),
            1,
        ),
        (
            "an item with no parts",
            with(&good, |b| word(b, at + 4, 10)),
            0,
        ),
        ("offsets that fall", with(&good, |b| word(b, at + 4, 8)), 0),
        (
            "an offset inside the table",
            with(&good, |b| word(b, at + 2, 6)),
            0,
        ),
        (
            "an offset past the section",
            with(&good, |b| word(b, at + 8, 400)),
            2,
        ),
    ];
    for (what, bytes, bad) in cases {
        assert!(decode(&bytes).is_err(), "{what}: the decoder refuses");
        let (mut rig, status, _) = opens(&bytes);
        if status == ST_OK {
            assert_eq!(rig.shape(bad as u16).0, ST_BAD, "{what}");
        } else {
            assert_eq!(status, ST_SECTION, "{what}");
        }
    }
}

#[test]
fn an_item_that_crosses_a_16_kb_chunk_is_refused_as_the_decoder_refuses_it() {
    let good = base();
    let entry_at = |b: &[u8]| row_of(b, SHAPES);
    let (count, first) = rows(&good);
    let section = &good[start(&good)..start(&good) + 22];
    let place = |at: usize| {
        let mut out = good[..first].to_vec();
        out.resize(at, 0);
        out.extend_from_slice(section);
        let table = out.len();
        out.extend_from_slice(&good[first..]);
        let row = table + (entry_at(&good) - first);
        put32(&mut out, row + 1, at as u32);
        let len = out.len() as u32;
        out[8..12].copy_from_slice(&len.to_le_bytes());
        reseal(&mut out);
        out
    };
    assert!(count == 4 && good.len() + 22 < CHUNK * 2 + TRAILER_LEN);
    let whole = place(2 * CHUNK - 24);
    assert!(decode(&whole).is_ok());
    let mut rig = opened(&whole);
    for i in 0..3 {
        assert_eq!(rig.shape(i).0, ST_OK);
    }
    let across = place(2 * CHUNK - 16);
    assert!(decode(&across).is_err(), "the decoder refuses");
    let mut rig = opened(&across);
    let crossing: Vec<u8> = (0..3).map(|i| rig.shape(i).0).collect();
    assert!(
        crossing.contains(&ST_BAD) && crossing.contains(&ST_OK),
        "{crossing:?}"
    );
    assert_eq!(get32(&across, row_of(&across, SHAPES) + 5), 22);
}
