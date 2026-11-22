use mcemu::{ExitState, Machine, STUB_ROM, Speed, Stop};

const BASE: u8 = 0x30;
const ENTRY: usize = 0x80;
const LIMIT: u32 = 100_000;

/// `bjump(_JForceCmdNoChar)`, which the stub takes as an exit as it does `_JForceCmd`.
const LEAVE: [u8; 5] = [0xCD, 0x50, 0x00, 0x27, 0x40];

fn run_app(code: &[u8]) -> Stop {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.launch(BASE).expect("the app launches");
    m.load_flash(BASE, ENTRY, code);
    m.run_app(LIMIT).stop
}

fn then_leave(first: &[u8]) -> Vec<u8> {
    [first, &LEAVE].concat()
}

fn left_as(first: &[u8], change: impl FnOnce(&mut ExitState)) -> Stop {
    let mut want = ExitState::CLEAN;
    change(&mut want);
    assert_ne!(want, ExitState::CLEAN);
    assert_eq!(run_app(&then_leave(first)), Stop::BadExit(want));
    Stop::BadExit(want)
}

#[test]
fn leaving_through_the_os_call_with_the_machine_as_it_was_given_is_a_clean_exit() {
    assert_eq!(run_app(&LEAVE), Stop::Exited);
}

#[test]
fn a_bad_port_5_is_not_a_clean_exit() {
    left_as(&[0x3E, 0x03, 0xD3, 0x05], |s| s.port5 = 3);
}

#[test]
fn a_bad_port_7_is_not_a_clean_exit() {
    left_as(&[0x3E, 0x84, 0xD3, 0x07], |s| s.port7 = 0x84);
}

#[test]
fn leaving_at_15_mhz_is_not_a_clean_exit() {
    left_as(&[0x3E, 0x01, 0xD3, 0x20], |s| s.speed = Speed::Mhz15);
}

#[test]
fn leaving_in_interrupt_mode_2_is_not_a_clean_exit() {
    left_as(&[0xED, 0x5E], |s| s.im = 2);
}

#[test]
fn every_deviation_at_once_is_reported_together() {
    let first = [0x3E, 0x03, 0xD3, 0x05, 0x3E, 0x01, 0xD3, 0x20, 0xED, 0x5E];
    let want = ExitState {
        port5: 3,
        port7: 0x81,
        speed: Speed::Mhz15,
        im: 2,
    };
    assert_eq!(run_app(&then_leave(&first)), Stop::BadExit(want));
}

#[test]
fn an_app_that_returns_from_its_entry_point_has_not_exited() {
    assert_eq!(run_app(&[0xC9]), Stop::Returned);
}

/// A pin on today's behaviour, not a proof of anything removed: it passes against the older stub
/// too, whose extra `di` was never reached.
#[test]
fn pin_an_app_that_returns_with_interrupts_on_stops_with_them_on() {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.launch(BASE).expect("the app launches");
    m.load_flash(BASE, ENTRY, &[0xFB, 0xC9]);
    assert_eq!(m.run_app(LIMIT).stop, Stop::Returned);
    assert_eq!(m.regs().iff1, 1);
}
