//! Writes PNG previews of the glyph chart and real statements to a directory.
//!
//! Usage: cargo run -p mcglyph --example preview -- <set.mm> [out-dir]

use mcglyph::{Bitmap, Font, png};
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
    for (page, names) in names.chunks(40).enumerate() {
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
        let frames = font.frames(&tokens, SCREEN).expect("statement renders");
        let note = (frames.len() > 1).then_some(label);
        write(&out, &format!("stmt-{label}"), &frames[0], SCALE, note);
    }
}

fn chart(font: &Font, names: &[&str]) -> Bitmap {
    const CELL: (usize, usize) = (68, 10);
    const COLS: usize = 5;
    let mut out = Bitmap::new(COLS * CELL.0, names.len().div_ceil(COLS) * CELL.1);
    for (i, name) in names.iter().enumerate() {
        let (x, y) = ((i % COLS) * CELL.0 + 2, (i / COLS) * CELL.1 + 1);
        out.blit(font.glyph(name).expect("glyph exists"), x, y);
        if name.len() > 1 {
            let mut pen = x + 11;
            for c in name.chars() {
                let letter = font
                    .glyph(&c.to_string())
                    .expect("names are spelled in glyphs");
                out.blit(letter, pen, y);
                pen += letter.width() + 1;
            }
        }
    }
    out
}

fn write(dir: &str, name: &str, bitmap: &Bitmap, scale: usize, truncated: Option<&str>) {
    if let Some(label) = truncated {
        eprintln!("{label}: wraps past {} rows, shown cut off", SCREEN.1);
    }
    let path = Path::new(dir).join(format!("{name}.png"));
    fs::write(&path, png::encode(bitmap, scale)).expect("write png");
    println!("{}", path.display());
}
