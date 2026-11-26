use crate::secrig::*;
use crate::sections::*;
use crate::support::*;
use mcpack::library::{
    ENTRY_LEN, GLYPHS, HEADER_LEN, KINDS, SHAPES, TITLES, TRAILER_LEN, TREES, crc32, decode,
    pack_kernel,
};

pub fn reseal(bytes: &mut [u8]) {
    let len = bytes.len() as u32;
    bytes[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

pub fn rows(bytes: &[u8]) -> (usize, usize) {
    let count = usize::from(bytes[bytes.len() - TRAILER_LEN]);
    (count, bytes.len() - TRAILER_LEN - count * ENTRY_LEN)
}

pub fn row_of(bytes: &[u8], id: u8) -> usize {
    let (count, first) = rows(bytes);
    (0..count)
        .map(|i| first + i * ENTRY_LEN)
        .find(|&at| bytes[at] == id)
        .expect("the library has the row")
}

pub fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn get32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

pub fn with(bytes: &[u8], change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut out = bytes.to_vec();
    change(&mut out);
    reseal(&mut out);
    out
}

/// What the reader says about `bytes` at open, and what the host decoder says.
pub fn both(bytes: &[u8]) -> (Rig, u8, bool) {
    let mut rig = Rig::with_library(bytes);
    let status = rig.open();
    (rig, status, decode(bytes).is_ok())
}

fn base() -> Vec<u8> {
    headed(60, 2, 0)
}

pub fn offset_of(bytes: &[u8], id: u8) -> u32 {
    get32(bytes, row_of(bytes, id) + 1)
}

#[test]
fn a_refusal_at_open_is_the_hosts_refusal_for_the_same_bytes() {
    let good = base();
    let (count, first) = rows(&good);
    assert_eq!(count, 6);
    let titles = row_of(&good, TITLES);
    let glyphs = row_of(&good, GLYPHS);
    let trees = row_of(&good, TREES);
    let at = good.len() - TRAILER_LEN;
    let len = good.len() as u32;
    let cases: Vec<(&str, Vec<u8>, u8)> = vec![
        ("version 2", with(&good, |b| b[at + 1] = 2), ST_TRAILER),
        ("version 0", with(&good, |b| b[at + 1] = 0), ST_TRAILER),
        ("version 255", with(&good, |b| b[at + 1] = 255), ST_TRAILER),
        ("255 rows", with(&good, |b| b[at] = 255), ST_TABLE),
        ("one row too many", with(&good, |b| b[at] = 7), ST_TABLE),
        (
            "a row at the file's end",
            with(&good, |b| put32(b, titles + 1, len)),
            ST_TABLE,
        ),
        (
            "a row past the file",
            with(&good, |b| put32(b, glyphs + 1, len + 400)),
            ST_TABLE,
        ),
        (
            "a length past the table",
            with(&good, |b| put32(b, trees + 5, 0x10_0000)),
            ST_TABLE,
        ),
        (
            "an offset past 24 bits",
            with(&good, |b| b[titles + 4] = 1),
            ST_TABLE,
        ),
        (
            "a length past 24 bits",
            with(&good, |b| b[glyphs + 8] = 1),
            ST_TABLE,
        ),
        (
            "a section at offset 0",
            with(&good, |b| put32(b, titles + 1, 0)),
            ST_TABLE,
        ),
        (
            "a section inside the header",
            with(&good, |b| put32(b, glyphs + 1, HEADER_LEN as u32)),
            ST_TABLE,
        ),
        (
            "an offset off a multiple of four",
            with(&good, |b| {
                let o = get32(b, titles + 1);
                put32(b, titles + 1, o + 2);
                let l = get32(b, titles + 5) - 2;
                put32(b, titles + 5, l);
            }),
            ST_TABLE,
        ),
        (
            "a repeated known id",
            with(&good, |b| b[glyphs] = TITLES),
            ST_TABLE,
        ),
        (
            "a repeated unknown id",
            with(&good, |b| {
                b[glyphs] = 9;
                b[trees] = 9;
            }),
            ST_TABLE,
        ),
        (
            "two sections that overlap",
            with(&good, |b| {
                let o = get32(b, titles + 1);
                put32(b, glyphs + 1, o + 4);
            }),
            ST_TABLE,
        ),
        (
            "a section that starts in the table",
            with(&good, |b| put32(b, trees + 1, first as u32)),
            ST_TABLE,
        ),
    ];
    for (name, bytes, want) in cases {
        let (_, status, host) = both(&bytes);
        assert_eq!((status, host), (want, false), "{name}");
    }
}

#[test]
fn a_section_that_starts_inside_the_last_record_is_refused_for_every_kind_of_record() {
    let mut libraries: Vec<(String, Vec<u8>)> = CORPUS
        .iter()
        .map(|name| (name.to_string(), pack_all(&corpus(name))))
        .collect();
    libraries.push(("big_library".into(), big_library()));
    libraries.push(("nested".into(), nested(20, &[1], &[(5, &[2])])));
    let (mut axioms, mut theorems) = (0, 0);
    for (name, good) in &libraries {
        let last = decoded(good).assertions.last().map(|a| a.theorem);
        axioms += usize::from(last == Some(false));
        theorems += usize::from(last == Some(true));
        let titles = row_of(good, TITLES);
        let offset = get32(good, titles + 1);
        let moved = with(good, |b| {
            put32(b, titles + 1, offset - 4);
            let l = get32(b, titles + 5) + 4;
            put32(b, titles + 5, l);
        });
        let (_, status, host) = both(&moved);
        assert_eq!(
            (status, host),
            (ST_TABLE, false),
            "{name}: four bytes early, last record {last:?}"
        );
        let (_, status, host) = both(good);
        assert_eq!(
            (status, host),
            (ST_OK, true),
            "{name}: where the packer put it"
        );
    }
    assert!(
        axioms > 0 && theorems > 0,
        "{axioms} axioms, {theorems} theorems last"
    );
}

#[test]
fn rows_the_reader_skips_or_does_not_need_open_as_the_decoder_opens_them() {
    let good = base();
    let (_, status, host) = both(&good);
    assert_eq!((status, host), (ST_OK, true));
    let trees = row_of(&good, TREES);
    let shapes = row_of(&good, SHAPES);
    let unknown = with(&good, |b| {
        b[trees] = 9;
        b[shapes] = 10;
    });
    let (mut rig, status, host) = both(&unknown);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(
        rig.sections(),
        (ST_OK, HAVE_TITLES | HAVE_GLYPHS | HAVE_KINDS | HAVE_ORDER)
    );
    assert!(decoded(&unknown).sections.unwrap().trees.is_none());
    assert_eq!(rig.syntax_count().0, ST_NONE);
    let at = good.len() - TRAILER_LEN;
    let empty = with(&good, |b| b[at] = 0);
    let (mut rig, status, host) = both(&empty);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(rig.sections(), (ST_OK, 0));
    let no_magic = with(&good, |b| {
        let n = b.len();
        b[n - 1] = b'Y';
    });
    let (mut rig, status, host) = both(&no_magic);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(rig.sections(), (ST_OK, 0));
    assert_eq!(rig.label_of(3).0, ST_OK);
}

/// A table of `rows` rows: the packer's six and zero-length rows of unknown ids at the start of
/// the first section.
fn with_rows(good: &[u8], rows: usize) -> Vec<u8> {
    let at = good.len() - TRAILER_LEN;
    let at_titles = offset_of(good, TITLES);
    with(good, |b| {
        b.truncate(at);
        for id in 0..rows - 6 {
            b.push(20 + id as u8);
            b.extend(at_titles.to_le_bytes());
            b.extend(0u32.to_le_bytes());
        }
        b.push(rows as u8);
        b.extend(&good[at + 1..]);
    })
}

#[test]
fn the_table_holds_28_rows_and_the_host_alone_accepts_a_29th() {
    let good = base();
    let (mut rig, status, host) = both(&with_rows(&good, 28));
    assert_eq!((status, host), (ST_OK, true), "28 rows");
    assert_eq!(
        rig.sections(),
        (
            ST_OK,
            HAVE_TITLES | HAVE_GLYPHS | HAVE_TREES | HAVE_SHAPES | HAVE_KINDS | HAVE_ORDER
        )
    );
    let (_, status, host) = both(&with_rows(&good, 29));
    assert_eq!(
        (status, host),
        (ST_TABLE, true),
        "29 rows: the reader's own limit"
    );
    let (_, status, host) = both(&with_rows(&good, 60));
    assert_eq!((status, host), (ST_TABLE, true), "60 rows");
}

#[test]
fn a_section_too_short_for_its_header_or_its_index_is_refused_at_open() {
    let good = base();
    let cases = [
        ("titles of 1 byte", TITLES, 1u32),
        ("glyphs of 2 bytes", GLYPHS, 2),
        ("trees of 1 byte", TREES, 1),
        ("titles that end inside their index", TITLES, 6),
        ("glyphs that end inside their index", GLYPHS, 8),
        ("trees that end inside their index", TREES, 10),
    ];
    for (name, id, length) in cases {
        let row = row_of(&good, id);
        let cut = with(&good, |b| put32(b, row + 5, length));
        let (_, status, host) = both(&cut);
        assert_eq!((status, host), (ST_SECTION, false), "{name}");
    }
    let start = offset_of(&good, TITLES) as usize;
    let inflated = with(&good, |b| {
        b[start..start + 2].copy_from_slice(&60000u16.to_le_bytes())
    });
    let (_, status, host) = both(&inflated);
    assert_eq!((status, host), (ST_SECTION, false), "60000 titles");
    let start = offset_of(&good, TREES) as usize;
    let inflated = with(&good, |b| {
        b[start..start + 2].copy_from_slice(&384u16.to_le_bytes())
    });
    let (_, status, host) = both(&inflated);
    assert_eq!((status, host), (ST_SECTION, false), "384 syntax axioms");
}

#[test]
fn a_glyph_map_for_another_font_or_symbol_count_opens_and_is_unusable() {
    let good = base();
    let start = offset_of(&good, GLYPHS) as usize;
    let (mut rig, status, host) = both(&good);
    assert_eq!((status, host), (ST_OK, true));
    assert_eq!(rig.state_byte("LS_HAVE") & FONT_MATCHES, FONT_MATCHES);
    let (_, fingerprint) = rig.glyph_map();
    let other = with(&good, |b| b[start] ^= 1);
    let (mut rig, status, host) = both(&other);
    assert_eq!(
        (status, host),
        (ST_OK, true),
        "the decoder does not compare fingerprints"
    );
    assert_eq!(rig.glyph_map(), (ST_GLYPHS, fingerprint ^ 1));
    assert_eq!(rig.glyph(0).0, ST_GLYPHS);
    assert_eq!(rig.glyphs().0, ST_GLYPHS);
    assert_eq!(
        rig.sections(),
        (
            ST_OK,
            HAVE_TITLES | HAVE_GLYPHS | HAVE_TREES | HAVE_SHAPES | HAVE_KINDS | HAVE_ORDER
        )
    );
    assert_eq!(rig.title_count().0, ST_OK, "the other sections still read");
    let counted = with(&good, |b| b[start + 2] += 1);
    let (mut rig, status, host) = both(&counted);
    assert_eq!(
        (status, host),
        (ST_OK, false),
        "the decoder refuses a wrong symbol count"
    );
    assert_eq!(rig.glyph_map().0, ST_GLYPHS);
    assert_eq!(rig.glyph(0).0, ST_GLYPHS);
    assert_eq!(rig.glyphs().0, ST_GLYPHS);
}

#[test]
fn a_glyph_section_of_65535_bytes_or_more_reports_a_length_of_ffff() {
    let good = base();
    let (mut rig, status, _) = both(&good);
    assert_eq!(status, ST_OK);
    let (_, offset, length) = rig.glyphs();
    let row = rig.label("LS_GL") + 3;
    rig.poke(row, &[length as u8, (length >> 8) as u8, 0]);
    assert_eq!(rig.glyphs(), (ST_OK, offset, length));
    rig.poke(row, &[length as u8, (length >> 8) as u8, 1]);
    assert_eq!(rig.glyphs(), (ST_OK, offset, 0xFFFF));
}

#[test]
fn a_foreign_font_image_reports_every_map_unusable() {
    let good = base();
    let mut rig = Rig::with_image(FOREIGN);
    rig.load(&good);
    rig.open_ok();
    let (status, fingerprint) = rig.glyph_map();
    assert_eq!(status, ST_GLYPHS);
    assert_eq!(
        fingerprint,
        decoded(&good).sections.unwrap().glyphs.unwrap().fingerprint
    );
    assert_eq!(rig.state_byte("LS_HAVE") & FONT_MATCHES, 0);
    assert_eq!(rig.glyph(0).0, ST_GLYPHS);
    assert_eq!(rig.syntax_count().0, ST_OK);
}

pub fn last_record(bytes: &[u8]) -> usize {
    let count = usize::from(u16::from_le_bytes([bytes[14], bytes[15]]));
    let directory = get32(bytes, 16) as usize;
    get32(bytes, directory + 4 * (count - 1)) as usize
}

#[test]
fn a_damaged_last_record_fails_the_open_only_when_there_are_sections_to_place() {
    let good = base();
    let at = last_record(&good);
    let broken = with(&good, |b| b[at] = 2);
    let (_, status, host) = both(&broken);
    assert_eq!((status, host), (ST_BAD, false), "with sections");
    let db = parse("$c wff |- $.\n$v p $.\nwp $f wff p $.\nax $a |- p $.\n");
    let kernel = pack_kernel(&db, &everything(&db)).unwrap();
    let at = last_record(&kernel);
    let broken = with(&kernel, |b| b[at] = 2);
    let (_, status, host) = both(&broken);
    assert_eq!(
        (status, host),
        (ST_OK, false),
        "without: the reader trusts the records"
    );
}

/// Whether any title, glyph entry or tree of the open library answers with a refusal.
fn refuses_somewhere(rig: &mut Rig, shapes: u16) -> bool {
    let (_, titles) = rig.title_count();
    let (_, assertions, constants, variables) = rig.counts();
    let symbols = u16::from(constants) + u16::from(variables);
    (0..titles).any(|n| rig.title(n).0 != ST_OK)
        || (0..assertions).step_by(3).any(|k| {
            let s = rig.title_for(k).0;
            s != ST_OK && s != ST_NONE
        })
        || (0..symbols).any(|id| {
            let s = rig.glyph(id as u8).0;
            s != ST_OK && s != ST_NONE && s != ST_GLYPHS
        })
        || (0..assertions).any(|n| {
            let s = rig.tree(n, 0xFF).status;
            s != ST_OK && s != ST_NONE
        })
        || (0..shapes).any(|i| {
            let s = rig.shape(i).0;
            s != ST_OK && s != ST_NONE
        })
}

#[test]
fn a_hostile_byte_in_the_trailer_or_table_never_opens_for_the_reader_what_the_decoder_refuses() {
    let good = base();
    let first = rows(&good).1;
    let shapes = u16::try_from(
        decoded(&good)
            .sections
            .unwrap()
            .shapes
            .unwrap()
            .axioms
            .len(),
    )
    .unwrap();
    assert!(shapes >= 3, "{shapes} shapes");
    let row_at = row_of(&good, SHAPES);
    let (offset, length) = (get32(&good, row_at + 1), get32(&good, row_at + 5));
    let kinds = offset_of(&good, KINDS);
    let padding = kinds - (offset + length);
    assert!(
        padding > 0,
        "the shapes section ends {padding} bytes short of the kinds section"
    );
    let (mut lazy, mut agreed, mut exempt) = (0, 0, 0);
    for at in first..good.len() {
        for flip in [0x01u8, 0x80, 0xFF] {
            let hostile = with(&good, |b| b[at] ^= flip);
            let (mut rig, status, host) = both(&hostile);
            match (status == ST_OK, host) {
                (true, true) => agreed += 1,
                (false, false) => agreed += 1,
                (false, true) => panic!("byte {at} ^ {flip:#x}: the reader refuses with {status}"),
                (true, false) => {
                    let row = (at - first) / ENTRY_LEN;
                    let column = (at - first) % ENTRY_LEN;
                    let now = get32(&hostile, row_at + 5);
                    let grown = first + row * ENTRY_LEN == row_at
                        && column >= 5
                        && now > length
                        && offset + now <= kinds;
                    if grown {
                        exempt += 1;
                        continue;
                    }
                    assert!(
                        refuses_somewhere(&mut rig, shapes),
                        "byte {at} (row {row}, column {column}) ^ {flip:#x}: the decoder refuses and no lookup does"
                    );
                    lazy += 1;
                }
            }
        }
    }
    println!(
        "{agreed} agree at open, {lazy} the reader refuses at the first lookup it fails, {exempt} a shapes length grown into the {padding} bytes of padding"
    );
    assert!(exempt > 0 && lazy > 0 && agreed > 0);
}
