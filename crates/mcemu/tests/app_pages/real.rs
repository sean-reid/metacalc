//! The same app on the reference OS, entered from the APPS menu with port 23h as TI-OS sets it for
//! three pages. The app's own path does the calls: no driver runs here.

use crate::app::*;
use crate::fixture;
use crate::realos::{GRAPH_FLAGS, KEY_CLEAR, RealOs, start_pixels};
use crate::ui_support::{pack, same_frame};

const DOWN: u8 = 0x01;
const GRAPH_DRAW: u8 = 0x01;
const ST_BAD: u8 = 3;

/// The start screen, a key with the font zeroed, and the exit, on the OS with `built`.
fn start_key_and_exit(built: &'static Built) {
    let library = built.library.as_deref();
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    let want = start_pixels(library, 0);
    assert!(os.screen() == want, "the start screen");
    same_frame(
        "the frame",
        &os.m.read_ram(UI_FRAME, FRAME_LEN),
        &pack(&crate::realos::start_model(library, 0)),
    );

    os.m.load_ram(UI_VARS, &[0; 0x1A]);
    os.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
    os.tap(DOWN, 150, 400);
    let font = (built.label("ui_font") - 8).to_le_bytes();
    assert_eq!(os.m.read_ram(UI_VARS + 0x14, 2), font, "the font pointer");
    assert!(os.screen() == want, "the screen after a key");

    os.tap(KEY_CLEAR, 150, 1500);
    assert_ne!(
        os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
        0,
        "the app left through its exit, which marks graphDraw"
    );
    assert!(os.screen() != want, "the OS has the screen back");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_app_without_a_library_draws_its_screen_and_leaves() {
    start_key_and_exit(plain());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_fixture_library_screen_is_the_models_and_a_key_redraws_it() {
    start_key_and_exit(with_library());
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn on_the_reference_os_the_prop_library_screen_is_the_models_and_a_key_redraws_it() {
    start_key_and_exit(shipped("prop"));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_refused_library_shows_its_status() {
    let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
    let built = refused("real-magic", &bad);
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    assert!(
        os.screen() == start_pixels(Some(&bad), ST_BAD),
        "the status"
    );
}
