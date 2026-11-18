//! The font as data for the Z80 canvas.
//!
//! A glyph is a slot of eight bytes: its width, then seven row bytes with the leftmost pixel in
//! bit 7. Bit 7 of the width byte is set when either of the last two rows has a pixel, so the
//! Z80 skips those rows for the glyphs without one. A glyph nine pixels wide takes a second slot whose first byte is unused and whose
//! other seven bytes hold the ninth pixel of each row in bit 7. A string is a run of glyph
//! codes ended by 0: code 1 is the space, and a code is the number of the glyph's first slot
//! plus one, in name order.

use crate::font::{Font, MAX_GLYPH_WIDTH, Pad};
use crate::text::{CHECK, CROSS, ELLIPSIS, SPACE_WIDTH, glyph_name};
use std::collections::BTreeMap;

pub const SLOT: usize = 8;
pub const ROWS: usize = 7;
pub const SPACE_CODE: u8 = 1;
/// Set in a width byte when row 5 or 6 of the glyph has a pixel.
pub const TALL: u8 = 0x80;

/// The slots of a font and the code of each glyph.
pub struct Table {
    slots: Vec<[u8; SLOT]>,
    codes: BTreeMap<String, u8>,
    question: Option<u8>,
    tokens: BTreeMap<String, (Vec<u8>, Pad)>,
}

impl Table {
    pub fn new(font: &Font) -> Result<Table, String> {
        if font.height() != ROWS {
            return Err(format!(
                "the font is {} rows high, not {ROWS}",
                font.height()
            ));
        }
        let mut space = [0; SLOT];
        space[0] = SPACE_WIDTH as u8;
        let mut slots = vec![space];
        let mut codes = BTreeMap::new();
        for name in font.glyph_names() {
            let glyph = font.glyph(name).expect("the name came from the font");
            let width = glyph.width();
            if width > MAX_GLYPH_WIDTH {
                return Err(format!("glyph {name} is {width} wide"));
            }
            codes.insert(
                name.to_string(),
                u8::try_from(slots.len() + 1).map_err(|_| "more glyphs than codes")?,
            );
            let mut rows = [0; SLOT];
            let mut extra = [0; SLOT];
            rows[0] = width as u8;
            for y in 0..ROWS {
                for x in 0..width {
                    if glyph.get(x, y) {
                        let target = if x < 8 {
                            &mut rows[y + 1]
                        } else {
                            &mut extra[y + 1]
                        };
                        *target |= 0x80 >> (x % 8);
                    }
                }
            }
            if rows[6..].iter().chain(&extra[6..]).any(|&b| b != 0) {
                rows[0] |= TALL;
            }
            slots.push(rows);
            if width > 8 {
                slots.push(extra);
            }
        }
        if slots.len() > 255 {
            return Err(format!("{} slots do not fit one-byte codes", slots.len()));
        }
        let question = codes.get("question").copied();
        let tokens = font
            .tokens()
            .map(|token| {
                let glyphs = font.token_glyphs(token).expect("the token is in the font");
                let codes = glyphs.iter().map(|g| codes[g]).collect();
                let pad = font.token_pad(token).expect("the token is in the font");
                (token.to_string(), (codes, pad))
            })
            .collect();
        Ok(Table {
            slots,
            codes,
            question,
            tokens,
        })
    }

    /// The table of the font the app ships.
    pub fn builtin() -> Table {
        Table::new(Font::builtin()).expect("the built-in font fits the Z80 table")
    }

    /// The table of the soft key font. Its codes are its own: a label for the key row is
    /// encoded with this table, everything else with [`Table::builtin`].
    pub fn keys() -> Table {
        Table::new(Font::keys()).expect("the key font fits the Z80 table")
    }

    pub fn slots(&self) -> &[[u8; SLOT]] {
        &self.slots
    }

    /// The code of the glyph called `name`.
    pub fn glyph(&self, name: &str) -> Option<u8> {
        self.codes.get(name).copied()
    }

    /// Glyph names with their codes, in code order.
    pub fn glyphs(&self) -> impl Iterator<Item = (&str, u8)> {
        let mut all: Vec<_> = self.codes.iter().map(|(n, &c)| (n.as_str(), c)).collect();
        all.sort_by_key(|&(_, c)| c);
        all.into_iter()
    }

    /// The code interface text uses for `c`: the space, the glyph named for it, or the question
    /// mark when the font has none; `None` when the font has neither.
    pub fn try_code(&self, c: char) -> Option<u8> {
        if c == ' ' {
            return Some(SPACE_CODE);
        }
        self.codes
            .get(glyph_name(c).as_ref())
            .copied()
            .or(self.question)
    }

    /// [`Table::try_code`] for a character the font is known to draw.
    pub fn code(&self, c: char) -> u8 {
        self.try_code(c)
            .unwrap_or_else(|| panic!("the font has no glyph for {c:?}"))
    }

    /// The glyph codes and spacing of a Metamath token. A token the font maps is its glyphs; any
    /// other is drawn as interface text, one code per character, with no spacing. `None` when
    /// the font cannot draw a character of it.
    pub fn token(&self, token: &str) -> Option<(Vec<u8>, Pad)> {
        if let Some((codes, pad)) = self.tokens.get(token) {
            return Some((codes.clone(), *pad));
        }
        let codes = token
            .chars()
            .map(|c| self.try_code(c))
            .collect::<Option<Vec<u8>>>()?;
        Some((codes, Pad::default()))
    }

    /// A 16-bit fingerprint of the slots, CRC-16/CCITT-FALSE over the bytes of the table in code
    /// order. Two tables with one fingerprint assign the same code to the same bitmap.
    pub fn fingerprint(&self) -> u16 {
        let mut crc = 0xFFFFu16;
        for &byte in self.slots.iter().flatten() {
            crc ^= u16::from(byte) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    crc << 1 ^ 0x1021
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    /// `text` as codes with the terminating 0.
    pub fn encode(&self, text: &str) -> Vec<u8> {
        text.chars().map(|c| self.code(c)).chain([0]).collect()
    }

    /// The assembler include for the table called `name`: the slots at the label `name`, then
    /// `<NAME>_GLYPHS` for the number of slots, `<NAME>_FINGERPRINT` for [`Table::fingerprint`]
    /// and `<NAME>_CH_<hex of the character>` for the code of each printable ASCII character, the
    /// check, the cross and the ellipsis.
    pub fn include(&self, name: &str) -> String {
        let upper = name.to_uppercase();
        let mut out = String::from("; generated by mcglyph z80\n");
        out += &format!("{upper}_GLYPHS\t.equ {}\n", self.slots.len());
        out += &format!("{upper}_FINGERPRINT\t.equ ${:04X}\n", self.fingerprint());
        let chars = (' '..='~').chain([CHECK, CROSS, ELLIPSIS]);
        for c in chars {
            if let Some(code) = self.try_code(c) {
                out += &format!("{upper}_CH_{:X}\t.equ {code}\n", c as u32);
            }
        }
        out += &format!("{name}:\n");
        for slot in &self.slots {
            let bytes: Vec<String> = slot.iter().map(|b| format!("${b:02X}")).collect();
            out += &format!("\t.db {}\n", bytes.join(","));
        }
        out
    }
}

/// The include `mcglyph z80` writes: the main font at `ui_font` and the soft key font at
/// `ui_key_font`, each with its own equates.
pub fn fonts_include() -> String {
    Table::builtin().include("ui_font") + &Table::keys().include("ui_key_font")
}
