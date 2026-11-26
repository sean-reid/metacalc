use super::*;
use mcpack::library::{CHUNK, CheckError, Entry, KINDS};

fn kinds_of(bytes: &[u8]) -> Vec<bool> {
    decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .kinds
        .expect("a kinds section")
        .theorems
}

/// A database of `count` assertions after one float, every third a theorem of the first axiom.
fn many(count: usize) -> Database {
    many_in(count, "wff")
}

fn many_in(count: usize, typecode: &str) -> Database {
    let mut src = format!("$c |- {typecode} $.\n$v ph $.\nwph $f {typecode} ph $.\n");
    for n in 0..count {
        if n % 3 == 2 {
            src += &format!("th-{n} $p |- ph $= wph ax-0 $.\n");
        } else {
            src += &format!("ax-{n} $a |- ph $.\n");
        }
    }
    parse(&src)
}

fn row(bytes: &[u8]) -> usize {
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    (first..bytes.len() - TRAILER_LEN)
        .step_by(ENTRY_LEN)
        .find(|&row| bytes[row] == KINDS)
        .expect("the table has a kinds row")
}

fn entry(bytes: &[u8]) -> Entry {
    let sections = decode(bytes).unwrap().sections.unwrap();
    *sections.table.iter().find(|e| e.id == KINDS).unwrap()
}

/// `bytes` with `change` applied to the kinds section and the file resealed.
fn damaged(bytes: &[u8], change: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let at = entry(bytes);
    let mut out = bytes.to_vec();
    change(&mut out[at.offset..at.offset + at.length]);
    reseal(&mut out);
    out
}

fn with_length(bytes: &[u8], length: usize) -> Vec<u8> {
    let at = row(bytes);
    let mut out = bytes.to_vec();
    out[at + 5..at + 9].copy_from_slice(&(length as u32).to_le_bytes());
    reseal(&mut out);
    out
}

#[test]
fn every_library_with_sections_has_a_kinds_section_after_the_others() {
    for src in [LOGIC.to_string(), headed()] {
        let db = parse(&src);
        let bytes = pack(&db, &closure(&db)).unwrap();
        let sections = decode(&bytes).unwrap().sections.unwrap();
        let ids: Vec<u8> = sections.table.iter().map(|e| e.id).collect();
        assert_eq!(ids, [1, 2, 3, 4, KINDS, library::LABEL_ORDER]);
        assert_eq!(KINDS, 5);
        library::check(&bytes).unwrap();
    }
    let db = many_in(5, "hom");
    let everything: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &everything).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    assert!(sections.trees.is_none(), "a database with no grammar");
    let ids: Vec<u8> = sections.table.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, KINDS, library::LABEL_ORDER]);
}

#[test]
fn a_library_with_no_sections_has_no_kinds() {
    let db = db();
    let bytes = pack_kernel(&db, &closure(&db)).unwrap();
    assert_eq!(decode(&bytes).unwrap().sections, None);
}

#[test]
fn the_bits_are_the_record_kinds_low_bit_first() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let library = decode(&bytes).unwrap();
    let want: Vec<bool> = library.assertions.iter().map(|a| a.theorem).collect();
    assert_eq!(want, [false, false, false, false, false, false, true]);
    assert_eq!(kinds_of(&bytes), want);
    let at = entry(&bytes);
    assert_eq!(at.offset % 4, 0);
    assert_eq!(
        &bytes[at.offset..at.offset + at.length],
        [7, 0, 0b0100_0000]
    );
}

#[test]
fn counts_either_side_of_a_byte_round_trip_with_zero_padding() {
    for count in [1usize, 2, 3, 7, 8, 9, 15, 16, 17, 24, 25] {
        let db = many(count);
        let bytes = pack(&db, &closure(&db)).unwrap();
        let library = decode(&bytes).unwrap();
        assert_eq!(library.assertions.len(), count);
        let want: Vec<bool> = (0..count).map(|n| n % 3 == 2).collect();
        let got = kinds_of(&bytes);
        assert_eq!(got, want, "{count} assertions");
        let at = entry(&bytes);
        assert_eq!(at.length, 2 + count.div_ceil(8));
        let last = bytes[at.offset + at.length - 1];
        if !count.is_multiple_of(8) {
            assert_eq!(last >> (count % 8), 0, "padding bits of {count}");
        }
        let theorems = got.iter().filter(|&&t| t).count();
        assert_eq!(theorems, count / 3);
        library::check(&bytes).unwrap();
    }
}

#[test]
fn a_section_of_the_wrong_length_does_not_decode() {
    let db = many(9);
    let bytes = pack(&db, &closure(&db)).unwrap();
    let length = entry(&bytes).length;
    assert_eq!(length, 4);
    assert!(decode(&with_length(&bytes, length)).is_ok());
    for wrong in [0, 1, 2, 3, 5, 6, 100] {
        assert!(decode(&with_length(&bytes, wrong)).is_err(), "{wrong}");
    }
}

#[test]
fn a_count_that_is_not_the_librarys_does_not_decode() {
    let db = many(9);
    let bytes = pack(&db, &closure(&db)).unwrap();
    for count in [0u16, 1, 8, 10, 16, 0xFFFF] {
        let bad = damaged(&bytes, |s| s[..2].copy_from_slice(&count.to_le_bytes()));
        assert!(decode(&bad).is_err(), "{count}");
    }
}

#[test]
fn a_bit_past_the_last_assertion_does_not_decode() {
    let db = many(9);
    let bytes = pack(&db, &closure(&db)).unwrap();
    assert!(
        decode(&damaged(&bytes, |s| s[3] ^= 0x01)).is_ok(),
        "assertion 8 is a real bit"
    );
    for bit in 1..8 {
        assert!(decode(&damaged(&bytes, |s| s[3] |= 1 << bit)).is_err());
    }
    let whole = many(8);
    let bytes = pack(&whole, &closure(&whole)).unwrap();
    assert!(
        decode(&damaged(&bytes, |s| s[2] ^= 0xFF)).is_ok(),
        "every bit of a full byte is an assertion"
    );
}

#[test]
fn a_flipped_kind_fails_the_check_and_names_the_assertion() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let flipped = damaged(&bytes, |s| s[2] ^= 0b0000_0100);
    assert!(decode(&flipped).is_ok());
    let Err(CheckError::Sections(e)) = library::check(&flipped) else {
        panic!("the check accepts a kind the record contradicts");
    };
    assert!(e.to_string().contains("ax-1"), "{e}");
}

#[test]
fn the_kinds_section_survives_a_hostile_byte_anywhere_in_it() {
    let db = many(20);
    let bytes = pack(&db, &closure(&db)).unwrap();
    let was = kinds_of(&bytes);
    let at = entry(&bytes);
    let (mut refused, mut same, mut other, mut caught) = (0, 0, 0, 0);
    for byte in at.offset..at.offset + at.length {
        for flip in [0x01u8, 0x10, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[byte] ^= flip;
            reseal(&mut hostile);
            let checked = library::check(&hostile);
            match decode(&hostile) {
                Err(_) => refused += 1,
                Ok(_) if kinds_of(&hostile) == was => {
                    assert!(checked.is_ok(), "byte {byte} ^ {flip:#x}: the same bitmap");
                    same += 1;
                }
                Ok(_) => {
                    assert!(
                        checked.is_err(),
                        "byte {byte} ^ {flip:#x}: another bitmap passes the check"
                    );
                    caught += 1;
                    other += 1;
                }
            }
        }
    }
    println!(
        "{refused} refused, {same} decoded to the same bitmap, {other} to another, {caught} of those caught by the check"
    );
    // A flip is refused (the count or a padding bit) or changes a bit of the bitmap, so none decodes
    // to the same one.
    assert_eq!(same, 0);
    assert!(
        refused >= 10 && other >= 8,
        "{refused} refused, {other} another"
    );
    assert_eq!(refused + same + other, 4 * at.length);
}

#[test]
fn the_section_may_cross_a_chunk() {
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
    println!(
        "{n} assertions: the section is {} bytes at {}, across {}",
        at.length,
        at.offset,
        (at.offset / CHUNK + 1) * CHUNK
    );
    assert!(
        at.offset < (at.offset / CHUNK + 1) * CHUNK
            && (at.offset / CHUNK + 1) * CHUNK < at.offset + at.length
    );
    assert_eq!(at.length, 2 + n.div_ceil(8));
    assert_eq!(
        kinds_of(&bytes).iter().filter(|&&t| t).count(),
        (0..n).filter(|k| k % 3 == 2).count()
    );
    assert!(library::check(&bytes).is_ok());
}
