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

/// A drawing call as a Z80 canvas receives it: strings already cut to fit, nothing left to
/// decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Text {
        x: usize,
        row: usize,
        text: String,
    },
    TextRight {
        row: usize,
        text: String,
    },
    TextCentered {
        row: usize,
        text: String,
    },
    Invert {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },
    InvertRow(usize),
    Title {
        left: String,
        right: String,
    },
    SoftKeys([String; 5]),
    ListRow {
        row: usize,
        left: String,
        right: String,
        selected: bool,
    },
    Marker(usize),
    /// Laid out tokens or a ready made bitmap, which only the host can draw: the pixels the
    /// call set, as a layer the size of the screen.
    Ink(Bitmap),
}

pub struct Canvas {
    bitmap: Bitmap,
    calls: Option<Vec<Call>>,
    depth: usize,
}

impl Default for Canvas {
    fn default() -> Self {
        Canvas {
            bitmap: Bitmap::new(WIDTH, HEIGHT),
            calls: None,
            depth: 0,
        }
    }
}

impl Canvas {
    /// A canvas that also keeps the calls made on it, the ones a screen makes and not the ones
    /// they make in turn.
    pub fn recording() -> Self {
        Canvas {
            calls: Some(Vec::new()),
            ..Canvas::default()
        }
    }

    pub fn into_bitmap(self) -> Bitmap {
        self.bitmap
    }

    pub fn into_parts(self) -> (Bitmap, Vec<Call>) {
        (self.bitmap, self.calls.unwrap_or_default())
    }

    fn record(&mut self, call: impl FnOnce() -> Call) {
        if self.depth == 0
            && let Some(calls) = &mut self.calls
        {
            calls.push(call());
        }
    }

    fn nested(&mut self, draw: impl FnOnce(&mut Self)) {
        self.depth += 1;
        draw(self);
        self.depth -= 1;
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
        self.record(|| Call::Text {
            x,
            row,
            text: text.into(),
        });
        let ink = font().text(text);
        self.bitmap.blit(&ink, x, Self::top(row) + 1);
        ink.width()
    }

    /// Draws `ink` with its top left corner at `x` on `row`.
    pub fn blit(&mut self, ink: &Bitmap, x: usize, row: usize) {
        let mut layer = Bitmap::new(WIDTH, HEIGHT);
        layer.blit(ink, x, Self::top(row) + 1);
        self.put(layer);
    }

    /// ORs `layer` onto the screen, recording it as one call.
    fn put(&mut self, layer: Bitmap) {
        self.bitmap.blit(&layer, 0, 0);
        self.record(|| Call::Ink(layer));
    }

    /// Draws `text` so that it ends at the right margin.
    pub fn text_right(&mut self, row: usize, text: &str) -> usize {
        let width = text_width(text);
        self.record(|| Call::TextRight {
            row,
            text: text.into(),
        });
        self.nested(|c| {
            c.text(WIDTH.saturating_sub(MARGIN + width), row, text);
        });
        width
    }

    pub fn text_centered(&mut self, row: usize, text: &str) {
        let width = text_width(text);
        self.record(|| Call::TextCentered {
            row,
            text: text.into(),
        });
        self.nested(|c| {
            c.text(WIDTH.saturating_sub(width) / 2, row, text);
        });
    }

    /// Flips every pixel of the rectangle.
    pub fn invert(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.record(|| Call::Invert {
            x,
            y,
            width,
            height,
        });
        for yy in y..y + height {
            for xx in x..x + width {
                let ink = self.bitmap.get(xx, yy);
                self.bitmap.set(xx, yy, !ink);
            }
        }
    }

    pub fn invert_row(&mut self, row: usize) {
        self.record(|| Call::InvertRow(row));
        self.nested(|c| c.invert(0, Self::top(row), WIDTH, ROW));
    }

    /// The title bar: `left` and `right` in white on a black row, the left side cut short with an
    /// ellipsis when the two would meet.
    pub fn title(&mut self, left: &str, right: &str) {
        let right_width = text_width(right);
        let room = Self::room(right_width, right);
        let left = truncate(left, room);
        self.record(|| Call::Title {
            left: left.clone(),
            right: right.into(),
        });
        self.nested(|c| {
            c.text(MARGIN, 0, &left);
            c.text_right(0, right);
            c.invert_row(0);
        });
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
    /// with the label centred in it; a key without one stays blank.
    pub fn soft_keys(&mut self, labels: [&str; 5]) {
        let labels: [String; 5] = std::array::from_fn(|i| {
            let (from, to) = Self::key_box(i);
            if labels[i].is_empty() {
                String::new()
            } else {
                truncate(labels[i], to - from - 2)
            }
        });
        self.record(|| Call::SoftKeys(labels.clone()));
        self.nested(|c| {
            for (i, label) in labels.iter().enumerate() {
                if label.is_empty() {
                    continue;
                }
                let (from, to) = Self::key_box(i);
                let x = from + (to - from - text_width(label)).div_ceil(2);
                c.text(x, CONTENT_ROWS + 1, label);
                c.invert(from, Self::top(CONTENT_ROWS + 1), to - from, ROW);
            }
        });
    }

    /// The first column of key `i`'s box and the one after its last.
    fn key_box(i: usize) -> (usize, usize) {
        ((WIDTH * i + 2) / 5, (WIDTH * (i + 1) + 2) / 5 - 1)
    }

    /// One list row: `left` cut to fit beside `right`, which ends at the right margin. A selected
    /// row is inverted across the whole width.
    pub fn list_row(&mut self, row: usize, left: &str, right: &str, selected: bool) {
        let right_width = text_width(right);
        let room = Self::room(right_width, right);
        let left = truncate(left, room);
        self.record(|| Call::ListRow {
            row,
            left: left.clone(),
            right: right.into(),
            selected,
        });
        self.nested(|c| {
            c.text(MARGIN, row, &left);
            c.text_right(row, right);
            if selected {
                c.invert_row(row);
            }
        });
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
        let mut layer = Bitmap::new(WIDTH, HEIGHT);
        for (r, line) in layout.lines.iter().enumerate() {
            let at = row + r as isize;
            if !window.contains(&at) {
                continue;
            }
            for placed in &line.tokens {
                let ink = piece_bitmap(&layout.pieces[placed.index]);
                layer.blit(&ink, x + placed.x, Self::top(at as usize) + 1);
            }
        }
        self.put(layer);
    }

    /// A two column bar in the left margin of `row`, the cursor of a whole statement.
    pub fn marker(&mut self, row: usize) {
        self.record(|| Call::Marker(row));
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
