use crate::bitmap::Bitmap;
use crate::font::{Font, Pad};
use std::fmt;

/// Blank columns between two tokens when [`crate::Pad::gap_before`] holds, on top of the one-column glyph gap.
pub const SPACE: usize = 2;

/// Blank rows between text lines.
pub const LEADING: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    UnknownToken(String),
    TooWide {
        token: String,
        width: usize,
        limit: usize,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownToken(t) => write!(f, "no glyphs for token {t}"),
            Self::TooWide {
                token,
                width,
                limit,
            } => {
                write!(f, "token {token} is {width} wide, limit is {limit}")
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// A token placed on a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub index: usize,
    pub x: usize,
    pub width: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub tokens: Vec<Placed>,
    pub width: usize,
}

/// Something to place on a line: its width in pixels and how it asks for blank space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub width: usize,
    pub pad: Pad,
}

/// Breaks `items` into lines no wider than `limit`, only ever between items. An item wider than
/// `limit` sits alone on its line.
pub fn place(items: &[Item], limit: usize) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut prev: Option<Pad> = None;
    for (index, &Item { width, pad }) in items.iter().enumerate() {
        let spaced = prev.is_some_and(|p| p.gap_before(pad));
        let gap = 1 + if spaced { SPACE } else { 0 };
        prev = Some(pad);
        let fits = lines
            .last()
            .is_some_and(|line| line.width + gap + width <= limit);
        if fits {
            let line = lines.last_mut().expect("a line exists");
            let x = line.width + gap;
            line.tokens.push(Placed { index, x, width });
            line.width = x + width;
        } else {
            lines.push(Line {
                tokens: vec![Placed { index, x: 0, width }],
                width,
            });
        }
    }
    lines
}

impl Font {
    /// Breaks `tokens` into lines no wider than `limit`, only ever between tokens.
    pub fn layout(&self, tokens: &[&str], limit: usize) -> Result<Vec<Line>, RenderError> {
        let mut items = Vec::with_capacity(tokens.len());
        for &token in tokens {
            let width = self
                .token_width(token)
                .ok_or_else(|| RenderError::UnknownToken(token.to_string()))?;
            if width > limit {
                return Err(RenderError::TooWide {
                    token: token.to_string(),
                    width,
                    limit,
                });
            }
            let pad = self.token_pad(token).expect("token is known");
            items.push(Item { width, pad });
        }
        Ok(place(&items, limit))
    }

    /// Renders `tokens` into a bitmap `limit` pixels wide, as tall as the lines it needs.
    pub fn render(&self, tokens: &[&str], limit: usize) -> Result<Bitmap, RenderError> {
        let lines = self.layout(tokens, limit)?;
        let height = (lines.len() * (self.height() + LEADING)).saturating_sub(LEADING);
        let mut out = Bitmap::new(limit, height);
        for (row, line) in lines.iter().enumerate() {
            for placed in &line.tokens {
                let ink = self
                    .token_bitmap(tokens[placed.index])
                    .expect("token is known");
                out.blit(&ink, placed.x, row * (self.height() + LEADING));
            }
        }
        Ok(out)
    }

    /// Renders `tokens` onto screens of `size` pixels, breaking between lines only. A screen shows
    /// whole lines and is padded with blank rows to its full height. Empty input gives one blank
    /// screen.
    pub fn frames(
        &self,
        tokens: &[&str],
        (width, height): (usize, usize),
    ) -> Result<Vec<Bitmap>, RenderError> {
        let lines = self.layout(tokens, width)?;
        let pitch = self.height() + LEADING;
        let per_frame = ((height + LEADING) / pitch).max(1);
        if lines.is_empty() {
            return Ok(vec![Bitmap::new(width, height)]);
        }
        lines
            .chunks(per_frame)
            .map(|chunk| {
                let first = chunk[0].tokens[0].index;
                let last = chunk[chunk.len() - 1]
                    .tokens
                    .last()
                    .expect("a line holds a token");
                let text = self.render(&tokens[first..=last.index], width)?;
                let mut frame = Bitmap::new(width, height);
                frame.blit(&text, 0, 0);
                Ok(frame)
            })
            .collect()
    }
}
