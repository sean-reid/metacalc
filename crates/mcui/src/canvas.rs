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

/// `text` cut after as many characters as fit in `limit` pixels, or before the first character
/// the key font cannot draw, with trailing spaces dropped; the key font has no ellipsis.
fn key_cut(text: &str, limit: usize) -> &str {
    let mut end = 0;
    for (at, c) in text.char_indices() {
        let next = at + c.len_utf8();
        if !(c.is_ascii_alphanumeric() || c == ' ') || key_text_width(&text[..next]) > limit {
            break;
        }
        end = next;
    }
    text[..end].trim_end_matches(' ')
}

/// Pixels a label of soft key `i` may use.
pub fn soft_key_room(i: usize) -> usize {
    let (from, to) = soft_key_box(i);
    to - from - 2
}

/// A drawing call as a Z80 canvas receives it: strings already cut to fit, nothing left to
/// decide. The recording API exists for the Z80 replay tests.
#[doc(hidden)]
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
    /// A progress bar of `done` of `total` on `row`.
    Bar {
        row: usize,
        done: usize,
        total: usize,
    },
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
    #[doc(hidden)]
    pub fn recording() -> Self {
        Canvas {
            calls: Some(Vec::new()),
            ..Canvas::default()
        }
    }

    pub fn into_bitmap(self) -> Bitmap {
        self.bitmap
    }

    #[doc(hidden)]
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
        self.ink(|bitmap| bitmap.blit(ink, x, Self::top(row) + 1));
    }

    /// Runs `draw` on the screen; while recording, on a fresh layer that is then ORed onto the
    /// screen and kept as one call.
    fn ink(&mut self, draw: impl FnOnce(&mut Bitmap)) {
        if self.calls.is_none() {
            draw(&mut self.bitmap);
            return;
        }
        let mut layer = Bitmap::new(WIDTH, HEIGHT);
        draw(&mut layer);
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

    /// Inverts a text row except its top pixel row, which keeps a highlight clear of whatever is
    /// above it.
    pub fn invert_row(&mut self, row: usize) {
        self.record(|| Call::InvertRow(row));
        self.nested(|c| c.invert(0, Self::top(row) + 1, WIDTH, ROW - 1));
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
            c.invert(0, 0, WIDTH, ROW);
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
    /// below the row's top pixel row, across its fifth of the width, one column short on the right
    /// so neighbours stay apart, with the label centred in it, set in the key font; a key without
    /// one stays blank.
    pub fn soft_keys(&mut self, labels: [&str; 5]) {
        let cut: [&str; 5] = std::array::from_fn(|i| {
            let label = labels[i];
            if label.is_empty() {
                return "";
            }
            let cut = key_cut(label, soft_key_room(i));
            debug_assert_eq!(cut, label, "label {label:?} does not fit key {i}");
            cut
        });
        self.record(|| Call::SoftKeys(cut.map(String::from)));
        self.nested(|c| {
            for (i, label) in cut.iter().enumerate() {
                if label.is_empty() {
                    continue;
                }
                let (from, to) = soft_key_box(i);
                let ink = key_font().text(label);
                let x = from + (to - from - ink.width()).div_ceil(2);
                c.bitmap.blit(&ink, x, Self::top(CONTENT_ROWS + 1) + 1);
                c.invert(from, Self::top(CONTENT_ROWS + 1) + 1, to - from, ROW - 1);
            }
        });
    }

    /// One list row: `left` cut to fit beside `right`, which ends at the right margin. A selected
    /// row is inverted across the whole width below its top pixel row.
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
        self.ink(|bitmap| {
            for (r, line) in layout.lines.iter().enumerate() {
                let at = row + r as isize;
                if !window.contains(&at) {
                    continue;
                }
                for placed in &line.tokens {
                    let ink = piece_bitmap(&layout.pieces[placed.index]);
                    bitmap.blit(&ink, x + placed.x, Self::top(at as usize) + 1);
                }
            }
        });
    }

    /// A progress bar across the text width on `row`, `done` of `total` filled; with nothing to
    /// do (`total` of zero) it is full.
    pub fn bar(&mut self, row: usize, done: usize, total: usize) {
        let (width, height) = (TEXT_WIDTH, ROW - 1);
        let inner = width - 4;
        let filled = (inner * done.min(total))
            .checked_div(total)
            .unwrap_or(inner);
        self.record(|| Call::Bar { row, done, total });
        self.nested(|c| {
            c.ink(|bitmap| {
                for y in 0..height {
                    for x in 0..width {
                        let edge = y == 0 || y == height - 1 || x == 0 || x == width - 1;
                        let fill = (2..height - 2).contains(&y) && (2..2 + filled).contains(&x);
                        if edge || fill {
                            bitmap.set(MARGIN + x, Self::top(row) + 1 + y, true);
                        }
                    }
                }
            });
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_cut_to_the_letters_that_fit() {
        for limit in 0..40 {
            let cut = key_cut("Prove it now", limit);
            assert!(key_text_width(cut) <= limit, "{cut:?} at {limit}");
            assert!("Prove it now".starts_with(cut));
            assert!(!cut.ends_with(' '), "{cut:?} at {limit}");
        }
        assert_eq!(key_cut("Prove it now", 16), "Prov");
        assert_eq!(key_cut("Prove it now", 22), "Prove");
        assert_eq!(key_cut("Open", 16), "Open");
    }

    #[test]
    fn a_label_is_cut_at_the_first_character_the_key_font_lacks() {
        assert_eq!(key_cut("A?", 16), "A");
        assert_eq!(key_cut("\u{dc}n\u{ef}", 16), "");
        assert_eq!(key_cut("Go\u{2026}", 16), "Go");
        assert_eq!(key_cut("a b", 16), "a b");
    }

    #[test]
    fn a_bar_records_one_call_and_draws_what_an_unrecorded_one_does() {
        let mut recording = Canvas::recording();
        recording.bar(2, 3, 7);
        let (bitmap, calls) = recording.into_parts();
        assert_eq!(
            calls,
            [Call::Bar {
                row: 2,
                done: 3,
                total: 7
            }]
        );
        let mut plain = Canvas::default();
        plain.bar(2, 3, 7);
        assert!(bitmap == plain.into_bitmap());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn soft_keys_draw_a_cut_label_without_panicking_on_odd_text() {
        let mut canvas = Canvas::default();
        canvas.soft_keys(["A?", "\u{dc}n\u{ef}", "Prove it now", "", ""]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "does not fit key 0")]
    fn a_label_that_does_not_fit_trips_the_debug_assertion() {
        Canvas::default().soft_keys(["Prove it now", "", "", "", ""]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "does not fit key 0")]
    fn a_label_with_an_undrawable_character_trips_the_debug_assertion() {
        Canvas::default().soft_keys(["A?", "", "", "", ""]);
    }
}
