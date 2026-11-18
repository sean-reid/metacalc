//! Bitmap font and token typesetting for the metacalc display.
//!
//! The font and the token mapping are text files under `font/`, embedded at build time.
//! A token maps to a run of glyphs; [`Font::render`] lays tokens out left to right and wraps between tokens.

mod bitmap;
mod font;
mod layout;
pub mod png;
mod text;
pub mod z80;

pub use bitmap::Bitmap;
pub use font::{Font, FontError, FontErrorKind, MAX_GLYPH_WIDTH, Pad};
pub use layout::{Item, LEADING, Line, Placed, RenderError, SPACE, place};
pub use text::{CHECK, CROSS, ELLIPSIS, SPACE_WIDTH};
