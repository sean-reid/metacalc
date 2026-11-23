//! The Libraries screen, a key, a refused call, and leaving: the app's own path from `launch` through
//! the kernel, the reader, fm and the canvas, compared with the model's canvas.

use crate::app::*;
use crate::fixture;
use crate::realos::{
    BASE, KEY_CLEAR, error_model, error_pixels, libraries_model, libraries_pixels,
};
use crate::ui_support::{pack, same_frame};
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Stop};

const DOWN: u8 = 0x01;
const F4: u8 = 0x32;
const SECOND: u8 = 0x36;
const MODE: u8 = 0x37;
const TAP: u32 = 120_000;
/// The reader's status for a library it cannot open (ST_BAD in the reader's header), fm's for a
/// library with no glyph map (FM_ERR_NO_MAP) and for a glyph map of another font (FM_ERR_FONT).
const ST_BAD: u8 = 3;
const FM_ERR_NO_MAP: u8 = 3;
const FM_ERR_FONT: u8 = 4;
const ON_INTERRUPT_BIT: u8 = 0x10;

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

/// The Libraries screen on the LCD, the frame and the page port for `built`.
fn libraries_screen_is_the_models(built: &'static Built) {
    let mut app = App::start(built);
    let want = libraries_pixels(built.shown().as_ref());
    assert!(lcd(&mut app) == want, "the LCD shows the frame");
    assert_eq!(app.m.in_port(0x06), BASE, "the app is on its entry page");
}

#[test]
fn the_libraries_screen_is_the_frame_the_models_canvas_draws() {
    libraries_screen_is_the_models(plain());
    libraries_screen_is_the_models(with_library());
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn the_libraries_screen_of_the_prop_library_is_the_frame_the_models_canvas_draws() {
    libraries_screen_is_the_models(shipped("prop"));
}

fn after_a_key(built: &'static Built) {
    let want = pack(&libraries_model(built.shown().as_ref()));
    let mut app = App::start(built);
    // What auto power down does to saveSScreen, and a frame that is not the screen's.
    app.m.load_ram(UI_VARS, &[0; 0x1A]);
    app.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
    tap(&mut app, F4);
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

/// The screen of a build whose library is refused: the error notice with the status as two hex
/// digits, over the Libraries screen, which a key on it leaves in place.
fn refusal(name: &str, library: Vec<u8>, status: u8) {
    let built = refused(name, &library);
    let mut app = App::boot(built);
    let want = pack(&error_model(status));
    same_frame(name, &app.frame(), &want);
    assert!(lcd(&mut app) == error_pixels(status), "{name}: the LCD");
    let font = (app.label("ui_font") - 8).to_le_bytes();
    app.m.load_ram(UI_VARS, &[0; 0x1A]);
    tap(&mut app, DOWN);
    assert_eq!(app.m.read_ram(UI_VARS + 0x14, 2), font);
    same_frame(&format!("{name} after a key"), &app.frame(), &want);
    assert_eq!(
        app.m.read_ram(app.label("scr_count"), 1),
        [2],
        "{name}: the depth"
    );
}

#[test]
fn a_library_the_reader_refuses_shows_the_error_notice_with_its_status() {
    let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
    refusal("magic", bad, ST_BAD);
}

#[test]
fn a_library_fm_refuses_shows_the_error_notice_with_its_status() {
    refusal("no-map", fixture::plain(), FM_ERR_NO_MAP);
    let font = fixture::patched(|b| {
        let at = fixture::section(b);
        b[at] ^= 1;
    });
    refusal("font", font, FM_ERR_FONT);
}

#[test]
fn back_from_the_error_notice_of_a_refused_library_leaves() {
    let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
    let mut app = App::boot(refused("magic-back", &bad));
    app.m.press_key(KEY_CLEAR);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
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
    tap(&mut app, F4);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit, "still running");
    same_frame("after the key", &app.frame(), &pack(&libraries_model(None)));
}

/// A library whose first assertion has more essential hypotheses than fm holds.
fn too_many_statements() -> Vec<u8> {
    let hypotheses: String = (0..33).map(|n| format!("h{n} $e |- ph $.\n")).collect();
    let source = format!(
        "$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n${{\n{hypotheses}big $a |- ph $.\n$}}\n"
    );
    crate::support::pack_all(&crate::support::parse(&source))
}

#[test]
fn a_first_assertion_fm_refuses_does_not_stop_the_libraries_screen() {
    let built = around("too-many", &too_many_statements());
    let mut app = App::boot(built);
    same_frame(
        "the stage row",
        &app.frame(),
        &pack(&libraries_model(built.shown().as_ref())),
    );
}

#[test]
fn a_page_that_holds_no_library_is_refused_by_the_reader_across_the_pages() {
    // app_lib_page's `sub 3` becomes `sub 1`: chunk 0 is taken to be on the draw page, one below
    // the entry page, which holds the canvas and fm and no library.
    let built = with_library();
    let page0 = &mcemu::decode_app(&built.file).unwrap()[0];
    let at = usize::from(built.label("app_lib_page") - 0x4000);
    assert_eq!(
        page0[at..at + 3],
        [0xD6, 0x03, 0xC9],
        "sub 3, ret at app_lib_page"
    );
    let mut app = App::boot_with(built, |m| {
        m.load_flash(BASE, at + 1, &[0x01]);
    });
    let want = pack(&error_model(ST_BAD));
    same_frame("a library page with no library", &app.frame(), &want);
}

#[test]
fn the_font_is_set_before_the_kernel_and_fm_are_opened() {
    const HOOK: u16 = 0xA600;
    const SNAP: u16 = 0xA620;
    // At each far entry the hook copies the canvas's font pointer aside and goes on.
    let hook = |far: u16, snap: u16| {
        let [f0, f1] = far.to_le_bytes();
        let [s0, s1] = snap.to_le_bytes();
        let ptr = (UI_VARS + 0x14).to_le_bytes();
        vec![0xE5, 0x2A, ptr[0], ptr[1], 0x22, s0, s1, 0xE1, 0xC3, f0, f1]
    };
    let built = with_library();
    let (reset, open) = (built.label("far_page2"), built.label("far_page1"));
    let app = App::boot_with(built, |m| {
        // What is in UI_VARS before the app runs must not reach the kernel or fm.
        m.load_ram(UI_VARS, &[0xEE; 0x1A]);
        m.load_ram(HOOK, &hook(reset, SNAP));
        m.load_ram(HOOK + 0x10, &hook(open, SNAP + 2));
        hook_stub(m, built, "far_kernel_reset", HOOK);
        hook_stub(m, built, "far_fm_open", HOOK + 0x10);
    });
    let mut app = app;
    let font = (app.label("ui_font") - 8).to_le_bytes();
    assert_eq!(
        app.m.read_ram(SNAP, 2),
        font,
        "when kernel_reset was called"
    );
    assert_eq!(app.m.read_ram(SNAP + 2, 2), font, "when fm_open was called");
    same_frame(
        "the screen",
        &app.frame(),
        &pack(&libraries_model(built.shown().as_ref())),
    );
}

#[test]
fn a_break_flagged_as_the_app_reads_clear_is_cleared_by_its_exit() {
    const HOOK: u16 = 0xA630;
    let built = plain();
    // The app is booted and waiting for a key first, so nothing flagged by the hook before it can be
    // taken by app_key's own branch. Then every time the app sets the font, which app_key does after
    // the OS returns a key and before it compares it, the OS "flags a break": set 4,(iy+9), then on
    // to ui_set_font. The flag is set after app_key's own test of it, so only the exit can clear it.
    let mut app = App::boot(built);
    let far = built.label("far_page1").to_le_bytes();
    app.m
        .load_ram(HOOK, &[0xFD, 0xCB, 0x09, 0xE6, 0xC3, far[0], far[1]]);
    hook_stub(&mut app.m, built, "far_ui_set_font", HOOK);
    assert_eq!(
        app.m.read_ram(0x89F9, 1)[0] & ON_INTERRUPT_BIT,
        0,
        "no break flagged yet"
    );
    app.m.press_key(KEY_CLEAR);
    assert_eq!(app.m.run_app(3 * TAP).stop, Stop::Exited);
    assert_eq!(
        app.m.read_ram(0x89F9, 1)[0] & ON_INTERRUPT_BIT,
        0,
        "the break flagged just before the exit"
    );
}
