use super::*;
use mcpack::library::{CheckError, Entry, HEADER_LEN, SHAPES, TREES};

fn shapes_of(bytes: &[u8]) -> library::Shapes {
    decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .shapes
        .expect("a shapes section")
}

fn packed() -> Vec<u8> {
    let db = db();
    pack(&db, &closure(&db)).unwrap()
}

/// `bytes` with `change` applied to a copy of the shapes section and the file resealed.
fn damaged(bytes: &[u8], change: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let entry = section(bytes, SHAPES);
    let mut out = bytes.to_vec();
    change(&mut out[entry.offset..entry.offset + entry.length]);
    reseal(&mut out);
    out
}

/// Checks that the decoder refuses; the callers damage only the shapes section of a library
/// that decodes undamaged, and the decoder reports no kind of error.
fn refused(bytes: &[u8]) {
    assert!(decode(bytes).is_err(), "the damaged library decodes");
}

/// `bytes` with the table row of section `id` renumbered to `to`.
fn renumbered(bytes: &[u8], id: u8, to: u8) -> Vec<u8> {
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    let row = (first..bytes.len() - TRAILER_LEN)
        .step_by(ENTRY_LEN)
        .find(|&row| bytes[row] == id)
        .unwrap();
    let mut out = bytes.to_vec();
    out[row] = to;
    reseal(&mut out);
    out
}

#[test]
fn the_shapes_round_trip_in_reading_order_with_children_by_hypothesis() {
    let bytes = packed();
    let shapes = shapes_of(&bytes);
    let library = decode(&bytes).unwrap();
    let trees = library.sections.as_ref().unwrap().trees.as_ref().unwrap();
    let labels: Vec<&str> = trees
        .syntax
        .iter()
        .map(|a| library.assertions[a.assertion].label.as_str())
        .collect();
    assert_eq!(labels, ["wi", "wal"]);
    assert_eq!(shapes.axioms, [vec![0, 1, 0, 2, 0], vec![0, 2, 1]]);
    assert_eq!(shapes, trees.shapes());
    library::check(&bytes).unwrap();
}

#[test]
fn the_section_starts_at_a_multiple_of_four_and_closes_the_records_in_table_order() {
    let bytes = packed();
    let entry = section(&bytes, SHAPES);
    let trees = section(&bytes, TREES);
    assert_eq!(entry.offset % 4, 0);
    assert!(trees.offset + trees.length <= entry.offset);
    assert_eq!(entry.length, 2 * (2 + 2) + 5 + 3);
}

#[test]
fn a_count_that_differs_from_the_trees_does_not_decode() {
    for count in [0u16, 1, 3, 400] {
        refused(&damaged(&packed(), |s| {
            s[..2].copy_from_slice(&count.to_le_bytes())
        }));
    }
}

#[test]
fn an_offset_table_that_does_not_fit_or_does_not_rise_does_not_decode() {
    let bytes = packed();
    let set = |at: usize, value: u16| {
        damaged(&bytes, |s| {
            s[2 + 2 * at..4 + 2 * at].copy_from_slice(&value.to_le_bytes())
        })
    };
    let length = section(&bytes, SHAPES).length as u16;
    refused(&set(0, 6));
    refused(&set(0, 9));
    refused(&set(1, 8));
    refused(&set(1, 7));
    refused(&set(0, 12));
    refused(&set(2, length - 1));
    refused(&set(2, length + 1));
    refused(&set(2, 0xFFFF));
    refused(&damaged(&bytes, |s| s[..4].fill(0)));

    let entry = section(&bytes, SHAPES);
    let mut cut = bytes.clone();
    let row = table_row(&cut, SHAPES);
    cut[row + 5..row + 9].copy_from_slice(&(entry.length as u32 - 1).to_le_bytes());
    reseal(&mut cut);
    refused(&cut);
}

fn table_row(bytes: &[u8], id: u8) -> usize {
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    (first..bytes.len() - TRAILER_LEN)
        .step_by(ENTRY_LEN)
        .find(|&row| bytes[row] == id)
        .unwrap()
}

/// An item of no parts is two equal offsets, which the strictly rising rule refuses too; the
/// two rules cannot be tried apart.
#[test]
fn an_axiom_with_no_parts_is_also_a_non_rising_offset_and_does_not_decode() {
    let bytes = packed();
    refused(&damaged(&bytes, |s| {
        s[4..6].copy_from_slice(&10u16.to_le_bytes())
    }));
    refused(&damaged(&bytes, |s| {
        s[6..8].copy_from_slice(&12u16.to_le_bytes())
    }));
}

#[test]
fn a_child_at_or_past_the_arity_does_not_decode() {
    let bytes = packed();
    refused(&damaged(&bytes, |s| s[8 + 1] = 3));
    refused(&damaged(&bytes, |s| s[8 + 1] = 0xFF));
    refused(&damaged(&bytes, |s| s[8 + 5 + 2] = 3));
}

#[test]
fn a_child_repeated_or_missing_does_not_decode() {
    let bytes = packed();
    refused(&damaged(&bytes, |s| s[8 + 3] = 1));
    refused(&damaged(&bytes, |s| s[8 + 1] = 0));
    refused(&damaged(&bytes, |s| s[8 + 5 + 1] = 0));
    refused(&damaged(&bytes, |s| s[8 + 5 + 2] = 2));
}

#[test]
fn shapes_without_trees_and_trees_without_shapes_do_not_decode() {
    let bytes = packed();
    refused(&renumbered(&bytes, TREES, 0x7F));
    refused(&renumbered(&bytes, SHAPES, 0x7F));
}

#[test]
fn a_library_without_a_grammar_has_neither_section() {
    let db = parse("$c |- foo $.\nax $a |- foo $.\n");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &all).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    assert!(sections.trees.is_none() && sections.shapes.is_none());
    assert!(sections.table.iter().all(|e| e.id != SHAPES));
    library::check(&bytes).unwrap();
}

/// `bytes` with a copy of the shapes section placed at `start`, a multiple of four, right before the
/// table, and the table row moved to it.
fn relocated(bytes: &[u8], start: usize) -> Vec<u8> {
    let entry: Entry = section(bytes, SHAPES);
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    assert!(start >= first);
    let mut out = bytes[..first].to_vec();
    out.resize(start, 0);
    out.extend_from_slice(&bytes[entry.offset..entry.offset + entry.length]);
    let row = out.len() + (table_row(bytes, SHAPES) - first);
    out.extend_from_slice(&bytes[first..]);
    out[row + 1..row + 5].copy_from_slice(&(start as u32).to_le_bytes());
    let length = out.len() as u32;
    out[8..12].copy_from_slice(&length.to_le_bytes());
    reseal(&mut out);
    out
}

#[test]
fn an_item_that_crosses_a_16_kb_boundary_does_not_decode() {
    let bytes = packed();
    let table = 2 * (2 + 2);
    let boundary = 2 * library::CHUNK;
    assert!(bytes.len() < boundary - 64);
    let whole = relocated(&bytes, boundary - table - 4 - 4);
    assert_eq!(
        decode(&whole).unwrap().sections.unwrap().shapes.unwrap(),
        shapes_of(&bytes)
    );
    let first_item_end = table + 5;
    let start = (boundary - table - 2) / 4 * 4;
    assert!(start + table < boundary && start + first_item_end > boundary);
    refused(&relocated(&bytes, start));
}

#[test]
fn a_whole_section_that_would_cross_a_boundary_is_packed_at_the_next_one() {
    let chunk = library::CHUNK;
    let mut padded = 0;
    for n in 150..=383 {
        let db = super::trees::axioms(n);
        let bytes = pack(&db, &(0..db.stmts.len() as StmtId).collect::<Vec<_>>()).unwrap();
        let (shapes, trees) = (section(&bytes, SHAPES), section(&bytes, TREES));
        let natural = (trees.offset + trees.length).next_multiple_of(4);
        let crosses = natural / chunk != (natural + shapes.length - 1) / chunk;
        if crosses {
            assert_eq!(shapes.offset % chunk, 0, "{n} axioms");
            assert!(trees.offset + trees.length <= shapes.offset, "{n} axioms");
            assert!(shapes.offset > natural, "{n} axioms");
            library::check(&bytes).unwrap();
            padded += 1;
        } else {
            assert_eq!(shapes.offset, natural, "{n} axioms");
        }
    }
    assert!(padded > 0, "no fixture needed the padding");
}

#[test]
fn a_stored_shape_that_is_not_the_axioms_statement_fails_the_check() {
    let bytes = packed();
    let swapped = damaged(&bytes, |s| s.swap(8 + 1, 8 + 3));
    assert_eq!(shapes_of(&swapped).axioms[0], [0, 2, 0, 1, 0]);
    let Err(CheckError::Sections(e)) = library::check(&swapped) else {
        panic!("the check accepts a shape that reads the children the wrong way round");
    };
    assert!(e.to_string().contains("wi"), "{e}");
    let wal = damaged(&bytes, |s| s.swap(8 + 5 + 1, 8 + 5 + 2));
    assert!(matches!(library::check(&wal), Err(CheckError::Sections(_))));
    let token = damaged(&bytes, |s| s[8 + 5] = 1);
    assert!(decode(&token).is_err(), "[1,2,1] repeats a child");
}

#[test]
fn the_shapes_survive_a_hostile_byte_anywhere_in_the_section() {
    let bytes = packed();
    let entry = section(&bytes, SHAPES);
    for at in entry.offset..entry.offset + entry.length {
        for flip in [0x01u8, 0x10, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
            let _ = library::check(&hostile);
        }
    }
}

#[test]
fn every_syntax_axiom_of_the_corpus_has_the_parts_of_its_statement() {
    let (mut axioms, mut libraries) = (0, 0);
    for (name, db) in super::trees::corpus() {
        let Some(ids) = super::trees::kept(&db) else {
            continue;
        };
        let bytes = pack(&db, &ids).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let library = decode(&bytes).unwrap();
        let sections = library.sections.as_ref().unwrap();
        let (trees, shapes) = (
            sections.trees.as_ref().unwrap(),
            sections.shapes.as_ref().unwrap(),
        );
        assert_eq!(trees.syntax.len(), shapes.axioms.len(), "{name}");
        for (axiom, shape) in trees.syntax.iter().zip(&shapes.axioms) {
            let source = &library.assertions[axiom.assertion];
            let floats: Vec<&str> = source
                .hypotheses
                .iter()
                .map(|h| match h {
                    library::Hypothesis::Float { variable, .. } => variable.as_str(),
                    library::Hypothesis::Essential(_) => unreachable!(),
                })
                .collect();
            let want: Vec<u8> = source.conclusion[1..]
                .iter()
                .map(|t| {
                    floats
                        .iter()
                        .position(|f| f == t)
                        .map_or(0, |k| k as u8 + 1)
                })
                .collect();
            assert_eq!(shape, &want, "{name}: {}", source.label);
            axioms += 1;
        }
        libraries += 1;
    }
    assert!(
        libraries >= 3 && axioms > 10,
        "{libraries} libraries, {axioms} axioms"
    );
}

#[test]
fn the_longest_syntax_axiom_a_record_holds_has_254_parts_and_round_trips() {
    let body = "a ".repeat(254);
    let db = parse(&format!(
        "$c wff |- a $.\nwl $a wff {body}$.\nax $a |- {body}$.\n"
    ));
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &all).unwrap();
    assert_eq!(shapes_of(&bytes).axioms, [vec![0; 254]]);
    library::check(&bytes).unwrap();
}
