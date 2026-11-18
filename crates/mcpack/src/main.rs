mod cmd;

use clap::{Parser, Subcommand};
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
    /// Build the library a manifest describes from the pinned database
    Build {
        manifest: PathBuf,
        /// The pinned set.mm
        #[arg(long)]
        database: PathBuf,
        /// Where to write the library
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Write the reference page of a packed library
    Doc {
        manifest: PathBuf,
        /// A library written by `build`
        #[arg(long)]
        library: PathBuf,
        /// Where to write the Markdown page
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Verify { database } => cmd::load(&database).map(|db| cmd::verify(&db)),
        Command::Parse { database } => cmd::load(&database).and_then(|db| cmd::parse(&db)),
        Command::Pack {
            database,
            roots,
            output,
        } => cmd::load(&database).and_then(|db| cmd::pack(&db, &roots, &output)),
        Command::Closure {
            database,
            roots,
            splits,
        } => cmd::load(&database).and_then(|db| cmd::closure(&db, &roots, &splits)),
        Command::Build {
            manifest,
            database,
            output,
        } => cmd::build(&manifest, &database, &output),
        Command::Doc {
            manifest,
            library,
            output,
        } => cmd::doc(&manifest, &library, &output),
    }
    .unwrap_or_else(|code| code)
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
