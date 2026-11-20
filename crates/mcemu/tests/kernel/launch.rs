use crate::app::build;
use crate::realos::start_model;
use crate::support::*;
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Lcd, Machine, STUB_ROM, Stop};

const SLICE: u32 = 600_000;
const CLEAR: u8 = 0x0F;
const LIB_PAGE_ADDR: u16 = 0x9872;

/// Whether the screen is what the model's canvas draws for the app's start screen with `library`.
fn shows_the_start_screen(lcd: &Lcd, library: Option<&[u8]>) -> bool {
    let want = start_model(library, 0);
    (0..LCD_HEIGHT).all(|y| (0..LCD_WIDTH).all(|x| lcd.pixel(x, y) == want.get(x, y)))
}

fn launched(file: &[u8], base: u8) -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(file, base).expect("the app installs");
    m.launch(base).expect("the app launches");
    m
}

/// Every key but CLEAR redraws the same screen, so the unchanged screen after one shows only that a
/// redraw leaves the screen whole; that each key is read and redraws is shown by the app_pages
/// tests, which fill the frame with a pattern first.
fn assert_waits_for_clear_then_exits(m: &mut Machine, library: Option<&[u8]>) {
    assert_eq!(m.run_app(SLICE).stop, Stop::Limit);
    assert!(
        shows_the_start_screen(&m.lcd(), library),
        "the start screen is on screen"
    );
    let ink = m.lcd().lit();
    assert!(ink > 100);
    for (name, key) in [("enter", 0x09), ("2nd", 0x36), ("y=", 0x35), ("up", 0x04)] {
        m.press_key(key);
        assert_eq!(m.run_app(SLICE).stop, Stop::Limit, "{name} down");
        m.release_key(key);
        assert_eq!(m.run_app(SLICE).stop, Stop::Limit, "{name} up");
        assert!(
            shows_the_start_screen(&m.lcd(), library),
            "{name}: the screen is unchanged"
        );
    }
    m.press_key(CLEAR);
    assert_eq!(m.run_app(SLICE).stop, Stop::Exited);
}

#[test]
fn the_app_without_a_library_draws_its_start_screen_waits_for_clear_and_exits() {
    let app = build("run-one-page", None);
    assert_eq!(app.pages.len(), 3);
    let mut m = launched(&app.file, 0x69);
    assert_waits_for_clear_then_exits(&mut m, None);
}

#[test]
fn the_app_with_a_library_does_the_same_and_leaves_its_library_page_in_ram() {
    let (_, library) = small_library();
    let app = build("run-multi-page", Some(&library));
    assert!(app.pages.len() > 3);
    let mut m = launched(&app.file, 0x30);
    assert_waits_for_clear_then_exits(&mut m, Some(&library));
    assert_eq!(m.read_ram(LIB_PAGE_ADDR, 1), [0x30 - 3]);
}
