use crate::bitmap::Bitmap;
use crate::font::Font;
use std::fmt;

/// Blank columns between two tokens when either one asks for spacing, on top of the one-column glyph gap.
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

impl Font {
    /// Breaks `tokens` into lines no wider than `limit`, only ever between tokens.
    pub fn layout(&self, tokens: &[&str], limit: usize) -> Result<Vec<Line>, RenderError> {
        let mut lines: Vec<Line> = Vec::new();
        let mut prev_after = false;
        for (index, &token) in tokens.iter().enumerate() {
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
            let gap = 1 + if prev_after || pad.before { SPACE } else { 0 };
            prev_after = pad.after;
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
        Ok(lines)
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
}
