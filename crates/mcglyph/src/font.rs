use crate::bitmap::Bitmap;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::LazyLock;

/// Widest glyph the font may contain, in pixels.
pub const MAX_GLYPH_WIDTH: usize = 9;

const FONT_SRC: &str = include_str!("../font/metacalc.font");
const MAP_SRC: &str = include_str!("../font/tokens.map");

static BUILTIN: LazyLock<Font> =
    LazyLock::new(|| Font::parse(FONT_SRC, MAP_SRC).expect("embedded font is valid"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontError {
    pub file: &'static str,
    pub line: usize,
    pub kind: FontErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontErrorKind {
    BadHeight,
    DuplicateHeight,
    HeightInsideGlyph,
    MissingHeight,
    GlyphBeforeHeight,
    RowCount,
    GlyphWidth,
    BadRow,
    RaggedRow,
    RowOutsideGlyph,
    DuplicateGlyph,
    BadSpacing,
    NoGlyphs,
    UnknownGlyph,
    DuplicateToken,
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} line {}: {}", self.file, self.line, self.message)
    }
}

impl std::error::Error for FontError {}

/// How a token asks for blank space around it.
///
/// `before` and `after` ask for a gap on that side. `var` asks for one next to another `var` token.
/// `open` removes any gap after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pad {
    pub before: bool,
    pub after: bool,
    pub var: bool,
    pub open: bool,
}

impl Pad {
    /// Whether a gap separates `self` from the token that follows it.
    pub fn gap_before(self, next: Pad) -> bool {
        !self.open && (self.after || next.before || (self.var && next.var))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub glyphs: Vec<String>,
    pub pad: Pad,
}

/// A bitmap font and the mapping from Metamath tokens to glyph runs.
#[derive(Debug, Clone)]
pub struct Font {
    height: usize,
    glyphs: BTreeMap<String, Bitmap>,
    tokens: BTreeMap<String, Token>,
}

fn err(
    file: &'static str,
    line: usize,
    kind: FontErrorKind,
    message: impl Into<String>,
) -> FontError {
    FontError {
        file,
        line,
        kind,
        message: message.into(),
    }
}

impl Font {
    /// The font and mapping checked into `font/`.
    pub fn builtin() -> &'static Font {
        &BUILTIN
    }

    /// Parses the font and the token mapping.
    ///
    /// The font file starts with one `height N` line. A glyph is a `glyph NAME` line followed by exactly
    /// N rows of `#` and `.` of equal width, ended by a blank line; lines starting with `;` are comments.
    ///
    /// The mapping has one line per token: the token, a spacing class, then the glyph names.
    /// Any printable ASCII string without `$` or whitespace is a Metamath token, so the mapping has no comments.
    /// Subscript g and r are small glyphs on the baseline rows, and symbol variables carry a dotted underline.
    /// The classes are `tight`, `before`, `after`, `both`, `var` (gap next to another `var`),
    /// `binder` (gap before) and `open` (no gap after).
    pub fn parse(font_src: &str, map_src: &str) -> Result<Font, FontError> {
        let (height, glyphs) = parse_glyphs(font_src)?;
        let mut font = Font {
            height,
            glyphs,
            tokens: BTreeMap::new(),
        };
        font.parse_tokens(map_src)?;
        Ok(font)
    }

    /// Pixel height of every glyph; the baseline sits above the last two rows.
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn glyph(&self, name: &str) -> Option<&Bitmap> {
        self.glyphs.get(name)
    }

    pub fn glyph_names(&self) -> impl Iterator<Item = &str> {
        self.glyphs.keys().map(String::as_str)
    }

    pub fn tokens(&self) -> impl Iterator<Item = &str> {
        self.tokens.keys().map(String::as_str)
    }

    pub fn token_glyphs(&self, token: &str) -> Option<&[String]> {
        self.tokens.get(token).map(|t| t.glyphs.as_slice())
    }

    pub fn token_pad(&self, token: &str) -> Option<Pad> {
        self.tokens.get(token).map(|t| t.pad)
    }

    /// Pixel width of the token's glyphs with one blank column between them, spacing excluded.
    pub fn token_width(&self, token: &str) -> Option<usize> {
        let def = self.tokens.get(token)?;
        let ink: usize = def.glyphs.iter().map(|g| self.glyphs[g].width()).sum();
        Some(ink + def.glyphs.len().saturating_sub(1))
    }

    /// The token alone, without spacing.
    pub fn token_bitmap(&self, token: &str) -> Option<Bitmap> {
        let def = self.tokens.get(token)?;
        let mut out = Bitmap::new(self.token_width(token)?, self.height);
        let mut x = 0;
        for name in &def.glyphs {
            let glyph = &self.glyphs[name];
            out.blit(glyph, x, 0);
            x += glyph.width() + 1;
        }
        Some(out)
    }

    fn parse_tokens(&mut self, src: &str) -> Result<(), FontError> {
        const FILE: &str = "tokens.map";
        for (i, line) in src.lines().enumerate() {
            let n = i + 1;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let token = parts.next().expect("line is not empty");
            let pad = match parts.next() {
                Some("tight") => Pad::default(),
                Some("before") | Some("binder") => Pad {
                    before: true,
                    ..Pad::default()
                },
                Some("after") => Pad {
                    after: true,
                    ..Pad::default()
                },
                Some("both") => Pad {
                    before: true,
                    after: true,
                    ..Pad::default()
                },
                Some("var") => Pad {
                    var: true,
                    ..Pad::default()
                },
                Some("open") => Pad {
                    open: true,
                    ..Pad::default()
                },
                other => {
                    return Err(err(
                        FILE,
                        n,
                        FontErrorKind::BadSpacing,
                        format!("bad spacing {other:?}"),
                    ));
                }
            };
            let glyphs: Vec<String> = parts.map(str::to_string).collect();
            if glyphs.is_empty() {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::NoGlyphs,
                    format!("token {token} has no glyphs"),
                ));
            }
            if let Some(missing) = glyphs.iter().find(|g| !self.glyphs.contains_key(*g)) {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::UnknownGlyph,
                    format!("no glyph named {missing}"),
                ));
            }
            if self
                .tokens
                .insert(token.to_string(), Token { glyphs, pad })
                .is_some()
            {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::DuplicateToken,
                    format!("token {token} defined twice"),
                ));
            }
        }
        Ok(())
    }
}

fn parse_glyphs(src: &str) -> Result<(usize, BTreeMap<String, Bitmap>), FontError> {
    const FILE: &str = "metacalc.font";
    let mut height = None;
    let mut glyphs = BTreeMap::new();
    let mut current: Option<(String, usize, Vec<Vec<bool>>)> = None;
    let mut finish = |current: &mut Option<(String, usize, Vec<Vec<bool>>)>,
                      height: Option<usize>|
     -> Result<(), FontError> {
        let Some((name, line, rows)) = current.take() else {
            return Ok(());
        };
        let h = height.ok_or_else(|| {
            err(
                FILE,
                line,
                FontErrorKind::GlyphBeforeHeight,
                "glyph before the height line",
            )
        })?;
        if rows.len() != h {
            return Err(err(
                FILE,
                line,
                FontErrorKind::RowCount,
                format!("glyph {name} has {} rows, want {h}", rows.len()),
            ));
        }
        let bitmap = Bitmap::from_rows(&rows);
        if bitmap.width() == 0 || bitmap.width() > MAX_GLYPH_WIDTH {
            return Err(err(
                FILE,
                line,
                FontErrorKind::GlyphWidth,
                format!("glyph {name} is {} wide", bitmap.width()),
            ));
        }
        if glyphs.insert(name.clone(), bitmap).is_some() {
            return Err(err(
                FILE,
                line,
                FontErrorKind::DuplicateGlyph,
                format!("glyph {name} defined twice"),
            ));
        }
        Ok(())
    };
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        let line = line.trim();
        if line.starts_with(';') {
            continue;
        }
        if line.is_empty() {
            finish(&mut current, height)?;
        } else if let Some(h) = line.strip_prefix("height ") {
            if current.is_some() {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::HeightInsideGlyph,
                    "height line inside a glyph",
                ));
            }
            if height.is_some() {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::DuplicateHeight,
                    "second height line",
                ));
            }
            let h = h
                .trim()
                .parse()
                .map_err(|_| err(FILE, n, FontErrorKind::BadHeight, "bad height"))?;
            height = Some(h);
        } else if let Some(name) = line.strip_prefix("glyph ") {
            finish(&mut current, height)?;
            current = Some((name.trim().to_string(), n, Vec::new()));
        } else if let Some((_, _, rows)) = current.as_mut() {
            let row: Option<Vec<bool>> = line
                .chars()
                .map(|c| match c {
                    '#' => Some(true),
                    '.' => Some(false),
                    _ => None,
                })
                .collect();
            let row = row.ok_or_else(|| {
                err(
                    FILE,
                    n,
                    FontErrorKind::BadRow,
                    "row holds something besides # and .",
                )
            })?;
            if rows.first().is_some_and(|first| first.len() != row.len()) {
                return Err(err(
                    FILE,
                    n,
                    FontErrorKind::RaggedRow,
                    "row width differs from the glyph's first row",
                ));
            }
            rows.push(row);
        } else {
            return Err(err(
                FILE,
                n,
                FontErrorKind::RowOutsideGlyph,
                "row outside a glyph",
            ));
        }
    }
    finish(&mut current, height)?;
    let height =
        height.ok_or_else(|| err(FILE, 1, FontErrorKind::MissingHeight, "missing height line"))?;
    Ok((height, glyphs))
}
