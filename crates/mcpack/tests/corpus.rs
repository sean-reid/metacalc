use mcpack::mm::{self, Verifier};
use std::path::Path;

fn accepted(name: &str) -> bool {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name);
    let Ok(db) = mm::parse(std::fs::read(path).expect("corpus file reads")) else {
        return false;
    };
    let mut verifier = Verifier::new(&db);
    db.theorems().all(|id| verifier.verify(id).is_ok())
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
        assert!(accepted(name), "{name} should verify");
    }
}

#[test]
fn malformed_databases_are_rejected() {
    for name in [
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier-bad1.mm",
        "big-unifier-bad2.mm",
        "big-unifier-bad3.mm",
        "demo0-bad1.mm",
    ] {
        assert!(!accepted(name), "{name} should be rejected");
    }
}
