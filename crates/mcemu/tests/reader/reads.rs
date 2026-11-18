use crate::support::*;

fn opened(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    rig
}

#[test]
fn a_read_that_crosses_a_chunk_matches_the_library_at_every_split_point() {
    let bytes = big_library();
    let mut rig = opened(&bytes);
    for boundary in [CHUNK, 2 * CHUNK, 5 * CHUNK, bytes.len() / CHUNK * CHUNK] {
        for split in 0..=256 {
            let offset = boundary - split;
            let (status, got) = rig.read(offset as u32, 256);
            assert_eq!(status, ST_OK, "boundary {boundary} split {split}");
            assert_eq!(
                got,
                bytes[offset..offset + 256],
                "boundary {boundary} split {split}"
            );
        }
    }
}

#[test]
fn reads_of_every_length_agree_with_the_library_across_a_chunk() {
    let bytes = big_library();
    let mut rig = opened(&bytes);
    for len in 1..=256usize {
        for offset in [
            CHUNK - 1,
            CHUNK - len / 2,
            CHUNK - len,
            CHUNK - 3,
            3 * CHUNK - 100,
        ] {
            let (status, got) = rig.read(offset as u32, len as u16);
            assert_eq!(status, ST_OK, "offset {offset} length {len}");
            assert_eq!(
                got,
                bytes[offset..offset + len],
                "offset {offset} length {len}"
            );
        }
    }
}

#[test]
fn the_last_bytes_of_the_library_read_and_the_next_one_does_not() {
    for bytes in [big_library(), pack_all(&corpus("demo0.mm"))] {
        let len = bytes.len();
        let mut rig = opened(&bytes);
        assert_eq!(rig.read(len as u32 - 1, 1), (ST_OK, vec![bytes[len - 1]]));
        assert_eq!(
            rig.read(len as u32 - 256, 256),
            (ST_OK, bytes[len - 256..].to_vec())
        );
        assert_eq!(rig.read(0, 256), (ST_OK, bytes[..256].to_vec()));
        assert_eq!(rig.read(0, 1), (ST_OK, vec![bytes[0]]));
        let canary = vec![0xC3; 256];
        for (offset, count) in [
            (len, 1u16),
            (len + 1, 1),
            (len - 255, 256),
            (len - 1, 2),
            (len - 1, 256),
            (0x00FF_FFFF, 1),
            (0x00FF_FFFF, 256),
            (len + CHUNK, 16),
        ] {
            rig.poke(BUF, &canary);
            let status = rig.read_call(offset as u32, count, BUF).status();
            assert_eq!(status, ST_RANGE, "offset {offset} count {count}");
            assert_eq!(
                rig.peek(BUF, 256),
                canary,
                "offset {offset} count {count}: nothing is copied"
            );
        }
    }
}

#[test]
fn a_bad_count_or_destination_is_refused_and_nothing_is_copied() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let mut rig = opened(&bytes);
    let canary = vec![0x5E; 0x300];
    rig.poke(BUF, &canary);
    for (count, dest) in [
        (0u16, BUF),
        (257, BUF),
        (0xFFFF, BUF),
        (8, 0x7FFF),
        (8, 0x0100),
        (8, 0xC000),
        (8, 0xFFFC),
        (2, 0xBFFF),
        (256, 0xBF01),
        (9, 0xBFF8),
    ] {
        let status = rig.read_call(0, count, dest).status();
        assert_eq!(status, ST_ARG, "count {count} dest {dest:#x}");
        assert_eq!(rig.peek(BUF, 0x300), canary, "count {count} dest {dest:#x}");
    }
    for (count, dest) in [(1u16, 0xBFFFu16), (256, 0xBF00), (1, 0x8000), (256, 0x8000)] {
        assert_eq!(
            rig.read_call(0, count, dest).status(),
            ST_OK,
            "count {count} dest {dest:#x}"
        );
    }
}

#[test]
fn a_reader_that_was_never_opened_refuses_the_accessors() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let mut rig = Rig::with_library(&bytes);
    assert_eq!(rig.counts().0, ST_CLOSED);
    assert_eq!(rig.symbol_name(0).0, ST_CLOSED);
    assert_eq!(rig.label_of(0).0, ST_CLOSED);
    assert_eq!(rig.record(0), ST_CLOSED);
    assert_eq!(rig.conclusion(0).0, ST_CLOSED);
    assert_eq!(rig.hypothesis(0, 0).0, ST_CLOSED);
    rig.open_ok();
    assert_eq!(rig.counts().0, ST_OK);
}

fn resealed(mut bytes: Vec<u8>) -> Vec<u8> {
    let crc = mcpack::library::crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
    bytes
}

#[test]
fn a_header_with_impossible_fields_is_refused_and_leaves_the_reader_closed() {
    let good = pack_all(&corpus("demo0.mm"));
    let length = good.len();
    type Patch = Box<dyn Fn(&mut Vec<u8>)>;
    let patches: Vec<(&str, Patch)> = vec![
        ("magic", Box::new(|b| b[0] = b'X')),
        ("length byte 3", Box::new(|b| b[11] = 1)),
        (
            "length below the header",
            Box::new(|b| b[8..12].copy_from_slice(&19u32.to_le_bytes())),
        ),
        (
            "length past the flash",
            Box::new(|b| b[8..12].copy_from_slice(&0x0100_0000u32.wrapping_sub(1).to_le_bytes())),
        ),
        (
            "symbols past 255",
            Box::new(|b| {
                b[12] = 200;
                b[13] = 100;
            }),
        ),
        ("directory byte 3", Box::new(|b| b[19] = 1)),
        ("directory not aligned", Box::new(|b| b[16] |= 1)),
        (
            "directory before the symbols",
            Box::new(|b| b[16..20].copy_from_slice(&20u32.to_le_bytes())),
        ),
        (
            "directory past the end",
            Box::new(move |b| b[16..20].copy_from_slice(&(length as u32).to_le_bytes())),
        ),
        (
            "more assertions than the directory holds",
            Box::new(|b| b[14..16].copy_from_slice(&0x7FFFu16.to_le_bytes())),
        ),
        ("a constant with a typecode", Box::new(|b| b[20] = 0)),
        (
            "a variable whose typecode is not a constant",
            Box::new(|b| {
                let mut at = 20;
                for _ in 0..b[12] {
                    at += 2 + usize::from(b[at + 1]);
                }
                b[at] = b[12];
            }),
        ),
    ];
    for (what, patch) in patches {
        let mut bytes = good.clone();
        patch(&mut bytes);
        let bytes = resealed(bytes);
        let mut rig = Rig::new();
        rig.load(&good);
        rig.open_ok();
        rig.load(&bytes);
        assert_eq!(rig.open(), ST_BAD, "{what}");
        assert_eq!(
            rig.counts().0,
            ST_CLOSED,
            "{what}: the reader is closed again"
        );
    }
    let mut rig = Rig::with_library(&good);
    rig.lib_page = 0x80;
    assert_eq!(rig.open(), ST_BAD, "a page that is not flash");
}
