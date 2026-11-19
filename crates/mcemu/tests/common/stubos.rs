//! What the tests know about the stub OS: its interrupt timer and the keyboard state its handler
//! keeps, read from the stub's own label file.

#![allow(dead_code)]

const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/os.lab"));

/// The stub leaves port 4 at 0, which sets the timer interrupt period to 1953 us (tmrvalues in the
/// core's x4_io.c).
pub const TIMER_PERIOD_US: u32 = 1953;
/// T-states in one timer period at 15 MHz.
pub const TIMER_PERIOD_15MHZ: u32 = TIMER_PERIOD_US * 15;

pub fn stub_label(name: &str) -> usize {
    LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| usize::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no stub label {name}"))
}

/// Offsets in RAM page 81h of the scan code the handler sets for a new key and the last key it saw.
pub fn kbd_scan_code() -> usize {
    stub_label("KBDSCANCODE") - 0x8000
}

pub fn kbd_key() -> usize {
    stub_label("KBDKEY") - 0x8000
}
