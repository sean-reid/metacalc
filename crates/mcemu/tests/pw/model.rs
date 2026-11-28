use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::decode;

/// What the walk answers for every assertion of `bytes` against the host model: the numbers for a
/// theorem the model gives a tree, the syntax status for one it refuses, the axiom status for an
/// axiom. Returns (trees, syntax statements, axioms) seen.
pub fn against_the_model(name: &str, bytes: &[u8], limit: u64) -> (usize, usize, usize) {
    let library = decode(bytes).expect("it decodes");
    let tc = provable(bytes).expect("the library has the provable typecode");
    let model = model_shapes(name, bytes);
    assert_eq!(model.len(), library.assertions.len());
    let mut rig = opened(bytes);
    let (mut trees, mut syntax, mut axioms) = (0, 0, 0);
    let mut spent = 0u64;
    for (n, a) in library.assertions.iter().enumerate() {
        rig.seed_result();
        let out = rig.shape(n as u16, tc);
        spent += u64::from(out.tstates);
        if !a.theorem {
            assert_eq!(out.status(), NONE, "{name}: {n} {}", a.label);
            assert_eq!(rig.result_bytes(), [MARK; 8], "{name}: {n}: untouched");
            axioms += 1;
        } else if let Some(shape) = model[n] {
            assert_eq!(out.status(), ST_OK, "{name}: {n} {}", a.label);
            assert_eq!(rig.result(), Counts::of(shape), "{name}: {n} {}", a.label);
            trees += 1;
        } else {
            assert_eq!(out.status(), SYNTAX, "{name}: {n} {}", a.label);
            assert_eq!(rig.result_bytes(), [MARK; 8], "{name}: {n}: untouched");
            syntax += 1;
        }
    }
    assert!(spent < limit, "{name}: {spent} T-states");
    (trees, syntax, axioms)
}

#[test]
fn the_fixture_gives_the_counts_the_model_and_the_stated_values_give() {
    let bytes = fixture();
    let model = model_shapes("fixture", &bytes);
    for (label, steps, goals, links, total) in KNOWN {
        let n = usize::from(number_of(&bytes, label));
        assert_eq!(
            model[n],
            Some(Shape {
                steps,
                goals,
                links,
                total
            }),
            "{label}: the model"
        );
    }
    let (trees, syntax, axioms) = against_the_model("fixture", &bytes, 5_000_000);
    assert_eq!((trees, syntax, axioms), (7, 1, 6));
}

#[test]
fn the_libraries_with_more_than_one_chunk_give_the_models_counts() {
    let bytes = packed(&multi_source(1500));
    assert!(bytes.len() > 3 * CHUNK);
    let (trees, syntax, axioms) = against_the_model("multi", &bytes, 200_000_000);
    assert_eq!((trees, syntax, axioms), (1500, 0, 6));
}

#[test]
fn the_corpus_libraries_give_the_models_counts() {
    let mut seen = (0, 0, 0);
    let mut libraries = 0;
    for name in ["anatomy.mm", "demo0.mm", "miu.mm", "big-unifier.mm"] {
        let bytes = pack_all(&corpus(name));
        if provable(&bytes).is_none() {
            continue;
        }
        let (t, s, a) = against_the_model(name, &bytes, 100_000_000);
        seen = (seen.0 + t, seen.1 + s, seen.2 + a);
        libraries += 1;
    }
    assert!(
        libraries >= 3 && seen.0 >= 3 && seen.2 >= 20,
        "{libraries} {seen:?}"
    );
}
