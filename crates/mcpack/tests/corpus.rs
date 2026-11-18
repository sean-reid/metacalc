use mcpack::mm::{self, ProofError, Verifier};
use std::path::Path;

fn first_failure(name: &str) -> Option<ProofError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name);
    let db =
        mm::parse(std::fs::read(path).expect("corpus file reads")).expect("corpus file parses");
    let mut verifier = Verifier::new(&db);
    db.theorems().find_map(|id| verifier.verify(id).err())
}

#[test]
fn valid_databases_are_accepted() {
    for name in [
        "anatomy.mm",
        "big-unifier.mm",
        "demo0.mm",
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
