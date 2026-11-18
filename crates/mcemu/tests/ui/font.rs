use crate::support::*;
use mcemu::Speed;
use mcglyph::z80::{SLOT, Table};
use mcglyph::{Bitmap, Font};
use mcui::canvas::Canvas;

const WIDE: &str = include_str!("../../../mcglyph/tests/fixtures/wide.font");
const TALL: &str = include_str!("../../../mcglyph/tests/fixtures/tall.font");

fn wide_font() -> Font {
    Font::parse(WIDE, "").unwrap()
}

fn flat(table: &Table) -> Vec<u8> {
    table.slots().iter().flatten().copied().collect()
}

/// `text` in `font` as the model lays it out, ORed in at `x` on `row`.
fn model_text(frame: &mut Bitmap, font: &Font, x: usize, row: usize, text: &str) {
    frame.blit(&font.text(text), x, row * 8 + 1);
}

#[test]
fn the_assembled_image_holds_both_generated_tables() {
    for (name, table, size) in [
        ("ui_font", Table::builtin(), 1328),
        ("ui_key_font", Table::keys(), 504),
    ] {
        let at = usize::from(label(REAL, name)) - 0x4000;
        let bytes = flat(&table);
        assert_eq!(&REAL.code[at..at + bytes.len()], bytes, "{name}");
        assert_eq!(bytes.len(), size, "{name}");
        let glyphs = label(REAL, &format!("{name}_GLYPHS"));
        assert_eq!(usize::from(glyphs) * SLOT, bytes.len(), "{name}");
    }
    let (main, keys) = (Table::builtin(), Table::keys());
    let equate = |name: &str, c: char| label(REAL, &format!("{name}_CH_{:X}", c as u32));
    for c in ['A', 'a', 'z', '0', ' '] {
        assert_eq!(equate("ui_font", c), u16::from(main.code(c)));
        assert_eq!(equate("ui_key_font", c), u16::from(keys.code(c)));
    }
}

#[test]
fn glyphs_nine_wide_draw_at_every_alignment_and_clip_at_the_edge() {
    let font = wide_font();
    let table = Table::new(&font).unwrap();
    let texts = [
        "a",
        "aa",
        "a b a",
        "ab.c?",
        "cacbc",
        "a a a a a a a a a a a a",
    ];
    for text in texts {
        for x in (0..=100usize).chain([250, 255, 256]) {
            let mut model = Bitmap::new(96, 64);
            model_text(&mut model, &font, x, 2, text);
            let got = run(
                REAL,
                &[
                    Op::SetFont(flat(&table)),
                    Op::Text {
                        x,
                        row: 2,
                        codes: table.encode(text),
                    },
                    Op::Width(table.encode(text)),
                ],
            );
            same_frame(&format!("{text:?} at {x}"), &got.frame, &pack(&model));
            assert_eq!(
                usize::from(got.results[1]),
                font.text_width(text),
                "{text:?}"
            );
        }
    }
}

#[test]
fn glyphs_nine_wide_on_the_last_row_at_the_last_frame_byte() {
    let font = wide_font();
    let table = Table::new(&font).unwrap();
    for text in ["a", "ab", "a a", "cacbc"] {
        for x in 80..=100usize {
            let mut model = Bitmap::new(96, 64);
            model_text(&mut model, &font, x, 7, text);
            let got = run(
                REAL,
                &[
                    Op::SetFont(flat(&table)),
                    Op::Text {
                        x,
                        row: 7,
                        codes: table.encode(text),
                    },
                ],
            );
            same_frame(
                &format!("{text:?} at {x} on row 7"),
                &got.frame,
                &pack(&model),
            );
        }
    }
}

/// Glyphs six to nine wide with ink in rows 5 and 6: at shifts 1 to 3 the six, seven and eight
/// wide ones take the two-byte path with the descender rows, `ui_tv1` to `ui_tv3`. The test
/// also covers the other shifts and the nine wide glyph on the way.
#[test]
fn flagged_glyphs_six_to_nine_wide_draw_at_every_alignment_on_three_rows() {
    let font = Font::parse(TALL, "").unwrap();
    let table = Table::new(&font).unwrap();
    let cases: [(&str, std::ops::RangeInclusive<usize>); 7] = [
        ("a", 0..=100),
        ("b", 0..=100),
        ("c", 0..=100),
        ("d", 0..=100),
        ("ab", 80..=100),
        ("cba", 80..=100),
        ("dcba", 80..=100),
    ];
    for (text, xs) in cases {
        for row in [0, 3, 7] {
            for x in xs.clone() {
                let mut model = Bitmap::new(96, 64);
                model_text(&mut model, &font, x, row, text);
                let got = run(
                    REAL,
                    &[
                        Op::SetFont(flat(&table)),
                        Op::Text {
                            x,
                            row,
                            codes: table.encode(text),
                        },
                    ],
                );
                same_frame(
                    &format!("{text:?} at {x} on row {row}"),
                    &got.frame,
                    &pack(&model),
                );
            }
        }
    }
}

/// Runs `text` at `x` on row 0 with a HALT written over the first byte of `entry` in a copy of
/// the image, and returns where the run stopped.
fn stop_with_halt_at(entry: &str, text: &str, x: usize) -> u16 {
    let font = Font::parse(TALL, "").unwrap();
    let table = Table::new(&font).unwrap();
    let mut code = REAL.code.to_vec();
    code[usize::from(label(REAL, entry)) - 0x4000] = 0x76;
    run_code(
        REAL,
        &code,
        (Speed::Mhz6, true),
        &[
            Op::SetFont(flat(&table)),
            Op::Text {
                x,
                row: 0,
                codes: table.encode(text),
            },
        ],
        true,
    )
    .pc
}

#[test]
fn the_descender_entries_run_for_the_glyphs_and_shifts_that_reach_them() {
    for (entry, reaching, other) in [("ui_tv1", 81, 82), ("ui_tv2", 82, 81), ("ui_tv3", 83, 81)] {
        let at = label(REAL, entry);
        assert_eq!(stop_with_halt_at(entry, "c", reaching), at, "{entry}");
        assert_ne!(
            stop_with_halt_at(entry, "c", other),
            at,
            "{entry} at {other}"
        );
        assert_ne!(
            stop_with_halt_at(entry, "c", 89),
            at,
            "{entry} past column 88"
        );
    }
    assert_eq!(stop_with_halt_at("ui_tv3", "a", 83), label(REAL, "ui_tv3"));
    assert_ne!(stop_with_halt_at("ui_tv2", "a", 82), label(REAL, "ui_tv2"));
}

#[test]
fn the_jump_tables_sit_on_the_boundaries_the_or_trick_needs() {
    for image in [REAL, MOVED] {
        assert_eq!(label(image, "ui_vtab") % 64, 0);
        assert_eq!(label(image, "ut_rows") % 16, 0);
    }
}

#[test]
fn a_font_set_once_is_used_by_every_routine_until_another_is_set() {
    let font = wide_font();
    let table = Table::new(&font).unwrap();
    let mut model = Bitmap::new(96, 64);
    model_text(&mut model, &font, 4, 1, "abc");
    model_text(&mut model, &font, 0, 5, "ca.");
    let right = font.text("aab");
    model.blit(&right, 95 - right.width(), 3 * 8 + 1);
    let centred = font.text("bab");
    model.blit(&centred, (96 - centred.width()) / 2, 4 * 8 + 1);
    let got = run(
        REAL,
        &[
            Op::SetFont(flat(&table)),
            Op::Text {
                x: 4,
                row: 1,
                codes: table.encode("abc"),
            },
            Op::Text {
                x: 0,
                row: 5,
                codes: table.encode("ca."),
            },
            Op::Right {
                row: 3,
                codes: table.encode("aab"),
            },
            Op::Centered {
                row: 4,
                codes: table.encode("bab"),
            },
        ],
    );
    same_frame("wide font", &got.frame, &pack(&model));
}

#[test]
fn the_soft_key_row_uses_its_own_font_and_restores_the_one_in_use() {
    let font = wide_font();
    let table = Table::new(&font).unwrap();
    let mut keys = Canvas::recording();
    keys.soft_keys(["Open", "", "Info", "Find", "Up"]);
    let (key_frame, calls) = keys.into_parts();
    let mut model = key_frame;
    model_text(&mut model, &font, 10, 2, "aba");
    let mut script = vec![Op::SetFont(flat(&table))];
    script.extend(ops(&calls));
    script.push(Op::Text {
        x: 10,
        row: 2,
        codes: table.encode("aba"),
    });
    let got = run(REAL, &script);
    same_frame("keys between wide text", &got.frame, &pack(&model));
}

#[test]
fn switching_back_to_the_built_in_font_restores_its_glyphs() {
    let font = wide_font();
    let table = Table::new(&font).unwrap();
    let mut model = Canvas::default();
    model.text(0, 0, "Hello");
    let got = run(
        REAL,
        &[
            Op::SetFont(flat(&table)),
            Op::Text {
                x: 0,
                row: 6,
                codes: table.encode("a"),
            },
            Op::BuiltinFont,
            Op::Text {
                x: 0,
                row: 0,
                codes: Table::builtin().encode("Hello"),
            },
        ],
    );
    let mut want = model.into_bitmap();
    model_text(&mut want, &font, 0, 6, "a");
    same_frame("builtin again", &got.frame, &pack(&want));
}

#[test]
fn the_canvas_works_at_another_ram_base_and_writes_nothing_outside_its_two_blocks() {
    let mut canvas = Canvas::recording();
    canvas.title("Libraries", "metacalc");
    canvas.list_row(
        2,
        "ax-1 and a long name running off the edge of the screen",
        "\u{2713}",
        true,
    );
    canvas.soft_keys(["Open", "Run", "Info", "", ""]);
    canvas.marker(4);
    canvas.invert(90, 40, 30, 30);
    let (bitmap, calls) = canvas.into_parts();
    for image in [REAL, MOVED] {
        let got = run(image, &ops(&calls));
        same_frame("moved base", &got.frame, &pack(&bitmap));
    }
}
