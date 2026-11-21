use crate::stackfree;
use crate::support::*;

const PORT_RAM_PAGE: u8 = 5;

fn at(image: Image, name: &str) -> usize {
    usize::from(label_in(image, name) - 0x4000)
}

#[test]
fn the_entry_and_exit_windows_hold_no_stack_instruction() {
    for (_, image) in LAYOUTS {
        let code = image.code;
        let out_5 = [0xD3, PORT_RAM_PAGE];

        let (start, end) = (at(image, "KE_SAVE"), at(image, "KE_WINDOW_END"));
        let found = stackfree::breaks(code, start, end, &["store sp"]);
        assert!(found.is_empty(), "entry window: {found:x?}");
        assert_eq!(code[start], 0xF3, "the window opens with DI");
        let write = at(image, "KE_WINDOW");
        assert_eq!(code[write..write + 2], out_5, "the port 5 write");
        assert_eq!(code[end], 0x31, "the stack switch follows the window");
        assert_eq!(
            code[end - 4..end - 2],
            [0xED, 0x73],
            "the window ends storing SP"
        );

        let (start, end) = (at(image, "LEAVE_WINDOW"), at(image, "LEAVE_WINDOW_END"));
        let refs = (at(image, "REFS_WINDOW"), at(image, "REFS_WINDOW_END"));
        assert!(
            stackfree::breaks(code, refs.0, refs.1, &[]).is_empty(),
            "refs exit window"
        );
        assert_eq!(
            code[refs.0..refs.1],
            out_5,
            "the refs window is the port 5 restore"
        );
        assert_eq!(
            code[refs.0 - 4..refs.0 - 2],
            [0xED, 0x7B],
            "after the stack restore"
        );
        let found = stackfree::breaks(code, start, end, &[]);
        assert!(found.is_empty(), "exit window: {found:x?}");
        assert_eq!(code[start..end], out_5, "the window is the port 5 restore");
        assert_eq!(
            code[start - 4..start - 2],
            [0xED, 0x7B],
            "after the stack restore"
        );
    }
}

#[test]
fn the_window_check_sees_a_stack_instruction_put_in_a_window() {
    let image = DEFAULT;
    let (start, end) = (at(image, "KE_SAVE"), at(image, "KE_WINDOW_END"));
    let write = at(image, "KE_WINDOW");
    for (patch, why) in [(0xC5u8, "push"), (0xD1, "pop"), (0xFB, "ei"), (0xC9, "ret")] {
        let mut code = image.code.to_vec();
        code[write + 2] = patch;
        let found = stackfree::breaks(&code, start, end, &["store sp"]);
        assert_eq!(found, [(write + 2, why)]);
    }
    let mut code = image.code.to_vec();
    let at_restore = at(image, "LEAVE_WINDOW");
    code[at_restore] = 0xC5;
    let found = stackfree::breaks(&code, at_restore, at_restore + 1, &[]);
    assert_eq!(found, [(at_restore, "push")]);
}

fn cost(entry: &str) -> u32 {
    Device::new().run(entry)
}

#[test]
fn the_entry_and_exit_overhead_per_call() {
    let overhead = cost("ENTRY_OVERHEAD") - cost("ENTRY_BASELINE");
    println!("overhead of one call: {overhead} T-states");
    assert_eq!(overhead, 415);
}
