use crate::support::*;
use mcglyph::z80::{SLOT, Table};
use mcglyph::{Bitmap, Font};
use mcui::canvas::Canvas;

const WIDE: &str = include_str!("../../../mcglyph/tests/fixtures/wide.font");

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
fn the_assembled_image_holds_the_generated_table() {
    let table = Table::builtin();
    let at = usize::from(label(REAL, "ui_font")) - 0x4000;
    let bytes = flat(&table);
    assert_eq!(&REAL.code[at..at + bytes.len()], bytes);
    assert_eq!(bytes.len(), 1328);
    assert_eq!(label(REAL, "UI_FONT_GLYPHS") as usize * SLOT, bytes.len());
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
    keys.soft_keys(["See", "", "Info", "Find", "Up"]);
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
    canvas.soft_keys(["See", "Run", "Info", "", ""]);
    canvas.marker(4);
    canvas.invert(90, 40, 30, 30);
    let (bitmap, calls) = canvas.into_parts();
    for image in [REAL, MOVED] {
        let got = run(image, &ops(&calls));
        same_frame("moved base", &got.frame, &pack(&bitmap));
        assert!(
            got.around.iter().all(|&b| b == MARK),
            "bytes outside the two blocks changed"
        );
    }
}
