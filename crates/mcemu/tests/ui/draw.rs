use crate::support::*;
use mcglyph::Font;
use mcui::canvas::{Canvas, text_width};

#[test]
fn every_glyph_alone_at_all_eight_bit_alignments() {
    let font = Font::builtin();
    for (name, code) in table().glyphs() {
        if code == 1 {
            continue;
        }
        let glyph = font.glyph(name).unwrap();
        let mut canvas = Canvas::default();
        let mut script = Vec::new();
        for r in 0..8usize {
            canvas.blit(glyph, 9 * r, r);
            script.push(Op::Text {
                x: 9 * r,
                row: r,
                codes: vec![code, 0],
            });
        }
        let got = run(REAL, &script);
        same_frame(name, &got.frame, &pack(&canvas.into_bitmap()));
    }
}

#[test]
fn strings_mixing_spaces_and_marks() {
    let texts = [
        "a b  c",
        "\u{2713} ok \u{2717} no",
        "wait\u{2026} ~ done",
        "~~ ~ ~",
        " leading and trailing ",
        "Libraries 1.2 (3/4)",
        "[x] <y> = z + 1 - 2 * 3 / 4",
        "a'b,c:d!e?f_g",
        "\u{00e9} unknown",
        "    ",
    ];
    for (i, text) in texts.iter().enumerate() {
        for x in [0, 1, 3, 7, 8, 13, 31] {
            agree(&format!("{text:?} at {x}"), |c| {
                c.text(x, i % 8, text);
            });
        }
    }
}

#[test]
fn text_at_the_right_edge_and_past_it() {
    let texts = [
        "a",
        "metacalc",
        "Metamath verifier and proof editor",
        "WWWW",
        "\u{2026}",
    ];
    for text in texts {
        for x in (78..=100).chain([120, 255, 256, 300]) {
            agree(&format!("{text:?} at {x}"), |c| {
                c.text(x, 3, text);
            });
        }
    }
}

#[test]
fn right_aligned_and_centred_text_of_odd_and_even_widths() {
    let texts = [
        "",
        "a",
        "ab",
        "abc",
        "abcd",
        "abcde",
        "Hello",
        "a b",
        "a bc",
        "metacalc 0.1.0",
        "This line is much wider than the whole screen can show at once, for sure.",
    ];
    for text in texts {
        for row in [0, 5] {
            agree(&format!("right {text:?}"), |c| {
                c.text_right(row, text);
            });
            agree(&format!("centred {text:?}"), |c| c.text_centered(row, text));
        }
    }
}

#[test]
fn a_string_is_measured_as_the_model_measures_it() {
    let texts = [
        "",
        "a",
        " ",
        "  ",
        "ab",
        "a b",
        "\u{2713}\u{2717}\u{2026}~",
        "metacalc 0.1.0",
        "This line is much wider than the whole screen can show at once, for sure.",
    ];
    let calls: Vec<Op> = texts.iter().map(|t| Op::Width(table().encode(t))).collect();
    let got = run(REAL, &calls);
    for (text, width) in texts.iter().zip(&got.results) {
        assert_eq!(usize::from(*width), text_width(text), "{text:?}");
    }
}

#[test]
fn text_returns_the_x_after_it() {
    let texts = [
        "",
        "a",
        "a b",
        "Hello, world",
        "wider than the screen by a long way, really",
    ];
    let mut script = Vec::new();
    let mut want = Vec::new();
    for (i, text) in texts.iter().enumerate() {
        for x in [0usize, 5, 90, 200] {
            script.push(Op::Text {
                x,
                row: i % 8,
                codes: table().encode(text),
            });
            want.push(x + text_width(text));
        }
    }
    let got = run(REAL, &script);
    let got: Vec<usize> = got.results.iter().map(|&r| usize::from(r)).collect();
    assert_eq!(got, want);
}

#[test]
fn drawing_text_does_not_touch_other_rows_or_the_ram_around_the_frame() {
    let mut script = vec![Op::Text {
        x: 0,
        row: 7,
        codes: table().encode("The last row, running off the right edge"),
    }];
    script.push(Op::Text {
        x: 90,
        row: 0,
        codes: table().encode("MMMM"),
    });
    let got = run(REAL, &script);
    let mut canvas = Canvas::default();
    canvas.text(0, 7, "The last row, running off the right edge");
    canvas.text(90, 0, "MMMM");
    same_frame("edges", &got.frame, &pack(&canvas.into_bitmap()));
}
