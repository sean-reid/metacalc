//! The start screen, a key, and leaving.

use crate::app::*;
use crate::realos::{BASE, KEY_CLEAR};
use crate::ui_support::{pack, same_frame};
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Stop};
use mcui::canvas::Canvas;

const DOWN: u8 = 0x01;
const SECOND: u8 = 0x36;
const MODE: u8 = 0x37;
const TAP: u32 = 120_000;

/// What the model's canvas draws for the start screen: a title bar and a line of text on row 3.
pub fn model() -> mcglyph::Bitmap {
    let mut canvas = Canvas::default();
    canvas.title("metacalc", "");
    canvas.text(1, 3, "CLEAR to quit");
    canvas.into_bitmap()
}

fn lcd(app: &mut App) -> Vec<bool> {
    let lcd = app.m.lcd();
    (0..LCD_HEIGHT)
        .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| lcd.pixel(x, y))
        .collect()
}

pub fn pixels(bitmap: &mcglyph::Bitmap) -> Vec<bool> {
    (0..LCD_HEIGHT)
        .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| bitmap.get(x, y))
        .collect()
}

fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(2 * TAP).stop, Stop::Limit);
}

#[test]
fn the_start_screen_is_the_frame_the_models_canvas_draws() {
    let want = model();
    for built in [plain(), with_library()] {
        let mut app = App::start(built);
        same_frame("the start screen", &app.frame(), &pack(&want));
        assert!(lcd(&mut app) == pixels(&want), "the LCD shows the frame");
        assert_eq!(app.m.in_port(0x06), BASE, "the app is on its entry page");
    }
}

#[test]
fn a_key_redraws_the_screen_with_the_font_set_again() {
    let want = pack(&model());
    for built in [plain(), with_library()] {
        let mut app = App::start(built);
        // ui_set_font keeps the table's address less one slot of 8 bytes.
        let font = (app.label("ui_font") - 8).to_le_bytes();
        // What auto power down does to saveSScreen, and a frame that is not the screen's.
        app.m.load_ram(UI_VARS, &[0; 0x1A]);
        app.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
        tap(&mut app, DOWN);
        assert_eq!(app.m.read_ram(UI_VARS + 0x14, 2), font, "the font pointer");
        same_frame("after a key", &app.frame(), &want);
        assert_eq!(app.m.in_port(0x06), BASE);
    }
}

#[test]
fn clear_leaves_the_app_cleanly() {
    let mut app = App::start(plain());
    app.m.press_key(KEY_CLEAR);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
}

#[test]
fn quit_leaves_the_app_cleanly() {
    let mut app = App::start(plain());
    tap(&mut app, SECOND);
    app.m.press_key(MODE);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
}

#[test]
fn another_key_leaves_the_app_where_it_was() {
    let mut app = App::start(plain());
    tap(&mut app, DOWN);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit, "still running");
}
