use clap::{Parser, Subcommand};
use mcpack::measure::measure;
use mcpack::mm::{self, Database, StmtId, Verifier};
use std::path::{Path, PathBuf};
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
    /// Measure what the given theorems depend on
    Closure {
        database: PathBuf,
        /// Labels of the theorems to keep
        #[arg(required = true)]
        roots: Vec<String>,
        /// Start a new row of the report at this label
        #[arg(long = "split", value_name = "LABEL")]
        splits: Vec<String>,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Verify { database } => load(&database).map(|db| verify(&db)),
        Command::Closure {
            database,
            roots,
            splits,
        } => load(&database).and_then(|db| closure(&db, &roots, &splits)),
    }
    .unwrap_or_else(|code| code)
}

fn load(path: &Path) -> Result<Database, ExitCode> {
    let src = std::fs::read(path).map_err(|e| {
        eprintln!("{}: {e}", path.display());
        ExitCode::from(2)
    })?;
    mm::parse(src).map_err(|e| {
        eprintln!("{}: {e}", path.display());
        ExitCode::FAILURE
    })
}

fn find_all(db: &Database, labels: &[String]) -> Result<Vec<StmtId>, ExitCode> {
    labels
        .iter()
        .map(|label| {
            db.find(label).ok_or_else(|| {
                eprintln!("no statement is labelled {label}");
                ExitCode::from(2)
            })
        })
        .collect()
}

fn closure(db: &Database, roots: &[String], splits: &[String]) -> Result<ExitCode, ExitCode> {
    let roots = find_all(db, roots)?;
    let splits = find_all(db, splits)?;
    let report = measure(db, &mm::closure(db, &roots), &splits);
    print!("{report}");
    for (label, e) in &report.failures {
        eprintln!("{label}: {e:?}");
    }
    Ok(if report.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn verify(db: &Database) -> ExitCode {
    let mut verifier = Verifier::new(db);
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
