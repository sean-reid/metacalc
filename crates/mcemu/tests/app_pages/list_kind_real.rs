//! The kind filter on the reference OS with the shipped libraries, ignored in the default run: F3 round
//! the kinds on each stage, the walks, and what a move and an F3 cost with their frames against the
//! budgets.

use crate::app::*;
use crate::list_kind_rig::*;
use crate::list_real::{PROP, RING};
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcui::{Key, Kind, Status};

const MOVE_BUDGET: u32 = 300_000;
const F3_BUDGET: u32 = 1_800_000;

/// The theorem bit of each assertion of the shipped ring library.
pub fn ring_bits(list: &Lister) -> Vec<bool> {
    crate::support::decoded(list.built.library.as_ref().unwrap())
        .assertions
        .iter()
        .map(|a| a.theorem)
        .collect()
}

/// scr_draw then ui_blit for the screen on top, timed.
fn timed_frame(app: &mut App) -> u32 {
    let draw = app.label("scr_draw");
    let draw = app.timed(BASE, draw, In::default()).tstates;
    let blit = app
        .timed(BASE, app.label("far_ui_blit"), In::default())
        .tstates;
    draw + blit
}

/// A list of ring opened on the ROM, with `status` as its marks when given.
pub fn ring(status: Option<&[Status]>) -> Lister {
    let mut list = Lister::on_rom(RING);
    if let Some(status) = status {
        list.partial_check(1);
        let data = table_of(list.built, status);
        let (at, size) = crate::real::vat::table_at(list.run.m()).expect("the check made a table");
        assert_eq!(usize::from(size), data.len());
        list.run.m().load_ram(at, &data);
        list.status = status.to_vec();
        let mut ui = crate::shellrig::model::ui(list.built, status);
        for _ in 0..RING {
            ui.press(Key::Down);
        }
        list.ui = ui;
    }
    list.open_on_rom();
    assert_eq!(list.var("LV_HASK"), 1);
    list
}

/// The assertions of `kind` (a frame's kind byte) that the model shows on ring, from the library's
/// theorem bits and the marks the model was given.
fn shown_on_ring(list: &Lister, kind: u8) -> Vec<usize> {
    let bits = ring_bits(list);
    let values = (!list.status.is_empty()).then(|| crate::table::values(list.built, &list.status));
    let end = list.built.stages[RING].end;
    (0..end)
        .filter(|&i| match kind {
            0 => true,
            1 => bits[i],
            2 => !bits[i],
            _ => values.as_ref().is_none_or(|v| v[i] != 1),
        })
        .collect()
}

const ROW_KEYS: [(u8, Key); 5] = [
    (1, Key::Up),
    (2, Key::Down),
    (3, Key::Left),
    (4, Key::Right),
    (9, Key::F3),
];

/// The model driven to kind `kind` with its cursor on `cursor` and its first row on `top`, then pressed
/// with `key`: the cursor and the top it lands on, as assertion numbers, and the kind byte. The place
/// is reached by F3 from All, rows down to four below `top` and then rows up.
fn model_after(list: &Lister, kind: u8, cursor: usize, top: usize, key: u8) -> (usize, usize, u8) {
    let items = shown_on_ring(list, kind);
    let at = |n: usize| items.iter().position(|&i| i == n);
    let (cursor_at, top_at) = (
        at(cursor).unwrap_or_else(|| panic!("cursor {cursor} is not shown by kind {kind}")),
        at(top).unwrap_or_else(|| panic!("top {top} is not shown by kind {kind}")),
    );
    assert!(
        (top_at..=top_at + 4).contains(&cursor_at),
        "the cursor {cursor_at} is not in the view that starts at {top_at}"
    );
    let mut ui = crate::shellrig::model::ui(list.built, &list.status);
    for _ in 0..RING {
        ui.press(Key::Down);
    }
    ui.press(Key::Enter);
    for _ in 0..kind {
        ui.press(Key::F3);
    }
    for _ in 0..top_at + 4 {
        ui.press(Key::Down);
    }
    for _ in 0..top_at + 4 - cursor_at {
        ui.press(Key::Up);
    }
    let place = |ui: &mcui::Ui, items: &[usize]| match ui.state() {
        mcui::State::List {
            cursor, top, len, ..
        } if len == items.len() => (items[cursor], items[top]),
        other => panic!("{other:?}"),
    };
    let start = place(&ui, &items);
    assert_eq!(
        start,
        (cursor, top),
        "the model's cursor and top for kind {kind} are the case's"
    );
    let key = ROW_KEYS.iter().find(|k| k.0 == key).expect("a key").1;
    ui.press(key);
    let now = match ui.state() {
        mcui::State::List { kind, .. } => code(kind),
        other => panic!("{other:?}"),
    };
    let (c, t) = place(&ui, &shown_on_ring(list, now));
    (c, t, now)
}

/// What `key` costs from a frame of kind `kind` at `cursor` and `top` on ring, and the frame it leaves,
/// as the shell runs them: list_key, then scr_draw and ui_blit. The cursor and the top are shown by the
/// kind and the model's own for that place; the answer is SCR_REDRAW when the model's key moved
/// anything, and the frame the key leaves holds the model's kind, cursor and top. Returns (key, frame,
/// answer).
fn cost(list: &mut Lister, kind: u8, cursor: usize, top: usize, key: u8) -> (u32, u32, u8) {
    let (c, t, k) = model_after(list, kind, cursor, top, key);
    let app = &mut list.run.shell.app;
    let frame = app.label("SCR_STACK") + 16;
    let (cb, tb) = ((cursor as u16).to_le_bytes(), (top as u16).to_le_bytes());
    app.m.load_ram(
        frame,
        &[KIND_LIST, RING as u8, cb[0], cb[1], tb[0], tb[1], kind],
    );
    let keyed = app.timed(
        BASE,
        app.label("list_key"),
        In {
            a: key,
            hl: frame,
            ..In::default()
        },
    );
    let left = list.run.peek(frame, 7);
    let got = |at: usize| usize::from(u16::from_le_bytes([left[at], left[at + 1]]));
    assert_eq!(
        (got(2), got(4), left[6]),
        (c, t, k),
        "the frame the key leaves"
    );
    let moved = (c, t, k) != (cursor, top, kind);
    assert_eq!(keyed.status(), u8::from(moved), "the answer to key {key}");
    let app = &mut list.run.shell.app;
    let drawn = timed_frame(app);
    (keyed.tstates, drawn, keyed.status())
}

const UP_KEY: u8 = 1;
const DOWN_KEY: u8 = 2;
const LEFT_KEY: u8 = 3;
const RIGHT_KEY: u8 = 4;
const F3_KEY: u8 = 9;

/// The place of `n` in `items` and the first row shown that puts it on the last row.
fn last_row(items: &[usize], n: usize) -> usize {
    let at = items.iter().position(|&i| i == n).expect("an item");
    items[at.saturating_sub(4)]
}

fn items_of(bits: &[bool], theorem: bool, end: usize) -> Vec<usize> {
    (0..end).filter(|&i| bits[i] == theorem).collect()
}

/// The figure for every move and F3 of decision 10, printed on every run. A `None` pin is a figure
/// not yet measured.
fn report(name: &str, key: u32, frame: u32, budget: u32, pin: Option<u32>) {
    let total = key + frame;
    println!("ROM, {name}: list_key {key}, frame {frame}: {total} T-states against {budget}");
    assert!(total < budget, "{name}: {total}");
    if let Some(pin) = pin {
        within(name, total, pin);
    }
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_moves_under_axioms_and_theorems_fit_a_move_with_its_frame() {
    let mut list = ring(None);
    let bits = ring_bits(&list);
    let end = bits.len();
    let axioms = items_of(&bits, false, end);
    let theorems = items_of(&bits, true, end);
    assert_eq!((axioms.len(), theorems.len()), (275, 3051));
    let after = |items: &[usize], n: usize| items[items.iter().position(|&i| i == n).unwrap() + 1];
    let before = |items: &[usize], n: usize| items[items.iter().position(|&i| i == n).unwrap() - 1];
    assert_eq!(after(&axioms, 298), 567);
    let ax = code(Kind::Axioms);
    let th = code(Kind::Theorems);

    let (k, f, a) = cost(&mut list, ax, 298, last_row(&axioms, 298), DOWN_KEY);
    assert_eq!(a, 1);
    report(
        "Axioms, a row from 298 to 567",
        k,
        f,
        MOVE_BUDGET,
        Some(281630),
    );
    let (k, f, a) = cost(&mut list, ax, 567, last_row(&axioms, 567), UP_KEY);
    assert_eq!(a, 1);
    report(
        "Axioms, a row from 567 to 298",
        k,
        f,
        MOVE_BUDGET,
        Some(235996),
    );
    let (k, f, a) = cost(&mut list, ax, 5, 0, RIGHT_KEY);
    assert_eq!(a, 1);
    assert_eq!(
        axioms[axioms.iter().position(|&i| i == 5).unwrap() + 5],
        567
    );
    report("Axioms, a page from 5", k, f, MOVE_BUDGET, Some(295691));
    let a5 = after(&axioms, 137);
    let (k, f, a) = cost(&mut list, ax, a5, 137, UP_KEY);
    assert_eq!(a, 1);
    assert_eq!(before(&axioms, a5), 137);
    report(
        "Axioms, a row whose frame draws the five rows from 137",
        k,
        f,
        MOVE_BUDGET,
        Some(231166),
    );
    let (k, f, a) = cost(&mut list, th, 2988, last_row(&theorems, 2988), DOWN_KEY);
    assert_eq!(a, 1);
    assert_eq!(after(&theorems, 2988), 3019);
    report(
        "Theorems, a row across 2,988's run",
        k,
        f,
        MOVE_BUDGET,
        Some(269716),
    );
    let (k, f, a) = cost(&mut list, th, 3019, last_row(&theorems, 3019), UP_KEY);
    assert_eq!(a, 1);
    report(
        "Theorems, a row back across 2,988's run",
        k,
        f,
        MOVE_BUDGET,
        Some(241538),
    );
    let (k, f, _) = cost(&mut list, th, 100, 90, LEFT_KEY);
    report("Theorems, a page", k, f, MOVE_BUDGET, Some(252794));

    let just_after = 299;
    assert!(bits[just_after]);
    let (k, f, a) = cost(
        &mut list,
        th,
        just_after,
        last_row(&theorems, just_after),
        F3_KEY,
    );
    assert_eq!(a, 1);
    report(
        "F3 from Theorems to Axioms, the cursor just after an axiom",
        k,
        f,
        F3_BUDGET,
        Some(259343),
    );
    let past_last = 3320;
    assert!(past_last > *axioms.last().unwrap());
    let (k, f, a) = cost(
        &mut list,
        th,
        past_last,
        last_row(&theorems, past_last),
        F3_KEY,
    );
    assert_eq!(a, 1);
    report(
        "F3 from Theorems to Axioms, the cursor past the last axiom",
        k,
        f,
        F3_BUDGET,
        Some(248676),
    );
    let (k, f, _) = cost(&mut list, code(Kind::All), 1600, 1596, F3_KEY);
    report("F3 from All to Theorems", k, f, F3_BUDGET, Some(248336));
    let (k, f, _) = cost(&mut list, code(Kind::Unchecked), 1600, 1596, F3_KEY);
    report(
        "F3 from Unchecked to All, no table",
        k,
        f,
        F3_BUDGET,
        Some(207412),
    );
    let (k, f, _) = cost(&mut list, code(Kind::All), 1600, 1596, DOWN_KEY);
    report("All, a row", k, f, MOVE_BUDGET, Some(205512));
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_unchecked_on_a_table_of_ones_with_the_first_and_the_last_not_is_the_worst_walk()
 {
    let probe = Lister::on_rom(RING);
    let n = count(probe.built);
    let mut status = vec![Status::Checked; n];
    status[0] = Status::Unchecked;
    status[n - 1] = Status::Unchecked;
    drop(probe);
    let mut list = ring(Some(&status));
    let un = code(Kind::Unchecked);
    let (k, f, a) = cost(&mut list, un, 0, 0, DOWN_KEY);
    assert_eq!(a, 1);
    report(
        "Unchecked, a row from 0 to the last",
        k,
        f,
        MOVE_BUDGET + 20_000,
        Some(306616),
    );
    let (k, f, a) = cost(&mut list, un, n - 1, 0, UP_KEY);
    assert_eq!(a, 1);
    report(
        "Unchecked, a row from the last to 0",
        k,
        f,
        MOVE_BUDGET + 20_000,
        Some(290212),
    );
    let (k, f, a) = cost(&mut list, code(Kind::Axioms), 1, 0, F3_KEY);
    assert_eq!(a, 1);
    report(
        "F3 into Unchecked from assertion 1",
        k,
        f,
        F3_BUDGET,
        Some(314817),
    );
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_f3_goes_round_the_kinds_of_every_stage_and_the_arrows_walk_each_kind() {
    for stage in [PROP, 1, RING] {
        let mut list = Lister::on_rom(stage);
        list.open_on_rom();
        for _ in 0..4 {
            list.key(F3, Key::F3);
            list.skip_the_wait();
            list.settle();
            list.same_kind_position(&format!("stage {stage}"));
            // A press that follows a release before two idle points have passed is a key the shell
            // may still see held, and the rig holds it until the shell takes it.
            list.run.run(600_000);
        }
        assert_eq!(list.kind(), Kind::All);
    }
    for (stage, kinds) in [
        (PROP, vec![Kind::Theorems, Kind::Axioms]),
        (RING, vec![Kind::Axioms]),
    ] {
        for kind in kinds {
            let mut list = Lister::on_rom(stage);
            list.open_on_rom();
            list.go_to_kind(kind);
            let len = list.walk_with(&format!("stage {stage} {kind:?}"), true);
            println!("ROM, stage {stage} {kind:?}: {len} assertions walked");
            assert!(len > 0);
        }
    }
}
