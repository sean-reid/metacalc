use crate::stackfree;
use crate::support::*;

const OUT_5: [u8; 2] = [0xD3, 0x05];

fn at(image: Image, name: &str) -> usize {
    usize::from(label_in(image, name) - image.org)
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
        let checked = at(image, "LR_CHECKED");
        let enter = at(image, "LR_ENTER");
        let found = stackfree::breaks(code, checked, enter, &[]);
        assert!(
            found.is_empty(),
            "from the first instruction after the argument checks to the DI: {found:x?}"
        );
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

#[test]
fn the_entry_check_sees_a_shadow_register_use_before_the_di() {
    let image = DEFAULT;
    let (checked, enter) = (at(image, "LR_CHECKED"), at(image, "LR_ENTER"));
    assert_eq!(
        image.code[checked + 4],
        0x07,
        "the rlca after the first load"
    );
    for (patch, why) in [
        (0xD9u8, "shadow registers"),
        (0x08, "shadow registers"),
        (0xF3, ""),
    ] {
        let mut code = image.code.to_vec();
        code[checked + 4] = patch;
        let found = stackfree::breaks(&code, checked, enter, &[]);
        if why.is_empty() {
            assert!(found.is_empty(), "a DI is not a stack instruction");
        } else {
            assert_eq!(found, [(checked + 4, why)]);
        }
    }
}

/// What an instruction at `at` touches that the section routines leave alone: the shadow registers,
/// a port, the interrupt flag or mode, a halt.
fn machine_use(code: &[u8], at: usize) -> Option<&'static str> {
    match code[at] {
        0xD9 | 0x08 => Some("shadow registers"),
        0xDB | 0xD3 => Some("port"),
        0xF3 | 0xFB => Some("interrupt flag"),
        0x76 => Some("halt"),
        0xED => match code[at + 1] {
            0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x76 | 0x7E => Some("interrupt mode"),
            n if n & 0xC0 == 0x40 && n & 6 == 0 => Some("port"),
            0xA2 | 0xAA | 0xB2 | 0xBA | 0xA3 | 0xAB | 0xB3 | 0xBB => Some("port"),
            _ => None,
        },
        _ => None,
    }
}

/// Every instruction of the label order calls, which are in lib/order.inc from FO_BEGIN to the end
/// of the image apart from the 8 bytes of the bit mask table in the middle.
fn order_machine_use(code: &[u8], image: Image) -> Vec<(usize, &'static str)> {
    let (begin, table, walk) = (
        at(image, "FO_BEGIN"),
        at(image, "FWK_BITS"),
        at(image, "LIB_FIND_WALK"),
    );
    assert_eq!(
        walk,
        table + 8,
        "the table is 8 bytes and the walk follows it"
    );
    let mut found = Vec::new();
    for (from, to) in [(begin, table), (walk, code.len())] {
        let mut pc = from;
        while pc < to {
            found.extend(machine_use(code, pc).map(|why| (pc, why)));
            pc += stackfree::decode(&code[pc..]).0;
        }
        assert_eq!(pc, to, "instruction boundaries miss the end");
    }
    found
}

#[test]
fn the_find_calls_hold_no_shadow_register_port_or_interrupt_instruction() {
    for image in [DEFAULT, MOVED, STRADDLE] {
        let found = order_machine_use(image.code, image);
        assert!(found.is_empty(), "{found:x?}");
        let size = image.code.len() - at(image, "FO_BEGIN");
        assert!(size > 800, "lib/order.inc is {size} bytes of the image");
    }
}

#[test]
fn the_find_scan_sees_each_instruction_it_looks_for() {
    let image = DEFAULT;
    let at = at(image, "FO_BEGIN") + 3;
    assert_eq!(image.code[at], 0xC0, "the ret nz after the call");
    for (patch, why) in [
        (&[0xD9u8][..], "shadow registers"),
        (&[0x08], "shadow registers"),
        (&[0xF3], "interrupt flag"),
        (&[0xFB], "interrupt flag"),
        (&[0x76], "halt"),
    ] {
        let mut code = image.code.to_vec();
        code[at..at + patch.len()].copy_from_slice(patch);
        assert_eq!(order_machine_use(&code, image), [(at, why)]);
    }
    assert_eq!(image.code[at + 1], 0x3A, "the ld a,(ls_have) after it");
    for (patch, why) in [
        ([0xDBu8, 0x05, 0x00], "port"),
        ([0xD3, 0x05, 0x00], "port"),
        ([0xED, 0x78, 0x00], "port"),
        ([0xED, 0x41, 0x00], "port"),
        ([0xED, 0xA3, 0x00], "port"),
        ([0xED, 0x56, 0x00], "interrupt mode"),
    ] {
        let mut code = image.code.to_vec();
        code[at + 1..at + 4].copy_from_slice(&patch);
        assert_eq!(order_machine_use(&code, image), [(at + 1, why)]);
    }
}
