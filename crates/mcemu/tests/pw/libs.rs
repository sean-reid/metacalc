use crate::model::against_the_model;
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::decode;

pub const LIBRARIES: [&str; 3] = ["prop", "pred", "ring"];

pub fn shipped(name: &str) -> Vec<u8> {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    std::fs::read(format!("{dir}/{name}.mclib")).expect("the library reads")
}

/// (theorems with a goal tree, theorems that prove a syntax statement, axioms) of each library.
const CLASSES: [(&str, (usize, usize, usize)); 3] = [
    ("prop", (756, 0, 23)),
    ("pred", (1125, 2, 44)),
    ("ring", (3049, 2, 275)),
];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_theorem_of_the_shipped_libraries_gives_the_models_counts() {
    let mut got = Vec::new();
    for name in LIBRARIES {
        let bytes = shipped(name);
        let classes = against_the_model(name, &bytes, 4_000_000_000);
        println!(
            "{name}: {} theorems with a goal tree, {} that prove a syntax statement, {} axioms",
            classes.0, classes.1, classes.2
        );
        let assertions = decode(&bytes).unwrap().assertions.len();
        assert_eq!(classes.0 + classes.1 + classes.2, assertions, "{name}");
        got.push((name, classes));
    }
    assert_eq!(got, CLASSES);
}
