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

/// A ring list under Unchecked with every assertion shown, its cursor on an axiom with four axioms
/// before it and its top four above, then the table rewritten under it with the cursor checked, and the
/// top too when `hide_top`, and entered again: the frame is the model's F3 landing, the cursor on the next
/// assertion shown and the top four shown above it.
fn rewritten(hide_top: bool) {
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
    if hide_top {
        new[cursor - 4] = Status::Checked;
    }
    let mut list = ring(Some(&old));
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(list.pos().len, n, "nothing is hidden under the old marks");
    move_to(&mut list, cursor);
    let frame = list.run.frames()[1].clone();
    let got = |f: &[u8], at: usize| usize::from(u16::from_le_bytes([f[at], f[at + 1]]));
    let (was_cursor, was_top) = (got(&frame, 2), got(&frame, 4));
    assert_eq!(
        (was_cursor, was_top),
        (cursor, cursor - 4),
        "forward moves leave the top four above"
    );
    let hidden = |i: usize| new[i] == Status::Checked;
    assert_eq!(
        (hidden(was_cursor), hidden(was_top)),
        (true, hide_top),
        "the new marks hide the cursor, and the top when asked"
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
    for _ in 0..axioms.len() {
        model.press(Key::Up);
    }
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
    let top = if hide_top { cursor - 5 } else { cursor - 4 };
    assert_eq!(
        (got(&now, 2), got(&now, 4)),
        (cursor + 1, top),
        "the cursor and the top landed"
    );
    assert_eq!(now[6], 3);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_table_rewritten_under_an_open_unchecked_list_is_met_as_f3_would_meet_it() {
    rewritten(true);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_table_that_hides_only_the_cursor_is_met_as_f3_would_meet_it() {
    rewritten(false);
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

/// Ring's status table with its first 1,500 assertions checked, as a check that runs in order leaves
/// it: one run of 1,500 that Unchecked hides, at the front, so that the list starts after it and no
/// move crosses it. A checked assertion after an unchecked one rests on it in the model when its proof
/// uses it, and Unchecked shows it, so a run hidden in the middle takes marks of its own (below).
const CHECKED: std::ops::Range<usize> = 0..1500;

/// The longest run of consecutive assertions the model marks checked.
fn longest_hidden(values: &[u8]) -> usize {
    values
        .split(|&v| v != 1)
        .map(<[u8]>::len)
        .max()
        .unwrap_or(0)
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_ring_is_walked_under_unchecked_by_row_from_the_first_shown_after_a_hidden_prefix_to_the_last_and_back()
 {
    let started = Instant::now();
    let probe = Lister::on_rom(RING);
    let probe_built = probe.built;
    let n = count(probe_built);
    drop(probe);
    let mut status = vec![Status::Unchecked; n];
    status[CHECKED].fill(Status::Checked);
    let values = crate::table::values(probe_built, &status);
    let want = values.iter().filter(|&&v| v != 1).count();
    let longest = longest_hidden(&values);
    assert_eq!(longest, CHECKED.len(), "the run the table hides");
    assert_eq!(want, n - CHECKED.len(), "what Unchecked shows");
    let mut list = ring(Some(&status));
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(list.pos().len, want, "the marks the model shows");
    assert_eq!(list.items().len(), want);
    let pressed = list.walk_plan(
        "ring Unchecked",
        &[(DOWN, Key::Down, want + 1), (UP, Key::Up, want + 1)],
    );
    assert_eq!(pressed, 2 * (want + 1));
    println!(
        "ROM, ring Unchecked: {want} of {n} shown, {pressed} keys by row after a prefix of {longest} the table hides, in {:?}",
        started.elapsed()
    );
}

/// Ring with everything checked but assertion 500: the model shows 500 and the assertions whose proofs
/// rest on it, and hides the others, so 18 runs of hidden assertions lie between shown ones, the longest
/// 1,971 long.
const ONE_UNCHECKED: usize = 500;

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_ring_is_walked_under_unchecked_by_row_across_runs_the_table_hides_in_the_middle()
 {
    let started = Instant::now();
    let probe = Lister::on_rom(RING);
    let probe_built = probe.built;
    let n = count(probe_built);
    drop(probe);
    let mut status = vec![Status::Checked; n];
    status[ONE_UNCHECKED] = Status::Unchecked;
    let values = crate::table::values(probe_built, &status);
    let shown: Vec<usize> = (0..n).filter(|&i| values[i] != 1).collect();
    let gaps: Vec<usize> = shown
        .windows(2)
        .map(|w| w[1] - w[0] - 1)
        .filter(|&g| g > 0)
        .collect();
    let (middle_runs, longest) = (gaps.len(), gaps.iter().copied().max().unwrap_or(0));
    assert_eq!(
        shown.first(),
        Some(&ONE_UNCHECKED),
        "the list starts at the unchecked one"
    );
    assert_eq!(
        (shown.len(), middle_runs, longest),
        (114, 18, 1971),
        "what the model hides"
    );
    let mut list = ring(Some(&status));
    list.go_to_kind(Kind::Unchecked);
    assert_eq!(list.items(), shown, "the model's items");
    let pressed = list.walk_plan(
        "ring Unchecked, runs in the middle",
        &[
            (DOWN, Key::Down, shown.len() + 1),
            (UP, Key::Up, shown.len() + 1),
        ],
    );
    assert_eq!(pressed, 2 * (shown.len() + 1));
    println!(
        "ROM, ring Unchecked with one unchecked assertion: {} of {n} shown, {middle_runs} hidden runs between them, the longest {longest}, {pressed} keys by row in {:?}",
        shown.len(),
        started.elapsed()
    );
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn nothing_is_written_below_ram_on_the_reference_os_under_any_kind() {
    for kind in [Kind::All, Kind::Theorems, Kind::Axioms, Kind::Unchecked] {
        let mut list = Lister::on_rom(PROP);
        list.partial_check(40);
        crate::list_kind_stays::nothing_below_ram(
            &mut list,
            &format!("ROM {kind:?}"),
            false,
            |l| {
                l.open_on_rom();
                l.go_to_kind(kind);
            },
        );
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
    // (down, up, left, right, f3, enter plain, enter landing, enter below the view, ends, draw, idle fit, f5)
    let (down, up, left, right, f3, plain, land, below, ends, draw, fit, f5) = match kind {
        Kind::Theorems => (16, 16, 16, 20, 24, 36, 36, 36, 0, 34, 16, 18),
        Kind::Axioms => (16, 16, 16, 20, 42, 36, 36, 36, 16, 34, 16, 18),
        _ => (36, 34, 34, 38, 36, 38, 42, 40, 0, 36, 36, 36),
    };
    crate::list_kind_depth::fig(
        [
            ("clear", 4),
            ("down", down),
            ("enter", down),
            ("f1", down),
            ("f2", 4),
            ("f3", f3),
            ("f4", 6),
            ("f5", f5),
            ("left", left),
            ("letter", if kind == Kind::Unchecked { 40 } else { 20 }),
            ("right", right),
            ("up", up),
        ],
        [f3, f3],
        [plain, land, below],
        [draw, draw],
        ends,
        [fit, 2, 14],
    )
}
