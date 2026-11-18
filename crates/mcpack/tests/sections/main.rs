mod trees;

use mcglyph::z80::Table;
use mcpack::library::{
    self, ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION, Title, crc32, decode, pack,
    pack_kernel,
};
use mcpack::mm::{self, Database, Grammar, StmtId};

const LOGIC: &str = "
$c ( ) -> wff |- setvar A. $.
$v ph ps ch x $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
wi $a wff ( ph -> ps ) $.
wal $a wff A. x ph $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  $d x ph $.
  ax-5 $a |- ( ph -> A. x ph ) $.
$}
${
  a1i.1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.
$}
";

fn parse(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("the database parses")
}

fn db() -> Database {
    parse(LOGIC)
}

/// Everything the database holds except the named assertions and what only they need.
fn closure_without(db: &Database, skip: &[&str]) -> Vec<StmtId> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).expect("the logic has a grammar");
    let roots: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| {
            db.stmt(id).kind.is_assertion() && !skip.contains(&db.stmt(id).label.as_str())
        })
        .collect();
    mm::closure(db, &grammar, &roots).expect("the logic closes")
}

fn closure(db: &Database) -> Vec<StmtId> {
    closure_without(db, &[])
}

fn number(db: &Database, kept: &[StmtId], label: &str) -> usize {
    let id = db.find(label).expect("the label exists");
    kept.iter()
        .filter(|&&k| db.stmt(k).kind.is_assertion())
        .position(|&k| k == id)
        .expect("the library keeps it")
}

fn rule(pattern: &str) -> String {
    pattern.repeat(80)[..79].to_string()
}

fn heading(pattern: &str, title: &str) -> String {
    format!("$(\n{0}\n  {title}\n{0}\n\n  Text.\n$)\n", rule(pattern))
}

/// The logic with titles at all four levels: a title over nothing the library keeps, a title
/// whose assertions are all in its subsections, and a theorem the library leaves out.
fn headed() -> String {
    let cut = |label: &str| LOGIC.find(label).unwrap();
    let (syntax, axioms, mp, quant, theorems) = (
        cut("wi $a"),
        cut("ax-1 $a"),
        cut("${\n  min"),
        cut("${\n  $d x ph"),
        cut("${\n  a1i.1"),
    );
    let mut out = heading("##", "Classical logic");
    out += &LOGIC[..syntax];
    out += &heading("=-", "Syntax");
    out += &LOGIC[syntax..axioms];
    out += &heading("#*", "Propositional calculus");
    out += &heading("=-", "Axioms");
    out += &LOGIC[axioms..mp];
    out += &heading("-.", "Modus ponens");
    out += &LOGIC[mp..quant];
    out += &heading("=-", "Nothing here");
    out += &heading("=-", "Quantifiers");
    out += &LOGIC[quant..theorems];
    out += &heading("=-", "Theorems");
    out += &LOGIC[theorems..];
    out
}

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn a_kernel_only_library_has_no_trailer_and_reports_no_sections() {
    let db = db();
    let bytes = pack_kernel(&db, &closure(&db)).unwrap();
    assert_ne!(&bytes[bytes.len() - 4..], TRAILER_MAGIC);
    assert_eq!(decode(&bytes).unwrap().sections, None);
}

#[test]
fn the_trailer_closes_the_file_and_the_length_and_crc_cover_it() {
    let db = db();
    let kernel = pack_kernel(&db, &closure(&db)).unwrap();
    let bytes = pack(&db, &closure(&db)).unwrap();
    assert_eq!(&bytes[bytes.len() - 4..], TRAILER_MAGIC);
    assert_eq!(bytes[bytes.len() - 5], TRAILER_VERSION);
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(length, bytes.len());
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        crc32(&bytes[8..])
    );
    let sections = decode(&bytes).unwrap().sections.expect("a trailer");
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    assert_eq!(sections.table.len(), count);
    assert_eq!(&bytes[..4], &kernel[..4]);
    assert_eq!(&bytes[12..kernel.len()], &kernel[12..]);
    library::check(&bytes).expect("the kernel accepts a library with sections");
}

#[test]
fn every_byte_of_the_trailer_and_table_decodes_or_errors() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    for at in first..bytes.len() {
        for flip in [0x01u8, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
        }
    }
}

fn titles_of(bytes: &[u8]) -> Vec<(usize, u8, String)> {
    let sections = decode(bytes).unwrap().sections.expect("a trailer");
    let titles = sections.titles.expect("a titles section");
    titles
        .into_iter()
        .map(
            |Title {
                 assertion,
                 depth,
                 text,
             }| (assertion, depth, text),
        )
        .collect()
}

#[test]
fn titles_keep_what_the_library_has_something_under() {
    let db = parse(&headed());
    let kept = closure_without(&db, &["a1i"]);
    let bytes = pack(&db, &kept).unwrap();
    let at = |label: &str| number(&db, &kept, label);
    let want = [
        (at("wi"), 1, "Classical logic"),
        (at("wi"), 3, "Syntax"),
        (at("ax-1"), 2, "Propositional calculus"),
        (at("ax-1"), 3, "Axioms"),
        (at("ax-mp"), 4, "Modus ponens"),
        (at("ax-5"), 3, "Quantifiers"),
    ];
    let want: Vec<_> = want
        .iter()
        .map(|&(n, d, t)| (n, d, t.to_string()))
        .collect();
    assert_eq!(titles_of(&bytes), want);
    library::check(&bytes).unwrap();
}

#[test]
fn a_theorem_the_library_includes_brings_its_title_in() {
    let db = parse(&headed());
    let kept = closure(&db);
    let titles = titles_of(&pack(&db, &kept).unwrap());
    let last = titles.last().unwrap();
    assert_eq!(
        (last.0, last.1, last.2.as_str()),
        (number(&db, &kept, "a1i"), 3, "Theorems")
    );
}

#[test]
fn a_database_with_no_titles_has_an_empty_titles_section() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    assert!(titles_of(&bytes).is_empty());
}

#[test]
fn titles_survive_a_hostile_byte_anywhere_in_the_section() {
    let db = parse(&headed());
    let bytes = pack(&db, &closure(&db)).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    let entry = sections
        .table
        .iter()
        .find(|e| e.id == library::TITLES)
        .unwrap();
    for at in entry.offset..entry.offset + entry.length {
        for flip in [0x01u8, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
        }
    }
}

#[test]
fn a_library_whose_titles_run_backwards_fails_the_check() {
    let db = parse(&headed());
    let mut bytes = pack(&db, &closure(&db)).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    let entry = sections
        .table
        .iter()
        .find(|e| e.id == library::TITLES)
        .unwrap();
    let first = u32::from_le_bytes(
        bytes[entry.offset + 2..entry.offset + 6]
            .try_into()
            .unwrap(),
    ) as usize;
    bytes[first..first + 2].copy_from_slice(&1000u16.to_le_bytes());
    reseal(&mut bytes);
    assert!(
        decode(&bytes).is_err(),
        "a number past the last assertion does not decode"
    );
    bytes[first..first + 2].copy_from_slice(&6u16.to_le_bytes());
    reseal(&mut bytes);
    assert!(matches!(
        library::check(&bytes),
        Err(library::CheckError::Sections(_))
    ));
}

#[test]
fn no_title_crosses_a_16_kb_boundary() {
    let mut src = String::from("$c |- wff $.\n$v ph $.\nwph $f wff ph $.\n");
    let titles: Vec<String> = (0..700)
        .map(|i| format!("Section {i} {}", "x".repeat(40)))
        .collect();
    for (i, title) in titles.iter().enumerate() {
        src += &heading("=-", title);
        src += &format!("ax-{i} $a |- ph $.\n");
    }
    let db = parse(&src);
    let bytes = pack(&db, &closure(&db)).unwrap();
    let sections = decode(&bytes).unwrap().sections.unwrap();
    let entry = sections
        .table
        .iter()
        .find(|e| e.id == library::TITLES)
        .unwrap();
    assert!(entry.length > 2 * library::CHUNK);
    let got: Vec<String> = titles_of(&bytes).into_iter().map(|t| t.2).collect();
    assert_eq!(got, titles);
}

fn section(bytes: &[u8], id: u8) -> mcpack::library::Entry {
    let sections = decode(bytes).unwrap().sections.expect("a trailer");
    *sections
        .table
        .iter()
        .find(|e| e.id == id)
        .expect("the section exists")
}

fn glyphs_of(bytes: &[u8]) -> library::GlyphMap {
    decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .glyphs
        .expect("a glyph map")
}

fn symbol_names(bytes: &[u8]) -> Vec<String> {
    let library = decode(bytes).unwrap();
    let variables = library.variables.iter().map(|v| v.name.clone());
    library.constants.iter().cloned().chain(variables).collect()
}

#[test]
fn the_glyph_map_has_one_entry_per_symbol_and_equals_what_mcglyph_gives() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let map = glyphs_of(&bytes);
    let table = Table::builtin();
    assert_eq!(map.fingerprint, table.fingerprint());
    let names = symbol_names(&bytes);
    assert_eq!(map.symbols.len(), names.len());
    for (symbol, name) in map.symbols.iter().zip(&names) {
        let (codes, pad) = table.token(name).unwrap();
        assert_eq!((&symbol.codes, symbol.pad()), (&codes, pad), "{name}");
    }
    let arrow = names.iter().position(|n| n == "->").unwrap();
    assert_eq!(map.symbols[arrow].codes, [table.glyph("rarr").unwrap()]);
    assert_eq!(map.symbols[arrow].flags, 3);
}

#[test]
fn a_symbol_the_font_does_not_map_is_stored_as_the_host_draws_it() {
    let db = parse("$c |- wff foo a|b $.\n$v ph $.\nwph $f wff ph $.\nax $a |- foo a|b ph $.\n");
    let everything: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &everything).unwrap();
    let map = glyphs_of(&bytes);
    let names = symbol_names(&bytes);
    let table = Table::builtin();
    let at = |name: &str| names.iter().position(|n| n == name).unwrap();
    let foo = &map.symbols[at("foo")];
    assert_eq!(
        foo.codes,
        [table.code('f'), table.code('o'), table.code('o')]
    );
    assert_eq!(foo.flags, 0);
    let odd = &map.symbols[at("a|b")];
    assert_eq!(odd.codes[1], table.glyph("question").unwrap());
    let font = mcglyph::Font::builtin();
    let slot_width =
        |c: u8| usize::from(table.slots()[usize::from(c) - 1][0] & !mcglyph::z80::TALL);
    let width = odd.codes.iter().map(|&c| slot_width(c)).sum::<usize>() + odd.codes.len() - 1;
    assert_eq!(width, font.text_width("a|b"));
}

#[test]
fn a_glyph_map_for_another_font_or_with_another_code_fails_the_check() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let entry = section(&bytes, library::GLYPHS);
    let mut other_font = bytes.clone();
    other_font[entry.offset] ^= 1;
    reseal(&mut other_font);
    assert!(matches!(
        library::check(&other_font),
        Err(library::CheckError::Sections(_))
    ));

    let mut other_code = bytes.clone();
    let first = entry.offset
        + u16::from_le_bytes([bytes[entry.offset + 3], bytes[entry.offset + 4]]) as usize;
    other_code[first + 2] ^= 1;
    reseal(&mut other_code);
    let Err(library::CheckError::Sections(e)) = library::check(&other_code) else {
        panic!("the check accepts a wrong code");
    };
    assert!(e.to_string().contains("glyph map"), "{e}");
}

#[test]
fn the_glyph_map_survives_a_hostile_byte_anywhere_in_the_section() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let entry = section(&bytes, library::GLYPHS);
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
