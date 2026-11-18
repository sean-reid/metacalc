use mcglyph::z80::{ROWS, SLOT, SPACE_CODE, Table};
use mcglyph::{Font, SPACE_WIDTH};

const WIDE: &str = include_str!("fixtures/wide.font");

fn decode(include: &str) -> Vec<u8> {
    include
        .lines()
        .filter_map(|l| l.strip_prefix("\t.db "))
        .flat_map(|l| {
            l.split(',')
                .map(|b| u8::from_str_radix(&b[1..], 16).unwrap())
        })
        .collect()
}

fn equate(include: &str, name: &str) -> usize {
    include
        .lines()
        .find_map(|l| l.strip_prefix(name)?.strip_prefix("\t.equ "))
        .unwrap_or_else(|| panic!("no equate {name}"))
        .parse()
        .unwrap()
}

fn check(font: &Font) {
    let table = Table::new(font).unwrap();
    let include = table.include("ui_font");
    let bytes = decode(&include);
    assert_eq!(bytes.len(), equate(&include, "UI_FONT_GLYPHS") * SLOT);
    assert_eq!(bytes.len(), table.slots().len() * SLOT);
    assert_eq!(&bytes[..SLOT], [SPACE_WIDTH as u8, 0, 0, 0, 0, 0, 0, 0]);
    for (name, code) in table.glyphs() {
        let glyph = font.glyph(name).unwrap();
        let slot = &bytes[(usize::from(code) - 1) * SLOT..];
        assert_eq!(usize::from(slot[0]), glyph.width(), "{name}");
        for y in 0..ROWS {
            for x in 0..glyph.width() + 7 {
                let bit = match x {
                    0..8 => slot[1 + y] & (0x80 >> x) != 0,
                    8 if glyph.width() == 9 => slot[SLOT + 1 + y] & 0x80 != 0,
                    _ => false,
                };
                assert_eq!(bit, glyph.get(x, y), "{name} at {x},{y}");
            }
        }
        if glyph.width() == 9 {
            assert_eq!(slot[SLOT], 0, "{name}");
            assert!(
                slot[SLOT + 1..2 * SLOT].iter().all(|b| b & 0x7F == 0),
                "{name}"
            );
        }
    }
}

#[test]
fn the_emitted_table_holds_every_glyph_of_the_built_in_font() {
    check(Font::builtin());
}

#[test]
fn a_glyph_nine_wide_takes_a_second_slot() {
    let font = Font::parse(WIDE, "").unwrap();
    check(&font);
    let table = Table::new(&font).unwrap();
    let (a, b) = (table.glyph("a").unwrap(), table.glyph("b").unwrap());
    assert_eq!(b, a + 2);
}

#[test]
fn the_built_in_table_is_1328_bytes() {
    assert_eq!(Table::builtin().slots().len() * SLOT, 1328);
}

#[test]
fn characters_map_to_codes_as_the_text_layout_picks_glyphs() {
    let table = Table::builtin();
    let font = Font::builtin();
    assert_eq!(table.code(' '), SPACE_CODE);
    assert_eq!(table.code('\u{e9}'), table.glyph("question").unwrap());
    assert_eq!(table.code('~'), table.glyph("tilde").unwrap());
    assert_eq!(table.encode(""), [0]);
    assert_eq!(
        table.encode("a b"),
        [table.code('a'), 1, table.code('b'), 0]
    );
    let include = table.include("ui_font");
    for c in (' '..='~').chain(['\u{2713}', '\u{2717}', '\u{2026}']) {
        let name = format!("UI_FONT_CH_{:X}", c as u32);
        assert_eq!(equate(&include, &name), usize::from(table.code(c)), "{c:?}");
        if c != ' ' {
            let width = font.text_width(&c.to_string());
            let slot = usize::from(table.code(c)) - 1;
            assert_eq!(usize::from(decode(&include)[slot * SLOT]), width, "{c:?}");
        }
    }
}

#[test]
fn a_font_that_is_not_seven_rows_high_is_refused() {
    let font = Font::parse("height 5\n\nglyph question\n##\n##\n##\n##\n##\n", "").unwrap();
    assert!(Table::new(&font).is_err());
}

#[test]
fn a_font_without_a_question_mark_is_refused() {
    let font = Font::parse("height 7\n\nglyph a\n#\n#\n#\n#\n#\n#\n#\n", "").unwrap();
    assert!(Table::new(&font).is_err());
}

/// A font of `n` glyphs in seven rows, one of them the question mark.
fn font_of(n: usize) -> Font {
    let mut src = String::from("height 7\n\nglyph question\n#\n#\n#\n#\n#\n#\n#\n");
    for i in 1..n {
        src += &format!("\nglyph g{i:03}\n#\n#\n#\n#\n#\n#\n#\n");
    }
    Font::parse(&src, "").unwrap()
}

#[test]
fn a_table_of_255_slots_is_accepted_and_256_refused() {
    let table = Table::new(&font_of(254)).unwrap();
    assert_eq!(table.slots().len(), 255);
    assert_eq!(table.glyph("question"), Some(255));
    assert!(Table::new(&font_of(255)).is_err());
}

#[test]
fn a_wide_last_glyph_that_needs_a_256th_slot_is_refused() {
    let mut src = String::from("height 7\n\nglyph question\n#\n#\n#\n#\n#\n#\n#\n");
    for i in 1..254 {
        src += &format!("\nglyph g{i:03}\n#\n#\n#\n#\n#\n#\n#\n");
    }
    src += "\nglyph zwide\n#########\n#########\n#########\n#########\n#########\n#########\n#########\n";
    let font = Font::parse(&src, "").unwrap();
    assert!(Table::new(&font).is_err());
}
