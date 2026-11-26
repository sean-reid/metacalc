//! The kind filter on the reference OS, ignored in the default run: marks that change under an open
//! list, every assertion a kind shows walked by row, the gap between two F3 presses, and the writes
//! below RAM.

use crate::list_depth::Rig;
use crate::list_kind_depth::{KindFigures, Kit, check};
use crate::list_kind_real::{ring, ring_bits};
use crate::list_kind_rig::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind, Status};
use std::time::Instant;

/// Moves the open list's cursor, which is at assertion `from` of a list that shows every assertion,
/// to `to` by pages and rows, comparing every frame with the model's.
fn move_to(list: &mut Lister, to: usize) {
    while list.items()[list.pos().cursor] != to {
        let at = list.items()[list.pos().cursor];
        let (scan, key) = if at + 5 <= to {
            (RIGHT, Key::Right)
        } else {
            (DOWN, Key::Down)
        };
        list.key(scan, key);
        list.skip_the_wait();
        list.settle();
        list.run.run(600_000);
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_table_rewritten_under_an_open_unchecked_list_is_met_as_f3_would_meet_it() {
    let probe = ring(None);
    let n = count(probe.built);
    let bits = ring_bits(&probe);
    drop(probe);
    let axioms: Vec<usize> = (0..n).filter(|&i| !bits[i]).collect();
    let cursor = *axioms
        .iter()
        .find(|&&a| a >= 8 && bits[a - 3..a].iter().all(|&t| !t) && !bits[a - 4])
        .expect("five axioms in a row");
    let old = vec![Status::Unchecked; n];
    let mut new = old.clone();
    new[cursor] = Status::Checked;
    new[cursor - 4] = Status::Checked;
    let mut list = ring(Some(&old));
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(list.pos().len, n, "nothing is hidden under the old marks");
    move_to(&mut list, cursor);
    assert_eq!(
        list.pos().top,
        cursor - 4,
        "forward moves leave the top four above"
    );
    let rewritten = table_of(list.built, &new);
    let (at, size) = crate::real::vat::table_at(list.run.m()).expect("a table");
    assert_eq!(usize::from(size), rewritten.len());
    list.run.m().load_ram(at, &rewritten);
    list.tap(F4);
    list.run.run(600_000);
    list.tap(CLEAR);
    let frame = list.run.next_blit();
    list.run.run(2_000);
    let mut model = crate::shellrig::model::ui(list.built, &new);
    for _ in 0..RING {
        model.press(Key::Down);
    }
    model.press(Key::Enter);
    model.press(Key::F3);
    model.press(Key::F3);
    let place = axioms.iter().position(|&a| a == cursor).unwrap();
    for _ in 0..place / 5 {
        model.press(Key::Right);
    }
    for _ in 0..place % 5 {
        model.press(Key::Down);
    }
    model.press(Key::F3);
    same_frame("rewritten", &frame, &crate::shellrig::model::frame(&model));
    list.ui = model;
    list.status = new;
    list.same_kind_position("rewritten");
    let now = list.run.frames()[1].clone();
    let got = |at: usize| usize::from(u16::from_le_bytes([now[at], now[at + 1]]));
    assert_eq!(
        (got(2), got(4)),
        (cursor + 1, cursor - 5),
        "the cursor and the top landed"
    );
    assert_eq!(now[6], 3);
}

/// Walks every assertion `kind` shows on `stage` by row, up and down, and by page, every frame the
/// model's, with the wait of each preview skipped. Returns how many it shows and prints the time.
fn rom_walk(stage: usize, kind: Kind, partial: Option<usize>, what: &str) -> usize {
    let started = Instant::now();
    let mut list = Lister::on_rom(stage);
    if let Some(n) = partial {
        list.partial_check(n);
    }
    list.open_on_rom();
    list.go_to_kind(kind);
    let shown = list.items().len();
    let len = list.walk_with(what, true);
    assert_eq!(len, shown, "{what}: the model's count");
    println!(
        "ROM, {what}: {len} assertions walked in {:?}",
        started.elapsed()
    );
    len
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_ring_is_walked_under_theorems_by_row() {
    assert_eq!(rom_walk(RING, Kind::Theorems, None, "ring Theorems"), 3051);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_pred_is_walked_under_theorems_and_axioms_by_row() {
    assert_eq!(rom_walk(1, Kind::Theorems, None, "pred Theorems"), 1127);
    assert_eq!(rom_walk(1, Kind::Axioms, None, "pred Axioms"), 44);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_ring_is_walked_under_unchecked_with_a_partial_table_by_row() {
    let len = rom_walk(RING, Kind::Unchecked, Some(100), "ring Unchecked");
    assert!(len > 3000, "{len}");
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn nothing_is_written_below_ram_on_the_reference_os_under_any_kind() {
    for kind in [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        let mut list = Lister::on_rom(PROP);
        list.partial_check(40);
        list.open_on_rom();
        list.go_to_kind(kind);
        crate::list_kind_stays::nothing_below_ram(&mut list, &format!("ROM {kind:?}"), false);
    }
}

fn rom_open(kind: Kind) -> Lister {
    let mut list = Lister::on_rom(PROP);
    list.partial_check(40);
    list.open_on_rom();
    list.go_to_kind(kind);
    list
}

fn rom_open_down(kind: Kind) -> Lister {
    let mut list = rom_open(kind);
    list.key(DOWN, Key::Down);
    list
}

fn rom_rig(kind: Kind) -> Rig {
    let open: fn() -> Lister = match kind {
        Kind::Theorems => || rom_open_down(Kind::Theorems),
        Kind::Axioms => || rom_open_down(Kind::Axioms),
        _ => || rom_open_down(Kind::Unchecked),
    };
    Rig {
        name: "reference kind",
        open,
        before: || {
            let mut list = Lister::on_rom(PROP + 1);
            list.partial_check(40);
            list
        },
    }
}

const ROM: Kit = Kit {
    name: "reference",
    open: rom_open,
    rig: rom_rig,
    cap: 40,
    speeds: &[0],
};

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_depths_under_theorems_are() {
    check(&ROM, Kind::Theorems, &rom_want(Kind::Theorems));
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_depths_under_axioms_are() {
    check(&ROM, Kind::Axioms, &rom_want(Kind::Axioms));
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_depths_under_unchecked_with_a_table_are() {
    check(&ROM, Kind::Unchecked, &rom_want(Kind::Unchecked));
}

fn rom_want(kind: Kind) -> KindFigures {
    let _ = kind;
    KindFigures::default()
}
