//! The packed kinds bitmap says what the model's library says for every assertion.

mod support;

use mcpack::library;
use mcui::data::{Installed, Lib};
use support::{logic_bytes, statuses};

fn bitmap_of(bytes: &[u8]) -> Vec<bool> {
    library::decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .kinds
        .expect("a kinds section")
        .theorems
}

/// Checks the bitmap against `Lib::is_theorem` for every assertion and returns the theorem count.
fn agrees(name: &str, bytes: Vec<u8>) -> (usize, usize) {
    let bitmap = bitmap_of(&bytes);
    let status = statuses(&bytes, &[], &[]);
    let installed = Installed {
        name: name.into(),
        bytes,
    };
    let lib = Lib::new(installed, status).unwrap();
    assert_eq!(bitmap.len(), lib.len());
    for (i, &bit) in bitmap.iter().enumerate() {
        assert_eq!(bit, lib.is_theorem(i), "{name} assertion {i}");
    }
    let theorems = bitmap.iter().filter(|&&t| t).count();
    (theorems, bitmap.len() - theorems)
}

#[test]
fn the_fixtures_bitmap_is_the_models_kinds() {
    let (theorems, axioms) = agrees("logic", logic_bytes());
    assert_eq!((theorems, axioms), (6, 7));
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_bitmaps_are_the_models_kinds() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the libraries");
    for (name, want) in [
        ("prop", (756, 23)),
        ("pred", (1127, 44)),
        ("ring", (3051, 275)),
    ] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        let got = agrees(name, bytes);
        assert!(got.0 > 0 && got.1 > 0);
        assert_eq!(got, want, "{name}");
    }
}
