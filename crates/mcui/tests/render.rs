use mcglyph::{Bitmap, CHECK, ELLIPSIS, Font};
use mcui::canvas::{Canvas, HEIGHT, ROW, WIDTH, text_width, truncate};

fn ink_columns(frame: &Bitmap, rows: std::ops::Range<usize>) -> Vec<usize> {
    (0..WIDTH)
        .filter(|&x| rows.clone().any(|y| frame.get(x, y)))
        .collect()
}

#[test]
fn text_that_fits_is_not_cut() {
    assert_eq!(truncate("ringcom", 96), "ringcom");
    assert_eq!(truncate("", 10), "");
}

#[test]
fn text_that_does_not_fit_is_cut_to_fit_with_an_ellipsis() {
    let long = "mulgrpbasedefinition";
    for limit in 12..70 {
        let cut = truncate(long, limit);
        assert!(text_width(&cut) <= limit, "{cut} at {limit}");
        assert!(cut.ends_with(ELLIPSIS));
        assert!(long.starts_with(cut.trim_end_matches(ELLIPSIS)));
    }
    let wide = truncate(long, 40);
    let wider = truncate(long, 41);
    assert!(wider.chars().count() >= wide.chars().count());
}

#[test]
fn truncation_is_exact_at_the_limit() {
    let width = text_width("ringcom");
    assert_eq!(truncate("ringcom", width), "ringcom");
    assert!(truncate("ringcom", width - 1).ends_with(ELLIPSIS));
}

#[test]
fn the_title_bar_is_white_text_on_black_with_the_status_at_the_right_margin() {
    let mut canvas = Canvas::default();
    canvas.title("ring", "12/34");
    let frame = canvas.into_bitmap();
    let font = Font::builtin();
    let (left, right) = (font.text("ring"), font.text("12/34"));
    let right_x = WIDTH - 1 - right.width();
    for y in 0..ROW {
        for x in 0..WIDTH {
            let ink = left_ink(&left, x, y, 1) || left_ink(&right, x, y, right_x);
            assert_eq!(frame.get(x, y), !ink, "pixel {x},{y}");
        }
    }
    for y in ROW..HEIGHT {
        assert!((0..WIDTH).all(|x| !frame.get(x, y)));
    }
}

fn left_ink(text: &Bitmap, x: usize, y: usize, at: usize) -> bool {
    x >= at && y >= 1 && text.get(x - at, y - 1)
}

#[test]
fn a_long_title_is_cut_short_of_its_status() {
    let mut canvas = Canvas::default();
    canvas.title("a-label-far-too-long-for-the-bar", "99/99");
    let frame = canvas.into_bitmap();
    let right = Font::builtin().text("99/99").width();
    let right_start = WIDTH - 1 - right;
    let gap = (right_start - 3..right_start).all(|x| (1..ROW).all(|y| frame.get(x, y)));
    assert!(gap, "three blank columns precede the status");
}

#[test]
fn every_used_soft_key_is_its_own_inverted_box_with_the_label_centred_inside() {
    let labels = [
        ["Open", "Run", "Info", "", ""],
        ["Open", "Find", "Kind", "Try", ""],
        ["Fold", "Dist", "Why", "Try", ""],
        ["Fold", "Up", "Thm", "", ""],
        ["Aply", "Hyp", "Auto", "Undo", "Tree"],
        ["Use", "Hyp", "Auto", "Undo", "Tree"],
        ["Go", "", "", "Undo", "Tree"],
        ["Stop", "", "", "", ""],
    ];
    let font = Font::keys();
    let slots = [(0, 18), (19, 37), (38, 57), (58, 76), (77, 95)];
    for set in labels {
        let mut canvas = Canvas::default();
        canvas.soft_keys(set);
        let frame = canvas.into_bitmap();
        for y in 0..56 {
            assert!((0..WIDTH).all(|x| !frame.get(x, y)));
        }
        for (i, label) in set.iter().enumerate() {
            let (from, to) = slots[i];
            for y in 56..64 {
                for x in from..=to {
                    let inside = x >= from && x < to;
                    let ink = if label.is_empty() || !inside || y == 56 {
                        false
                    } else {
                        let text = font.text(label);
                        let slack = (to - from) - text.width();
                        assert!(slack >= 2, "{label} touches its box");
                        let left = from + slack.div_ceil(2);
                        let glyph = x >= left && y >= 57 && text.get(x - left, y - 57);
                        !glyph
                    };
                    assert_eq!(frame.get(x, y), ink, "{label} at {x},{y}");
                }
            }
        }
    }
}

#[test]
fn a_list_row_has_its_mark_at_the_right_margin_and_a_selected_row_is_inverted() {
    let mut plain = Canvas::default();
    plain.list_row(2, "ringcom", &CHECK.to_string(), false);
    let plain = plain.into_bitmap();
    let columns = ink_columns(&plain, 2 * ROW..3 * ROW);
    assert_eq!(columns[0], 1);
    assert_eq!(*columns.last().unwrap(), WIDTH - 2);

    let mut chosen = Canvas::default();
    chosen.list_row(2, "ringcom", &CHECK.to_string(), true);
    let chosen = chosen.into_bitmap();
    for y in 2 * ROW..3 * ROW {
        for x in 0..WIDTH {
            let flipped = y > 2 * ROW;
            assert_eq!(chosen.get(x, y), plain.get(x, y) ^ flipped, "{x},{y}");
        }
    }
    for y in (0..2 * ROW).chain(3 * ROW..HEIGHT) {
        assert!((0..WIDTH).all(|x| !chosen.get(x, y)));
    }
}

fn row_is_clear(frame: &Bitmap, y: usize) -> bool {
    (0..WIDTH).all(|x| !frame.get(x, y))
}

#[test]
fn a_highlighted_first_row_leaves_a_clear_line_under_the_title_bar() {
    let mut canvas = Canvas::default();
    canvas.title("", "");
    canvas.list_row(1, "", "", true);
    let frame = canvas.into_bitmap();
    assert!((0..WIDTH).all(|x| frame.get(x, ROW - 1)));
    assert!(row_is_clear(&frame, ROW));
    assert!((0..WIDTH).all(|x| frame.get(x, ROW + 1)));
}

#[test]
fn a_highlighted_sixth_row_leaves_a_clear_line_above_the_key_boxes() {
    let mut canvas = Canvas::default();
    canvas.list_row(6, "", "", true);
    canvas.soft_keys(["ace", "oxen", "sum", "zeus", "van"]);
    let frame = canvas.into_bitmap();
    assert!((0..WIDTH).all(|x| frame.get(x, 55)));
    assert!(row_is_clear(&frame, 56));
    for x in 0..WIDTH {
        let in_box = (0..5).any(|i| {
            let (from, to) = ((WIDTH * i + 2) / 5, (WIDTH * (i + 1) + 2) / 5 - 1);
            (from..to).contains(&x)
        });
        assert_eq!(frame.get(x, 57), in_box, "column {x}");
    }
}

#[test]
fn a_descender_does_not_touch_the_highlight_below_it() {
    let mut canvas = Canvas::default();
    canvas.list_row(1, "gypsy", "", false);
    canvas.list_row(2, "", "", true);
    let frame = canvas.into_bitmap();
    assert!((0..WIDTH).any(|x| frame.get(x, 2 * ROW - 1)));
    assert!(row_is_clear(&frame, 2 * ROW));
    assert!((0..WIDTH).all(|x| frame.get(x, 2 * ROW + 1)));
}

#[test]
fn a_long_label_is_cut_before_its_mark() {
    let mut canvas = Canvas::default();
    canvas.list_row(1, "mulgrpbasedefinitionlong", &CHECK.to_string(), false);
    let frame = canvas.into_bitmap();
    let mark = Font::builtin().text(&CHECK.to_string()).width();
    let mark_start = WIDTH - 1 - mark;
    let columns = ink_columns(&frame, ROW..2 * ROW);
    assert!(
        columns
            .iter()
            .filter(|&&x| x < mark_start)
            .all(|&x| x < mark_start - 2)
    );
}

#[test]
fn text_wider_than_the_bar_draws_without_arithmetic_trouble() {
    let long = "a-status-far-wider-than-the-whole-bar-of-ninety-six-pixels";
    let mut canvas = Canvas::default();
    canvas.title("ring", long);
    canvas.list_row(1, "ring", long, false);
    canvas.text_right(2, long);
    let _ = canvas.into_bitmap();
}

#[test]
fn the_tail_of_a_long_prefix_keeps_the_newest_characters() {
    use mcui::canvas::{tail, wrap};
    assert_eq!(tail("abc", 50), "abc");
    let cut = tail("abcdefghijklmnopqrstuvwxyz", 40);
    assert!(text_width(&cut) <= 40);
    assert!(cut.starts_with(ELLIPSIS));
    assert!("abcdefghijklmnopqrstuvwxyz".ends_with(cut.trim_start_matches(ELLIPSIS)));
    assert_eq!(tail("abcdefghij", 4), ELLIPSIS.to_string());
    assert_eq!(
        wrap("the library does not export", 94),
        ["the library does not", "export"]
    );
    assert_eq!(wrap("", 94), Vec::<String>::new());
    assert!(
        wrap("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz", 40)
            .iter()
            .all(|l| text_width(l) <= 40)
    );
}
