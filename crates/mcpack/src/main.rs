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
    ///
    /// Files named by `$[ file $]` are read, and may lie outside the database's directory.
    Verify { database: PathBuf },
    /// Parse every statement with the database's syntax axioms
    ///
    /// Files named by `$[ file $]` are read, and may lie outside the database's directory.
    Parse { database: PathBuf },
    /// Print the proof of one theorem as a backward goal tree
    ///
    /// Each line is a statement and the label applied to it in brackets, indented under the
    /// statement that needed it. Steps that build syntax are left out. A step a compressed proof
    /// saved appears at every place it is used. Exits 2 for an unknown label and 1 for an axiom
    /// or a proof that does not verify. Files named by `$[ file $]` are read, and may lie outside
    /// the database's directory.
    Outline { database: PathBuf, label: String },
    /// Pack what the given theorems depend on into a library and check it
    ///
    /// A database that includes files is refused: a library comes from the one file named.
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
    ///
    /// A database that includes files is refused: a library comes from the one file named.
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
        /// Fail, writing nothing, when a symbol of the library has no Unicode spelling
        #[arg(long)]
        require_unicode: bool,
        /// Fail, writing nothing, when a symbol of the notation table has no meaning
        #[arg(long)]
        require_meaning: bool,
    },
    /// Print the index table of the library pages, one row per manifest in manifest order
    Summary {
        /// The directory holding the libraries written by `build`, as NAME.mclib
        libdir: PathBuf,
        #[arg(required = true)]
        manifests: Vec<PathBuf>,
    },
    /// Write a packed library back out as a Metamath database
    Export {
        library: PathBuf,
        /// Where to write the database
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Verify { database } => cmd::load(&database).map(|db| cmd::verify(&db)),
        Command::Parse { database } => cmd::load(&database).and_then(|db| cmd::parse(&db)),
        Command::Outline { database, label } => {
            cmd::load(&database).and_then(|db| cmd::outline(&db, &label))
        }
        Command::Pack {
            database,
            roots,
            output,
        } => cmd::load_plain(&database).and_then(|db| cmd::pack(&db, &roots, &output)),
        Command::Closure {
            database,
            roots,
            splits,
        } => cmd::load_plain(&database).and_then(|db| cmd::closure(&db, &roots, &splits)),
        Command::Build {
            manifest,
            database,
            output,
        } => cmd::build(&manifest, &database, &output),
        Command::Doc {
            manifest,
            library,
            output,
            require_unicode,
            require_meaning,
        } => cmd::doc(
            &manifest,
            &library,
            &output,
            require_unicode,
            require_meaning,
        ),
        Command::Summary { libdir, manifests } => cmd::summary(&manifests, &libdir),
        Command::Export { library, output } => cmd::export(&library, &output),
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
