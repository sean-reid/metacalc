use mcemu::{LCD_HEIGHT, LCD_WIDTH, Lcd, Machine, STUB_ROM, Speed, Stop};

const GETCSC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_getcsc.bin"));
const EXIT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_exit.bin"));
const BCALL_MISSING: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_bcall_missing.bin"));
const BJUMP_MISSING: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_bjump_missing.bin"));
const GR_BUF_CPY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy.bin"));
const CLR_LCD_FULL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full.bin"));
const PRESERVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_preserve.bin"));

const ENTRY: u16 = 0x9000;
const RESULT: u16 = 0xA000;
const PLOT_S_SCREEN: u16 = 0x9340;
const LIMIT: u32 = 20_000_000;

/// Every key the matrix has except ON, which _GetCSC does not report. The codes are the
/// ti83plus.inc skX values, and the real ROM returned each of them in the local differential run.
const KEYS: [(&str, u8); 49] = [
    ("down", 0x01),
    ("left", 0x02),
    ("right", 0x03),
    ("up", 0x04),
    ("enter", 0x09),
    ("add", 0x0A),
    ("sub", 0x0B),
    ("mul", 0x0C),
    ("div", 0x0D),
    ("power", 0x0E),
    ("clear", 0x0F),
    ("chs", 0x11),
    ("3", 0x12),
    ("6", 0x13),
    ("9", 0x14),
    ("rparen", 0x15),
    ("tan", 0x16),
    ("vars", 0x17),
    ("decpnt", 0x19),
    ("2", 0x1A),
    ("5", 0x1B),
    ("8", 0x1C),
    ("lparen", 0x1D),
    ("cos", 0x1E),
    ("prgm", 0x1F),
    ("stat", 0x20),
    ("0", 0x21),
    ("1", 0x22),
    ("4", 0x23),
    ("7", 0x24),
    ("comma", 0x25),
    ("sin", 0x26),
    ("matrix", 0x27),
    ("graphvar", 0x28),
    ("store", 0x2A),
    ("ln", 0x2B),
    ("log", 0x2C),
    ("square", 0x2D),
    ("recip", 0x2E),
    ("math", 0x2F),
    ("alpha", 0x30),
    ("graph", 0x31),
    ("trace", 0x32),
    ("zoom", 0x33),
    ("window", 0x34),
    ("yequ", 0x35),
    ("2nd", 0x36),
    ("mode", 0x37),
    ("del", 0x38),
];
const ON: u8 = 0x29;

fn machine_with(code: &[u8]) -> Machine {
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    m.load_ram(ENTRY, code);
    m
}

fn getcsc(m: &mut Machine) -> u8 {
    let run = m.run(ENTRY, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    m.read_ram(RESULT, 1)[0]
}

#[test]
fn getcsc_reports_no_key_as_zero() {
    assert_eq!(getcsc(&mut machine_with(GETCSC)), 0);
}

#[test]
fn getcsc_returns_the_scan_code_of_every_key() {
    for (name, code) in KEYS {
        let mut m = machine_with(GETCSC);
        m.press_key(code);
        assert_eq!(getcsc(&mut m), code, "{name}");
    }
}

#[test]
fn getcsc_ignores_the_on_key() {
    let mut m = machine_with(GETCSC);
    m.press_key(ON);
    assert_eq!(getcsc(&mut m), 0);
}

#[test]
fn getcsc_reports_a_held_key_once() {
    let mut m = machine_with(GETCSC);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09);
    assert_eq!(getcsc(&mut m), 0);
    assert_eq!(getcsc(&mut m), 0);
    m.release_key(0x09);
    assert_eq!(getcsc(&mut m), 0);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09);
}

#[test]
fn getcsc_reports_two_keys_in_sequence() {
    let mut m = machine_with(GETCSC);
    m.press_key(0x35);
    assert_eq!(getcsc(&mut m), 0x35);
    m.release_key(0x35);
    m.press_key(0x0F);
    assert_eq!(getcsc(&mut m), 0x0F);
    m.release_key(0x0F);
    m.press_key(0x35);
    assert_eq!(getcsc(&mut m), 0x35);
}

#[test]
fn getcsc_reports_the_lowest_code_while_two_keys_are_down() {
    let mut m = machine_with(GETCSC);
    m.press_key(0x35);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09);
    assert_eq!(getcsc(&mut m), 0);
}

#[test]
fn a_bcall_keeps_the_registers_and_the_stack() {
    let mut m = machine_with(PRESERVE);
    let run = m.run(ENTRY, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    let words: Vec<u16> = m
        .read_ram(RESULT, 18)
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    assert_eq!(words[..4], [0x1234, 0x5678, 0x9ABC, 0xDEF0]);
    assert_eq!(words[4], words[5], "the stack pointer");
    assert_eq!(words[6..], [0x1111, 0x2222, 0x3333]);
}

#[test]
fn bjump_to_force_cmd_no_char_ends_the_run_as_a_clean_exit() {
    let mut m = machine_with(EXIT);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Exited);
}

#[test]
fn an_unsupported_bcall_stops_the_machine_naming_the_address() {
    let mut m = machine_with(BCALL_MISSING);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::UnsupportedBcall(0x4A4A));
}

#[test]
fn an_unsupported_bjump_stops_the_machine_naming_the_address() {
    let mut m = machine_with(BJUMP_MISSING);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::UnsupportedBjump(0x4B4B));
}

fn buffer() -> Vec<u8> {
    let mut state = 0x9E37_79B9u32;
    (0..LCD_WIDTH / 8 * LCD_HEIGHT)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 7) as u8
        })
        .collect()
}

fn lcd_matches(lcd: &Lcd, buf: &[u8]) -> bool {
    (0..LCD_HEIGHT).all(|y| {
        (0..LCD_WIDTH).all(|x| lcd.pixel(x, y) == (buf[y * 12 + x / 8] >> (7 - x % 8) & 1 == 1))
    })
}

#[test]
fn gr_buf_cpy_puts_the_graph_buffer_on_the_lcd_at_both_speeds() {
    let buf = buffer();
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        let mut m = machine_with(GR_BUF_CPY);
        m.set_speed(speed);
        m.load_ram(PLOT_S_SCREEN, &buf);
        assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
        let lcd = m.lcd();
        assert!(lcd.lit() > 1000, "{speed:?}: the pattern is dense");
        assert!(lcd_matches(&lcd, &buf), "{speed:?}");
        assert_eq!(m.read_ram(PLOT_S_SCREEN, buf.len()), buf, "{speed:?}");
    }
}

#[test]
fn clr_lcd_full_blanks_the_lcd_and_leaves_the_graph_buffer() {
    let buf = vec![0xFF; 768];
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        let mut m = machine_with(GR_BUF_CPY);
        m.set_speed(speed);
        m.load_ram(PLOT_S_SCREEN, &buf);
        assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
        assert_eq!(m.lcd().lit(), LCD_WIDTH * LCD_HEIGHT);
        m.load_ram(ENTRY, CLR_LCD_FULL);
        assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
        assert_eq!(m.lcd().lit(), 0, "{speed:?}");
        assert_eq!(m.read_ram(PLOT_S_SCREEN, buf.len()), buf, "{speed:?}");
    }
}
