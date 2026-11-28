use crate::secrig::*;
use crate::support::*;
pub use crate::treelibs::pack_with_sections;
use mcpack::library::{GLYPHS, Library, Sections, decode, pack_kernel};
use mcpack::mm::{Database, StmtId};

/// A grammar with a few rules over the variables p q r s.
pub const GRAMMAR: &str = "$c wff |- ( ) -> & -. $.\n$v p q r s $.\n\
    wp $f wff p $.\nwq $f wff q $.\nwr $f wff r $.\nws $f wff s $.\n\
    wn $a wff -. p $.\nwi $a wff ( p -> q ) $.\nwa $a wff ( p & q ) $.\n";

pub fn heading(depth: usize, title: &str) -> String {
    let pattern = ["##", "#*", "=-", "-."][depth - 1];
    let rule = pattern.repeat(80)[..79].to_string();
    format!("$(\n{rule}\n  {title}\n{rule}\n\n  Text.\n$)\n")
}

/// `count` axioms of every small shape, a heading above every `every`th of them, and trees for the
/// statements the grammar reads. Titles are `Section <n>` with `pad` extra characters.
pub fn headed(count: usize, every: usize, pad: usize) -> Vec<u8> {
    let mut src = String::from(GRAMMAR);
    let body = [
        "p",
        "( p -> q )",
        "( p & ( q -> r ) )",
        "( ( p -> q ) & ( r -> s ) )",
        "-. ( p & -. q )",
    ];
    for i in 0..count {
        if i % every == 0 {
            let text = format!("Section {i}{}", "x".repeat(pad));
            src += &heading(1 + (i / every) % 4, &text);
        }
        let expr = |k: usize| body[(i + k) % body.len()];
        let mut hyps = String::new();
        for h in 0..i % 4 {
            hyps += &format!("h{h}-{i} $e |- {} $.\n", expr(h + 1));
        }
        let disjoint = if i % 5 == 0 { "$d p q $.\n" } else { "" };
        let tail = "( p -> q ) ".repeat(i % 11);
        src += &format!(
            "${{\n{disjoint}{hyps}ax-{i:05}{pad} $a |- ( {} {tail}) $.\n$}}\n",
            expr(0),
            pad = "x".repeat(i % 23),
        );
    }
    pack_all(&parse(&src))
}

/// The assertions the grammar takes before the first axiom of `nested`.
pub const SYNTAX: usize = 3;

/// `count` axioms after the grammar, each a single variable, with the headings `top` above the
/// first statement of the database and those of `plan` above the axiom each names: (axiom
/// number, depths in source order).
pub fn nested(count: usize, top: &[usize], plan: &[(usize, &[usize])]) -> Vec<u8> {
    let mut src = String::new();
    for &d in top {
        src += &heading(d, &format!("Top at depth {d}"));
    }
    src += GRAMMAR;
    for i in 0..count {
        for (at, depths) in plan {
            if *at == i {
                for &d in *depths {
                    src += &heading(d, &format!("Heading {at} at depth {d}"));
                }
            }
        }
        src += &format!("ax-{i} $a |- p $.\n");
    }
    pack_all(&parse(&src))
}

pub fn everything(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

fn mask_of(sections: &Sections) -> u8 {
    let bit = |present: bool, mask: u8| if present { mask } else { 0 };
    bit(sections.titles.is_some(), HAVE_TITLES)
        | bit(sections.glyphs.is_some(), HAVE_GLYPHS)
        | bit(sections.trees.is_some(), HAVE_TREES)
        | bit(sections.shapes.is_some(), HAVE_SHAPES)
        | bit(sections.kinds.is_some(), HAVE_KINDS)
        | bit(sections.label_order.is_some(), HAVE_ORDER)
}

/// Every answer of the section routines for `bytes`, against what the decoder says. `order`
/// picks the sequence the titles, glyphs and statements are asked in.
pub fn check_sections(
    rig: &mut Rig,
    bytes: &[u8],
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
) {
    check_sections_with(rig, bytes, what, order, true);
}

/// `check_sections`, with the glyph entries left out when the library's map is for another font.
pub fn check_sections_with(
    rig: &mut Rig,
    bytes: &[u8],
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
    glyphs: bool,
) {
    let library = decoded(bytes);
    let Some(sections) = library.sections.as_ref() else {
        assert_eq!(rig.sections(), (ST_OK, 0), "{what}");
        assert_eq!(rig.title_count().0, ST_NONE, "{what}");
        assert_eq!(rig.shapes().0, ST_NONE, "{what}");
        assert_eq!(rig.shape(0).0, ST_NONE, "{what}");
        return;
    };
    assert_eq!(rig.sections(), (ST_OK, mask_of(sections)), "{what}");
    let state = rig.section_state();
    for (row, id) in state.rows.iter().zip(ids()) {
        match sections.table.iter().find(|e| e.id == id) {
            Some(e) => assert_eq!(
                (row.0 as usize, row.1 as usize),
                (e.offset, e.length),
                "{what}: row {id}"
            ),
            None => assert_eq!(state.have & have_bit(id), 0, "{what}: row {id}"),
        }
    }
    check_titles(rig, &library, sections, what, order);
    if glyphs {
        check_glyphs(rig, &library, sections, what, order);
    }
    check_trees(rig, &library, sections, what, order);
    check_shapes(rig, sections, what, order);
}

fn check_shapes(
    rig: &mut Rig,
    sections: &Sections,
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
) {
    let Some(shapes) = &sections.shapes else {
        assert_eq!(rig.shapes().0, ST_NONE, "{what}");
        assert_eq!(rig.shape(0).0, ST_NONE, "{what}");
        return;
    };
    let entry = sections
        .table
        .iter()
        .find(|e| e.id == mcpack::library::SHAPES);
    let entry = entry.expect("a shapes row");
    assert_eq!(
        rig.shapes(),
        (ST_OK, entry.offset as u32, entry.length as u16),
        "{what}"
    );
    for i in order(shapes.axioms.len()) {
        assert_eq!(
            rig.shape(i as u16),
            (ST_OK, shapes.axioms[i].clone()),
            "{what}: shape {i}"
        );
    }
    assert_eq!(rig.shape(shapes.axioms.len() as u16).0, ST_INDEX, "{what}");
}

fn check_titles(
    rig: &mut Rig,
    library: &Library,
    sections: &Sections,
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
) {
    let Some(titles) = &sections.titles else {
        assert_eq!(rig.title_count().0, ST_NONE, "{what}");
        assert_eq!(rig.title(0).0, ST_NONE, "{what}");
        assert_eq!(rig.title_for(0).0, ST_NONE, "{what}");
        return;
    };
    assert_eq!(rig.title_count(), (ST_OK, titles.len() as u16), "{what}");
    for n in order(titles.len()) {
        let t = &titles[n];
        let want = (
            ST_OK,
            t.assertion as u16,
            t.depth,
            t.text.as_bytes().to_vec(),
        );
        assert_eq!(rig.title(n as u16), want, "{what}: title {n}");
    }
    assert_eq!(rig.title(titles.len() as u16).0, ST_INDEX, "{what}");
    let assertions = library.assertions.len();
    for k in order(assertions) {
        let after = titles.partition_point(|t| t.assertion <= k);
        let want = match after {
            0 => (ST_NONE, 0),
            n => (ST_OK, (n - 1) as u16),
        };
        assert_eq!(rig.title_for(k as u16), want, "{what}: title for {k}");
    }
    assert_eq!(rig.title_for(assertions as u16).0, ST_INDEX, "{what}");
}

fn check_glyphs(
    rig: &mut Rig,
    library: &Library,
    sections: &Sections,
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
) {
    let Some(map) = &sections.glyphs else {
        assert_eq!(rig.glyph_map().0, ST_NONE, "{what}");
        assert_eq!(rig.glyphs().0, ST_NONE, "{what}");
        assert_eq!(rig.glyph(0).0, ST_NONE, "{what}");
        return;
    };
    assert_eq!(rig.glyph_map(), (ST_OK, map.fingerprint), "{what}");
    let row = sections.table.iter().find(|e| e.id == GLYPHS).unwrap();
    let section = (ST_OK, row.offset as u32, row.length as u16);
    assert_eq!(rig.glyphs(), section, "{what}: glyph section");
    for id in order(map.symbols.len()) {
        let g = &map.symbols[id];
        assert_eq!(
            rig.glyph(id as u8),
            (ST_OK, g.flags, g.codes.clone()),
            "{what}: glyphs of symbol {id}"
        );
    }
    let symbols = library.constants.len() + library.variables.len();
    assert_eq!(rig.glyph(symbols as u8).0, ST_INDEX, "{what}");
}

fn check_trees(
    rig: &mut Rig,
    library: &Library,
    sections: &Sections,
    what: &str,
    order: &dyn Fn(usize) -> Vec<usize>,
) {
    let Some(trees) = &sections.trees else {
        assert_eq!(rig.syntax_count().0, ST_NONE, "{what}");
        assert_eq!(rig.tree(0, 0xFF).status, ST_NONE, "{what}");
        return;
    };
    assert_eq!(
        rig.syntax_count(),
        (ST_OK, trees.syntax.len() as u16),
        "{what}"
    );
    for i in order(trees.syntax.len()) {
        let s = &trees.syntax[i];
        let want = (ST_OK, s.assertion as u16, s.arity as u8);
        assert_eq!(rig.syntax(i as u16), want, "{what}: syntax {i}");
    }
    assert_eq!(rig.syntax(trees.syntax.len() as u16).0, ST_INDEX, "{what}");
    for n in order(trees.items.len()) {
        let item = &trees.items[n];
        for (k, stored) in item.iter().enumerate() {
            let last = k + 1 == item.len();
            let asked = if last && n % 2 == 0 { 0xFF } else { k as u8 };
            let got = rig.tree(n as u16, asked);
            if stored.is_none() {
                assert_eq!(got.status, ST_NONE, "{what}: tree {k} of {n}");
                continue;
            }
            assert_eq!(got.status, ST_OK, "{what}: tree {k} of {n}");
            let file = &bytes_of(rig, got.offset, got.len);
            assert_eq!(
                &got.bytes[..],
                &file[..file.len().min(255)],
                "{what}: tree {k} of {n}"
            );
        }
        let past = item.len() as u8;
        assert_eq!(
            rig.tree(n as u16, past).status,
            ST_INDEX,
            "{what}: tree {past} of {n}"
        );
    }
    let n = library.assertions.len() as u16;
    assert_eq!(rig.tree(n, 0xFF).status, ST_INDEX, "{what}");
}

/// The tree bytes at a library offset, read through lib_read.
fn bytes_of(rig: &mut Rig, offset: u32, len: u16) -> Vec<u8> {
    let mut out = Vec::new();
    while out.len() < usize::from(len) {
        let take = (usize::from(len) - out.len()).min(256);
        let (status, bytes) = rig.read(offset + out.len() as u32, take as u16);
        assert_eq!(status, ST_OK);
        out.extend(bytes);
    }
    out
}

pub fn ascending(n: usize) -> Vec<usize> {
    (0..n).collect()
}

/// Every index once, in a fixed scrambled order.
pub fn scrambled(n: usize) -> Vec<usize> {
    let mut all: Vec<usize> = (0..n).collect();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        all.swap(i, (state % (i as u64 + 1)) as usize);
    }
    all
}

pub fn opened(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    rig
}

#[test]
fn every_corpus_library_reads_back_its_sections_as_the_decoder_decodes_them() {
    for name in CORPUS {
        let bytes = pack_all(&corpus(name));
        let mut rig = opened(&bytes);
        check_sections(&mut rig, &bytes, name, &ascending);
        check_sections(&mut rig, &bytes, name, &scrambled);
    }
}

#[test]
fn the_many_chunk_library_reads_back_its_sections() {
    let bytes = big_library();
    let mut rig = opened(&bytes);
    check_sections(&mut rig, &bytes, "big_library", &ascending);
}

#[test]
fn a_library_with_titles_and_trees_across_many_chunks_reads_back_in_and_out_of_order() {
    let bytes = headed(2400, 12, 0);
    let sections = decoded(&bytes).sections.unwrap();
    assert!(sections.titles.as_ref().unwrap().len() > 150);
    assert!(sections.trees.as_ref().unwrap().without_tree() > 0);
    assert!(bytes.len() > 8 * CHUNK);
    let mut rig = opened(&bytes);
    check_sections(&mut rig, &bytes, "headed", &ascending);
    check_sections(&mut rig, &bytes, "headed", &scrambled);
}

#[test]
fn a_library_packed_without_sections_opens_and_reads_as_before() {
    for name in CORPUS {
        let db = corpus(name);
        let bytes = pack_kernel(&db, &everything(&db)).expect("it packs");
        let mut rig = opened(&bytes);
        assert_eq!(rig.sections(), (ST_OK, 0), "{name}");
        assert_eq!(rig.section_state().have, 0, "{name}");
        for call in [
            "LIB_TITLE_COUNT",
            "LIB_GLYPH_MAP",
            "LIB_GLYPHS",
            "LIB_SYNTAX_COUNT",
        ] {
            assert_eq!(rig.call(call, Regs::default()).status(), ST_NONE, "{name}");
        }
        let library = decode(&bytes).unwrap();
        assert!(library.sections.is_none());
        assert_eq!(rig.counts().1 as usize, library.assertions.len(), "{name}");
        for (n, assertion) in library.assertions.iter().enumerate() {
            let (status, label) = rig.label_of(n as u16);
            assert_eq!(status, ST_OK);
            assert_eq!(label, assertion.label.as_bytes());
        }
        assert_eq!(rig.read(0, 4).1, b"MCL1", "{name}");
    }
    let bytes = {
        let db = parse("$c wff $.\n");
        pack_kernel(&db, &everything(&db)).unwrap()
    };
    assert_eq!(opened(&bytes).sections(), (ST_OK, 0));
}

#[test]
fn a_database_without_a_grammar_has_no_trees_and_a_library_may_lack_any_one_section() {
    let bytes = pack_with_sections("$c |- foo $.\nax $a |- foo $.\n");
    let library = decoded(&bytes);
    let sections = library.sections.as_ref().unwrap();
    assert!(sections.trees.is_none());
    assert_eq!(
        opened(&bytes).sections(),
        (ST_OK, HAVE_TITLES | HAVE_GLYPHS | HAVE_KINDS | HAVE_ORDER)
    );
}

fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[test]
fn a_library_packed_against_the_font_table_the_image_carries_is_usable_with_it() {
    let image = DEFAULT;
    let slots = constant_of(image.labels, "UI_FONT_GLYPHS") as usize;
    let at = usize::from(label_in(image, "UI_FONT") - image.org);
    let table = &image.code[at..at + 8 * slots];
    let carried = constant_of(image.labels, "UI_FONT_FINGERPRINT") as u16;
    assert_eq!(
        crc16(table),
        carried,
        "the equate is the CRC of the bytes in the image"
    );
    assert_eq!(
        constant_of(image.labels, "LIB_FONT_FINGERPRINT") as u16,
        carried
    );
    for name in ["demo0.mm", "miu.mm"] {
        let bytes = pack_all(&corpus(name));
        let packed = decoded(&bytes).sections.unwrap().glyphs.unwrap();
        assert_eq!(packed.fingerprint, carried, "{name}");
        for glyphs in &packed.symbols {
            assert!(
                glyphs.codes.iter().all(|&c| (1..=slots as u8).contains(&c)),
                "{name}"
            );
        }
        let mut rig = opened(&bytes);
        assert_eq!(rig.glyph_map(), (ST_OK, carried), "{name}");
        assert_eq!(
            rig.state_byte("LS_HAVE") & FONT_MATCHES,
            FONT_MATCHES,
            "{name}"
        );
    }
}

#[test]
fn several_titles_on_one_assertion_and_none_below_the_first_title_are_found_as_the_host_finds_them()
{
    let runs: &[(usize, &[usize])] = &[
        (3, &[2]),
        (4, &[1, 2]),
        (8, &[2, 3, 4]),
        (9, &[1]),
        (11, &[3, 4]),
    ];
    let bytes = nested(14, &[1, 2, 3], runs);
    let titles = decoded(&bytes).sections.unwrap().titles.unwrap();
    let on = |n: usize| titles.iter().filter(|t| t.assertion == n).count();
    assert_eq!((on(0), on(SYNTAX + 4), on(SYNTAX + 8)), (3, 2, 3));
    let mut rig = opened(&bytes);
    check_sections(&mut rig, &bytes, "runs of titles", &ascending);
    check_sections(&mut rig, &bytes, "runs of titles", &scrambled);
    assert_eq!(
        rig.title_for(0),
        (ST_OK, 2),
        "the last of three on assertion 0"
    );
    let last = titles
        .iter()
        .rposition(|t| t.assertion == SYNTAX + 8)
        .unwrap();
    assert_eq!(rig.title_for((SYNTAX + 8) as u16), (ST_OK, last as u16));

    let late: &[(usize, &[usize])] = &[(5, &[1, 2]), (6, &[3]), (9, &[2])];
    let bytes = nested(12, &[], late);
    let titles = decoded(&bytes).sections.unwrap().titles.unwrap();
    let first = titles[0].assertion;
    assert_eq!(first, SYNTAX + 5, "the first title sits above assertion 0");
    let mut rig = opened(&bytes);
    check_sections(&mut rig, &bytes, "late first title", &ascending);
    for k in 0..first {
        assert_eq!(
            rig.title_for(k as u16),
            (ST_NONE, 0),
            "assertion {k} is above every title"
        );
    }
    assert_eq!(rig.title_for(first as u16), (ST_OK, 1));
}
