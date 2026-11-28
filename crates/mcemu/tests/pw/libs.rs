use crate::host::read_table;
use crate::model::against_the_model;
use crate::rig::*;
use crate::shape::{compare, model_of};
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

/// The table of every theorem of every shipped library against the model's tree and the host's
/// reading of the proof, and the largest numbers of links and goals, so the margin under the
/// caps is known.
#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_theorem_of_the_shipped_libraries_gives_the_models_table() {
    let mut got = Vec::new();
    for name in LIBRARIES {
        let bytes = shipped(name);
        let library = decode(&bytes).unwrap();
        let lib = model_of(name, &bytes);
        let tc = provable(&bytes).unwrap();
        let mut rig = opened(&bytes);
        let (mut trees, mut syntax, mut axioms, mut links, mut goals, mut total) =
            (0, 0, 0, 0, 0, 0);
        for (n, a) in library.assertions.iter().enumerate() {
            if !a.theorem {
                axioms += 1;
                continue;
            }
            rig.seed_result();
            let out = rig.shape(n as u16, tc);
            if out.status() == SYNTAX {
                syntax += 1;
                continue;
            }
            assert_eq!(out.status(), ST_OK, "{name}: {}", a.label);
            let z = read_table(&mut rig);
            compare(&lib, &library, &bytes, n, &z);
            trees += 1;
            links = links.max(z.links);
            goals = goals.max(z.rows.len());
            total = total.max(z.total);
        }
        println!("{name}: most links {links}, most goals {goals}, largest total {total}");
        got.push((name, (trees, syntax, axioms)));
    }
    assert_eq!(got, CLASSES);
}
