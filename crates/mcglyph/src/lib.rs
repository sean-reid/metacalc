//! Bitmap font and token typesetting for the metacalc display.
//!
//! The font and the token mapping are text files under `font/`, embedded at build time.
//! A token maps to a run of glyphs; [`Font::render`] lays tokens out left to right and wraps between tokens.

mod bitmap;
mod font;
mod layout;

pub use bitmap::Bitmap;
pub use font::{Font, FontError, MAX_GLYPH_WIDTH, Pad};
pub use layout::{LEADING, Line, Placed, RenderError, SPACE};
