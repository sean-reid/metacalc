//! The start screen, a key, a refused call, and leaving: the app's own path from `launch` through
//! the kernel, the reader, fm and the canvas, compared with the model's canvas.

use crate::app::*;
use crate::fixture;
use crate::realos::{BASE, KEY_CLEAR, start_model, start_pixels};
use crate::ui_support::{pack, same_frame};
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Stop};

const DOWN: u8 = 0x01;
const SECOND: u8 = 0x36;
const MODE: u8 = 0x37;
const TAP: u32 = 120_000;
/// The reader's status for a library it cannot open (ST_BAD in the reader's header), fm's for a
/// library with no glyph map (FM_ERR_NO_MAP) and for a glyph map of another font (FM_ERR_FONT).
const ST_BAD: u8 = 3;
const FM_ERR_NO_MAP: u8 = 3;
const FM_ERR_FONT: u8 = 4;

fn lcd(app: &mut App) -> Vec<bool> {
    let lcd = app.m.lcd();
    (0..LCD_HEIGHT)
        .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| lcd.pixel(x, y))
        .collect()
}

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(2 * TAP).stop, Stop::Limit);
}

/// The start screen on the LCD, the frame and the page port for `built`.
fn start_screen_is_the_models(built: &'static Built) {
    let mut app = App::start(built);
    let want = start_pixels(built.library.as_deref(), 0);
    assert!(lcd(&mut app) == want, "the LCD shows the frame");
    assert_eq!(app.m.in_port(0x06), BASE, "the app is on its entry page");
}

#[test]
fn the_start_screen_is_the_frame_the_models_canvas_draws() {
    start_screen_is_the_models(plain());
    start_screen_is_the_models(with_library());
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn the_start_screen_of_the_prop_library_is_the_frame_the_models_canvas_draws() {
    start_screen_is_the_models(shipped("prop"));
}

fn after_a_key(built: &'static Built) {
    let want = pack(&start_model(built.library.as_deref(), 0));
    let mut app = App::start(built);
    // What auto power down does to saveSScreen, and a frame that is not the screen's.
    app.m.load_ram(UI_VARS, &[0; 0x1A]);
    app.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
    tap(&mut app, DOWN);
    // ui_set_font keeps the table's address less one slot of 8 bytes.
    let font = (app.label("ui_font") - 8).to_le_bytes();
    assert_eq!(app.m.read_ram(UI_VARS + 0x14, 2), font, "the font pointer");
    same_frame("after a key", &app.frame(), &want);
    assert_eq!(app.m.in_port(0x06), BASE);
}

#[test]
fn a_key_redraws_the_screen_with_the_font_set_again() {
    after_a_key(plain());
    after_a_key(with_library());
}

/// The screen of a build whose library is refused: the title bar and the status as two hex digits
/// on row 7, and nothing else.
fn refusal(name: &str, library: Vec<u8>, status: u8) {
    let built = refused(name, &library);
    let mut app = App::boot(built);
    let want = pack(&start_model(Some(&library), status));
    same_frame(name, &app.frame(), &want);
    assert!(
        lcd(&mut app) == start_pixels(Some(&library), status),
        "{name}: the LCD"
    );
    let font = (app.label("ui_font") - 8).to_le_bytes();
    app.m.load_ram(UI_VARS, &[0; 0x1A]);
    tap(&mut app, DOWN);
    assert_eq!(app.m.read_ram(UI_VARS + 0x14, 2), font);
    same_frame(&format!("{name} after a key"), &app.frame(), &want);
}

#[test]
fn a_library_the_reader_refuses_shows_its_status_in_place_of_the_formula() {
    let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
    refusal("magic", bad, ST_BAD);
}

#[test]
fn a_library_fm_refuses_shows_its_status_in_place_of_the_formula() {
    refusal("no-map", fixture::plain(), FM_ERR_NO_MAP);
    let font = fixture::patched(|b| {
        let at = fixture::section(b);
        b[at] ^= 1;
    });
    refusal("font", font, FM_ERR_FONT);
}

#[test]
fn clear_leaves_the_app_cleanly() {
    let mut app = App::start(plain());
    app.m.press_key(KEY_CLEAR);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
}

#[test]
fn quit_leaves_the_app_cleanly() {
    let mut app = App::start(with_library());
    tap(&mut app, SECOND);
    app.m.press_key(MODE);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
}

#[test]
fn another_key_leaves_the_app_where_it_was() {
    let mut app = App::start(plain());
    tap(&mut app, DOWN);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit, "still running");
    same_frame("after the key", &app.frame(), &pack(&start_model(None, 0)));
}
