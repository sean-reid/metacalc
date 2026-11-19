use crate::support::*;
use mcui::canvas::{Canvas, TEXT_WIDTH};

const FILL: &str = "The quick brown fox jumps over the lazy dog 0123456789 MWmw";

/// The model's frame for `bars`, each at its own row, over text and an inverted row when `under`.
fn model(bars: &[(usize, usize, usize)], under: bool) -> Vec<u8> {
    let mut c = Canvas::default();
    if under {
        for row in 0..8 {
            c.text(0, row, FILL);
        }
        c.invert_row(3);
    }
    for &(row, done, total) in bars {
        c.bar(row, done, total);
    }
    pack(&c.into_bitmap())
}

fn script(bars: &[(usize, usize, usize)], under: bool) -> Vec<Op> {
    let mut ops = Vec::new();
    if under {
        for row in 0..8 {
            ops.push(Op::Text {
                x: 0,
                row,
                codes: table().encode(FILL),
            });
        }
        ops.push(Op::InvertRow(3));
    }
    for &(row, done, total) in bars {
        ops.push(Op::Bar {
            row: byte(row),
            done: count(done),
            total: count(total),
        });
    }
    ops
}

/// Draws `bars` on the Z80 and the model and requires equal frames.
fn same(what: &str, bars: &[(usize, usize, usize)], under: bool) {
    let got = run(REAL, &script(bars, under));
    same_frame(what, &got.frame, &model(bars, under));
}

/// The pairs where the model's arithmetic changes behavior or rounds.
fn edges() -> Vec<(usize, usize)> {
    let inner = TEXT_WIDTH - 4;
    let mut pairs = vec![
        (0, 0),
        (5, 0),
        (65535, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (0, 2),
        (1, 2),
        (2, 2),
        (3, 2),
        (0, 3),
        (1, 3),
        (2, 3),
        (3, 3),
        (4, 3),
        (1, 65535),
        (32767, 65535),
        (32768, 65535),
        (65534, 65535),
        (65535, 65535),
        (65535, 3),
        (65535, 1),
        (0, 65535),
        (3325, 3326),
        (3326, 3326),
        (3327, 3326),
        (1663, 3326),
        (1664, 3326),
    ];
    for total in [inner - 1, inner, inner + 1, TEXT_WIDTH, TEXT_WIDTH + 1] {
        pairs.extend([(0, total), (1, total), (total - 1, total), (total, total)]);
        pairs.push((total + 1, total));
    }
    pairs
}

#[test]
fn every_row_matches_the_model_for_the_edge_cases_of_done_and_total() {
    for (done, total) in edges() {
        let bars: Vec<_> = (0..8).map(|row| (row, done, total)).collect();
        same(&format!("done {done} of {total}"), &bars, false);
    }
}

#[test]
fn a_bar_is_ored_over_text_and_an_inverted_row() {
    for (done, total) in edges() {
        let bars: Vec<_> = (0..8).map(|row| (row, done, total)).collect();
        same(&format!("over text, done {done} of {total}"), &bars, true);
    }
}

/// Every done of 0 to `total`, eight at a time on the eight rows of one frame.
fn sweep(total: usize, step: usize) {
    let mut dones: Vec<usize> = (0..=total).step_by(step).collect();
    dones.extend([total, total + 1, total + 7]);
    dones.retain(|&done| done <= usize::from(u16::MAX));
    for chunk in dones.chunks(8) {
        let bars: Vec<_> = chunk
            .iter()
            .enumerate()
            .map(|(row, &done)| (row, done, total))
            .collect();
        same(&format!("{chunk:?} of {total}"), &bars, false);
    }
}

#[test]
fn every_done_of_small_totals_and_the_widths_around_the_bar() {
    let inner = TEXT_WIDTH - 4;
    for total in [
        1,
        2,
        3,
        inner - 1,
        inner,
        inner + 1,
        TEXT_WIDTH,
        TEXT_WIDTH + 1,
    ] {
        sweep(total, 1);
    }
}

#[test]
fn every_done_of_779() {
    sweep(779, 1);
}

#[test]
fn done_sampled_across_3326_and_the_largest_totals() {
    sweep(3326, 7);
    sweep(3326, 90);
    sweep(65535, 331);
    sweep(40000, 997);
}

#[test]
fn a_row_of_8_or_more_draws_nothing() {
    let rows = [8usize, 9, 100, 255];
    for under in [false, true] {
        let bars: Vec<_> = rows.iter().map(|&row| (row, 50, 100)).collect();
        let want = model(&bars, under);
        assert_eq!(
            want,
            model(&[], under),
            "the model draws a bar on row 8 or more"
        );
        let got = run(REAL, &script(&bars, under));
        same_frame("rows past 7", &got.frame, &want);
    }
}

#[test]
fn the_last_row_stays_in_the_frame_and_the_stack_stays_shallow() {
    let ops = script(&[(7, 3326, 3326), (7, 1, 3)], true);
    let got = run(REAL, &ops);
    same_frame(
        "last row",
        &got.frame,
        &model(&[(7, 3326, 3326), (7, 1, 3)], true),
    );
    let depth = stack_depth(REAL, &script(&[(7, 3325, 3326)], false));
    assert!(depth <= 8, "ui_bar reached {depth} bytes of stack");
}

#[test]
fn the_bar_follows_a_moved_frame() {
    let bars: Vec<_> = (0..8).map(|row| (row, row * 11 + 1, 77)).collect();
    let got = run(MOVED, &script(&bars, true));
    same_frame("moved", &got.frame, &model(&bars, true));
}
