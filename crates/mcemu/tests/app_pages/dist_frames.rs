//! The Dist screen's frames on the stub OS against the host model: the first frame of every assertion
//! of a library that has pairs, the rows the app counts, and the same over the shipped libraries.

use crate::app::*;
use crate::dist_rig::*;
use crate::list_rig::proofs_three;
use crate::theorem_rig::*;
use mcui::data::Lib;
use std::collections::BTreeMap;

/// What a sweep of one library's first frames found, from the model.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub opened: usize,
    pub most_pairs: usize,
    pub most_rows: usize,
    /// How many assertions take each number of rows.
    pub by_rows: BTreeMap<usize, usize>,
}

/// The assertions of `lib` below `end` with pairs, or all of them.
pub fn targets(lib: &Lib, end: usize, only_pairs: bool) -> Vec<usize> {
    (0..end)
        .filter(|&n| !only_pairs || !lib.dist(n).is_empty())
        .collect()
}

/// Opens each of `targets`, F2 on each, and compares the first frame and the rows the app counted with
/// the model's.
pub fn sweep(built: &'static Built, stage: usize, targets: &[usize]) -> Found {
    let lib = lib_of(built);
    let mut walker = Walker::on_list(built, None, stage);
    walker.mirrored = false;
    let mut found = Found::default();
    for &n in targets {
        walker.goto(n);
        walker.hold_preview();
        walker.open();
        let at = format!("assertion {n} ({})", lib.label(n));
        open_dist(&mut walker);
        let rows = rows_of(&lib, n);
        assert_eq!(app_rows(&mut walker), rows.len(), "{at}: rows");
        assert_eq!(walker.var16("DV_NPAIRS"), lib.dist(n).len(), "{at}: pairs");
        found.opened += 1;
        found.most_pairs = found.most_pairs.max(lib.dist(n).len());
        found.most_rows = found.most_rows.max(rows.len());
        *found.by_rows.entry(rows.len()).or_default() += 1;
        leave(&mut walker);
        walker.back();
        walker.hold_preview();
    }
    found
}

fn found(opened: usize, most_pairs: usize, most_rows: usize, by_rows: &[(usize, usize)]) -> Found {
    Found {
        opened,
        most_pairs,
        most_rows,
        by_rows: by_rows.iter().copied().collect(),
    }
}

#[test]
fn the_first_frame_of_every_assertion_of_the_proofs_library_is_the_models() {
    let built = proofs_three();
    let lib = lib_of(built);
    let all = targets(&lib, lib.len(), false);
    let with = targets(&lib, lib.len(), true);
    assert_eq!((all.len(), with.len()), (74, 2), "the model's pairs");
    let got = sweep(built, 2, &all);
    println!("proofs: {got:?}");
    assert_eq!(got, found(74, 1, 1, &[(0, 72), (1, 2)]));
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_first_frame_of_every_assertion_with_pairs_of_pred_and_ring_is_the_models() {
    let prop = lib_of(shipped("prop"));
    assert_eq!(prop.len(), 779);
    assert!(
        targets(&prop, prop.len(), true).is_empty(),
        "prop has no condition at all"
    );
    for (name, count, want) in [
        ("pred", 1171, found(158, 10, 2, &[(1, 153), (2, 5)])),
        (
            "ring",
            3326,
            found(
                745,
                33,
                6,
                &[(1, 638), (2, 82), (3, 10), (4, 9), (5, 5), (6, 1)],
            ),
        ),
    ] {
        let built = shipped(name);
        let lib = lib_of(built);
        assert_eq!(lib.len(), count, "{name}");
        let with = targets(&lib, lib.len(), true);
        let got = sweep(built, 0, &with);
        println!("{name}: {got:?}");
        assert_eq!(got, want, "{name}");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_most_pairs_on_one_row_of_pred_and_ring_from_the_model() {
    for name in ["pred", "ring"] {
        let lib = lib_of(shipped(name));
        let most = (0..lib.len())
            .flat_map(|n| rows_of(&lib, n))
            .map(|row| row.len())
            .max()
            .unwrap();
        let widest = (0..lib.len()).flat_map(|n| widths(&lib, n)).max().unwrap();
        println!("{name}: at most {most} pairs on a row, the widest pair {widest} px");
    }
}
