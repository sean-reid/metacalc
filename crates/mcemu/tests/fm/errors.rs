use crate::fixture::{self, entry, patched, plain, section};
use crate::rig::*;
use crate::support::*;

const IY: u16 = 0x2468;

fn symbols(bytes: &[u8]) -> usize {
    decoded(bytes)
        .sections
        .unwrap()
        .glyphs
        .unwrap()
        .symbols
        .len()
}

fn assertion(label: &str) -> u16 {
    let library = decoded(&fixture::library());
    library
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap() as u16
}

/// One refused call: fm in the state that makes it refuse, the routine, its registers and the
/// status it must return.
pub struct Refusal {
    pub what: &'static str,
    pub fm: Fm,
    pub target: &'static str,
    pub regs: Regs,
    pub status: u8,
}

fn open_with(what: &'static str, bytes: &[u8], status: &str) -> Refusal {
    Refusal {
        what,
        fm: Fm::bare(bytes),
        target: "FM_OPEN",
        regs: Regs::default(),
        status: code(status),
    }
}

fn statement_with(what: &'static str, bytes: &[u8], at: (u16, u8), status: &str) -> Refusal {
    Refusal {
        what,
        fm: Fm::new(bytes),
        target: "FM_STATEMENT",
        regs: Regs {
            hl: at.0,
            de: u16::from(at.1),
            ..Regs::default()
        },
        status: code(status),
    }
}

fn record_of(bytes: &[u8], n: u16, field: &str) -> usize {
    let mut fm = Fm::new(bytes);
    assert_eq!(fm.rig.record(n), 0);
    let at = fm.rig.named(field, 3);
    usize::from(at[0]) | usize::from(at[1]) << 8
}

/// Every way a call can be refused, one case each.
fn refusals() -> Vec<Refusal> {
    let good = fixture::library();
    let plain = plain();
    let last = symbols(&good) - 1;
    let one = assertion("ax-1");
    let mut out = vec![
        open_with("no glyph map", &plain, "FM_ERR_NO_MAP"),
        open_with(
            "another font",
            &patched(|b| {
                let at = section(b);
                b[at] ^= 1;
            }),
            "FM_ERR_FONT",
        ),
        open_with(
            "an offset past the section",
            &patched(|b| {
                let at = section(b) + 3;
                b[at] = 0xFF;
                b[at + 1] = 0xFF;
            }),
            "FM_ERR_BAD",
        ),
        open_with(
            "an entry with no codes",
            &patched(|b| {
                let at = entry(b, 1);
                b[at + 1] = 0;
            }),
            "FM_ERR_BAD",
        ),
        open_with(
            "a flag above bit 3",
            &patched(|b| {
                let at = entry(b, 1);
                b[at] |= 0x10;
            }),
            "FM_ERR_BAD",
        ),
        open_with(
            "a last entry whose codes run past the end",
            &patched(|b| {
                let at = entry(b, last);
                b[at + 1] = 255;
            }),
            "FM_ERR_BAD",
        ),
        open_with(
            "an entry cut after its flags",
            &patched(|b| {
                let at = section(b) + 3 + 2 * last;
                let end = u16::try_from(decoded(b).sections.unwrap().table[1].length - 1).unwrap();
                b[at..at + 2].copy_from_slice(&end.to_le_bytes());
            }),
            "FM_ERR_BAD",
        ),
    ];
    let mut too_big = open_with("a section over 2048 bytes", &good, "FM_ERR_TOO_BIG");
    let row = too_big.fm.rig.label("LS_GL") + 3;
    too_big.fm.rig.poke(row, &[0x01, 0x08, 0]);
    out.push(too_big);
    let mut closed = open_with("the reader closed", &good, "FM_ERR_CLOSED");
    closed.fm.rig = Rig::with_image(WITH_FM);
    closed.fm.rig.load(&good);
    out.push(closed);
    let statement = Regs {
        hl: one,
        de: 0xFF,
        ..Regs::default()
    };
    for (what, target, regs) in [
        ("statement", "FM_STATEMENT", statement),
        ("rows", "FM_ROWS", Regs::default()),
        ("draw", "FM_DRAW", Regs::default()),
    ] {
        out.push(Refusal {
            what,
            fm: Fm::bare(&good),
            target,
            regs,
            status: code("FM_ERR_NOT_OPEN"),
        });
    }
    let index = "FM_ERR_INDEX";
    out.extend([
        statement_with("no such assertion", &good, (0x7FFF, 0xFF), index),
        statement_with(
            "one past the last",
            &good,
            (assertion("longest") + 1, 0xFF),
            index,
        ),
        statement_with("no such hypothesis", &good, (one, 0), index),
        statement_with("a statement number past them", &good, (one, 0xFE), index),
    ]);
    let at = record_of(&good, 1, "LR_DCONC");
    out.push(statement_with(
        "a symbol past the glyph map",
        &patched(|b| b[at + 1] = 250),
        (1, 0xFF),
        "FM_ERR_BAD",
    ));
    let at = record_of(&good, 1, "LR_DREC");
    out.push(statement_with(
        "a record the reader refuses",
        &patched(|b| b[at + 1] = 0xFF),
        (1, 0xFF),
        "FM_ERR_READ",
    ));
    let mut wide = statement_with("a token over 255 pixels", &good, (one, 0xFF), "FM_ERR_BAD");
    let copy = wide.fm.rig.label("FM_GLYPHS") - SPARE_BASE;
    let spare = wide.fm.rig.label("FM_SECOND") - SPARE_BASE;
    let page = wide.fm.rig.m.ram_page(SPARE_PAGE);
    let codes = crate::ui_support::table().encode(&"m".repeat(60));
    let at = usize::from(spare);
    page[at] = 0;
    page[at + 1] = codes.len() as u8;
    page[at + 2..at + 2 + codes.len()].copy_from_slice(&codes);
    for symbol in 0..16 {
        let at = usize::from(copy) + 3 + 2 * symbol;
        page[at..at + 2].copy_from_slice(&(spare - copy).to_le_bytes());
    }
    out.push(wide);
    out.extend(crate::damaged::cases().into_iter().map(|d| d.refusal));
    out
}

#[test]
fn every_refusal_names_its_cause() {
    for mut case in refusals() {
        let out = case.fm.rig.call(case.target, case.regs);
        assert_eq!(out.status(), case.status, "{}", case.what);
        assert_eq!(out.bc >> 8, 0, "{}: nothing counted", case.what);
    }
}

#[test]
fn every_refusal_hands_the_callers_state_back() {
    for ei in [false, true] {
        for port5 in [0, 5] {
            for mut case in refusals() {
                case.fm.rig.ei = ei;
                case.fm.rig.m.out_port(5, port5);
                let regs = Regs {
                    iy: IY,
                    ..case.regs
                };
                let out = case.fm.rig.call(case.target, regs);
                let what = format!("{} with interrupts {ei}, port 5 {port5}", case.what);
                assert_eq!(out.status(), case.status, "{what}");
                assert_eq!(
                    (out.port5, out.port7, out.iff),
                    (port5, 0x81, u8::from(ei)),
                    "{what}"
                );
                assert_eq!(
                    (out.iy, case.fm.rig.m.regs().sp),
                    (IY, case.fm.rig.sp),
                    "{what}"
                );
                assert_eq!(out.af & 0x40, 0, "{what}: the zero flag is clear");
            }
        }
    }
}

#[test]
fn a_glyph_section_of_2048_bytes_is_not_too_big_and_one_of_no_bytes_is_bad() {
    let bytes = fixture::library();
    let state = |length: u16| {
        let mut fm = Fm::bare(&bytes);
        let row = fm.rig.label("LS_GL") + 3;
        fm.rig.poke(row, &[length as u8, (length >> 8) as u8, 0]);
        fm.open().status()
    };
    assert_eq!(state(0xFFFF), code("FM_ERR_TOO_BIG"));
    assert_eq!(state(2049), code("FM_ERR_TOO_BIG"));
    assert_ne!(state(2048), code("FM_ERR_TOO_BIG"));
    assert_eq!(state(0), code("FM_ERR_BAD"));
}

#[test]
fn a_refused_statement_leaves_nothing_loaded() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    let one = assertion("ax-1");
    assert_eq!(fm.statement(one, 0xFF).status(), 0);
    assert_ne!(fm.rows(94).bc >> 8, 0);
    for (n, k) in [(one, 0), (one, 1), (0x7FFF, 0xFF)] {
        assert_eq!(fm.statement(n, k).status(), code("FM_ERR_INDEX"));
        assert_eq!(fm.rows(94).bc >> 8, 0, "{n} {k} left nothing loaded");
        assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
        assert_eq!(fm.frame(), [0; FRAME_LEN]);
        assert_eq!(fm.statement(one, 0xFF).status(), 0);
    }
}

#[test]
fn an_entry_of_one_code_opens_and_one_of_none_is_bad() {
    let bytes = fixture::library();
    let glyphs = decoded(&bytes).sections.unwrap().glyphs.unwrap();
    let symbol = glyphs
        .symbols
        .iter()
        .position(|g| g.codes.len() >= 2)
        .unwrap();
    for (codes, status) in [(1, 0), (0, code("FM_ERR_BAD"))] {
        let short = patched(|b| {
            let at = entry(b, symbol);
            b[at + 1] = codes;
        });
        assert_eq!(Fm::bare(&short).open().status(), status, "{codes} codes");
    }
}

fn first_with(bytes: &[u8], codes: usize) -> usize {
    let glyphs = decoded(bytes).sections.unwrap().glyphs.unwrap();
    glyphs
        .symbols
        .iter()
        .position(|g| g.codes.len() >= codes)
        .unwrap()
}

#[test]
fn a_zero_among_an_entrys_codes_is_refused_and_the_same_entry_without_it_opens() {
    let bytes = fixture::library();
    let symbol = first_with(&bytes, 3);
    for (codes, status) in [([5, 0, 6], code("FM_ERR_BAD")), ([5, 7, 6], 0)] {
        let patched = patched(|b| {
            let at = entry(b, symbol);
            b[at + 1] = 3;
            b[at + 2..at + 5].copy_from_slice(&codes);
        });
        assert_eq!(Fm::bare(&patched).open().status(), status, "{codes:?}");
    }
}

#[test]
fn an_entry_of_128_codes_opens_and_one_of_129_is_bad() {
    // Every 0 in the entries becomes 1 so that the bytes after the first entry are good codes.
    let bytes = fixture::nodes_library();
    let glyphs = decoded(&bytes)
        .sections
        .unwrap()
        .table
        .into_iter()
        .find(|e| e.id == mcpack::library::GLYPHS)
        .unwrap();
    let end = glyphs.offset + glyphs.length;
    for (count, status) in [(128, 0), (129, code("FM_ERR_BAD"))] {
        let patched = fixture::with(&bytes, |b| {
            let at = entry(b, 0);
            for byte in &mut b[at..end] {
                *byte = (*byte).max(1);
            }
            b[at + 1] = count;
        });
        assert_eq!(Fm::bare(&patched).open().status(), status, "{count} codes");
    }
}
