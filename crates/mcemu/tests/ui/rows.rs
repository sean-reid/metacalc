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

#[derive(Default, Debug)]
struct Counts {
    cleared: usize,
    shifted: usize,
    nothing: usize,
}

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
            changed & !mask,
            0,
            "{what}: a row outside the range changed"
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
            if range.0 == range.1 {
                counts.nothing += 1;
            } else {
                counts.cleared += 1;
            }
        }
    }
    assert_eq!(counts.cleared + counts.nothing, 90);
    assert_eq!((counts.cleared, counts.nothing), (72, 18));
}

#[test]
fn shifting_rows_matches_the_host_canvas_for_every_first_row_count_and_direction() {
    let mut counts = Counts::default();
    for dir in [0u8, 1, 0x80] {
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
                if range.0 == range.1 {
                    counts.nothing += 1;
                } else if range.1 - range.0 == 1 {
                    counts.cleared += 1;
                } else {
                    counts.shifted += 1;
                }
            }
        }
    }
    // per direction: 8 first rows x 8 counts mark rows; of those the ranges one row wide clear
    assert_eq!(counts.nothing, 3 * 18);
    assert!(counts.cleared >= 3 * 8 * 2, "{counts:?}");
    assert!(counts.shifted >= 3 * 8 * 5, "{counts:?}");
    assert_eq!(counts.nothing + counts.cleared + counts.shifted, 3 * 90);
}
