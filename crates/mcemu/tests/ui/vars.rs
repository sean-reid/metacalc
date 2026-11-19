use crate::support::*;
use mcui::canvas::{Call, Canvas};

fn screen() -> (mcglyph::Bitmap, Vec<Call>) {
    let mut c = Canvas::recording();
    c.title("Libraries", "metacalc");
    c.list_row(2, "ax-1 and a name", "\u{2713}", false);
    c.list_row(3, "ax-2", "", true);
    c.text(5, 4, "Select a library");
    c.soft_keys(["Open", "Run", "Info", "Find", "Up"]);
    c.into_parts()
}

fn twice(calls: &[Call], between: &[Op]) -> Outcome {
    let mut script = ops(calls);
    script.push(Op::ClearVars);
    script.push(Op::Clear);
    script.extend_from_slice(between);
    script.extend(ops(calls));
    run(REAL, &script)
}

#[test]
fn a_screen_drawn_again_after_the_variables_are_zeroed_and_the_font_set_is_the_same() {
    let (bitmap, calls) = screen();
    let got = twice(&calls, &[Op::BuiltinFont]);
    same_frame("after zeroed variables", &got.frame, &pack(&bitmap));
}

#[test]
fn a_screen_drawn_after_the_variables_are_zeroed_without_setting_the_font_is_not() {
    let (bitmap, calls) = screen();
    let got = twice(&calls, &[]);
    assert_ne!(
        got.frame,
        pack(&bitmap),
        "zeroed variables did not change the frame, so the test above proves nothing"
    );
}
