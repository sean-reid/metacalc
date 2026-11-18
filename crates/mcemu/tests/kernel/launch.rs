use crate::app::build;
use crate::support::*;
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Lcd, Machine, STUB_ROM, Stop};

const SLICE: u32 = 600_000;
const CLEAR: u8 = 0x0F;
const LIB_PAGE_ADDR: u16 = 0x9872;

/// The word on screen: eight 8x8 cells, the top left corner at (16, 28).
const GLYPHS: [[&str; 8]; 8] = [
    [
        "........", "##.#....", "#.#.#...", "#.#.#...", "#.#.#...", "#.#.#...", "#.#.#...",
        "........",
    ],
    [
        "........", ".###....", "#...#...", "#####...", "#.......", "#...#...", ".###....",
        "........",
    ],
    [
        ".#......", "####....", ".#......", ".#......", ".#......", ".#..#...", "..##....",
        "........",
    ],
    [
        "........", ".###....", "....#...", ".####...", "#...#...", "#..##...", ".##.#...",
        "........",
    ],
    [
        "........", ".###....", "#...#...", "#.......", "#.......", "#...#...", ".###....",
        "........",
    ],
    [
        "........", ".###....", "....#...", ".####...", "#...#...", "#..##...", ".##.#...",
        "........",
    ],
    [
        ".##.....", "..#.....", "..#.....", "..#.....", "..#.....", "..#.....", ".###....",
        "........",
    ],
    [
        "........", ".###....", "#...#...", "#.......", "#.......", "#...#...", ".###....",
        "........",
    ],
];

fn shows_the_word(lcd: &Lcd) -> bool {
    (0..LCD_HEIGHT).all(|y| {
        (0..LCD_WIDTH).all(|x| {
            let lit = (28..36).contains(&y)
                && (16..80).contains(&x)
                && GLYPHS[(x - 16) / 8][y - 28].as_bytes()[(x - 16) % 8] == b'#';
            lcd.pixel(x, y) == lit
        })
    })
}

fn launched(file: &[u8], base: u8) -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.install_app(file, base).expect("the app installs");
    m.launch(base).expect("the app launches");
    m
}

fn assert_waits_for_clear_then_exits(m: &mut Machine) {
    assert_eq!(m.run_app(SLICE).stop, Stop::Limit);
    assert!(shows_the_word(&m.lcd()), "the word is on screen");
    let ink = m.lcd().lit();
    assert!(ink > 100);
    for (name, key) in [("enter", 0x09), ("2nd", 0x36), ("y=", 0x35), ("up", 0x04)] {
        m.press_key(key);
        assert_eq!(m.run_app(SLICE).stop, Stop::Limit, "{name} down");
        m.release_key(key);
        assert_eq!(m.run_app(SLICE).stop, Stop::Limit, "{name} up");
        assert!(shows_the_word(&m.lcd()), "{name}: the screen is unchanged");
    }
    m.press_key(CLEAR);
    assert_eq!(m.run_app(SLICE).stop, Stop::Exited);
}

#[test]
fn the_one_page_app_draws_its_word_waits_for_clear_and_exits() {
    let app = build("run-one-page", None);
    assert_eq!(app.pages.len(), 1);
    let mut m = launched(&app.file, 0x69);
    assert_waits_for_clear_then_exits(&mut m);
}

#[test]
fn the_multi_page_app_does_the_same_and_leaves_its_library_page_in_ram() {
    let (_, library) = small_library();
    let app = build("run-multi-page", Some(&library));
    assert!(app.pages.len() > 1);
    let mut m = launched(&app.file, 0x30);
    assert_waits_for_clear_then_exits(&mut m);
    assert_eq!(m.read_ram(LIB_PAGE_ADDR, 1), [0x30 - 1]);
}
