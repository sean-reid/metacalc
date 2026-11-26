//! The packed label order is a bytewise sort of the model's labels.

mod support;

use mcpack::library;
use mcui::data::{Installed, Lib};
use support::{logic_bytes, statuses};

fn order_of(bytes: &[u8]) -> Vec<u16> {
    library::decode(bytes)
        .unwrap()
        .sections
        .unwrap()
        .label_order
        .expect("a label order section")
        .numbers
}

/// Checks the packed order against `Lib::label` for every assertion and returns its length.
fn agrees(name: &str, bytes: Vec<u8>) -> usize {
    let order = order_of(&bytes);
    let status = statuses(&bytes, &[], &[]);
    let installed = Installed {
        name: name.into(),
        bytes,
    };
    let lib = Lib::new(installed, status).unwrap();
    assert_eq!(order.len(), lib.len());
    let mut want: Vec<usize> = (0..lib.len()).collect();
    want.sort_by(|&a, &b| lib.label(a).as_bytes().cmp(lib.label(b).as_bytes()));
    let got: Vec<usize> = order.iter().map(|&n| n as usize).collect();
    assert_eq!(got, want, "{name}");
    let mut seen = vec![false; lib.len()];
    for &n in &got {
        assert!(!std::mem::replace(&mut seen[n], true), "{name}: {n} twice");
    }
    got.len()
}

#[test]
fn the_fixtures_order_is_the_models_labels_sorted() {
    assert_eq!(agrees("logic", logic_bytes()), 13);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_orders_are_the_models_labels_sorted() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the libraries");
    for (name, count) in [("prop", 779), ("pred", 1171), ("ring", 3326)] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
        assert_eq!(agrees(name, bytes), count, "{name}");
    }
}
