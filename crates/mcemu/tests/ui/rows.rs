//! ui_clear_rows and ui_shift_rows against the host canvas, with the rows they mark.

use crate::marks::{effect, rows_changed};
use crate::support::*;
use mcglyph::Bitmap;
use mcui::canvas::Canvas;

fn bitmap_of(frame: &[u8]) -> Bitmap {
    let px = crate::masks::pixels(frame);
    let mut b = Bitmap::new(96, 64);
    for (i, &on) in px.iter().enumerate() {
        b.set(i % 96, i / 96, on);
    }
    b
}

/// How many calls of a sweep did nothing, changed one row, were cut at row 7, or changed a range
/// of two rows or more inside the screen.
#[derive(Default, Debug, PartialEq)]
struct Counts {
    nothing: usize,
    one_row: usize,
    cut: usize,
    range: usize,
}

impl Counts {
    fn add(&mut self, first: usize, count: usize) {
        if first >= 8 || count == 0 {
            self.nothing += 1;
        } else if count == 1 {
            self.one_row += 1;
        } else if first + count > 8 {
            self.cut += 1;
        } else {
            self.range += 1;
        }
    }
}

const EVERY_CALL: Counts = Counts {
    nothing: 18,
    one_row: 8,
    cut: 36,
    range: 28,
};

fn check(op: Op, model: impl Fn(&mut Canvas), range: (usize, usize), what: &str) {
    let (a, _) = crate::masks::frames();
    let mut canvas = Canvas::with_bitmap(bitmap_of(&a));
    model(&mut canvas);
    let want = pack(&canvas.into_bitmap());
    let (first, end) = range;
    let mask = (first..end).fold(0u8, |m, r| m | (1 << r));
    for seed in [0x00u8, 0x5A & !mask] {
        let e = effect(&op, a.clone(), seed);
        same_frame(what, &e.after, &want);
        assert_eq!(
            e.dirty,
            seed | mask,
            "{what}: ui_dirty with seed {seed:02X}"
        );
        let changed = rows_changed(&e.before, &e.after);
        assert_eq!(
            changed, mask,
            "{what}: the rows that changed against the rows it marks, seed {seed:02X}"
        );
    }
}

#[test]
fn clearing_rows_matches_the_host_canvas_for_every_first_row_and_count() {
    let mut counts = Counts::default();
    for first in 0..=8usize {
        for count in 0..=9usize {
            let end = (first + count).min(8);
            let range = if first >= 8 || count == 0 {
                (0, 0)
            } else {
                (first, end)
            };
            let op = Op::ClearRows {
                first: byte(first),
                count: byte(count),
            };
            check(
                op,
                |c| c.clear_rows(first, count),
                range,
                &format!("clear_rows({first}, {count})"),
            );
            counts.add(first, count);
        }
    }
    assert_eq!(counts, EVERY_CALL);
}

#[test]
fn shifting_rows_matches_the_host_canvas_for_every_first_row_count_and_direction() {
    for dir in [0u8, 1, 0x80] {
        let mut counts = Counts::default();
        for first in 0..=8usize {
            for count in 0..=9usize {
                let end = (first + count).min(8);
                let range = if first >= 8 || count == 0 {
                    (0, 0)
                } else {
                    (first, end)
                };
                let op = Op::ShiftRows {
                    first: byte(first),
                    count: byte(count),
                    dir,
                };
                check(
                    op,
                    |c| c.shift_rows(first, count, dir == 0),
                    range,
                    &format!("shift_rows({first}, {count}, {dir})"),
                );
                counts.add(first, count);
            }
        }
        assert_eq!(counts, EVERY_CALL, "direction {dir}");
    }
}
