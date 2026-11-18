mod inspect;
mod library;

pub use inspect::{closure, outline, parse, verify};
pub use library::{build, doc, export, pack, summary};

use mcpack::mm::{self, Database, Grammar};
use std::path::Path;
use std::process::ExitCode;

/// Prints the message and returns the failure exit code.
fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

/// Prints the message and returns the exit code for input that cannot be used.
fn unusable(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(2)
}

fn read(path: &Path) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(path).map_err(|e| unusable(format_args!("{}: {e}", path.display())))
}

fn parse_source(path: &Path, src: Vec<u8>) -> Result<Database, ExitCode> {
    mm::parse(src).map_err(|e| fail(format_args!("{}: {e}", path.display())))
}

/// Reads a database without following its inclusion commands.
pub fn load_plain(path: &Path) -> Result<Database, ExitCode> {
    parse_source(path, read(path)?)
}

pub fn load(path: &Path) -> Result<Database, ExitCode> {
    mm::parse_at(path, read(path)?).map_err(|e| fail(format_args!("{}: {e}", path.display())))
}

fn grammar(db: &Database) -> Result<Grammar<'_>, ExitCode> {
    Grammar::new(db, &[("|-", "wff")]).map_err(|e| {
        fail(format_args!(
            "the syntax axioms do not form a usable grammar: {e}"
        ))
    })
}
