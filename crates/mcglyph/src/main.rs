//! Renders a Metamath token sequence in the calculator font to PNG files.

use mcglyph::{Font, png};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

const SCREEN: (usize, usize) = (96, 64);
const USAGE: &str = "usage: mcglyph [--scale N] -o OUT.png TOKEN...
Draws the tokens in the calculator font on 96x64 screens. A statement that wraps past one screen
is written to OUT-1.png, OUT-2.png and so on. Prints each file written.";

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
    let (mut scale, mut output, mut tokens) = (4usize, None, Vec::new());
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
            "-h" | "--help" => return Err(USAGE.into()),
            _ => tokens.push(arg),
        }
    }
    let output = output.filter(|_| !tokens.is_empty()).ok_or(USAGE)?;
    let words: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let frames = Font::builtin()
        .frames(&words, SCREEN)
        .map_err(|e| e.to_string())?;
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
