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

/// Every glyph of both fonts alone at all eight alignments on the first and last text row, in
/// the middle of the frame and at the last bytes.
#[test]
fn every_glyph_of_both_fonts_at_every_alignment_on_the_first_and_last_row() {
    for (font, table, keys) in [
        (Font::builtin(), table(), false),
        (Font::keys(), key_table(), true),
    ] {
        let flat: Vec<u8> = table.slots().iter().flatten().copied().collect();
        for (name, code) in table.glyphs() {
            if code == 1 {
                continue;
            }
            let glyph = font.glyph(name).unwrap();
            for row in [0usize, 7] {
                for s in 0..8usize {
                    let mut canvas = Canvas::default();
                    let mut script = Vec::new();
                    if keys {
                        script.push(Op::SetFont(flat.clone()));
                    }
                    for x in [s, 24 + s, 48 + s, 72 + s, 80 + s, 88 + s] {
                        canvas.blit(glyph, x, row);
                        script.push(Op::Text {
                            x,
                            row,
                            codes: vec![code, 0],
                        });
                    }
                    let got = run(REAL, &script);
                    same_frame(
                        &format!("{name} row {row} shift {s}"),
                        &got.frame,
                        &pack(&canvas.into_bitmap()),
                    );
                }
            }
        }
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
fn a_long_mixed_string_at_every_x_on_every_row() {
    let text = "Wgj, (pqy) MW 0123 [i|l] {a~b} \u{2713}\u{2717}\u{2026} Quick brown fox; jumps!";
    let codes = table().encode(text);
    for row in 0..=8usize {
        for x in 0..=100usize {
            let mut canvas = Canvas::default();
            let want = x + canvas.text(x, row, text);
            let got = run(
                REAL,
                &[Op::Text {
                    x,
                    row,
                    codes: codes.clone(),
                }],
            );
            same_frame(
                &format!("row {row} at {x}"),
                &got.frame,
                &pack(&canvas.into_bitmap()),
            );
            assert_eq!(usize::from(got.results[0]), want, "row {row} at {x}");
        }
    }
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

/// Runs of one glyph, the last of which ends exactly at column 95, one short of it and one past
/// it, in both fonts on the first and last text row.
#[test]
fn runs_of_every_glyph_ending_at_the_right_edge() {
    for (font, table, keys) in [
        (Font::builtin(), table(), false),
        (Font::keys(), key_table(), true),
    ] {
        let flat: Vec<u8> = table.slots().iter().flatten().copied().collect();
        for (name, code) in table.glyphs() {
            let glyph = font.glyph(name).unwrap();
            let w = glyph.width();
            for row in [0usize, 7] {
                for n in 1..=3usize {
                    let run = n * w + (n - 1);
                    for start in [95 - run, 96 - run, 97 - run] {
                        let mut canvas = Canvas::default();
                        for k in 0..n {
                            canvas.blit(glyph, start + k * (w + 1), row);
                        }
                        let mut script = Vec::new();
                        if keys {
                            script.push(Op::SetFont(flat.clone()));
                        }
                        script.push(Op::Text {
                            x: start,
                            row,
                            codes: [vec![code; n], vec![0]].concat(),
                        });
                        let got = run_ops(&script);
                        same_frame(
                            &format!("{name} x{n} from {start} on row {row}"),
                            &got.frame,
                            &pack(&canvas.into_bitmap()),
                        );
                    }
                }
            }
        }
    }
}

fn run_ops(script: &[Op]) -> Outcome {
    run(REAL, script)
}

/// Rows of the narrowest glyphs, long enough to run off the edge, from every bit alignment on
/// every row.
#[test]
fn rows_of_narrow_glyphs_from_every_alignment() {
    for unit in ["i", "l", ".", "il", ".il", "i.", "|i"] {
        let text = unit.repeat(60);
        for row in 0..8usize {
            for x in 0..=9usize {
                agree(&format!("{unit:?} x60 at {x} on row {row}"), |c| {
                    c.text(x, row, &text);
                });
            }
        }
    }
}

#[test]
fn text_reaches_at_most_eight_bytes_of_stack() {
    let t = table();
    let narrow = "i".repeat(60);
    let texts = [
        "",
        "a",
        narrow.as_str(),
        "Wgj, (pqy) MW 0123 [i|l] {a~b} \u{2713}\u{2717}\u{2026} Quick brown fox; jumps!",
    ];
    for text in texts {
        for (x, row) in [(0, 0), (1, 3), (7, 7), (88, 7), (95, 7), (200, 2), (1, 9)] {
            let depth = stack_depth(
                REAL,
                &[Op::Text {
                    x,
                    row,
                    codes: t.encode(text),
                }],
            );
            assert!(
                depth <= 8,
                "{text:?} at {x} on row {row} reached {depth} bytes of stack"
            );
        }
    }
}
