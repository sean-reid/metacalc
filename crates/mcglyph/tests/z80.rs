use mcglyph::z80::{ROWS, SLOT, SPACE_CODE, TALL, Table};
use mcglyph::{Font, SPACE_WIDTH};

const WIDE: &str = include_str!("fixtures/wide.font");
const FLAGS: &str = include_str!("fixtures/flags.font");

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
    assert_eq!(bytes[0] & TALL, 0, "the space slot is never flagged");
    for (name, code) in table.glyphs() {
        let glyph = font.glyph(name).unwrap();
        let slot = &bytes[(usize::from(code) - 1) * SLOT..];
        assert_eq!(usize::from(slot[0] & !TALL), glyph.width(), "{name}");
        let tall = (5..ROWS).any(|y| (0..glyph.width()).any(|x| glyph.get(x, y)));
        assert_eq!(slot[0] & TALL != 0, tall, "{name}");
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
fn the_flag_follows_the_ninth_column_of_the_last_two_rows() {
    let font = Font::parse(FLAGS, "").unwrap();
    check(&font);
    let table = Table::new(&font).unwrap();
    let bytes = decode(&table.include("ui_font"));
    let width = |name: &str| bytes[(usize::from(table.glyph(name).unwrap()) - 1) * SLOT];
    assert_eq!(width("ninth"), 9 | TALL);
    assert_eq!(width("blank9"), 9);
}

#[test]
fn the_flag_is_checked_on_the_key_font_and_no_space_slot_carries_it() {
    check(Font::keys());
    for table in [Table::builtin(), Table::keys()] {
        assert_eq!(table.slots()[0][0] & TALL, 0);
    }
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
            assert_eq!(
                usize::from(decode(&include)[slot * SLOT] & !TALL),
                width,
                "{c:?}"
            );
        }
    }
}

#[test]
fn braces_bar_backslash_and_caret_are_emitted_with_their_own_codes() {
    let table = Table::builtin();
    let include = table.include("ui_font");
    let question = table.glyph("question").unwrap();
    for (c, name) in [
        ('{', "lbrace"),
        ('}', "rbrace"),
        ('|', "bar"),
        ('\\', "backslash"),
        ('^', "caret"),
    ] {
        let code = table.glyph(name).unwrap();
        assert_ne!(code, question, "{c:?}");
        let equ = format!("UI_FONT_CH_{:X}", c as u32);
        assert_eq!(equate(&include, &equ), usize::from(code), "{c:?}");
    }
}

#[test]
fn a_font_that_is_not_seven_rows_high_is_refused() {
    let font = Font::parse("height 5\n\nglyph question\n##\n##\n##\n##\n##\n", "").unwrap();
    assert!(Table::new(&font).is_err());
}

#[test]
fn a_font_without_a_question_mark_maps_only_what_it_draws() {
    let font = Font::parse("height 7\n\nglyph a\n#\n#\n#\n#\n#\n#\n#\n", "").unwrap();
    let table = Table::new(&font).unwrap();
    assert_eq!(table.try_code('a'), table.glyph("a"));
    assert_eq!(table.try_code('?'), None);
    assert_eq!(table.try_code(' '), Some(SPACE_CODE));
    assert!(!table.include("ui_font").contains("UI_FONT_CH_3F"));
}

#[test]
fn the_key_table_is_its_own_encoding_of_letters_and_digits() {
    let keys = Table::keys();
    let main = Table::builtin();
    check(Font::keys());
    assert_eq!(keys.slots().len() * SLOT, 63 * SLOT);
    assert_ne!(keys.code('z'), main.code('z'));
    assert_eq!(keys.try_code('?'), None);
    assert_eq!(keys.encode("Aply").last(), Some(&0));
    let include = mcglyph::z80::fonts_include();
    assert!(include.contains("UI_KEY_FONT_CH_41\t.equ"));
    assert!(!include.contains("UI_KEY_FONT_CH_3F"));
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

fn crc16(bytes: impl Iterator<Item = u8>) -> u16 {
    let mut crc = 0xFFFFu32;
    for byte in bytes {
        for bit in (0..8).rev() {
            let top = (crc >> 15) & 1 ^ u32::from(byte >> bit & 1);
            crc = (crc << 1 & 0xFFFF) ^ if top == 1 { 0x1021 } else { 0 };
        }
    }
    crc as u16
}

#[test]
fn the_fingerprint_is_the_crc_of_the_slot_bytes_and_follows_the_font() {
    let builtin = Table::builtin();
    let slots = builtin.slots().iter().flatten().copied();
    assert_eq!(builtin.fingerprint(), crc16(slots));
    let wide = Table::new(&Font::parse(WIDE, "").unwrap()).unwrap();
    let flags = Table::new(&Font::parse(FLAGS, "").unwrap()).unwrap();
    let prints = [
        builtin.fingerprint(),
        wide.fingerprint(),
        flags.fingerprint(),
    ];
    assert!(prints[0] != prints[1] && prints[1] != prints[2] && prints[0] != prints[2]);
    assert_ne!(builtin.fingerprint(), Table::keys().fingerprint());
}

#[test]
fn a_mapped_token_is_its_glyph_codes_and_its_spacing() {
    let table = Table::builtin();
    let font = Font::builtin();
    for token in font.tokens() {
        let (codes, pad) = table.token(token).unwrap();
        let glyphs = font.token_glyphs(token).unwrap();
        let want: Vec<u8> = glyphs.iter().map(|g| table.glyph(g).unwrap()).collect();
        assert_eq!(codes, want, "{token}");
        assert_eq!(Some(pad), font.token_pad(token), "{token}");
    }
    assert_eq!(table.token("->").unwrap().1.bits(), 3);
}

#[test]
fn an_unmapped_token_is_drawn_as_interface_text_with_no_spacing() {
    let table = Table::builtin();
    let font = Font::builtin();
    let token = "zz.a@";
    assert!(font.token_glyphs(token).is_none());
    let (codes, pad) = table.token(token).unwrap();
    assert_eq!(pad.bits(), 0);
    let question = table.glyph("question").unwrap();
    assert_eq!(
        codes,
        [
            table.code('z'),
            table.code('z'),
            table.code('.'),
            table.code('a'),
            question
        ]
    );
    let width: usize = codes
        .iter()
        .map(|&c| usize::from(table.slots()[usize::from(c) - 1][0] & !TALL))
        .sum::<usize>()
        + codes.len()
        - 1;
    assert_eq!(width, font.text_width(token));
}

#[test]
fn pad_bits_round_trip_and_refuse_a_fifth_flag() {
    for bits in 0..16u8 {
        assert_eq!(mcglyph::Pad::from_bits(bits).unwrap().bits(), bits);
    }
    assert_eq!(mcglyph::Pad::from_bits(16), None);
}

#[test]
fn an_unmapped_token_with_a_braced_character_uses_that_characters_glyph() {
    let table = Table::builtin();
    let font = Font::builtin();
    for (c, name) in [
        ('{', "lbrace"),
        ('|', "bar"),
        ('}', "rbrace"),
        ('\\', "backslash"),
        ('^', "caret"),
    ] {
        let token = format!("x{c}x");
        assert!(font.token_glyphs(&token).is_none());
        let x = table.code('x');
        assert_eq!(
            table.token(&token).unwrap().0,
            [x, table.glyph(name).unwrap(), x]
        );
    }
}
