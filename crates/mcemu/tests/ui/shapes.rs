use crate::support::*;
use mcui::canvas::Canvas;

const FILL: &str = "The quick brown fox jumps over the lazy dog 0123456789 MWmw";

fn filled(c: &mut Canvas) {
    for row in 0..8 {
        c.text(0, row, FILL);
    }
}

#[test]
fn inverting_rectangles_at_every_alignment_of_both_edges() {
    let widths = [0usize, 1, 2, 7, 8, 9, 15, 16, 17, 33, 96, 200];
    for x in (0..=17usize).chain(80..96).chain([96, 100]) {
        agree(&format!("invert at x {x}"), |c| {
            filled(c);
            for (i, &w) in widths.iter().enumerate() {
                c.invert(x, 3 * i, w, 2);
            }
        });
    }
}

#[test]
fn inverting_up_to_the_edge_and_the_full_width() {
    for x in 0..96usize {
        agree(&format!("to the edge from {x}"), |c| {
            filled(c);
            c.invert(x, 5, 96 - x, 3);
            c.invert(x, 40, 96, 1);
        });
    }
}

#[test]
fn inverting_clips_at_the_bottom_and_ignores_empty_or_outside_rectangles() {
    agree("vertical clip", |c| {
        filled(c);
        c.invert(3, 60, 20, 10);
        c.invert(3, 63, 20, 1);
        c.invert(3, 64, 20, 5);
        c.invert(3, 200, 20, 5);
        c.invert(3, 10, 20, 0);
        c.invert(0, 0, 96, 64);
        c.invert(5, 5, 0, 0);
    });
}

#[test]
fn inverting_twice_restores_the_frame() {
    agree("twice", |c| {
        filled(c);
        c.invert(13, 9, 41, 17);
        c.invert(13, 9, 41, 17);
    });
}

#[test]
fn inverting_each_row() {
    for row in 0..8 {
        agree(&format!("invert row {row}"), |c| {
            filled(c);
            c.invert_row(row);
        });
    }
}

#[test]
fn the_marker_on_each_row() {
    for row in 0..8 {
        agree(&format!("marker {row}"), |c| {
            filled(c);
            c.marker(row);
        });
    }
}

#[test]
fn title_bars_with_short_and_long_texts() {
    let pairs = [
        ("Libraries", "metacalc"),
        ("About", ""),
        ("", ""),
        ("", "right only"),
        ("A", "B"),
        ("A very long theorem name that cannot fit", "ok"),
        ("syl", "\u{2713} 12/40"),
        ("Fits exactly or nearly so here", "1/2"),
        (
            "x",
            "A very long right text that takes the whole bar and more",
        ),
    ];
    for (left, right) in pairs {
        agree(&format!("title {left:?} {right:?}"), |c| {
            c.text(0, 3, FILL);
            c.title(left, right);
        });
    }
    agree("title with tail", |c| {
        c.title_with_tail("set.mm library name", " thms", "12/4000")
    });
}

#[test]
fn soft_key_rows_with_all_some_and_no_labels() {
    const LABELS: [&str; 17] = [
        "See", "Vrfy", "Info", "Find", "Kind", "Try", "Fold", "Dist", "Why", "Up", "Thm", "Aply",
        "Hyp", "Auto", "Undo", "Tree", "Stop",
    ];
    for start in 0..LABELS.len() {
        let five: [&str; 5] = std::array::from_fn(|i| LABELS[(start + i) % LABELS.len()]);
        agree(&format!("keys {five:?}"), |c| c.soft_keys(five));
    }
    for mask in 0..32u32 {
        let labels: [&str; 5] = std::array::from_fn(|i| {
            if mask >> i & 1 == 1 {
                LABELS[(i * 3 + 1) % 17]
            } else {
                ""
            }
        });
        agree(&format!("keys mask {mask}"), |c| {
            c.text(0, 7, "under");
            c.soft_keys(labels);
        });
    }
    agree("keys cut by the model", |c| {
        c.soft_keys(["Overlong", "WWWW", "", "", "i"])
    });
}

#[test]
fn list_rows_selected_and_not() {
    let rows = [
        ("ax-1", "\u{2713}"),
        ("a longer name than most", "12"),
        ("", ""),
        ("name only", ""),
        ("", "right only"),
        ("a name that must be cut short to fit beside", "\u{2717} 3"),
    ];
    for (left, right) in rows {
        for selected in [false, true] {
            for row in [1, 4, 6] {
                agree(&format!("list {left:?} {right:?} {selected} {row}"), |c| {
                    c.list_row(row, left, right, selected);
                });
            }
        }
    }
}
