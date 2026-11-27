use mcemu::{LCD_HEIGHT, LCD_WIDTH, Lcd, Machine, STUB_ROM, Speed, Stop};

const GETCSC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_getcsc.bin"));
const GETCSC_ONCE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_getcsc_once.bin"));
const EXIT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_exit.bin"));
const BCALL_MISSING: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_bcall_missing.bin"));
const BJUMP_MISSING: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_bjump_missing.bin"));
const GR_BUF_CPY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy.bin"));
const CLR_LCD_FULL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full.bin"));
const CLOBBER_GETCSC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clobber_getcsc.bin"));
const CLOBBER_CLR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clobber_clr_lcd_full.bin"));
const CLOBBER_GR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clobber_gr_buf_cpy.bin"));
const CLOBBER_PUT_C: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_clobber_put_c.bin"));
const CLOBBER_INTERRUPT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clobber_interrupt.bin"));
const GETCSC_BLIND: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_getcsc_blind.bin"));
const IFF_GETCSC_OFF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_getcsc_off.bin"));
const IFF_GETCSC_ON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_getcsc_on.bin"));
const IFF_PUT_C_OFF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_put_c_off.bin"));
const IFF_PUT_C_ON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_put_c_on.bin"));
const IFF_CLR_OFF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_clr_lcd_full_off.bin"));
const IFF_CLR_ON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_clr_lcd_full_on.bin"));
const IFF_GR_ON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_gr_buf_cpy_on.bin"));
const IFF_GR_OFF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_iff_gr_buf_cpy_off.bin"));
const GR_BUF_CPY_MODE4: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_mode4.bin"));
const GR_BUF_CPY_MODE6: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_mode6.bin"));
const GR_BUF_CPY_MODE7: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_mode7.bin"));
const CLR_LCD_FULL_MODE4: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_mode4.bin"));
const CLR_LCD_FULL_MODE6: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_mode6.bin"));
const CLR_LCD_FULL_MODE7: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_mode7.bin"));
const GR_BUF_CPY_INTS_MODE4: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_ints_mode4.bin"));
const GR_BUF_CPY_INTS_MODE5: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_ints_mode5.bin"));
const GR_BUF_CPY_INTS_MODE6: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_ints_mode6.bin"));
const GR_BUF_CPY_INTS_MODE7: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_gr_buf_cpy_ints_mode7.bin"));
const CLR_LCD_FULL_INTS_MODE4: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_ints_mode4.bin"));
const CLR_LCD_FULL_INTS_MODE5: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_ints_mode5.bin"));
const CLR_LCD_FULL_INTS_MODE6: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_ints_mode6.bin"));
const CLR_LCD_FULL_INTS_MODE7: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/os_clr_lcd_full_ints_mode7.bin"));
const ON_IRQ_FLAGS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_on_irq_flags.bin"));
const ON_IRQ_OTHER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_on_irq_other.bin"));
const RELIANT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/os_reliant.bin"));

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
    m.out_port(0x03, 0x0B);
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
    for call in 1..=3 {
        assert_eq!(getcsc(&mut m), 0, "call {call} with no key down");
    }
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09);
}

#[test]
fn getcsc_ignores_the_same_key_pressed_again_until_the_fifth_scan_with_no_key_down() {
    let mut m = machine_with(GETCSC_ONCE);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09);
    m.release_key(0x09);
    for scan in 1..=4 {
        assert_eq!(getcsc(&mut m), 0, "scan {scan} with no key down");
    }
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0, "the same key, one scan too soon");
    m.release_key(0x09);
    for scan in 1..=5 {
        assert_eq!(getcsc(&mut m), 0, "scan {scan} with no key down");
    }
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0x09, "the same key after the fifth scan");
    m.release_key(0x09);
    for scan in 1..=4 {
        assert_eq!(getcsc(&mut m), 0, "scan {scan} with no key down");
    }
    m.press_key(0x35);
    assert_eq!(getcsc(&mut m), 0x35, "a different key at once");
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

/// TI-OS 2.43 and 2.55MP both report nothing while two keys are down from the same scan, and the
/// one still down once the other is up, whichever code is the lower.
#[test]
fn getcsc_reports_nothing_while_two_keys_went_down_together_then_the_one_left_down() {
    for (first, second) in [(0x35, 0x09), (0x09, 0x35)] {
        let mut m = machine_with(GETCSC);
        m.press_key(first);
        m.press_key(second);
        assert_eq!(getcsc(&mut m), 0);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(first);
        assert_eq!(getcsc(&mut m), second);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(second);
        assert_eq!(getcsc(&mut m), 0);
    }
}

#[test]
fn getcsc_reports_nothing_when_two_keys_go_up_together() {
    let mut m = machine_with(GETCSC);
    m.press_key(0x35);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0);
    m.release_key(0x35);
    m.release_key(0x09);
    assert_eq!(getcsc(&mut m), 0);
}

#[test]
fn getcsc_reports_nothing_with_three_keys_down_until_one_is_left() {
    let mut m = machine_with(GETCSC);
    for key in [0x09, 0x0F, 0x35] {
        m.press_key(key);
    }
    assert_eq!(getcsc(&mut m), 0);
    m.release_key(0x35);
    assert_eq!(getcsc(&mut m), 0);
    m.release_key(0x0F);
    assert_eq!(getcsc(&mut m), 0x09);
}

/// What the real OS did for these sequences under the key-logging app: a key pressed while
/// another is down is never reported, whichever code is the lower, and it is reported once the
/// first is up if it is still down.
#[test]
fn getcsc_ignores_a_key_pressed_and_released_while_another_is_held() {
    for (held, other) in [(0x35, 0x0F), (0x0F, 0x35)] {
        let mut m = machine_with(GETCSC);
        m.press_key(held);
        assert_eq!(getcsc(&mut m), held);
        m.press_key(other);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(other);
        assert_eq!(getcsc(&mut m), 0);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(held);
        assert_eq!(getcsc(&mut m), 0);
    }
}

#[test]
fn getcsc_reports_a_second_key_once_the_first_is_released_whichever_code_is_lower() {
    for (first, second) in [(0x09, 0x36), (0x36, 0x09)] {
        let mut m = machine_with(GETCSC);
        m.press_key(first);
        assert_eq!(getcsc(&mut m), first);
        m.press_key(second);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(first);
        assert_eq!(getcsc(&mut m), second);
        assert_eq!(getcsc(&mut m), 0);
        m.release_key(second);
        assert_eq!(getcsc(&mut m), 0);
    }
}

/// AF, BC, DE, HL, IX, IY, SP after the call, then BC', DE', HL', AF', then SP before it. The
/// interrupt sources are masked, so only the call itself changes anything.
fn registers_after(code: &[u8]) -> Vec<u16> {
    let mut m = machine_with(code);
    m.out_port(0x03, 0);
    recorded(&mut m)
}

/// The same with the sources on, for a program that waits for an interrupt.
fn registers_after_interrupt(code: &[u8]) -> Vec<u16> {
    recorded(&mut machine_with(code))
}

fn recorded(m: &mut Machine) -> Vec<u16> {
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
    m.read_ram(RESULT, 26)
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect()
}

#[test]
fn getcsc_returns_the_scan_code_and_destroys_the_other_main_registers() {
    let w = registers_after(CLOBBER_GETCSC);
    assert_eq!(w[0] >> 8, 0, "A is the scan code, none here");
    assert_eq!(w[1..4], [0xBC0B, 0xDE0D, 0xA11A], "BC, DE, HL");
    assert_eq!(w[4..6], [0xDEF0, 0xBEEF], "IX and IY");
    assert_eq!(w[6], w[12], "SP");
    assert_eq!(
        w[7..11],
        [0x1111, 0x2222, 0x3333, 0xA5C3],
        "shadow registers"
    );
}

#[test]
fn clr_lcd_full_gr_buf_cpy_and_put_c_destroy_every_register_but_ix_iy_and_sp() {
    for code in [CLOBBER_CLR, CLOBBER_GR, CLOBBER_PUT_C] {
        let w = registers_after(code);
        assert_eq!(w[0..4], [0xAF5A, 0xBC0B, 0xDE0D, 0xA11A], "AF, BC, DE, HL");
        assert_eq!(w[4..6], [0xDEF0, 0xBEEF], "IX and IY");
        assert_eq!(w[6], w[12], "SP");
        assert_eq!(
            w[7..11],
            [0xBC27, 0xDE27, 0xA127, 0xAF27],
            "shadow registers"
        );
    }
}

#[test]
fn an_app_that_counts_on_bc_surviving_getcsc_gets_the_wrong_answer() {
    let mut m = machine_with(RELIANT);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
    assert_eq!(
        m.read_ram(RESULT, 1),
        [0x0B],
        "C, not the 42h the app put there"
    );
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

/// The controller drops a write inside its busy window, which is 125 clocks at 15 MHz. Every byte
/// differs from its neighbours, so a dropped write shows. With port 2Fh at 0 the core's
/// separate wait is 48 clocks at 15 MHz, so the stub has to wait on the busy bit itself.
#[test]
fn gr_buf_cpy_loses_no_byte_at_either_speed_whatever_port_2f_says() {
    let buf: Vec<u8> = (0..768)
        .map(|i| (i as u8).wrapping_mul(37) ^ 0x5A)
        .collect();
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        for port_2f in [0x00, 0x4B] {
            let mut m = machine_with(GR_BUF_CPY);
            m.out_port(0x2F, port_2f);
            m.set_speed(speed);
            m.load_ram(PLOT_S_SCREEN, &buf);
            assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
            assert!(
                lcd_matches(&m.lcd_ram(), &buf),
                "{speed:?}, port 2F {port_2f:02X}: the LCD memory differs from the buffer"
            );
        }
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

#[test]
fn an_interrupt_leaves_the_main_registers_ix_and_iy_and_changes_the_shadow_set() {
    let w = registers_after_interrupt(CLOBBER_INTERRUPT);
    assert_eq!(w[0..4], [0xC6A9, 0x1234, 0x5678, 0x9ABC], "AF, BC, DE, HL");
    assert_eq!(w[4..6], [0xDEF0, 0xBEEF], "IX and IY");
    assert_eq!(w[6], w[12], "SP");
    // The shadow set started as 1111h, 2222h, 3333h and A5C3h. HL' counts as changed because L
    // is: the handler leaves H as it was.
    for (i, (was, name)) in [
        (0x1111, "BC'"),
        (0x2222, "DE'"),
        (0x3333, "HL'"),
        (0xA5C3, "AF'"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_ne!(w[7 + i], was, "{name} is as the app left it");
    }
}

#[test]
fn getcsc_sees_no_key_while_interrupts_stay_off() {
    let mut m = machine_with(GETCSC_BLIND);
    m.press_key(0x09);
    assert_eq!(getcsc(&mut m), 0);
    m.load_ram(ENTRY, GETCSC);
    assert_eq!(
        getcsc(&mut m),
        0x09,
        "the key was waiting for the first call to ask"
    );
}

/// Whether the service returned with interrupts on: bit 2 of the flags LD A,I left.
fn returns_with_interrupts_on(code: &[u8]) -> bool {
    let mut m = machine_with(code);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
    m.read_ram(RESULT, 1)[0] & 4 != 0
}

/// TI-OS 2.43 and 2.55MP return from these four with the same interrupt state whatever it was
/// on entry, measured on the real ROM.
#[test]
fn services_return_with_the_interrupt_state_the_real_os_leaves() {
    assert!(
        returns_with_interrupts_on(IFF_GETCSC_OFF),
        "GetCSC from off"
    );
    assert!(returns_with_interrupts_on(IFF_GETCSC_ON), "GetCSC from on");
    assert!(
        returns_with_interrupts_on(IFF_CLR_OFF),
        "ClrLCDFull from off"
    );
    assert!(returns_with_interrupts_on(IFF_CLR_ON), "ClrLCDFull from on");
    assert!(returns_with_interrupts_on(IFF_GR_OFF), "GrBufCpy from off");
    assert!(returns_with_interrupts_on(IFF_GR_ON), "GrBufCpy from on");
    assert!(!returns_with_interrupts_on(IFF_PUT_C_OFF), "PutC from off");
    assert!(!returns_with_interrupts_on(IFF_PUT_C_ON), "PutC from on");
}

/// Each LCD service from increment mode 4, 5, 6 and 7. The status byte shows only the low two
/// bits of the mode: 60h to 63h. Whether TI-OS also restores the mode it found was measured from
/// mode 5 only.
#[test]
fn the_lcd_services_leave_the_controller_in_the_mode_they_found_it() {
    let services: [(&str, [&[u8]; 4]); 2] = [
        (
            "_GrBufCpy",
            [
                GR_BUF_CPY_MODE4,
                GR_BUF_CPY,
                GR_BUF_CPY_MODE6,
                GR_BUF_CPY_MODE7,
            ],
        ),
        (
            "_ClrLCDFull",
            [
                CLR_LCD_FULL_MODE4,
                CLR_LCD_FULL,
                CLR_LCD_FULL_MODE6,
                CLR_LCD_FULL_MODE7,
            ],
        ),
    ];
    for (name, codes) in services {
        for (mode, code) in (4u8..).zip(codes) {
            let mut m = machine_with(code);
            assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
            let status = 0x60 | (mode & 3);
            assert_eq!(
                m.read_ram(RESULT, 2),
                [status, status],
                "{name} from {mode}"
            );
        }
    }
}

/// The timer fires about every 55,000 T-states and a copy takes about 98,000 at 6 MHz, so with
/// interrupts on one lands in the middle; with a key held the handler also has a key to report.
/// The handler works in the shadow registers, so a copy that keeps its mode there loses it.
#[test]
fn an_interrupt_during_an_lcd_copy_does_not_change_the_mode_it_restores() {
    let services: [(&str, [&[u8]; 4]); 2] = [
        (
            "_GrBufCpy",
            [
                GR_BUF_CPY_INTS_MODE4,
                GR_BUF_CPY_INTS_MODE5,
                GR_BUF_CPY_INTS_MODE6,
                GR_BUF_CPY_INTS_MODE7,
            ],
        ),
        (
            "_ClrLCDFull",
            [
                CLR_LCD_FULL_INTS_MODE4,
                CLR_LCD_FULL_INTS_MODE5,
                CLR_LCD_FULL_INTS_MODE6,
                CLR_LCD_FULL_INTS_MODE7,
            ],
        ),
    ];
    for (name, codes) in services {
        for (mode, code) in (4u8..).zip(codes) {
            for key in [None, Some(0x09)] {
                let mut m = machine_with(code);
                if let Some(key) = key {
                    m.press_key(key);
                }
                assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
                let status = 0x60 | (mode & 3);
                assert_eq!(
                    m.read_ram(RESULT, 2),
                    [status, status],
                    "{name} from {mode}, key {key:?}"
                );
            }
        }
    }
}

/// A controller that never comes ready stops the copy at the stub's fault instead of hanging
/// the run or letting it write into a busy controller. The harness forces the busy bit on.
#[test]
fn an_lcd_that_never_comes_ready_stops_the_stub_with_lcd_stuck() {
    for code in [GR_BUF_CPY, CLR_LCD_FULL] {
        let mut m = machine_with(code);
        m.stick_lcd_busy(true);
        assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::LcdStuck);
    }
    let mut m = machine_with(GR_BUF_CPY);
    m.stick_lcd_busy(true);
    m.stick_lcd_busy(false);
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
}

/// What `os_on_irq` recorded: AF, BC, DE, HL, IX and IY, then port 4, the byte at 89F9h and the one
/// at BEF8h.
fn on_irq(code: &[u8], on: Option<bool>) -> (Vec<u16>, [u8; 3]) {
    let mut m = machine_with(code);
    if let Some(held) = on {
        m.press_key(0x29);
        if !held {
            m.release_key(0x29);
        }
    }
    assert_eq!(m.run(ENTRY, LIMIT).stop, Stop::Halted);
    let b = m.read_ram(RESULT, 15);
    let w = b[..12]
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    (w, [b[12], b[13], b[14]])
}

const ON_IRQ_REGS: [u16; 6] = [0xC6A9, 0x1234, 0x5678, 0x9ABC, 0xDEF0, 0];

#[test]
fn an_interrupt_with_on_held_sets_the_break_flag_and_keeps_the_registers() {
    let (w, [_, flag, other]) = on_irq(ON_IRQ_FLAGS, Some(true));
    assert_eq!(w[..5], ON_IRQ_REGS[..5], "AF, BC, DE, HL, IX");
    assert_eq!(w[5], 0x89F0, "IY");
    assert_eq!(flag, 0x10, "the break flag");
    assert_eq!(other, 0);
}

#[test]
fn an_interrupt_after_an_on_tap_leaves_the_flag_clear_and_acknowledges_the_latch() {
    let (w, [port4, flag, _]) = on_irq(ON_IRQ_FLAGS, Some(false));
    assert_eq!(w[..5], ON_IRQ_REGS[..5], "AF, BC, DE, HL, IX");
    assert_eq!(flag, 0, "no break flag");
    assert_eq!(port4 & 1, 0, "the ON latch is acknowledged");
    let (_, [port4, flag, _]) = on_irq(ON_IRQ_FLAGS, None);
    assert_eq!((port4 & 1, flag), (0, 0), "no key");
}

#[test]
fn the_break_flag_is_written_only_through_the_flags_pointer() {
    let (w, [_, flag, other]) = on_irq(ON_IRQ_OTHER, Some(true));
    assert_eq!(w[..5], ON_IRQ_REGS[..5], "AF, BC, DE, HL, IX");
    assert_eq!(w[5], 0xBEEF, "IY");
    assert_eq!((flag, other), (0, 0), "nothing is written");
}
