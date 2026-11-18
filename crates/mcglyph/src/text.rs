use crate::bitmap::Bitmap;
use crate::font::Font;
use std::borrow::Cow;

/// Characters of interface text that stand for the status and fold glyphs.
pub const CHECK: char = '\u{2713}';
pub const CROSS: char = '\u{2717}';
pub const ELLIPSIS: char = '\u{2026}';

/// Blank columns a space occupies, before the glyph gap.
pub const SPACE_WIDTH: usize = 2;

fn glyph_name(c: char) -> Cow<'static, str> {
    Cow::Borrowed(match c {
        '.' => "period",
        ',' => "comma",
        ':' => "colon",
        '!' => "exclam",
        '?' => "question",
        '(' => "lparen",
        ')' => "rparen",
        '[' => "lbracket",
        ']' => "rbracket",
        '<' => "lt",
        '>' => "gt",
        '=' => "equals",
        '+' => "plus",
        '-' => "minus",
        '_' => "underscore",
        '/' => "slash",
        '*' => "star",
        '~' => "tilde",
        '\'' => "apostrophe",
        CHECK => "check",
        CROSS => "cross",
        ELLIPSIS => "ellipsis",
        c if c.is_ascii_alphanumeric() => return Cow::Owned(c.to_string()),
        _ => "question",
    })
}

impl Font {
    /// Pixel width of `text` set as interface text: one blank column between glyphs, a space
    /// [`SPACE_WIDTH`] columns wide, and `?` for a character the font lacks.
    pub fn text_width(&self, text: &str) -> usize {
        let mut width = 0;
        for (i, c) in text.chars().enumerate() {
            width += usize::from(i > 0) + self.advance(c);
        }
        width
    }

    /// `text` as interface text, as tall as a glyph.
    pub fn text(&self, text: &str) -> Bitmap {
        let mut out = Bitmap::new(self.text_width(text), self.height());
        let mut x = 0;
        for c in text.chars() {
            if c != ' ' {
                let glyph = self.glyph(&glyph_name(c)).expect("interface glyphs exist");
                out.blit(glyph, x, 0);
            }
            x += self.advance(c) + 1;
        }
        out
    }

    fn advance(&self, c: char) -> usize {
        if c == ' ' {
            return SPACE_WIDTH;
        }
        self.glyph(&glyph_name(c))
            .expect("interface glyphs exist")
            .width()
    }
}
