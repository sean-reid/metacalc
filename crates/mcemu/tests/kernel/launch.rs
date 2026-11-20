use crate::app::build;
use crate::support::*;
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Lcd, Machine, STUB_ROM, Stop};
use mcui::canvas::Canvas;

const SLICE: u32 = 600_000;
const CLEAR: u8 = 0x0F;
const LIB_PAGE_ADDR: u16 = 0x9872;

/// Whether the screen is what the model's canvas draws for the start screen: the title bar and
/// the line of text on row 3.
fn shows_the_start_screen(lcd: &Lcd) -> bool {
    let mut canvas = Canvas::default();
    canvas.title("metacalc", "");
    canvas.text(1, 3, "CLEAR to quit");
    let want = canvas.into_bitmap();
    (0..LCD_HEIGHT).all(|y| (0..LCD_WIDTH).all(|x| lcd.pixel(x, y) == want.get(x, y)))
}

fn launched(file: &[u8], base: u8) -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(file, base).expect("the app installs");
    m.launch(base).expect("the app launches");
    m
}

fn assert_waits_for_clear_then_exits(m: &mut Machine) {
    assert_eq!(m.run_app(SLICE).stop, Stop::Limit);
    assert!(
        shows_the_start_screen(&m.lcd()),
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
            shows_the_start_screen(&m.lcd()),
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
    assert_waits_for_clear_then_exits(&mut m);
}

#[test]
fn the_app_with_a_library_does_the_same_and_leaves_its_library_page_in_ram() {
    let (_, library) = small_library();
    let app = build("run-multi-page", Some(&library));
    assert!(app.pages.len() > 3);
    let mut m = launched(&app.file, 0x30);
    assert_waits_for_clear_then_exits(&mut m);
    assert_eq!(m.read_ram(LIB_PAGE_ADDR, 1), [0x30 - 3]);
}
