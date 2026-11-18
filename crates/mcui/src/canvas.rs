//! Drawing helpers shared by the screens: the title bar, the soft key row, list rows and text.

use crate::fold::{Layout, Piece, Rect};
use mcglyph::{Bitmap, ELLIPSIS, Font};

pub const WIDTH: usize = 96;
pub const HEIGHT: usize = 64;
/// Pixel height of a text row: a glyph and one blank row above it.
pub const ROW: usize = 8;
/// Rows of content between the title bar and the soft key row.
pub const CONTENT_ROWS: usize = 6;
/// Blank columns at each side of the text.
pub const MARGIN: usize = 1;
/// Pixels a list row or a one-line preview may use.
pub const TEXT_WIDTH: usize = WIDTH - 2 * MARGIN;
/// Pixels the title bar gives the typed prefix of a Find.
pub const PREFIX_WIDTH: usize = 50;
/// Columns left of a statement for the marker of a cursor on its root.
pub const GUTTER: usize = 3;
/// Pixels a statement may use.
pub const STATEMENT_WIDTH: usize = WIDTH - GUTTER - MARGIN;

fn font() -> &'static Font {
    Font::builtin()
}

pub fn text_width(text: &str) -> usize {
    font().text_width(text)
}

/// `text` cut after as many characters as fit in `limit` pixels together with an ellipsis, or
/// whole when it fits.
pub fn truncate(text: &str, limit: usize) -> String {
    if text_width(text) <= limit {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        let mut longer = out.clone();
        longer.push(c);
        longer.push(ELLIPSIS);
        if text_width(&longer) > limit {
            break;
        }
        out.push(c);
    }
    out.push(ELLIPSIS);
    out
}

/// `text` broken between words into lines no wider than `limit`; a word wider than that is cut
/// with an ellipsis.
pub fn wrap(text: &str, limit: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let word = truncate(word, limit);
        match lines.last_mut() {
            Some(line) if text_width(&format!("{line} {word}")) <= limit => {
                line.push(' ');
                line.push_str(&word);
            }
            _ => lines.push(word),
        }
    }
    lines
}

/// The newest characters of `text` that fit `limit` pixels behind an ellipsis, or all of it when
/// it fits.
pub fn tail(text: &str, limit: usize) -> String {
    if text_width(text) <= limit {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    for from in 0..=chars.len() {
        let cut: String = std::iter::once(ELLIPSIS)
            .chain(chars[from..].iter().copied())
            .collect();
        if text_width(&cut) <= limit {
            return cut;
        }
    }
    ELLIPSIS.to_string()
}

/// The first and last column of the box of soft key `i`.
fn soft_key_box(i: usize) -> (usize, usize) {
    ((WIDTH * i + 2) / 5, (WIDTH * (i + 1) + 2) / 5 - 1)
}

fn key_font() -> &'static Font {
    Font::keys()
}

/// Pixel width of `text` in the key font.
pub fn key_text_width(text: &str) -> usize {
    key_font().text_width(text)
}

/// `text` cut after as many characters as fit in `limit` pixels; the key font has no ellipsis.
fn key_cut(text: &str, limit: usize) -> &str {
    let mut end = 0;
    for (at, c) in text.char_indices() {
        let next = at + c.len_utf8();
        if key_text_width(&text[..next]) > limit {
            break;
        }
        end = next;
    }
    &text[..end]
}

/// Pixels a label of soft key `i` may use.
pub fn soft_key_room(i: usize) -> usize {
    let (from, to) = soft_key_box(i);
    to - from - 2
}

pub struct Canvas {
    bitmap: Bitmap,
}

impl Default for Canvas {
    fn default() -> Self {
        Canvas {
            bitmap: Bitmap::new(WIDTH, HEIGHT),
        }
    }
}

impl Canvas {
    pub fn into_bitmap(self) -> Bitmap {
        self.bitmap
    }

    /// Top of text row `row`, counting the title bar as row 0.
    fn top(row: usize) -> usize {
        row * ROW
    }

    /// Pixels left of a row for its left text beside a right text of `right_width`.
    fn room(right_width: usize, right: &str) -> usize {
        let gap = if right.is_empty() { 0 } else { 3 };
        WIDTH.saturating_sub(2 * MARGIN + right_width + gap)
    }

    /// Draws `text` with its first column at `x` on `row`; returns its width.
    pub fn text(&mut self, x: usize, row: usize, text: &str) -> usize {
        let ink = font().text(text);
        self.bitmap.blit(&ink, x, Self::top(row) + 1);
        ink.width()
    }

    /// Draws `ink` with its top left corner at `x` on `row`.
    pub fn blit(&mut self, ink: &Bitmap, x: usize, row: usize) {
        self.bitmap.blit(ink, x, Self::top(row) + 1);
    }

    /// Draws `text` so that it ends at the right margin.
    pub fn text_right(&mut self, row: usize, text: &str) -> usize {
        let width = text_width(text);
        self.text(WIDTH.saturating_sub(MARGIN + width), row, text);
        width
    }

    pub fn text_centered(&mut self, row: usize, text: &str) {
        let width = text_width(text);
        self.text(WIDTH.saturating_sub(width) / 2, row, text);
    }

    /// Flips every pixel of the rectangle.
    pub fn invert(&mut self, x: usize, y: usize, width: usize, height: usize) {
        for yy in y..y + height {
            for xx in x..x + width {
                let ink = self.bitmap.get(xx, yy);
                self.bitmap.set(xx, yy, !ink);
            }
        }
    }

    pub fn invert_row(&mut self, row: usize) {
        self.invert(0, Self::top(row), WIDTH, ROW);
    }

    /// The title bar: `left` and `right` in white on a black row, the left side cut short with an
    /// ellipsis when the two would meet.
    pub fn title(&mut self, left: &str, right: &str) {
        let right_width = text_width(right);
        let room = Self::room(right_width, right);
        self.text(MARGIN, 0, &truncate(left, room));
        self.text_right(0, right);
        self.invert_row(0);
    }

    /// A title bar whose left side is `name` then `tail`: when the two do not fit beside `right`,
    /// the name is what gets cut and the tail stays whole.
    pub fn title_with_tail(&mut self, name: &str, tail: &str, right: &str) {
        let room = Self::room(text_width(right), right).saturating_sub(text_width(tail));
        let left = format!("{}{tail}", truncate(name, room));
        self.title(&left, right);
    }

    /// The labels of the five keys under the screen. A key with a label is an inverted box
    /// across its fifth of the width, one column short on the right so neighbours stay apart,
    /// with the label centred in it, set in the key font; a key without one stays blank.
    pub fn soft_keys(&mut self, labels: [&str; 5]) {
        for (i, label) in labels.iter().enumerate() {
            if label.is_empty() {
                continue;
            }
            let (from, to) = soft_key_box(i);
            let label = key_cut(label, soft_key_room(i));
            let ink = key_font().text(label);
            let x = from + (to - from - ink.width()).div_ceil(2);
            self.blit(&ink, x, CONTENT_ROWS + 1);
            self.invert(from, Self::top(CONTENT_ROWS + 1), to - from, ROW);
        }
    }

    /// One list row: `left` cut to fit beside `right`, which ends at the right margin. A selected
    /// row is inverted across the whole width.
    pub fn list_row(&mut self, row: usize, left: &str, right: &str, selected: bool) {
        let right_width = text_width(right);
        let room = Self::room(right_width, right);
        self.text(MARGIN, row, &truncate(left, room));
        self.text_right(row, right);
        if selected {
            self.invert_row(row);
        }
    }

    /// Draws a laid out statement with its first column at `x` and its first line on `row`,
    /// showing only the rows from `window.start` to `window.end`.
    pub fn layout(
        &mut self,
        layout: &Layout,
        x: usize,
        row: isize,
        window: std::ops::Range<isize>,
    ) {
        for (r, line) in layout.lines.iter().enumerate() {
            let at = row + r as isize;
            if !window.contains(&at) {
                continue;
            }
            for placed in &line.tokens {
                let ink = piece_bitmap(&layout.pieces[placed.index]);
                self.bitmap
                    .blit(&ink, x + placed.x, Self::top(at as usize) + 1);
            }
        }
    }

    /// A progress bar across the text width on `row`, `done` of `total` filled; with nothing to
    /// do (`total` of zero) it is full.
    pub fn bar(&mut self, row: usize, done: usize, total: usize) {
        let (width, height) = (TEXT_WIDTH, ROW - 1);
        let inner = width - 4;
        let filled = (inner * done.min(total))
            .checked_div(total)
            .unwrap_or(inner);
        for y in 0..height {
            for x in 0..width {
                let edge = y == 0 || y == height - 1 || x == 0 || x == width - 1;
                let fill = (2..height - 2).contains(&y) && (2..2 + filled).contains(&x);
                if edge || fill {
                    self.bitmap.set(MARGIN + x, Self::top(row) + 1 + y, true);
                }
            }
        }
    }

    /// A two column bar in the left margin of `row`, the cursor of a whole statement.
    pub fn marker(&mut self, row: usize) {
        for y in Self::top(row) + 1..Self::top(row) + ROW {
            self.bitmap.set(0, y, true);
            self.bitmap.set(1, y, true);
        }
    }

    /// Inverts the runs of a cursor, padded by the blank column on each side of its tokens.
    pub fn cursor(&mut self, rects: &[Rect], x: usize, row: isize, window: std::ops::Range<isize>) {
        for rect in rects {
            let at = row + rect.row as isize;
            if window.contains(&at) {
                self.invert(
                    (x + rect.x).saturating_sub(1),
                    Self::top(at as usize),
                    rect.width + 2,
                    ROW,
                );
            }
        }
    }
}

fn piece_bitmap(piece: &Piece) -> Bitmap {
    match piece {
        Piece::Tok(token) => font()
            .token_bitmap(token)
            .unwrap_or_else(|| font().text(token)),
        Piece::Ellipsis => font().text(&ELLIPSIS.to_string()),
    }
}
