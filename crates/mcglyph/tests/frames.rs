use mcglyph::{Font, LEADING};

fn tokens(n: usize) -> Vec<&'static str> {
    ["(", "ph", "->", "ps", ")"]
        .into_iter()
        .cycle()
        .take(n)
        .collect()
}

#[test]
fn short_statement_fills_one_screen() {
    let font = Font::builtin();
    let words = tokens(10);
    let frames = font.frames(&words, (96, 64)).expect("renders");
    assert_eq!(frames.len(), 1);
    let text = font.render(&words, 96).expect("renders");
    for y in 0..64 {
        for x in 0..96 {
            assert_eq!(frames[0].get(x, y), text.get(x, y));
        }
    }
}

#[test]
fn long_statement_continues_on_the_next_screen_without_cutting_a_line() {
    let font = Font::builtin();
    let words = tokens(400);
    let frames = font.frames(&words, (96, 64)).expect("renders");
    assert!(frames.len() > 1);
    let text = font.render(&words, 96).expect("renders");
    let pitch = font.height() + LEADING;
    let per_frame = (64 + LEADING) / pitch;
    for (i, frame) in frames.iter().enumerate() {
        assert_eq!((frame.width(), frame.height()), (96, 64));
        for y in 0..per_frame * pitch {
            for x in 0..96 {
                assert_eq!(frame.get(x, y), text.get(x, y + i * per_frame * pitch));
            }
        }
    }
}
