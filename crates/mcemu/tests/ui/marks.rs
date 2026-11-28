//! Every routine that writes the frame sets the bits of the text rows it writes in ui_dirty, ORs
//! them in, and sets none for a call that returns before it writes.

use crate::masks::{dirty_at, frames};
use crate::support::*;

/// What the routine did to a frame it was given and to ui_dirty.
pub struct Effect {
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub dirty: u8,
}

pub fn effect(op: &Op, frame: Vec<u8>, seed: u8) -> Effect {
    let script = [
        Op::SetFrame(frame.clone()),
        Op::SetVar(dirty_at(), seed),
        op.clone(),
    ];
    let got = run(REAL, &script);
    Effect {
        before: frame,
        after: got.frame,
        dirty: got.vars[usize::from(dirty_at())],
    }
}

/// The text rows whose bytes differ.
pub fn rows_changed(before: &[u8], after: &[u8]) -> u8 {
    (0..8).fold(0, |mask, r| {
        let same = before[96 * r..96 * (r + 1)] == after[96 * r..96 * (r + 1)];
        mask | (u8::from(!same) << r)
    })
}

/// The bits of `rows` as a mask.
fn bits(rows: &[usize]) -> u8 {
    rows.iter().fold(0, |m, &r| m | (1 << r))
}

/// Outside the routine's rows, so that a store in place of an OR shows.
fn seed_outside(written: u8) -> u8 {
    0x5A & !written
}

/// How a routine writes: it changes bytes in every row it marks, or it ORs pixels that may be set
/// already, so that on a full frame no byte changes and the row is marked all the same.
#[derive(Clone, Copy, PartialEq)]
enum Writes {
    Flips,
    Ors,
}

/// Runs `op` expecting it to write `rows`. On a clear frame the rows whose bytes changed are the
/// rows written and ui_dirty is exactly them; seeded with other bits it keeps them and adds the
/// rows; on a frame of set pixels a routine that ORs changes nothing and still marks the rows, and
/// one that flips changes the same rows.
fn marks(what: &str, op: Op, rows: &[usize], writes: Writes) {
    let want = bits(rows);
    let clear = vec![0u8; FRAME_BYTES];
    let set = vec![0xFFu8; FRAME_BYTES];

    let e = effect(&op, clear.clone(), 0);
    let changed = rows_changed(&e.before, &e.after);
    assert_eq!(changed, want, "{what}: the rows whose bytes changed");
    assert_eq!(
        e.dirty, changed,
        "{what}: ui_dirty against the rows that changed"
    );

    let seed = seed_outside(want);
    let e = effect(&op, clear, seed);
    assert_eq!(
        e.dirty,
        seed | want,
        "{what}: ui_dirty keeps the bits it had"
    );
    assert!(
        seed != 0 || want == 0xFF,
        "{what}: the seed has bits to keep"
    );

    let e = effect(&op, set, 0);
    let changed = rows_changed(&e.before, &e.after);
    match writes {
        Writes::Flips => assert_eq!(changed, want, "{what}: flipping a full frame"),
        Writes::Ors => assert_eq!(
            changed, 0,
            "{what}: ORing onto a full frame changes no byte"
        ),
    }
    assert_eq!(
        e.dirty, want,
        "{what}: ui_dirty after writing onto set pixels"
    );
}

fn text(x: usize, row: usize, s: &str) -> Op {
    Op::Text {
        x,
        row,
        codes: table().encode(s),
    }
}

#[test]
fn ui_text_marks_the_row_it_draws_on_at_every_row() {
    for row in 0..8 {
        marks(
            &format!("row {row}"),
            text(3, row, "Ab"),
            &[row],
            Writes::Ors,
        );
        marks(
            &format!("clipped at the right edge, row {row}"),
            text(93, row, "WWW"),
            &[row],
            Writes::Ors,
        );
    }
}

#[test]
fn ui_text_marks_its_row_when_the_string_starts_right_of_the_screen_and_draws_nothing() {
    for x in [96, 97, 200, 4000] {
        for seed in [0x00, 0x5A] {
            let e = effect(&text(x, 4, "Ab"), vec![0; FRAME_BYTES], seed);
            assert_eq!(e.after, e.before, "x {x}: nothing is drawn");
            assert_eq!(
                e.dirty,
                seed | 0x10,
                "x {x}: the row is marked all the same"
            );
        }
    }
}

#[test]
fn ui_text_marks_nothing_for_an_empty_string_or_a_row_of_8_or_more() {
    for row in [0, 3, 7] {
        marks(
            &format!("empty, row {row}"),
            text(3, row, ""),
            &[],
            Writes::Ors,
        );
    }
    for row in [8, 9, 100, 255] {
        marks(&format!("row {row}"), text(3, row, "Ab"), &[], Writes::Ors);
    }
}

#[test]
fn the_right_and_centred_texts_mark_their_row() {
    for row in 0..8 {
        let codes = table().encode("Ab");
        let right = Op::Right {
            row,
            codes: codes.clone(),
        };
        marks(&format!("right, row {row}"), right, &[row], Writes::Ors);
        let centred = Op::Centered { row, codes };
        marks(&format!("centred, row {row}"), centred, &[row], Writes::Ors);
    }
    for row in [8, 200] {
        let codes = table().encode("Ab");
        let right = Op::Right {
            row,
            codes: codes.clone(),
        };
        marks(&format!("right, row {row}"), right, &[], Writes::Ors);
        marks(
            &format!("centred, row {row}"),
            Op::Centered { row, codes },
            &[],
            Writes::Ors,
        );
    }
}

fn invert(x: u8, y: u8, w: u8, h: u8) -> Op {
    Op::Invert { x, y, w, h }
}

#[test]
fn ui_invert_marks_the_rows_its_clipped_rectangle_touches() {
    type Case = ((u8, u8, u8, u8), Vec<usize>);
    let mut cases: Vec<Case> = vec![];
    for r in 0..8u8 {
        let top = usize::from(r);
        cases.push(((10, 8 * r, 20, 8), vec![top]));
        cases.push(((0, 8 * r, 96, 8), vec![top]));
        cases.push(((3, 8 * r + 7, 90, 1), vec![top]));
        cases.push(((10, 8 * r, 1, 1), vec![top]));
        if r < 7 {
            cases.push(((10, 8 * r + 4, 20, 8), vec![top, top + 1]));
            cases.push(((0, 8 * r + 7, 96, 2), vec![top, top + 1]));
        }
    }
    cases.extend([
        ((5, 4, 30, 17), vec![0, 1, 2]),
        ((0, 0, 96, 64), (0..8).collect()),
        ((10, 60, 20, 8), vec![7]),
        ((90, 16, 20, 8), vec![2]),
        ((95, 63, 1, 1), vec![7]),
        ((0, 63, 96, 200), vec![7]),
        ((90, 60, 100, 100), vec![7]),
        ((7, 1, 82, 62), (0..8).collect()),
        ((0, 0, 200, 200), (0..8).collect()),
    ]);
    let mut spans = [0usize; 9];
    for ((x, y, w, h), rows) in cases {
        spans[rows.len()] += 1;
        marks(
            &format!("invert x {x} y {y} w {w} h {h}"),
            invert(x, y, w, h),
            &rows,
            Writes::Flips,
        );
    }
    assert_eq!(
        spans[1..],
        [37, 14, 1, 0, 0, 0, 0, 3],
        "rectangles by rows touched"
    );
}

#[test]
fn ui_invert_marks_nothing_for_an_empty_or_off_screen_rectangle() {
    for (x, y, w, h) in [
        (10, 10, 0, 8),
        (10, 10, 20, 0),
        (96, 10, 20, 8),
        (200, 10, 20, 8),
        (10, 64, 20, 8),
        (10, 255, 20, 8),
        (0, 0, 0, 0),
    ] {
        marks(
            &format!("invert x {x} y {y} w {w} h {h}"),
            invert(x, y, w, h),
            &[],
            Writes::Flips,
        );
    }
}

#[test]
fn the_row_routines_mark_their_row() {
    for row in 0..8u8 {
        let r = usize::from(row);
        marks(
            &format!("invert_row {row}"),
            Op::InvertRow(row),
            &[r],
            Writes::Flips,
        );
        marks(&format!("marker {row}"), Op::Marker(row), &[r], Writes::Ors);
        let bar = Op::Bar {
            row,
            done: 1,
            total: 3,
        };
        marks(&format!("bar {row}"), bar, &[r], Writes::Ors);
        let full = Op::Bar {
            row,
            done: 0,
            total: 0,
        };
        marks(&format!("empty total, bar {row}"), full, &[r], Writes::Ors);
    }
    for row in [8, 9, 100, 255] {
        marks(
            &format!("invert_row {row}"),
            Op::InvertRow(row),
            &[],
            Writes::Flips,
        );
        marks(&format!("marker {row}"), Op::Marker(row), &[], Writes::Ors);
        let bar = Op::Bar {
            row,
            done: 1,
            total: 3,
        };
        marks(&format!("bar {row}"), bar, &[], Writes::Ors);
    }
}

fn enc(s: &str) -> Vec<u8> {
    table().encode(s)
}

#[test]
fn ui_title_marks_row_0_with_both_texts_one_or_none() {
    for (left, right) in [
        ("Libraries", "12/40"),
        ("", "12/40"),
        ("Libraries", ""),
        ("", ""),
    ] {
        let op = Op::Title {
            left: enc(left),
            right: enc(right),
        };
        marks(
            &format!("title {left:?} {right:?}"),
            op,
            &[0],
            Writes::Flips,
        );
    }
}

#[test]
fn ui_list_row_marks_its_row_for_a_text_on_either_side_or_a_selection() {
    for row in 0..8u8 {
        let r = usize::from(row);
        for (left, right, selected, rows) in [
            ("ax-1", "\u{2713}", false, vec![r]),
            ("ax-1", "", false, vec![r]),
            ("", "\u{2713}", false, vec![r]),
            ("ax-1", "\u{2713}", true, vec![r]),
            ("", "", true, vec![r]),
            ("", "", false, vec![]),
        ] {
            let op = Op::List {
                row,
                left: enc(left),
                right: enc(right),
                selected,
            };
            let writes = if selected { Writes::Flips } else { Writes::Ors };
            marks(
                &format!("list row {row} {left:?} {right:?} selected {selected}"),
                op,
                &rows,
                writes,
            );
        }
    }
    for row in [8, 9, 255] {
        for selected in [false, true] {
            let op = Op::List {
                row,
                left: enc("ax-1"),
                right: enc("\u{2713}"),
                selected,
            };
            marks(
                &format!("list row {row}, selected {selected}"),
                op,
                &[],
                Writes::Flips,
            );
        }
    }
}

fn keys(labels: [&str; 5]) -> Op {
    Op::Keys(labels.map(|l| key_table().encode(l)))
}

#[test]
fn ui_soft_keys_marks_row_7_for_any_label_and_nothing_for_five_blank_keys() {
    for labels in [
        ["Open", "Find", "Kind", "Try", ""],
        ["", "", "", "", "Up"],
        ["Open", "", "", "", ""],
    ] {
        marks(
            &format!("keys {labels:?}"),
            keys(labels),
            &[7],
            Writes::Flips,
        );
    }
    marks("five blank keys", keys([""; 5]), &[], Writes::Flips);
}

#[test]
fn ui_clear_stores_every_row_over_whatever_ui_dirty_held() {
    for seed in [0x00, 0x5A, 0xA5, 0xFF] {
        let (a, _) = frames();
        let e = effect(&Op::Clear, a, seed);
        assert_eq!(e.dirty, 0xFF, "seed {seed:02X}");
        assert!(e.after.iter().all(|&b| b == 0));
        assert_ne!(e.before, e.after);
    }
}
