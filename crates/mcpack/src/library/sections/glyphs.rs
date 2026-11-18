use super::{Counts, Cursor, Entry, GLYPHS, SectionError, Writer, within_chunk};
use crate::library::PackError;
use mcglyph::Pad;
use mcglyph::z80::Table;

/// How the display draws one symbol: the codes of its glyphs and its spacing flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyphs {
    pub codes: Vec<u8>,
    /// [`Pad::bits`] of the token's spacing.
    pub flags: u8,
}

impl Glyphs {
    pub fn pad(&self) -> Pad {
        Pad::from_bits(self.flags).expect("a decoded entry has only the four flags")
    }
}

/// The glyph map: one entry per symbol of the library, by symbol id, and the fingerprint of the
/// font table the codes belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphMap {
    pub fingerprint: u16,
    pub symbols: Vec<Glyphs>,
}

impl GlyphMap {
    /// The map the built-in font gives `names`, the symbols of a library in id order.
    pub fn of<S: AsRef<str>>(names: &[S]) -> Result<GlyphMap, PackError> {
        let table = Table::builtin();
        let symbols = names
            .iter()
            .map(|name| {
                let (codes, pad) = table
                    .token(name.as_ref())
                    .ok_or_else(|| PackError::NoGlyphs(name.as_ref().to_string()))?;
                Ok(Glyphs {
                    codes,
                    flags: pad.bits(),
                })
            })
            .collect::<Result<Vec<_>, PackError>>()?;
        Ok(GlyphMap {
            fingerprint: table.fingerprint(),
            symbols,
        })
    }
}

pub(super) fn encode(w: &mut Writer, map: &GlyphMap) -> Result<(), PackError> {
    let too_large = || PackError::SectionTooLarge;
    let count = u8::try_from(map.symbols.len()).map_err(|_| too_large())?;
    let start = w.len();
    w.open(GLYPHS);
    let start = start.next_multiple_of(4);
    w.put(&map.fingerprint.to_le_bytes());
    w.put(&[count]);
    let index = w.len();
    w.put(&vec![0; 2 * map.symbols.len()]);
    for (i, symbol) in map.symbols.iter().enumerate() {
        let mut item = vec![
            symbol.flags,
            u8::try_from(symbol.codes.len()).map_err(|_| too_large())?,
        ];
        item.extend_from_slice(&symbol.codes);
        let offset = w.item(&item).map_err(|_| too_large())? as usize - start;
        let offset = u16::try_from(offset).map_err(|_| too_large())?;
        w.patch(index + 2 * i, &offset.to_le_bytes());
    }
    w.close();
    Ok(())
}

pub(super) fn decode(
    bytes: &[u8],
    entry: &Entry,
    counts: &Counts,
) -> Result<GlyphMap, SectionError> {
    let bad = SectionError::Glyphs;
    let end = entry.offset + entry.length;
    let mut cur = Cursor::new(bytes, entry.offset, end);
    let fingerprint = cur.u16().ok_or(bad)? as u16;
    if cur.u8().ok_or(bad)? != counts.symbols {
        return Err(bad);
    }
    let first_item = cur.pos + 2 * counts.symbols;
    let mut symbols = Vec::with_capacity(counts.symbols);
    for _ in 0..counts.symbols {
        let at = entry.offset + cur.u16().ok_or(bad)?;
        if at < first_item {
            return Err(bad);
        }
        let mut item = Cursor::new(bytes, at, end);
        let flags = item.u8().ok_or(bad)? as u8;
        let len = item.u8().ok_or(bad)?;
        let codes = item.take(len).ok_or(bad)?.to_vec();
        if Pad::from_bits(flags).is_none() || !within_chunk(at, item.pos - at) {
            return Err(bad);
        }
        symbols.push(Glyphs { codes, flags });
    }
    Ok(GlyphMap {
        fingerprint,
        symbols,
    })
}
