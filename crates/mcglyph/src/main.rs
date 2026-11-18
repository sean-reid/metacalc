//! Renders a Metamath token sequence in the calculator font to PNG files.

use mcglyph::{Bitmap, Font, png};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

const SCREEN: (usize, usize) = (96, 64);
const USAGE: &str = "usage: mcglyph [--scale N] [--font main|keys] -o OUT.png TOKEN...
Draws the tokens in the calculator font on 96x64 screens. A statement that wraps past one screen
is written to OUT-1.png, OUT-2.png and so on. Prints each file written.
With --font keys each argument is a soft key label, drawn in the key font as an inverted box,
one label per 8 pixel row, and the output is a single screen.";

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (mut scale, mut output, mut tokens, mut keys) = (4usize, None, Vec::new(), false);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" => output = args.next().map(PathBuf::from),
            "--scale" => {
                scale = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|&n| n > 0)
                    .ok_or(USAGE)?;
            }
            "--font" => {
                keys = match args.next().as_deref() {
                    Some("main") => false,
                    Some("keys") => true,
                    _ => return Err(USAGE.into()),
                }
            }
            "-h" | "--help" => return Err(USAGE.into()),
            _ => tokens.push(arg),
        }
    }
    let output = output.filter(|_| !tokens.is_empty()).ok_or(USAGE)?;
    let words: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let frames = if keys {
        vec![key_labels(&words)?]
    } else {
        Font::builtin()
            .frames(&words, SCREEN)
            .map_err(|e| e.to_string())?
    };
    for (i, frame) in frames.iter().enumerate() {
        let path = if frames.len() == 1 {
            output.clone()
        } else {
            numbered(&output, i + 1)
        };
        fs::write(&path, png::encode(frame, scale))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

fn numbered(path: &Path, n: usize) -> PathBuf {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    path.with_file_name(format!("{stem}-{n}.png"))
}

fn key_labels(labels: &[&str]) -> Result<Bitmap, String> {
    let font = Font::keys();
    let rows = SCREEN.1 / 8;
    if labels.len() > rows {
        return Err(format!("at most {rows} labels fit a screen"));
    }
    if let Some(bad) = labels
        .iter()
        .find(|l| !l.chars().all(|c| c.is_ascii_alphanumeric()))
    {
        return Err(format!("label {bad} is not letters and digits"));
    }
    let mut screen = Bitmap::new(SCREEN.0, SCREEN.1);
    for (i, label) in labels.iter().enumerate() {
        let ink = font.text(label);
        let (top, width) = (i * 8, ink.width() + 4);
        for y in top..top + 8 {
            for x in 0..width.min(SCREEN.0) {
                screen.set(x, y, true);
            }
        }
        for y in 0..ink.height() {
            for x in 0..ink.width() {
                if ink.get(x, y) {
                    screen.set(2 + x, top + 1 + y, false);
                }
            }
        }
    }
    Ok(screen)
}
