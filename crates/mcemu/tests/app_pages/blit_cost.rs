//! What the canvas's blit, its row marking and its row routines cost on the reference OS at 6 MHz,
//! printed on passing runs and pinned within 2 percent. Ignored: they need the reference ROM.

use crate::app::*;
use crate::built::shipped_staged;
use crate::realos::BASE;
use mcemu::Speed;

/// Where the test puts the string ui_text draws, in the scratch range the app on the ROM keeps clear.
const STRING: u16 = 0xA9D0;

/// One call of ui_text before the rows were marked, "Metamath" at x 1 on row 3, and the full blit.
const OLD_TEXT: u32 = 6_274;
/// What marking the row adds to it.
const TEXT_MARK: u32 = 57;
const OLD_BLIT: u32 = 73_579;

fn app_on_rom() -> App {
    let app = App::on_real_os(shipped_staged());
    let (low, high) = app.scratch[1];
    assert!(
        (low..high - 16).contains(&STRING),
        "the string at {STRING:04X} is in scratch {low:04X} to {high:04X}"
    );
    app
}

fn timed(app: &mut App, name: &str, input: In) -> u32 {
    let at = app.label(name);
    app.timed(BASE, at, input).tstates
}

fn var(app: &App, name: &str) -> u16 {
    app.label(name)
}

fn blit(app: &mut App, mask: u8) -> u32 {
    let (dirty, sent) = (var(app, "ui_dirty"), var(app, "ui_sent"));
    app.poke(dirty, &[mask]);
    app.poke(sent, &[mask ^ 0xA5]);
    let took = timed(app, "far_ui_blit", In::default());
    assert_eq!(app.peek(sent, 1), [mask], "ui_sent after the blit");
    assert_eq!(app.peek(dirty, 1), [0], "ui_dirty after the blit");
    took
}

/// Within 2 percent of `want`, and printed.
fn pinned(what: &str, got: u32, want: u32) {
    println!("ROM 6 MHz: {what}: {got} T-states, pinned at {want}");
    let slack = want / 50;
    assert!(
        got.abs_diff(want) <= slack,
        "{what}: {got} against {want} within {slack}"
    );
}

fn text_cost(app: &mut App) -> u32 {
    let codes = mcglyph::z80::Table::builtin().encode("Metamath");
    app.poke(STRING, &codes);
    timed(
        app,
        "far_ui_text",
        In {
            hl: STRING,
            de: 1,
            bc: 3 << 8,
            ..In::default()
        },
    )
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_blit_costs_what_the_rows_it_sends_say() {
    let mut app = app_on_rom();
    let full = blit(&mut app, 0xFF);
    let one = blit(&mut app, 0x08);
    let two = blit(&mut app, 0x18);
    let none = blit(&mut app, 0x00);
    println!("ROM 6 MHz: the full blit was {OLD_BLIT} before the mask");
    pinned("full blit", full, FULL);
    pinned("blit of one row", one, ONE);
    pinned("blit of two adjacent rows", two, TWO);
    pinned("blit of no row", none, NONE);
    assert!(
        one > 12_000 && two > one + 7_000,
        "a blit that sends nothing fails"
    );
    assert!(full > two && none < 1_000);
    app.m.set_speed(Speed::Mhz15);
    let at15 = blit(&mut app, 0xFF);
    println!("ROM 15 MHz: full blit {at15} T-states");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_marking_a_row_adds_what_the_header_says_to_ui_text() {
    let mut app = app_on_rom();
    let dirty = var(&app, "ui_dirty");
    app.poke(dirty, &[0x00]);
    let got = text_cost(&mut app);
    println!(
        "ROM 6 MHz: ui_text once: {OLD_TEXT} before, {got} now, {} more",
        got - OLD_TEXT
    );
    assert_eq!(got - OLD_TEXT, TEXT_MARK, "what the mark costs");
    assert_eq!(app.peek(dirty, 1), [1 << 3], "the row's bit in ui_dirty");
    pinned("ui_text once", got, TEXT);
    app.poke(dirty, &[0x84]);
    text_cost(&mut app);
    assert_eq!(app.peek(dirty, 1), [0x8C], "the bit is ORed in");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_clearing_and_shifting_rows_cost_what_they_are_pinned_at() {
    let mut app = app_on_rom();
    let (dirty, frame) = (var(&app, "ui_dirty"), UI_FRAME);
    let call = |app: &mut App, name: &str, first: u8, count: u8, dir: u8| {
        app.poke(frame, &[0x5A; 768]);
        app.poke(dirty, &[0]);
        let took = timed(
            app,
            name,
            In {
                a: first,
                bc: u16::from(count) << 8 | u16::from(dir),
                ..In::default()
            },
        );
        let mask = (first..first + count).fold(0u8, |m, r| m | (1 << r));
        assert_eq!(app.peek(dirty, 1), [mask], "{name} {count}: ui_dirty");
        took
    };
    for (name, rows, dir, want) in [
        ("far_ui_clear_rows", 1, 0, CLEAR_ONE),
        ("far_ui_clear_rows", 6, 0, CLEAR_SIX),
        ("far_ui_shift_rows", 1, 0, SHIFT_ONE),
        ("far_ui_shift_rows", 6, 0, SHIFT_SIX_UP),
        ("far_ui_shift_rows", 6, 1, SHIFT_SIX_DOWN),
    ] {
        let got = call(&mut app, name, 1, rows, dir);
        pinned(
            &format!("{name} of {rows} rows, direction {dir}"),
            got,
            want,
        );
    }
}

const FULL: u32 = 74481;
const ONE: u32 = 13265;
const TWO: u32 = 22032;
const NONE: u32 = 646;
const TEXT: u32 = 6331;
const CLEAR_ONE: u32 = 1629;
const CLEAR_SIX: u32 = 5319;
const SHIFT_ONE: u32 = 1644;
const SHIFT_SIX_UP: u32 = 13807;
const SHIFT_SIX_DOWN: u32 = 13811;
