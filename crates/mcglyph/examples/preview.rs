//! Writes PNG previews of the glyph chart and real statements to a directory.
//!
//! Usage: cargo run -p mcglyph --example preview -- <set.mm> [out-dir]

use mcglyph::{Bitmap, Font};
use mcpack::mm;
use std::path::Path;
use std::{env, fs, process};

const LABELS: [&str; 10] = [
    "ringlz", "ringnegl", "ringcom", "ringgrp", "df-ring", "df-grp", "df-mnd", "df-f1o", "df-iota",
    "ringcl",
];
const SCALE: usize = 8;
const SCREEN: (usize, usize) = (96, 64);

fn main() {
    let mut args = env::args().skip(1);
    let Some(set_mm) = args.next() else {
        eprintln!("usage: preview <set.mm> [out-dir]");
        process::exit(2);
    };
    let out = args.next().unwrap_or_else(|| "target/preview".into());
    fs::create_dir_all(&out).expect("create output directory");
    let font = Font::builtin();

    let names: Vec<&str> = font.glyph_names().collect();
    for (page, names) in names.chunks(36).enumerate() {
        write(
            &out,
            &format!("chart-{}", page + 1),
            &chart(font, names),
            5,
            None,
        );
    }

    let db = mm::parse(fs::read(set_mm).expect("read set.mm")).expect("parse set.mm");
    for label in LABELS {
        let id = db
            .find(label)
            .unwrap_or_else(|| panic!("no statement {label}"));
        let mut tokens: Vec<&str> = vec!["|-"];
        let stmt = db.stmt(id);
        tokens.extend(
            stmt.expr[1..]
                .iter()
                .map(|&s| db.syms[s as usize].name.as_str()),
        );
        let bitmap = font.render(&tokens, SCREEN.0).expect("statement renders");
        let note = (bitmap.height() > SCREEN.1).then_some(label);
        write(
            &out,
            &format!("stmt-{label}"),
            &framed(&bitmap),
            SCALE,
            note,
        );
    }
}

fn chart(font: &Font, names: &[&str]) -> Bitmap {
    const CELL: (usize, usize) = (44, 20);
    const COLS: usize = 6;
    let mut out = Bitmap::new(COLS * CELL.0, names.len().div_ceil(COLS) * CELL.1);
    for (i, name) in names.iter().enumerate() {
        let (x, y) = ((i % COLS) * CELL.0 + 3, (i / COLS) * CELL.1 + 2);
        out.blit(font.glyph(name).expect("glyph exists"), x, y);
        let mut pen = x;
        for c in name.chars() {
            let letter = font
                .glyph(&c.to_string())
                .expect("names are spelled in glyphs");
            out.blit(letter, pen, y + 10);
            pen += letter.width() + 1;
        }
    }
    out
}

fn framed(text: &Bitmap) -> Bitmap {
    let mut screen = Bitmap::new(SCREEN.0, SCREEN.1);
    for y in 0..SCREEN.1 {
        for x in 0..SCREEN.0 {
            screen.set(x, y, text.get(x, y));
        }
    }
    screen
}

fn write(dir: &str, name: &str, bitmap: &Bitmap, scale: usize, truncated: Option<&str>) {
    if let Some(label) = truncated {
        eprintln!("{label}: wraps past {} rows, shown cut off", SCREEN.1);
    }
    let path = Path::new(dir).join(format!("{name}.png"));
    fs::write(&path, png(bitmap, scale)).expect("write png");
    println!("{}", path.display());
}

fn png(bitmap: &Bitmap, scale: usize) -> Vec<u8> {
    const MARGIN: usize = 2;
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
                (false, ..) => 0x55,
                (_, true, _) => 0x14,
                (_, _, true) => 0xb4,
                _ => 0xc4,
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

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut blocks = data.chunks(0xffff).peekable();
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        out.extend((block.len() as u16).to_le_bytes());
        out.extend((!(block.len() as u16)).to_le_bytes());
        out.extend(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend(((b << 16) | a).to_be_bytes());
    out
}
