//! The reference ROM through the real shell: real keys from Libraries to a theorem and every key of the
//! screen after it, every frame compared with the model's pixels. These are the keys a person tries
//! on the calculator.

use crate::app::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::shellrig::*;
use crate::theorem_frames::{find, random_keys};
use crate::theorem_rig::*;
use mcui::Key;

/// The shipped build's list of `stage`, open on the reference OS, with the model beside it.
fn walker(stage: usize) -> Walker {
    let mut list = Lister::on_rom(stage);
    list.open_on_rom();
    let mut walker = Walker::new(list, stage, shipped_staged());
    walker.gap = 600_000;
    walker
}

/// The first assertion of prop, the first stage, the model's facts select.
fn in_prop(lib: &mcui::data::Lib, want: impl Fn(&Facts) -> bool) -> usize {
    (0..779)
        .find(|&i| want(&facts(lib, i)))
        .expect("prop has the assertion the test needs")
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_first_frame_of_each_kind_of_theorem_from_each_stage_of_the_shipped_list_is_the_models() {
    let lib = lib_of(shipped_staged());
    let kinds = [
        (
            "no hypothesis",
            find(&lib, |f| f.hyps == 0 && !f.large && f.rows <= 6).0,
        ),
        (
            "hypotheses",
            find(&lib, |f| {
                f.hyps > 0 && f.hyps < 6 && !f.large && f.rows <= 6
            })
            .0,
        ),
        ("opens folded", find(&lib, |f| f.folded && !f.large).0),
        ("scrolls", find(&lib, |f| f.rows > 6 && !f.large).0),
    ];
    for stage in [PROP, 1, RING] {
        let mut w = walker(stage);
        for (name, n) in kinds {
            assert!(n < 779, "{name}: {n} is in every stage's list");
            w.goto(n);
            w.hold_preview();
            w.open();
            assert_eq!(
                u16::from_le_bytes([w.frame()[2], w.frame()[3]]),
                n as u16,
                "stage {stage}, {name}"
            );
            w.back();
            w.hold_preview();
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_key_of_the_screen_on_a_scrolling_theorem_draws_the_models_frame_on_the_reference_os() {
    let lib = lib_of(shipped_staged());
    let n = in_prop(&lib, |f| f.hyps >= 3 && f.rows > 6 && !f.large);
    let mut w = walker(PROP);
    w.goto(n);
    w.open();
    let (mut drew, mut stayed) = (0, 0);
    for (scan, key) in random_keys(0x7EA1, 60) {
        if w.press(scan, key) {
            drew += 1;
        } else {
            stayed += 1;
        }
    }
    println!("{drew} keys drew a frame and {stayed} stayed");
    assert!(drew >= 38 && stayed >= 15, "{drew} drew, {stayed} stayed");
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn fold_unfold_the_soft_keys_and_clear_are_the_models_on_the_reference_os() {
    let lib = lib_of(shipped_staged());
    let n = in_prop(&lib, |f| f.folded && !f.large);
    let mut w = walker(PROP);
    w.goto(n);
    w.open();
    crate::theorem_keys::onto_a_folded_node(&mut w);
    assert!(w.press(ENTER, Key::Enter), "the fold is undone");
    assert!(w.press(F1, Key::F1), "and made again");
    for (scan, key) in [(F2, Key::F2), (F3, Key::F3), (F4, Key::F4)] {
        w.excursion(scan, key);
    }
    assert!(!w.press(F5, Key::F5), "F5 does nothing");
    w.back();
    w.hold_preview();
    assert!(w.key(CLEAR, Key::Clear), "Libraries is drawn");
    assert_eq!(w.list.run.top_kind(), KIND_LIBRARIES);
}
