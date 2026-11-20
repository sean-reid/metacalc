//! app_key (app/src/keys.inc) against the stub OS and the reference OS: the key it returns for
//! each physical key, and what it restores after the wait.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/realos.rs"]
mod realos;

use keytables::{ALPHA, PLAIN, SECOND};
use mcemu::{LCD_HEIGHT, LCD_WIDTH, Machine, STUB_ROM, Stop};
use mcglyph::z80::Table;
use mcui::Key;
use mcui::canvas::Canvas;
use realos::*;

const KEYS_APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/keys_app.8xk"));
const LOG_APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/get_key_log.8xk"));

const STUB_BASE: u8 = 0x30;
const RAM_BASE: usize = 0x8000;
/// What both apps write: the number of returns, and then the codes for the log app or A and B
/// for the keys app; the keys app draws the string at MSG.
const COUNT: usize = 0xA800 - RAM_BASE;
const LOG: usize = COUNT + 1;
const MSG: usize = 0xA900 - RAM_BASE;
const UI_VARS: usize = 0x86EC - RAM_BASE;
const SAVE_SCREEN_SIZE: usize = 768;
/// Where the UI keeps its font pointer inside UI_VARS (ui/ram.inc).
const FONT_POINTER: usize = 0x14;
/// Bit 4 of port 3 reads set once the OS has run its auto power down.
const POWER_DOWN: u8 = 0x10;

/// Port 4: the ON interrupt latch, and the level, which reads clear while ON is down.
const ON_LATCH: u8 = 0x01;
const ON_LEVEL: u8 = 0x08;
const SCAN_ALPHA: u8 = 0x30;
const SCAN_2ND: u8 = 0x36;
const SCAN_MODE: u8 = 0x37;
const SCAN_LAST: u8 = 0x38;
/// Scancodes with no key behind them, and the two modifiers.
const NO_KEY: [u8; 7] = [5, 6, 7, 8, 0x10, 0x18, 0x29];

/// The first and last capital letter codes `_GetKey` returns after ALPHA.
const CAP_A: u8 = 0x9A;
const CAP_Z: u8 = 0xB3;

/// The model's keys in key.rs order, the ones `app_key` can return, and the code each gets:
/// its place in this list from 1, and 0 for a key it ignores. The match below fails to compile
/// when the model gains a key this test does not place.
const MODEL: [Key; 12] = [
    Key::Up,
    Key::Down,
    Key::Left,
    Key::Right,
    Key::Enter,
    Key::Clear,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::Letter('A'),
];
const QUIT: u8 = MODEL.len() as u8 + 1;

fn code(key: Key) -> u8 {
    let at = MODEL
        .iter()
        .position(|&k| match (k, key) {
            (Key::Letter(_), Key::Letter(_)) => true,
            (a, b) => a == b,
        })
        .expect("a key app_key returns");
    at as u8 + 1
}

/// The scancode of the key that carries a model key, as the faceplate has it; none for a letter,
/// which the OS tables place, and for ON, which `app_key` never returns.
fn scancode(key: Key) -> Option<u8> {
    match key {
        Key::Down => Some(0x01),
        Key::Left => Some(0x02),
        Key::Right => Some(0x03),
        Key::Up => Some(0x04),
        Key::Enter => Some(0x09),
        Key::Clear => Some(0x0F),
        Key::F5 => Some(0x31),
        Key::F4 => Some(0x32),
        Key::F3 => Some(0x33),
        Key::F2 => Some(0x34),
        Key::F1 => Some(0x35),
        Key::On | Key::Letter(_) => None,
    }
}

/// The letter a scancode's key carries: the capital `_GetKey` returns for it after ALPHA.
fn letter(scan: u8) -> Option<char> {
    let os = ALPHA[usize::from(scan) - 1];
    (CAP_A..=CAP_Z)
        .contains(&os)
        .then(|| char::from(b'A' + os - CAP_A))
}

fn keys() -> impl Iterator<Item = u8> {
    (1..=SCAN_LAST).filter(|s| !NO_KEY.contains(s) && ![SCAN_ALPHA, SCAN_2ND].contains(s))
}

enum Rig {
    Stub(Machine),
    Real(RealOs),
}

impl Rig {
    fn stub(app: &[u8]) -> Rig {
        let mut m = Machine::new(STUB_ROM).unwrap();
        m.install_app(app, STUB_BASE).unwrap();
        m.launch(STUB_BASE).unwrap();
        let mut rig = Rig::Stub(m);
        rig.run_ms(20);
        rig
    }

    fn real(app: &[u8]) -> Rig {
        let mut os = RealOs::boot(app);
        os.tap(KEY_APPS, 150, 600);
        os.tap(KEY_2, 150, 300);
        os.run_ms(500);
        Rig::Real(os)
    }

    fn m(&mut self) -> &mut Machine {
        match self {
            Rig::Stub(m) => m,
            Rig::Real(os) => &mut os.m,
        }
    }

    fn run_ms(&mut self, ms: u32) {
        match self {
            Rig::Stub(m) => assert_eq!(m.run_app(ms * 6000).stop, Stop::Limit),
            Rig::Real(os) => os.run_ms(ms),
        }
    }

    fn tap(&mut self, key: u8) {
        let ms = if matches!(self, Rig::Stub(_)) {
            20
        } else {
            150
        };
        self.m().press_key(key);
        self.run_ms(ms);
        self.m().release_key(key);
        self.run_ms(ms * 2);
    }

    fn ram(&mut self, at: usize, len: usize) -> Vec<u8> {
        self.m().ram_page(1)[at..at + len].to_vec()
    }

    fn poke(&mut self, at: usize, bytes: &[u8]) {
        self.m().ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn count(&mut self) -> u8 {
        self.ram(COUNT, 1)[0]
    }

    fn on_flags(&mut self) -> u8 {
        self.ram(ON_FLAGS, 1)[0]
    }

    /// The lit pixels, row by row.
    fn screen(&mut self) -> Vec<bool> {
        let lcd = self.m().lcd();
        (0..LCD_HEIGHT)
            .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| lcd.pixel(x, y))
            .collect()
    }

    /// Taps `presses` in turn and returns what the log app recorded from the last of them; the
    /// count is zeroed first.
    fn press_and_read(&mut self, presses: &[u8], len: usize) -> Vec<u8> {
        self.poke(COUNT, &[0]);
        for &p in presses {
            self.tap(p);
        }
        let n = usize::from(self.count());
        self.ram(LOG, n.min(len))
    }
}

/// What the log app should have recorded for `presses` under `modifier`: nothing for a code of 0.
fn raw_expected(table: &[u8; 56], scan: u8) -> Vec<u8> {
    match table[usize::from(scan) - 1] {
        0 => vec![],
        c => vec![c],
    }
}

fn raw_codes(rig: &mut Rig) {
    for (name, modifier, table) in [
        ("plain", None, &PLAIN),
        ("2nd", Some(SCAN_2ND), &SECOND),
        ("ALPHA", Some(SCAN_ALPHA), &ALPHA),
    ] {
        for scan in keys() {
            let presses: Vec<u8> = modifier.into_iter().chain([scan]).collect();
            assert_eq!(
                rig.press_and_read(&presses, 4),
                raw_expected(table, scan),
                "{name} key {scan:02X}"
            );
        }
    }
}

#[test]
fn the_stubs_tables_equal_the_tables_the_reference_os_test_checks() {
    let mut rig = Rig::stub(LOG_APP);
    raw_codes(&mut rig);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_returns_the_codes_in_the_tables() {
    let mut rig = Rig::real(LOG_APP);
    raw_codes(&mut rig);
}

/// The digit or symbol printed on the face of the key at `scan`, as the faceplate has it: the
/// ten digits, `-` on the subtract key and `.` on the dot key.
fn face(scan: u8) -> Option<char> {
    match scan {
        0x21 => Some('0'),
        0x22 => Some('1'),
        0x1A => Some('2'),
        0x12 => Some('3'),
        0x23 => Some('4'),
        0x1B => Some('5'),
        0x13 => Some('6'),
        0x24 => Some('7'),
        0x1C => Some('8'),
        0x14 => Some('9'),
        0x0B => Some('-'),
        0x19 => Some('.'),
        _ => None,
    }
}

/// The (A, B) `app_key` returns for the key at `scan` after `modifier`, from the model: a plain
/// press of a key that carries a letter gives the letter, else its face character; after ALPHA
/// the face character and nothing else; after 2nd nothing, but QUIT and Clear.
fn expected(modifier: Option<u8>, scan: u8) -> (u8, u8) {
    let by_scan = MODEL
        .iter()
        .copied()
        .find(|&k| scancode(k) == Some(scan))
        .map(code);
    let character = |c: char| (code(Key::Letter(c)), c as u8);
    match modifier {
        Some(SCAN_2ND) if scan == SCAN_MODE => (QUIT, 0),
        Some(SCAN_2ND) if scan == 0x0F => (code(Key::Clear), 0),
        Some(SCAN_2ND) => (0, 0),
        Some(_) => match (by_scan, face(scan)) {
            (Some(c), _) => (c, 0),
            (None, Some(c)) => character(c),
            (None, None) => (0, 0),
        },
        None => match (by_scan, letter(scan), face(scan)) {
            (Some(c), _, _) => (c, 0),
            (None, Some(l), _) => character(l),
            (None, None, Some(c)) => character(c),
            (None, None, None) => (0, 0),
        },
    }
}

fn mapping(rig: &mut Rig) {
    for (name, modifier) in [
        ("plain", None),
        ("2nd", Some(SCAN_2ND)),
        ("ALPHA", Some(SCAN_ALPHA)),
    ] {
        for scan in keys() {
            let table = match modifier {
                Some(SCAN_2ND) => &SECOND,
                Some(_) => &ALPHA,
                None => &PLAIN,
            };
            if table[usize::from(scan) - 1] == 0 {
                continue;
            }
            let presses: Vec<u8> = modifier.into_iter().chain([scan]).collect();
            rig.press_and_read(&presses, 0);
            let got = (rig.ram(LOG, 1)[0], rig.ram(LOG + 1, 1)[0]);
            assert_eq!(rig.count(), 1, "{name} key {scan:02X} returned once");
            assert_eq!(got, expected(modifier, scan), "{name} key {scan:02X}");
        }
    }
}

#[test]
fn every_model_key_letter_digit_and_symbol_maps_and_the_rest_give_none() {
    let mut rig = Rig::stub(KEYS_APP);
    mapping(&mut rig);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_every_model_key_letter_digit_and_symbol_maps_and_the_rest_give_none() {
    let mut rig = Rig::real(KEYS_APP);
    mapping(&mut rig);
}

#[test]
fn the_letters_are_the_26_the_keys_carry_and_the_model_keys_are_where_the_faceplate_has_them() {
    let letters: Vec<char> = (1..=SCAN_LAST).filter_map(letter).collect();
    let mut sorted = letters.clone();
    sorted.sort();
    assert_eq!(sorted, ('A'..='Z').collect::<Vec<_>>());
    let mut faces: Vec<char> = (1..=SCAN_LAST).filter_map(face).collect();
    faces.sort();
    assert_eq!(faces, "-.0123456789".chars().collect::<Vec<_>>());
    for key in MODEL.iter().filter_map(|&k| scancode(k)) {
        assert_eq!(letter(key), None, "a model key is not a letter key");
        assert_ne!(PLAIN[usize::from(key) - 1], 0);
    }
}

/// The screen the keys app draws for `text`: the frame cleared and the string on row 3.
fn drawn(text: &str) -> Vec<bool> {
    let mut canvas = Canvas::default();
    canvas.text(0, 3, text);
    let bitmap = canvas.into_bitmap();
    (0..LCD_HEIGHT)
        .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| bitmap.get(x, y))
        .collect()
}

fn message(rig: &mut Rig, text: &str) {
    let mut codes = Table::builtin().encode(text);
    codes.push(0);
    rig.poke(MSG, &codes);
}

fn same_screen(what: &str, got: &[bool], want: &[bool]) {
    if got == want {
        return;
    }
    let row = |s: &[bool], y: usize| -> String {
        (0..LCD_WIDTH)
            .map(|x| if s[y * LCD_WIDTH + x] { '#' } else { '.' })
            .collect()
    };
    let mut text = format!("{what}: the LCD (left) differs from the model (right)\n");
    for y in 0..LCD_HEIGHT {
        let (g, w) = (row(got, y), row(want, y));
        if g.contains('#') || w.contains('#') {
            text += &format!("{y:2} {g} {w} {}\n", if g == w { ' ' } else { '!' });
        }
    }
    panic!("{text}");
}

#[test]
fn text_drawn_after_the_wait_is_right_when_the_variables_were_zeroed_during_it() {
    let mut rig = Rig::stub(KEYS_APP);
    message(&mut rig, "Key 12 x");
    assert_eq!(rig.count(), 0, "the app is waiting");
    rig.poke(UI_VARS, &[0; SAVE_SCREEN_SIZE]);
    rig.tap(0x04);
    assert_eq!(rig.count(), 1);
    assert_eq!(rig.ram(LOG, 1), [code(Key::Up)]);
    let got = rig.screen();
    same_screen("after the zeroed wait", &got, &drawn("Key 12 x"));
    assert!(got.iter().any(|&p| p), "something is drawn");
}

#[test]
fn a_press_of_on_is_cleared_and_waited_out_inside_the_wait() {
    let mut rig = Rig::stub(KEYS_APP);
    assert!(!on_seen(&mut rig), "nothing is pending to start with");
    rig.m().press_key(Machine::KEY_ON);
    rig.run_ms(30);
    assert_ne!(rig.on_flags() & ON_INTERRUPT, 0, "the OS flagged the press");
    assert!(on_seen(&mut rig), "a long job would see the press held");
    rig.run_ms(30);
    rig.m().release_key(Machine::KEY_ON);
    rig.run_ms(30);
    assert_eq!(rig.count(), 0, "app_key did not return for ON");
    assert_eq!(rig.on_flags() & ON_INTERRUPT, 0, "the break is cleared");
    assert_eq!(rig.m().in_port(4) & ON_LATCH, 0, "no ON latch is up");
    assert!(!on_seen(&mut rig), "a long job would see no ON");
    rig.tap(0x04);
    assert_eq!(rig.count(), 1);
    assert_eq!(rig.ram(LOG, 1), [code(Key::Up)], "the next key is Up");
}

/// What a long job's poll reads (probe/kerndriver.inc, kd_on_seen): ON down on port 4 bit 3, the
/// latch in bit 0, or the OS break flag.
fn on_seen(rig: &mut Rig) -> bool {
    let port = rig.m().in_port(4) ^ ON_LEVEL;
    port & (ON_LATCH | ON_LEVEL) != 0 || rig.on_flags() & ON_INTERRUPT != 0
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_returns_zeros_until_the_break_is_cleared() {
    let mut rig = Rig::real(LOG_APP);
    rig.m().press_key(Machine::KEY_ON);
    rig.run_ms(150);
    rig.m().release_key(Machine::KEY_ON);
    rig.run_ms(300);
    let n = rig.count();
    assert!(n > 20, "_GetKey returned {n} times for one press");
    assert!(rig.ram(LOG, usize::from(n)).iter().all(|&c| c == 0));
    assert_ne!(rig.on_flags() & ON_INTERRUPT, 0);
    let before = rig.ram(LOG, 1);
    let flags = rig.on_flags();
    rig.poke(ON_FLAGS, &[flags & !ON_INTERRUPT]);
    rig.poke(COUNT, &[0]);
    rig.run_ms(500);
    assert_eq!(rig.count(), 0, "it waits again once the flag is clear");
    assert_eq!(before, [0]);
    rig.tap(0x04);
    assert_eq!(rig.ram(LOG, 1), [0x03]);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_tap_or_a_hold_of_on_leaves_no_break_and_no_return() {
    let mut rig = Rig::real(KEYS_APP);
    rig.tap(Machine::KEY_ON);
    assert_eq!(rig.count(), 0, "a tap");
    assert_eq!(rig.on_flags() & ON_INTERRUPT, 0);
    rig.m().press_key(Machine::KEY_ON);
    rig.run_ms(1500);
    assert_eq!(rig.count(), 0, "held");
    rig.m().release_key(Machine::KEY_ON);
    rig.run_ms(500);
    assert_eq!(rig.count(), 0, "released");
    assert_eq!(rig.on_flags() & ON_INTERRUPT, 0);
    assert_eq!(
        rig.m().in_port(4) & ON_LATCH,
        0,
        "no ON interrupt is latched"
    );
    assert!(!on_seen(&mut rig), "a long job would see no ON");
    rig.tap(0x01);
    assert_eq!(rig.count(), 1);
    assert_eq!(rig.ram(LOG, 1), [code(Key::Down)]);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_alpha_lock_gives_the_face_characters_and_leaves_the_model_keys() {
    let mut rig = Rig::real(KEYS_APP);
    rig.tap(SCAN_2ND);
    rig.tap(SCAN_ALPHA);
    for scan in keys().filter(|&s| ALPHA[usize::from(s) - 1] != 0) {
        rig.poke(COUNT, &[0]);
        rig.tap(scan);
        let got = (rig.ram(LOG, 1)[0], rig.ram(LOG + 1, 1)[0]);
        assert_eq!(rig.count(), 1, "key {scan:02X}");
        assert_eq!(
            got,
            expected(Some(SCAN_ALPHA), scan),
            "locked key {scan:02X}"
        );
    }
    rig.tap(SCAN_ALPHA);
    rig.poke(COUNT, &[0]);
    rig.tap(0x2F);
    assert_eq!(
        rig.ram(LOG, 1),
        [code(Key::Letter('A'))],
        "Math is A with no ALPHA"
    );
}

/// Runs the real OS until its auto power down has fired, which zeroes saveSScreen, and returns
/// the emulated seconds that took.
fn power_down(rig: &mut Rig) {
    let mut seconds = 0;
    while rig.m().in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        rig.run_ms(5_000);
        seconds += 5;
    }
    assert!(seconds >= 200, "auto power down at {seconds} s");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn after_auto_power_down_and_power_on_a_key_gives_its_code_and_the_text_is_right() {
    let mut rig = Rig::real(KEYS_APP);
    message(&mut rig, "After APD");
    let font = rig.ram(UI_VARS + FONT_POINTER, 2);
    assert_ne!(font, [0, 0], "the app set the font");
    power_down(&mut rig);
    assert_ne!(
        rig.ram(UI_VARS + FONT_POINTER, 2),
        font,
        "the power down took the font pointer"
    );
    assert_eq!(rig.count(), 0, "app_key is still waiting");
    rig.tap(Machine::KEY_ON);
    rig.run_ms(3000);
    assert_eq!(rig.count(), 0, "powering on is not a key");
    rig.tap(0x04);
    assert_eq!(rig.count(), 1);
    assert_eq!(rig.ram(LOG, 1), [code(Key::Up)]);
    let got = rig.screen();
    same_screen("after power on", &got, &drawn("After APD"));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_2nd_off_leaves_the_app_and_it_does_not_come_back() {
    let mut rig = Rig::real(KEYS_APP);
    rig.tap(SCAN_2ND);
    rig.m().press_key(Machine::KEY_ON);
    rig.run_ms(150);
    rig.m().release_key(Machine::KEY_ON);
    rig.run_ms(1500);
    assert_eq!(rig.m().in_port(3) & POWER_DOWN, POWER_DOWN, "powered down");
    assert_eq!(rig.screen().iter().filter(|&&p| p).count(), 0, "screen off");
    rig.tap(Machine::KEY_ON);
    rig.run_ms(8000);
    let home = rig.screen();
    rig.tap(0x1A);
    rig.run_ms(500);
    rig.tap(0x04);
    assert_eq!(rig.count(), 0, "app_key never returned");
    assert_ne!(rig.screen(), home, "the home screen took the 2");
}
