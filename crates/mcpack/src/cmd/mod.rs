mod inspect;
mod library;

pub use inspect::{closure, parse, verify};
pub use library::{build, doc, pack};

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

pub fn load(path: &Path) -> Result<Database, ExitCode> {
    parse_source(path, read(path)?)
}

fn grammar(db: &Database) -> Result<Grammar<'_>, ExitCode> {
    Grammar::new(db, &[("|-", "wff")]).map_err(|e| {
        fail(format_args!(
            "the syntax axioms do not form a usable grammar: {e:?}"
        ))
    })
}
