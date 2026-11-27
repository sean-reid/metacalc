//! The find on the reference OS with the shipped libraries, typed on the keypad through app_key:
//! prefixes of ring's labels spread over the stage and of prop's and pred's ends under All, a smaller
//! sample under each kind, each frame the model's. Ignored in the default run.

use crate::list_find_rig::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use mcui::{Key, Kind, Status};

/// Types `label` to the cap after F2, every frame the model's, and returns whether it missed.
fn type_label(list: &mut Lister, label: &str) -> bool {
    list.f2();
    let typed: String = label.chars().take(8).collect();
    list.type_text(&typed);
    let missed = list.find().missed;
    list.skip_the_wait();
    list.settle();
    missed
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_ring_prop_and_pred_labels_typed_on_the_keypad_land_as_the_model_lands_them()
{
    for stage in [PROP, 1, RING] {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        let labels = labels_of(list.built);
        let items = list.items();
        let mut sample: Vec<usize> = vec![items[0], items[items.len() - 1]];
        if stage == RING {
            sample.extend(items.iter().copied().step_by(97));
        }
        let (mut hits, mut misses) = (0, 0);
        for &i in &sample {
            assert!(
                !type_label(&mut list, &labels[i]),
                "{}: a label of the stage",
                labels[i]
            );
            hits += 1;
        }
        for foreign in ["zzz", "0abc"] {
            assert!(
                type_label(&mut list, foreign),
                "{foreign} is no label's start"
            );
            misses += 1;
        }
        type_label(&mut list, "k");
        println!(
            "stage {stage}: {} labels typed, {hits} hits, {misses} misses tried",
            sample.len()
        );
        assert!(sample.len() >= if stage == RING { 35 } else { 2 });
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_sample_of_ring_typed_under_each_kind_lands_as_the_model_lands_it() {
    let mut seed = 0x2468_ACE1;
    let status: Vec<Status> = crate::table::random_status(3326, &mut seed, 60, 6);
    for kind in [Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        let mut list = crate::list_kind_real::ring(Some(&status));
        list.go_to_kind(kind);
        let labels = labels_of(list.built);
        let items = list.items();
        assert!(!items.is_empty());
        let (mut hits, mut misses) = (0, 0);
        for &i in items.iter().step_by(items.len() / 9 + 1) {
            if type_label(&mut list, &labels[i]) {
                misses += 1;
            } else {
                hits += 1;
            }
        }
        for foreign in ["s", "df", "k"] {
            if type_label(&mut list, foreign) {
                misses += 1;
            } else {
                hits += 1;
            }
        }
        println!("{kind:?}: {hits} hits, {misses} misses");
        assert!(hits >= 8, "{kind:?}: {hits} hits");
    }
    let _ = Key::F3;
}
