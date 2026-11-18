use mcpack::mm::{self, ProofError, Verifier};
use std::path::Path;

fn first_failure(name: &str) -> Option<ProofError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name);
    let src = std::fs::read(&path).expect("corpus file reads");
    let db = mm::parse_at(&path, src).expect("corpus file parses");
    let mut verifier = Verifier::new(&db);
    db.theorems().find_map(|id| verifier.verify(id).err())
}

#[test]
fn valid_databases_are_accepted() {
    for name in [
        "anatomy.mm",
        "big-unifier.mm",
        "demo0.mm",
        "include/demo0-includer.mm",
        "miu.mm",
        "emptyline.mm",
    ] {
        assert_eq!(first_failure(name), None, "{name}");
    }
}

#[test]
fn malformed_databases_are_rejected() {
    for (name, error) in [
        ("anatomy-bad1.mm", ProofError::StackNotSingleton),
        ("anatomy-bad2.mm", ProofError::StackNotSingleton),
        ("anatomy-bad3.mm", ProofError::StackUnderflow),
        ("big-unifier-bad1.mm", ProofError::HypothesisMismatch),
        ("big-unifier-bad2.mm", ProofError::StackNotSingleton),
        ("big-unifier-bad3.mm", ProofError::HypothesisMismatch),
        ("demo0-bad1.mm", ProofError::HypothesisMismatch),
    ] {
        assert_eq!(first_failure(name), Some(error), "{name}");
    }
}

#[test]
fn the_command_line_reads_the_files_a_database_includes() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("verify")
        .arg(corpus.join("include/demo0-includer.mm"))
        .output()
        .expect("mcpack runs");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, b"1 proofs checked, 0 failed\n");
}
