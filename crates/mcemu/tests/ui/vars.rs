use crate::support::*;
use mcui::canvas::{Call, Canvas};

fn screen() -> (mcglyph::Bitmap, Vec<Call>) {
    let mut c = Canvas::recording();
    c.title("Libraries", "metacalc");
    c.list_row(2, "ax-1 and a name", "\u{2713}", false);
    c.list_row(3, "ax-2", "", true);
    c.text(5, 4, "Select a library");
    c.text_centered(6, "centred");
    c.invert(70, 40, 20, 12);
    c.soft_keys(["Open", "Run", "Info", "Find", "Up"]);
    c.into_parts()
}

/// The screen drawn once, then drawn again with the variables zeroed before every call and
/// `restore` run after the zeroing, and blitted.
fn twice(calls: &[Call], restore: &[Op]) -> Outcome {
    let mut script = ops(calls);
    script.push(Op::Clear);
    for op in ops(calls) {
        script.push(Op::ClearVars);
        script.extend_from_slice(restore);
        script.push(op);
    }
    script.push(Op::ClearVars);
    script.extend_from_slice(restore);
    script.push(Op::Blit);
    run(REAL, &script)
}

#[test]
fn a_screen_drawn_with_the_variables_zeroed_and_the_font_set_before_every_call_is_the_same() {
    let (bitmap, calls) = screen();
    let got = twice(&calls, &[Op::BuiltinFont]);
    same_frame("after zeroed variables", &got.frame, &pack(&bitmap));
    assert!(
        (0..64).all(|y| (0..96).all(|x| got.lcd[y * 96 + x] == bitmap.get(x, y))),
        "the LCD differs from the frame"
    );
}

#[test]
fn a_screen_drawn_with_the_variables_zeroed_and_no_font_set_is_not() {
    let (bitmap, calls) = screen();
    let got = twice(&calls, &[]);
    assert!(
        (0xA800..0xB000).contains(&got.pc),
        "the run ended at {:04x}, not at the script's halt",
        got.pc
    );
    assert_ne!(
        got.frame,
        pack(&bitmap),
        "zeroed variables did not change the frame, so the test above proves nothing"
    );
}
