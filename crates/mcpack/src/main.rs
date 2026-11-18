use clap::{Parser, Subcommand};
use mcpack::library::{self, Kernel, Limits};
use mcpack::measure::measure;
use mcpack::mm::{self, Database, Grammar, Kind, StmtId, Verifier};
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
    /// Parse every statement with the database's syntax axioms
    Parse { database: PathBuf },
    /// Pack what the given theorems depend on into a library and check it
    Pack {
        database: PathBuf,
        /// Labels of the theorems to keep
        #[arg(required = true)]
        roots: Vec<String>,
        /// Where to write the library
        #[arg(short, long)]
        output: PathBuf,
    },
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
        Command::Parse { database } => load(&database).and_then(|db| parse(&db)),
        Command::Pack {
            database,
            roots,
            output,
        } => load(&database).and_then(|db| pack(&db, &roots, &output)),
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

fn kept(db: &Database, roots: &[String]) -> Result<Vec<StmtId>, ExitCode> {
    let roots = find_all(db, roots)?;
    mm::closure(db, &grammar(db)?, &roots).map_err(|(id, e)| {
        eprintln!("{}: {e:?}", db.stmt(id).label);
        ExitCode::FAILURE
    })
}

fn pack(db: &Database, roots: &[String], output: &Path) -> Result<ExitCode, ExitCode> {
    let fail = |message: String| {
        eprintln!("{message}");
        ExitCode::FAILURE
    };
    let bytes = library::pack(db, &kept(db, roots)?).map_err(|e| fail(format!("{e:?}")))?;
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED)
        .map_err(|e| fail(format!("the packed library does not load: {e:?}")))?;
    let mut failed = 0usize;
    for id in 0..kernel.assertions() {
        if let Err(e) = kernel.verify(id) {
            failed += 1;
            let label = kernel.label(id).unwrap_or_default();
            eprintln!("{}: {e:?}", String::from_utf8_lossy(label));
        }
    }
    if failed != 0 {
        return Err(fail(format!("{failed} assertions failed the kernel check")));
    }
    std::fs::write(output, &bytes).map_err(|e| {
        eprintln!("{}: {e}", output.display());
        ExitCode::from(2)
    })?;
    println!(
        "{} assertions, {} bytes, {} pages of {} bytes",
        kernel.assertions(),
        bytes.len(),
        bytes.len().div_ceil(library::CHUNK),
        library::CHUNK
    );
    Ok(ExitCode::SUCCESS)
}

fn closure(db: &Database, roots: &[String], splits: &[String]) -> Result<ExitCode, ExitCode> {
    let splits = find_all(db, splits)?;
    let kept = kept(db, roots)?;
    let report = measure(db, &kept, &splits);
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

fn grammar(db: &Database) -> Result<Grammar<'_>, ExitCode> {
    Grammar::new(db, &[("|-", "wff")]).map_err(|e| {
        eprintln!("the syntax axioms do not form a usable grammar: {e:?}");
        ExitCode::FAILURE
    })
}

fn parse(db: &Database) -> Result<ExitCode, ExitCode> {
    let grammar = grammar(db)?;
    let (mut parsed, mut failed) = (0usize, 0usize);
    for id in 0..db.stmts.len() as StmtId {
        let stmt = db.stmt(id);
        if !matches!(stmt.kind, Kind::Axiom | Kind::Theorem) {
            continue;
        }
        parsed += 1;
        match grammar.parse(id) {
            Ok(parsed) => {
                let essentials = stmt.frame.hyps.iter().map(|&hyp| db.stmt(hyp));
                let essentials = essentials.filter(|hyp| hyp.kind == Kind::Essential);
                let spelled = essentials
                    .chain([stmt])
                    .zip(parsed.hyps.iter().chain([&parsed.conclusion]))
                    .all(|(stmt, tree)| grammar.expr(tree)[1..] == stmt.expr[1..]);
                if !spelled {
                    failed += 1;
                    eprintln!("{}: the parse does not spell the statement", stmt.label);
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e:?}", stmt.label);
            }
        }
    }
    println!("{parsed} assertions parsed, {failed} failed");
    Ok(if failed == 0 {
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
