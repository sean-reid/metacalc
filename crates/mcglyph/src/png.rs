//! A PNG writer for bitmaps: 8-bit grayscale, stored deflate blocks, no dependencies.

use crate::bitmap::Bitmap;

const MARGIN: usize = 2;
const OUTSIDE: u8 = 0x55;
const INK: u8 = 0x14;
const GRID: u8 = 0xb4;
const PAPER: u8 = 0xc4;

/// Encodes `bitmap` with each pixel `scale` times as wide and tall, a two-pixel border around it
/// and, from a scale of 4, a grid between pixels.
pub fn encode(bitmap: &Bitmap, scale: usize) -> Vec<u8> {
    let scale = scale.max(1);
    let (w, h) = (
        (bitmap.width() + 2 * MARGIN) * scale,
        (bitmap.height() + 2 * MARGIN) * scale,
    );
    let mut raw = Vec::with_capacity((w + 1) * h);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            let (px, py) = (x / scale, y / scale);
            let inside = (MARGIN..MARGIN + bitmap.width()).contains(&px)
                && (MARGIN..MARGIN + bitmap.height()).contains(&py);
            let ink = inside && bitmap.get(px - MARGIN, py - MARGIN);
            let grid = scale >= 4 && (x % scale == 0 || y % scale == 0);
            raw.push(match (inside, ink, grid) {
                (false, ..) => OUTSIDE,
                (_, true, _) => INK,
                (_, _, true) => GRID,
                _ => PAPER,
            });
        }
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend((w as u32).to_be_bytes());
    ihdr.extend((h as u32).to_be_bytes());
    ihdr.extend([8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend(kind);
    out.extend(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    if data.is_empty() {
        out.extend([1, 0, 0, 0xff, 0xff]);
    }
    let mut blocks = data.chunks(0xffff).peekable();
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        out.extend((block.len() as u16).to_le_bytes());
        out.extend((!(block.len() as u16)).to_le_bytes());
        out.extend(block);
    }
    out.extend(adler32(data).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn adler32_matches_the_reference_value() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }

    fn unstore(zlib: &[u8]) -> (Vec<u8>, usize) {
        assert_eq!(zlib[..2], [0x78, 0x01]);
        let (mut at, mut data, mut blocks) = (2, Vec::new(), 0);
        loop {
            let last = zlib[at] == 1;
            let len = u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]);
            let inverse = u16::from_le_bytes([zlib[at + 3], zlib[at + 4]]);
            assert_eq!(inverse, !len);
            data.extend(&zlib[at + 5..at + 5 + len as usize]);
            at += 5 + len as usize;
            blocks += 1;
            if last {
                break;
            }
        }
        assert_eq!(zlib[at..], adler32(&data).to_be_bytes());
        (data, blocks)
    }

    #[test]
    fn stored_blocks_split_past_65535_bytes() {
        for (size, blocks) in [(0, 1), (1, 1), (65_535, 1), (65_536, 2), (200_000, 4)] {
            let payload: Vec<u8> = (0..size).map(|i| (i * 7 % 251) as u8).collect();
            let (back, count) = unstore(&zlib_stored(&payload));
            assert_eq!(back, payload, "{size} bytes");
            assert_eq!(count, blocks, "{size} bytes");
        }
    }

    #[test]
    fn image_has_the_scaled_size_and_valid_chunk_checksums() {
        let mut bitmap = Bitmap::new(3, 2);
        bitmap.set(1, 0, true);
        let png = encode(&bitmap, 4);
        assert_eq!(png[..8], *b"\x89PNG\r\n\x1a\n");
        let mut at = 8;
        let mut kinds = Vec::new();
        let mut idat = Vec::new();
        while at < png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let body = &png[at + 4..at + 8 + len];
            let crc = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap());
            assert_eq!(crc32(body), crc);
            kinds.push(String::from_utf8(body[..4].to_vec()).unwrap());
            match &body[..4] {
                b"IHDR" => {
                    assert_eq!(body[4..8], ((3 + 4) as u32 * 4).to_be_bytes());
                    assert_eq!(body[8..12], ((2 + 4) as u32 * 4).to_be_bytes());
                }
                b"IDAT" => idat.extend(&body[4..]),
                _ => {}
            }
            at += 12 + len;
        }
        assert_eq!(kinds, ["IHDR", "IDAT", "IEND"]);
        let (raw, _) = unstore(&idat);
        let (w, h) = (28, 24);
        assert_eq!(raw.len(), (w + 1) * h);
        let pixel = |x: usize, y: usize| raw[y * (w + 1) + 1 + x];
        assert_eq!(pixel(1, 1), OUTSIDE);
        assert_eq!(pixel((MARGIN + 1) * 4 + 2, MARGIN * 4 + 2), INK);
        assert_eq!(pixel(MARGIN * 4 + 2, MARGIN * 4 + 2), PAPER);
        assert_eq!(pixel(MARGIN * 4, MARGIN * 4 + 2), GRID);
    }
}
