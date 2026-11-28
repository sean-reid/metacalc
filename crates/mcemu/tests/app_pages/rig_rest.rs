//! The rig's reads and writes of the app's RAM wait for the resting banks: stopped inside a reader
//! call with the library's flash in bank B, a raw read returns the flash and the rig's getters the
//! app's bytes.

use crate::app::*;
use crate::list_depth::alternating_marks;
use crate::list_rig::*;
use crate::shellrig::*;
use mcui::Key;

const STEP: u32 = 2_000;

/// What a stop inside a call has: the list, the canvas frame as a raw read gives it there, the frame
/// in the app's RAM, and the status table installed.
struct Inside {
    list: Lister,
    raw: Vec<u8>,
    app: Vec<u8>,
    table: Vec<u8>,
}

/// A list of three stages opened and moved a row, stopped inside a call that shows another bank
/// where the canvas frame is.
fn inside_a_call() -> Inside {
    let built = crate::builds::three_stages();
    let (table, status) = alternating_marks(built);
    let mut list = Lister::open(built, Some((&table, &status)), 0);
    list.key(DOWN, Key::Down);
    let mut spent = 0;
    loop {
        list.run.run(STEP);
        spent += STEP;
        assert!(
            spent < LIMIT,
            "no call showed another bank in {spent} T-states"
        );
        if !list.run.shell.in_call() {
            continue;
        }
        let raw = list.run.m().read_ram(UI_FRAME, FRAME_LEN);
        let app = list.run.peek(UI_FRAME, FRAME_LEN);
        if raw != app {
            println!("stopped inside a call after {spent} T-states");
            return Inside {
                list,
                raw,
                app,
                table,
            };
        }
    }
}

#[test]
fn a_raw_read_inside_a_call_is_not_the_apps_frame_and_the_shells_is() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app, "a raw read lands in the other bank");
    assert_eq!(at.list.run.shell.frame(), at.app, "Shell::frame");
    assert!(
        !at.list.run.shell.in_call(),
        "the wait ended outside the call"
    );
}

#[test]
fn shows_compares_the_apps_frame_and_not_the_other_bank() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app);
    at.list.run.shell.shows("inside a call", &at.app);
}

#[test]
fn a_peek_inside_a_call_reads_the_apps_ram() {
    let mut at = inside_a_call();
    let first = at
        .raw
        .iter()
        .zip(&at.app)
        .position(|(r, a)| r != a)
        .unwrap();
    let len = 16.min(FRAME_LEN - first);
    let want = &at.app[first..first + len];
    assert_ne!(&at.raw[first..first + len], want, "a raw read differs here");
    let got = at.list.run.shell.peek(UI_FRAME + first as u16, len);
    assert_eq!(got, want);
}

#[test]
fn the_status_table_is_read_from_the_apps_ram_inside_a_call() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app);
    assert_eq!(at.list.run.raw_table().as_deref(), Some(&at.table[..]));
}

#[test]
fn the_variables_are_listed_from_the_apps_ram_inside_a_call() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app);
    let vars = at.list.run.vars();
    assert!(vars.contains("MCSTAT"), "{vars}");
}

#[test]
fn a_flag_set_inside_a_call_lands_in_the_apps_ram() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app);
    at.list.run.shell.flag(true);
    assert_eq!(at.list.run.peek(FLAG, 1), [1]);
}

#[test]
fn a_power_down_inside_a_call_lands_in_the_apps_ram() {
    let mut at = inside_a_call();
    assert_ne!(at.raw, at.app);
    at.list.run.shell.power_down();
    let lcd = at.list.run.shell.lcd_bytes();
    let at_tail = SAVE_SCREEN + 0x40;
    assert_eq!(
        at.list.run.peek(at_tail, lcd.len() - 0x40),
        lcd[0x40..],
        "the pixels"
    );
}
