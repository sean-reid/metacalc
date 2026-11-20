//! The same app on the reference OS, entered from the APPS menu with port 23h as TI-OS sets it
//! for three pages.

use crate::app::*;
use crate::data::answers_from_the_entry_page;
use crate::draw::frames_on_the_draw_page;
use crate::realos::{KEY_CLEAR, RealOs};
use crate::screen::{model, pixels};
use crate::ui_support::{pack, same_frame};

const DOWN: u8 = 0x01;

fn screen(os: &mut RealOs) -> Vec<bool> {
    os.screen()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_start_screen_is_the_models_a_key_redraws_it_and_clear_leaves() {
    let built = with_library();
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    let want = model();
    assert!(screen(&mut os) == pixels(&want), "the start screen");

    os.m.load_ram(UI_VARS, &[0; 0x1A]);
    os.m.load_ram(UI_FRAME, &[0xAA; FRAME_LEN]);
    os.tap(DOWN, 150, 400);
    let font = (built.label("ui_font") - 8).to_le_bytes();
    assert_eq!(os.m.read_ram(UI_VARS + 0x14, 2), font, "the font pointer");
    same_frame(
        "after a key",
        &os.m.read_ram(UI_FRAME, FRAME_LEN),
        &pack(&want),
    );
    assert!(screen(&mut os) == pixels(&want), "the screen after a key");

    os.tap(KEY_CLEAR, 150, 1500);
    assert!(screen(&mut os) != pixels(&want), "the app has left");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_entry_page_calls_the_data_page() {
    answers_from_the_entry_page(&mut App::on_real_os(with_library()));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_fm_on_the_draw_page_calls_the_reader_and_draws_the_models_frames() {
    let mut app = App::on_real_os(with_library());
    assert_eq!(app.far("far_kernel_reset", In::default()).status(), 0);
    let open = In {
        a: LIBRARY_PAGE,
        de: SCRATCH,
        ..In::default()
    };
    assert_eq!(app.far("far_lib_open", open).status(), 0);
    let drawn = frames_on_the_draw_page(&mut app, 6);
    assert!(drawn > 20, "{drawn} frames compared");
}
