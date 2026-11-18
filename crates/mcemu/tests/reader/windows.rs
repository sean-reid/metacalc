use crate::stackfree;
use crate::support::*;

const OUT_5: [u8; 2] = [0xD3, 0x05];

fn at(image: Image, name: &str) -> usize {
    usize::from(label_in(image, name) - 0x4000)
}

#[test]
fn nothing_between_the_spare_page_going_in_and_coming_out_touches_the_stack() {
    for image in [DEFAULT, MOVED, STRADDLE] {
        let code = image.code;
        let (open, close) = (at(image, "LR_WINDOW"), at(image, "LR_WINDOW_END"));
        assert_eq!(
            code[open..open + 2],
            OUT_5,
            "the port 5 write that maps the spare page"
        );
        assert_eq!(
            code[close..close + 2],
            OUT_5,
            "the port 5 write that restores the caller's"
        );
        let found = stackfree::breaks(code, open + 2, close, &["shadow registers"]);
        assert!(found.is_empty(), "window: {found:x?}");
        assert!(
            close - open < 64,
            "the window is short: {} bytes",
            close - open
        );
        let enter = at(image, "LR_ENTER");
        assert_eq!(
            code[enter], 0xF3,
            "interrupts go off before anything is mapped"
        );
        let found = stackfree::breaks(code, enter, open, &["shadow registers"]);
        assert!(found.is_empty(), "entry: {found:x?}");
    }
}

#[test]
fn the_window_check_sees_a_stack_instruction_put_in_the_window() {
    let image = DEFAULT;
    let (open, close) = (at(image, "LR_WINDOW"), at(image, "LR_WINDOW_END"));
    for (patch, why) in [
        (0xC5u8, "push"),
        (0xD1, "pop"),
        (0xFB, "ei"),
        (0xC9, "ret"),
        (0xE3, "ex (sp)"),
    ] {
        let mut code = image.code.to_vec();
        code[open + 2] = patch;
        let found = stackfree::breaks(&code, open + 2, close, &["shadow registers"]);
        assert_eq!(found, [(open + 2, why)]);
    }
    let mut code = image.code.to_vec();
    code[close - 1] = 0xCD;
    let found = stackfree::breaks(&code, close - 1, close + 2, &["shadow registers"]);
    assert_eq!(found.len(), 1, "a call is a stack instruction");
}
