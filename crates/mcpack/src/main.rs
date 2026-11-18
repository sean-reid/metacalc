use clap::{Parser, Subcommand};
use mcpack::mm::{self, Verifier};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check every proof in a Metamath database
    Verify { database: PathBuf },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Verify { database } => verify(&database),
    }
}

fn verify(path: &PathBuf) -> ExitCode {
    let src = match std::fs::read(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    let db = match mm::parse(src) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut verifier = Verifier::new(&db);
    let (mut checked, mut failed) = (0usize, 0usize);
    for id in db.theorems() {
        checked += 1;
        if let Err(e) = verifier.verify(id) {
            failed += 1;
            eprintln!("{}: {e:?}", db.stmt(id).label);
        }
    }
    println!("{checked} proofs checked, {failed} failed");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }
}
