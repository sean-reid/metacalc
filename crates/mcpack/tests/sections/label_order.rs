use super::*;
use mcpack::library::{CHUNK, Entry, LABEL_ORDER};

fn order_of(bytes: &[u8]) -> Vec<u16> {
    decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .label_order
        .expect("a label order section")
        .numbers
}

fn entry(bytes: &[u8]) -> Entry {
    section(bytes, LABEL_ORDER)
}

/// The numbers of `labels` sorted bytewise, by the rule and not by the packer's code.
fn sorted(labels: &[String]) -> Vec<u16> {
    let mut pairs: Vec<(&[u8], u16)> = labels
        .iter()
        .enumerate()
        .map(|(n, l)| (l.as_bytes(), n as u16))
        .collect();
    pairs.sort();
    pairs.into_iter().map(|(_, n)| n).collect()
}

fn labels_of(bytes: &[u8]) -> Vec<String> {
    decode(bytes)
        .unwrap()
        .assertions
        .iter()
        .map(|a| a.label.clone())
        .collect()
}

fn many(count: usize) -> Database {
    many_in(count, "wff")
}

fn many_in(count: usize, typecode: &str) -> Database {
    let mut src = format!("$c |- {typecode} $.\n$v ph $.\nwph $f {typecode} ph $.\n");
    for n in 0..count {
        src += &format!("ax-{n} $a |- ph $.\n");
    }
    parse(&src)
}

/// Assertions whose labels put every case of the order side by side, written out of order.
const ODD: &[&str] = &[
    "ab", "a", "a-b", "A", "a_b", "a.b", "a-B", "a1", "A-b", "9", "a0", "ab-", "Z", "z", "_", ".",
    "-", "a-b.c", "a-", "b",
];

fn odd() -> Database {
    let mut src = String::from("$c |- wff $.\n$v ph $.\nwph $f wff ph $.\n");
    for label in ODD {
        src += &format!("{label} $a |- ph $.\n");
    }
    parse(&src)
}

fn damaged(bytes: &[u8], change: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let at = entry(bytes);
    let mut out = bytes.to_vec();
    change(&mut out[at.offset..at.offset + at.length]);
    reseal(&mut out);
    out
}

fn with_length(bytes: &[u8], length: usize) -> Vec<u8> {
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    let row = (first..bytes.len() - TRAILER_LEN)
        .step_by(ENTRY_LEN)
        .find(|&row| bytes[row] == LABEL_ORDER)
        .expect("the table has a label order row");
    let mut out = bytes.to_vec();
    out[row + 5..row + 9].copy_from_slice(&(length as u32).to_le_bytes());
    reseal(&mut out);
    out
}

fn put(section: &mut [u8], at: usize, number: u16) {
    section[2 + 2 * at..4 + 2 * at].copy_from_slice(&number.to_le_bytes());
}

#[test]
fn every_library_with_sections_has_a_label_order_section_after_the_kinds() {
    for src in [LOGIC.to_string(), headed()] {
        let db = parse(&src);
        let bytes = pack(&db, &closure(&db)).unwrap();
        let ids: Vec<u8> = decode(&bytes)
            .unwrap()
            .sections
            .unwrap()
            .table
            .iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, LABEL_ORDER]);
        assert_eq!(LABEL_ORDER, 6);
        library::check(&bytes).unwrap();
    }
    let db = many_in(5, "hom");
    let everything: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &everything).unwrap();
    assert!(
        decode(&bytes).unwrap().sections.unwrap().trees.is_none(),
        "a database with no grammar"
    );
    assert_eq!(entry(&bytes).length, 12);
    assert_eq!(order_of(&bytes), [0, 1, 2, 3, 4]);
}

#[test]
fn a_library_with_no_sections_has_no_label_order() {
    let db = db();
    let bytes = pack_kernel(&db, &closure(&db)).unwrap();
    assert_eq!(decode(&bytes).unwrap().sections, None);
}

#[test]
fn the_fixtures_numbers_are_in_label_order_as_two_little_endian_bytes() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let labels = labels_of(&bytes);
    assert_eq!(
        labels,
        ["wi", "wal", "ax-1", "ax-2", "ax-mp", "ax-5", "a1i"]
    );
    let at = entry(&bytes);
    assert_eq!(at.offset % 4, 0);
    assert_eq!(
        &bytes[at.offset..at.offset + at.length],
        [7, 0, 6, 0, 2, 0, 3, 0, 5, 0, 4, 0, 1, 0, 0, 0]
    );
    assert_ne!(order_of(&bytes), (0..7).collect::<Vec<u16>>());
}

#[test]
fn labels_with_every_character_class_sort_bytewise() {
    let db = odd();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let labels = labels_of(&bytes);
    assert_eq!(labels, ODD);
    let got = order_of(&bytes);
    assert_eq!(got, sorted(&labels));
    let in_order: Vec<&str> = got.iter().map(|&n| labels[n as usize].as_str()).collect();
    assert_eq!(
        in_order,
        [
            "-", ".", "9", "A", "A-b", "Z", "_", "a", "a-", "a-B", "a-b", "a-b.c", "a.b", "a0",
            "a1", "a_b", "ab", "ab-", "b", "z"
        ]
    );
    library::check(&bytes).unwrap();
}

#[test]
fn the_order_is_a_permutation_for_counts_either_side_of_a_byte() {
    for count in [1usize, 2, 9, 10, 11, 99, 100, 101, 257] {
        let db = many(count);
        let bytes = pack(&db, &closure(&db)).unwrap();
        let got = order_of(&bytes);
        assert_eq!(got.len(), count);
        let mut seen = vec![false; count];
        for &n in &got {
            assert!(!std::mem::replace(&mut seen[n as usize], true), "{count}");
        }
        assert_eq!(got, sorted(&labels_of(&bytes)), "{count} assertions");
        assert_eq!(entry(&bytes).length, 2 + 2 * count);
        library::check(&bytes).unwrap();
    }
}

#[test]
fn the_undamaged_section_decodes_and_each_damage_is_refused() {
    let db = many(12);
    let bytes = pack(&db, &closure(&db)).unwrap();
    let length = entry(&bytes).length;
    assert_eq!(length, 26);
    let want = order_of(&bytes);
    assert_eq!(want[..4], [0, 1, 10, 11]);
    assert!(decode(&damaged(&bytes, |_| {})).is_ok());
    assert!(decode(&with_length(&bytes, length)).is_ok());
    let refused = |what: &str, bad: Vec<u8>| {
        assert!(decode(&bad).is_err(), "{what}");
        assert!(library::check(&bad).is_err(), "{what}");
    };
    for wrong in [0, 1, 2, 3, 4, 24, 25, 27, 28, 100] {
        refused("length", with_length(&bytes, wrong));
    }
    for count in [0u16, 1, 11, 13, 0xFFFF] {
        refused(
            "count",
            damaged(&bytes, |s| s[..2].copy_from_slice(&count.to_le_bytes())),
        );
    }
    refused("a number at the count", damaged(&bytes, |s| put(s, 5, 12)));
    refused(
        "a number past the count",
        damaged(&bytes, |s| put(s, 5, 0xFFFF)),
    );
    refused("a number in the high byte", damaged(&bytes, |s| s[3] = 1));
    refused("a repeat", damaged(&bytes, |s| put(s, 7, want[6])));
    refused(
        "a repeat of the first",
        damaged(&bytes, |s| put(s, 11, want[0])),
    );
    refused(
        "two neighbours swapped",
        damaged(&bytes, |s| {
            put(s, 3, want[4]);
            put(s, 4, want[3]);
        }),
    );
    refused(
        "the first two swapped",
        damaged(&bytes, |s| {
            put(s, 0, want[1]);
            put(s, 1, want[0]);
        }),
    );
    refused(
        "the last two swapped",
        damaged(&bytes, |s| {
            put(s, 10, want[11]);
            put(s, 11, want[10]);
        }),
    );
    refused(
        "assertion order",
        damaged(&bytes, |s| {
            for n in 0..12u16 {
                put(s, n as usize, n);
            }
        }),
    );
    refused(
        "reversed",
        damaged(&bytes, |s| {
            for n in 0..12u16 {
                put(s, n as usize, want[11 - n as usize]);
            }
        }),
    );
}

#[test]
fn a_flip_of_any_byte_of_the_section_is_refused() {
    let db = many(20);
    let bytes = pack(&db, &closure(&db)).unwrap();
    let at = entry(&bytes);
    let mut refused = 0;
    for byte in at.offset..at.offset + at.length {
        for flip in [0x01u8, 0x10, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[byte] ^= flip;
            reseal(&mut hostile);
            assert!(decode(&hostile).is_err(), "byte {byte} ^ {flip:#x}");
            refused += 1;
        }
    }
    assert_eq!(refused, 4 * (2 + 2 * 20));
}

#[test]
fn the_section_may_cross_a_chunk_between_two_entries() {
    let (n, bytes) = (2_000..8_000)
        .step_by(37)
        .find_map(|n| {
            let db = many(n);
            let bytes = pack(&db, &closure(&db)).unwrap();
            let at = entry(&bytes);
            let crosses = at.offset / CHUNK != (at.offset + at.length - 1) / CHUNK;
            crosses.then_some((n, bytes))
        })
        .expect("some library size puts a chunk's end inside the section");
    let at = entry(&bytes);
    let end = (at.offset / CHUNK + 1) * CHUNK;
    println!(
        "{n} assertions: {} bytes at {}, across {end}",
        at.length, at.offset
    );
    assert!(at.offset < end && end < at.offset + at.length);
    assert_eq!(
        (end - at.offset) % 2,
        0,
        "no number is split by the boundary"
    );
    assert_eq!(at.length, 2 + 2 * n);
    assert_eq!(order_of(&bytes), sorted(&labels_of(&bytes)));
    library::check(&bytes).unwrap();
}
