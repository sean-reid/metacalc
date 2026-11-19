use crate::fixture;
use crate::rig::*;
use crate::support::*;
use mcpack::library::{GLYPHS, pack_kernel};
use mcpack::mm::StmtId;

const SPARE_BASE: u16 = 0xC000;

fn section(bytes: &[u8]) -> usize {
    let sections = decoded(bytes).sections.unwrap();
    sections
        .table
        .iter()
        .find(|e| e.id == GLYPHS)
        .unwrap()
        .offset
}

fn entry(bytes: &[u8], symbol: usize) -> usize {
    let at = section(bytes) + 3 + 2 * symbol;
    section(bytes) + usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn patched(change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut bytes = fixture::library();
    change(&mut bytes);
    bytes
}

fn open_status(bytes: &[u8]) -> u8 {
    Fm::bare(bytes).open().status()
}

#[test]
fn fm_open_names_why_a_library_cannot_be_drawn() {
    let db = parse(&fixture::source());
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let plain = pack_kernel(&db, &all).unwrap();
    assert_eq!(open_status(&plain), code("FM_ERR_NO_MAP"));
    let font = patched(|b| {
        let at = section(b);
        b[at] ^= 1;
    });
    assert_eq!(open_status(&font), code("FM_ERR_FONT"));
    let mut rig = Rig::with_image(WITH_FM);
    rig.load(&fixture::library());
    let mut closed = Fm { rig };
    assert_eq!(closed.open().status(), code("FM_ERR_CLOSED"));
}

#[test]
fn a_glyph_section_over_2048_bytes_is_refused_and_one_of_2048_is_not() {
    let bytes = fixture::library();
    let state = |length: u16| {
        let mut fm = Fm::bare(&bytes);
        let row = fm.rig.label("LS_GL") + 3;
        fm.rig.poke(row, &[length as u8, (length >> 8) as u8, 0]);
        fm.open().status()
    };
    assert_eq!(state(2049), code("FM_ERR_TOO_BIG"));
    assert_eq!(state(0xFFFF), code("FM_ERR_TOO_BIG"));
    assert_ne!(state(2048), code("FM_ERR_TOO_BIG"));
    assert_eq!(state(0), code("FM_ERR_BAD"));
}

#[test]
fn a_glyph_entry_the_copy_cannot_hold_is_refused_whole() {
    let bad = code("FM_ERR_BAD");
    let last = decoded(&fixture::library())
        .sections
        .unwrap()
        .glyphs
        .unwrap()
        .symbols
        .len()
        - 1;
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "an offset past the section",
            patched(|b| {
                let at = section(b) + 3;
                b[at] = 0xFF;
                b[at + 1] = 0xFF;
            }),
        ),
        (
            "a flag above bit 3",
            patched(|b| {
                let at = entry(b, 1);
                b[at] |= 0x10;
            }),
        ),
        (
            "a last entry whose codes run past the end",
            patched(|b| {
                let at = entry(b, last);
                b[at + 1] = 255;
            }),
        ),
        (
            "an entry cut after its flags",
            patched(|b| {
                let at = section(b) + 3 + 2 * last;
                let end = u16::try_from(decoded(b).sections.unwrap().table[1].length - 1).unwrap();
                b[at..at + 2].copy_from_slice(&end.to_le_bytes());
            }),
        ),
    ];
    for (what, bytes) in cases {
        assert_eq!(open_status(&bytes), bad, "{what}");
    }
}

#[test]
fn fm_statement_refuses_numbers_the_library_does_not_have_and_loads_nothing() {
    let bytes = fixture::library();
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    let index = code("FM_ERR_INDEX");
    let one = library
        .assertions
        .iter()
        .position(|a| a.label == "ax-1")
        .unwrap() as u16;
    assert_eq!(fm.statement(one, 0xFF).status(), 0);
    assert_ne!(fm.rows(94).bc >> 8, 0);
    for (n, k) in [
        (one, 0),
        (one, 1),
        (one, 0xFE),
        (library.assertions.len() as u16, 0xFF),
        (0xFFFF, 0xFF),
    ] {
        let out = fm.statement(n, k);
        assert_eq!((out.status(), out.bc >> 8), (index, 0), "{n} {k}");
        assert_eq!(fm.rows(94).bc >> 8, 0, "{n} {k} left nothing loaded");
        assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
        assert_eq!(fm.frame(), [0; FRAME_LEN]);
    }
}

#[test]
fn fm_statement_refuses_a_symbol_the_glyph_map_does_not_have() {
    let bytes = fixture::library();
    let mut probe = Fm::new(&bytes);
    assert_eq!(probe.rig.record(1), 0);
    let conclusion = probe.rig.named("LR_DCONC", 3);
    let at = usize::from(conclusion[0]) | usize::from(conclusion[1]) << 8;
    let broken = patched(|b| b[at + 1] = 250);
    let mut fm = Fm::new(&broken);
    let out = fm.statement(1, 0xFF);
    assert_eq!((out.status(), out.bc >> 8), (code("FM_ERR_BAD"), 0));
    assert_eq!(fm.rows(94).bc >> 8, 0);
}

#[test]
fn fm_statement_passes_a_reader_refusal_on_as_a_read_error() {
    let bytes = fixture::library();
    let mut probe = Fm::new(&bytes);
    assert_eq!(probe.rig.record(1), 0);
    let record = probe.rig.named("LR_DREC", 3);
    let at = usize::from(record[0]) | usize::from(record[1]) << 8;
    let broken = patched(|b| b[at + 1] = 0xFF);
    let mut fm = Fm::new(&broken);
    assert_eq!(fm.statement(1, 0xFF).status(), code("FM_ERR_READ"));
}

#[test]
fn a_token_wider_than_255_pixels_is_refused() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    let one = decoded(&bytes)
        .assertions
        .iter()
        .position(|a| a.label == "ax-1")
        .unwrap() as u16;
    let copy = fm.rig.label("FM_GLYPHS") - SPARE_BASE;
    let spare = fm.rig.label("FM_SECOND") - SPARE_BASE;
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    let codes = crate::ui_support::table().encode(&"m".repeat(60));
    let at = usize::from(spare);
    page[at] = 0;
    page[at + 1] = codes.len() as u8;
    page[at + 2..at + 2 + codes.len()].copy_from_slice(&codes);
    let wide = spare - copy;
    for symbol in 0..16 {
        let at = usize::from(copy) + 3 + 2 * symbol;
        page[at..at + 2].copy_from_slice(&wide.to_le_bytes());
    }
    assert_eq!(fm.statement(one, 0xFF).status(), code("FM_ERR_BAD"));
}
