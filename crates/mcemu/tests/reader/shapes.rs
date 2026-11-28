use crate::damaged::{offset_of, put32, reseal, row_of, rows, with};
use crate::secrig::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{CHUNK, ENTRY_LEN, SHAPES, TRAILER_MAGIC, TRAILER_VERSION, TREES, decode};

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
    assert_eq!(
        rig.sections(),
        (ST_OK, HAVE_TITLES | HAVE_GLYPHS | HAVE_KINDS | HAVE_ORDER)
    );
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

/// `good` with its shapes section replaced by `section`, which starts at `at`, a multiple of four
/// at or after the old start; the table and trailer follow it.
fn with_section(good: &[u8], at: usize, section: &[u8]) -> Vec<u8> {
    let (_, first) = rows(good);
    let mut out = good[..first].to_vec();
    out.resize(at, 0);
    out.extend_from_slice(section);
    let table = out.len();
    out.extend_from_slice(&good[first..first + 4 * ENTRY_LEN]);
    out.extend_from_slice(&[4, TRAILER_VERSION]);
    out.extend_from_slice(TRAILER_MAGIC);
    let row = table + (row_of(good, SHAPES) - first);
    put32(&mut out, row + 1, at as u32);
    put32(&mut out, row + 5, section.len() as u32);
    let len = out.len() as u32;
    out[8..12].copy_from_slice(&len.to_le_bytes());
    reseal(&mut out);
    out
}

/// A shapes section of `items`, `gap` zero bytes between its table and the first item and `slack`
/// after the last.
fn section_of(items: &[Vec<u8>], gap: usize, slack: usize) -> Vec<u8> {
    let table = 2 * (items.len() + 2);
    let mut out = (items.len() as u16).to_le_bytes().to_vec();
    let mut offset = table + gap;
    for item in items {
        out.extend((offset as u16).to_le_bytes());
        offset += item.len();
    }
    out.extend((offset as u16).to_le_bytes());
    out.resize(table + gap, 0);
    for item in items {
        out.extend(item);
    }
    out.resize(out.len() + slack, 0);
    out
}

fn items_of(bytes: &[u8]) -> Vec<Vec<u8>> {
    decoded(bytes).sections.unwrap().shapes.unwrap().axioms
}

fn longer(item: &[u8], extra: usize) -> Vec<u8> {
    let mut out = item.to_vec();
    out.resize(out.len() + extra, 0);
    out
}

#[test]
fn an_item_that_crosses_a_16_kb_chunk_is_refused_as_the_decoder_refuses_it() {
    let good = base();
    let items = items_of(&good);
    assert_eq!(rows(&good).0, 6);
    let section = section_of(&items, 0, 0);
    assert_eq!(section.len(), 22);
    let whole = with_section(&good, 2 * CHUNK - 24, &section_of(&items, 0, 0));
    assert!(decode(&whole).is_ok());
    let mut rig = opened(&whole);
    for i in 0..3 {
        assert_eq!(rig.shape(i).0, ST_OK);
    }
    let across = with_section(&good, 2 * CHUNK - 16, &section);
    assert!(decode(&across).is_err(), "the decoder refuses");
    let mut rig = opened(&across);
    let crossing: Vec<u8> = (0..3).map(|i| rig.shape(i).0).collect();
    assert_eq!(crossing, [ST_OK, ST_BAD, ST_OK]);
}

#[test]
fn the_last_item_may_end_on_the_chunk_boundary_and_not_one_byte_past_it() {
    let good = base();
    let items = items_of(&good);
    for (extra, want) in [(2, ST_OK), (3, ST_BAD)] {
        let mut items = items.clone();
        items[2] = longer(&items[2], extra);
        let section = section_of(&items, 0, 0);
        let at = 2 * CHUNK - 24;
        assert_eq!(at + section.len(), 2 * CHUNK + extra - 2);
        let bytes = with_section(&good, at, &section);
        assert_eq!(
            decode(&bytes).is_ok(),
            want == ST_OK,
            "{extra}: the decoder"
        );
        let mut rig = opened(&bytes);
        let got: Vec<u8> = (0..3).map(|i| rig.shape(i).0).collect();
        assert_eq!(got, [ST_OK, ST_OK, want], "{extra} extra bytes");
        if want == ST_OK {
            assert_eq!(rig.shape(2), (ST_OK, items[2].clone()));
        }
    }
}

#[test]
fn trailing_bytes_and_a_gap_after_the_table_are_never_read_though_the_decoder_refuses_them() {
    let good = base();
    let items = items_of(&good);
    for (what, section) in [
        ("slack", section_of(&items, 0, 6)),
        ("a gap", section_of(&items, 4, 0)),
        ("both", section_of(&items, 8, 12)),
    ] {
        let bytes = with_section(&good, start(&good), &section);
        assert!(decode(&bytes).is_err(), "{what}: the decoder refuses");
        let mut rig = opened(&bytes);
        for (i, item) in items.iter().enumerate() {
            assert_eq!(rig.shape(i as u16), (ST_OK, item.clone()), "{what}");
        }
        assert_eq!(rig.shapes().2 as usize, section.len(), "{what}");
    }
}

#[test]
fn a_trees_entry_with_an_assertion_past_the_count_still_serves_its_shape() {
    let good = base();
    let at = offset_of(&good, TREES) as usize + 2;
    let bytes = with(&good, |b| word(b, at, 0xFFFF));
    assert!(decode(&bytes).is_err(), "the decoder refuses");
    let mut rig = opened(&bytes);
    assert_eq!(rig.syntax(0).0, ST_BAD);
    assert_eq!(rig.shape(0), (ST_OK, items_of(&good)[0].clone()));
}

fn only_axiom(parts: usize) -> Vec<u8> {
    let body = "a ".repeat(parts);
    pack_with_sections(&format!(
        "$c wff |- a $.\nwl $a wff {body}$.\nax $a |- {body}$.\n"
    ))
}

#[test]
fn an_axiom_of_255_parts_reads_and_one_of_256_is_refused() {
    let good = only_axiom(254);
    assert_eq!(items_of(&good), [vec![0; 254]]);
    let at = start(&good);
    let wide = with_section(&good, at, &section_of(&[vec![0; 255]], 0, 0));
    assert!(
        decode(&wide).is_ok(),
        "255 constants is a shape the decoder takes"
    );
    let mut rig = opened(&wide);
    assert_eq!(rig.shape(0), (ST_OK, vec![0; 255]));
    let long = with_section(&good, at, &section_of(&[vec![0; 256]], 0, 0));
    assert!(decode(&long).is_err());
    assert_eq!(opened(&long).shape(0).0, ST_BAD);
}

#[test]
fn an_axiom_of_arity_255_with_255_children_reads_though_no_record_has_that_arity() {
    let good = only_axiom(254);
    let children: Vec<u8> = (1..=255).collect();
    let arity = offset_of(&good, TREES) as usize + 2 + 2;
    let bytes = with(
        &with_section(
            &good,
            start(&good),
            &section_of(std::slice::from_ref(&children), 0, 0),
        ),
        |b| b[arity] = 255,
    );
    assert!(
        decode(&bytes).is_err(),
        "the arity is not the hypothesis count"
    );
    let mut rig = opened(&bytes);
    assert_eq!(rig.shape(0), (ST_OK, children.clone()));
    let mut twice = children;
    twice[254] = 1;
    let bytes = with(
        &with_section(&good, start(&good), &section_of(&[twice], 0, 0)),
        |b| b[arity] = 255,
    );
    assert_eq!(opened(&bytes).shape(0).0, ST_BAD);
}

#[test]
fn rows_in_another_order_open_a_repeated_id_4_is_a_table_fault_and_a_long_row_is_a_section_fault() {
    let good = base();
    let (_, first) = rows(&good);
    let (trees, shapes) = (row_of(&good, TREES), row_of(&good, SHAPES));
    assert!(trees < shapes);
    let swapped = with(&good, |b| {
        let (a, c) = (
            b[trees..trees + ENTRY_LEN].to_vec(),
            b[shapes..shapes + ENTRY_LEN].to_vec(),
        );
        b[trees..trees + ENTRY_LEN].copy_from_slice(&c);
        b[shapes..shapes + ENTRY_LEN].copy_from_slice(&a);
    });
    let (mut rig, status, host) = opens(&swapped);
    assert_eq!((status, host), (ST_OK, true), "shapes before trees");
    assert_eq!(rig.shape(0), (ST_OK, items_of(&good)[0].clone()));
    let twice = with(&good, |b| b[first] = SHAPES);
    let (_, status, host) = opens(&twice);
    assert_eq!((status, host), (ST_TABLE, false), "a repeated id 4");
    let items = items_of(&good);
    for (length, status) in [(65535, ST_OK), (65536, ST_SECTION)] {
        let slack = length - section_of(&items, 0, 0).len();
        let bytes = with_section(&good, start(&good), &section_of(&items, 0, slack));
        let (_, got, host) = opens(&bytes);
        assert_eq!((got, host), (status, false), "a row of {length} bytes");
    }
}

#[test]
fn the_checks_run_in_order_closed_then_buffer_then_section_then_index() {
    let good = base();
    let state = Regs {
        hl: 60000,
        de: Rig::with_library(&good).label("LIB_STATE"),
        ..Regs::default()
    };
    let past = Regs {
        hl: 60000,
        de: BUF,
        ..Regs::default()
    };
    let mut closed = Rig::with_library(&good);
    assert_eq!(closed.call("LIB_SHAPE", state).status(), ST_CLOSED);
    assert_eq!(
        closed.call("LIB_SHAPES", Regs::default()).status(),
        ST_CLOSED
    );
    let mut rig = opened(&good);
    assert_eq!(rig.call("LIB_SHAPE", state).status(), ST_ARG);
    assert_eq!(rig.call("LIB_SHAPE", past).status(), ST_INDEX);
    let db = parse("$c wff |- $.\n$v p $.\nwp $f wff p $.\nax $a |- p $.\n");
    let kernel = mcpack::library::pack_kernel(&db, &everything(&db)).unwrap();
    let mut bare = opened(&kernel);
    assert_eq!(bare.call("LIB_SHAPE", state).status(), ST_ARG);
    assert_eq!(bare.call("LIB_SHAPE", past).status(), ST_NONE);
}
