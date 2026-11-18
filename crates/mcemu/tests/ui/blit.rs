use crate::support::*;
use mcemu::{STUB_ROM, Speed};
use mcui::canvas::{Call, Canvas};

/// Speed and whether the TI-OS wait states are programmed. Without them the LCD controller's
/// busy window is all that holds a write back.
const SETTINGS: [(Speed, bool); 4] = [
    (Speed::Mhz6, true),
    (Speed::Mhz15, true),
    (Speed::Mhz6, false),
    (Speed::Mhz15, false),
];

/// A frame that uses every byte pattern in some row: the real screens, a gradient of text and
/// inverted blocks.
fn busy_frame() -> (mcglyph::Bitmap, Vec<Call>) {
    let mut c = Canvas::recording();
    for row in 0..8 {
        c.text(
            row,
            row,
            "The quick brown fox 0123456789 \u{2713}\u{2717}\u{2026}~",
        );
    }
    for x in 0..12 {
        c.invert(x * 8 + x % 8, x * 5, 7 + x, 4);
    }
    c.title("Libraries", "metacalc");
    c.soft_keys(["Open", "Run", "Info", "Find", "Up"]);
    c.into_parts()
}

fn lcd_equals(frame: &mcglyph::Bitmap, lcd: &[bool]) -> bool {
    (0..64).all(|y| (0..96).all(|x| lcd[y * 96 + x] == frame.get(x, y)))
}

#[test]
fn the_lcd_equals_the_frame_after_a_blit_at_each_speed() {
    let (frame, calls) = busy_frame();
    assert!(frame.rows().flatten().filter(|&&p| p).count() > 500);
    let mut script = ops(&calls);
    script.push(Op::Blit);
    for settings in SETTINGS {
        let got = run_code(REAL, REAL.code, settings, &script, true);
        assert!(
            lcd_equals(&frame, &got.lcd),
            "{settings:?}: the LCD differs from the frame"
        );
        same_frame("frame", &got.frame, &pack(&frame));
    }
}

#[test]
fn a_blit_without_the_busy_wait_loses_writes() {
    let (frame, calls) = busy_frame();
    let mut script = ops(&calls);
    script.push(Op::Blit);
    let mut code = REAL.code.to_vec();
    // in f,(c); jp p,over; djnz back become a jump over them
    let poll = [0xED, 0x70, 0xF2];
    let start = usize::from(label(REAL, "ui_blit")) - 0x4000;
    let end = usize::from(label(REAL, "ui_blit_end")) - 0x4000;
    let mut patched = 0;
    for at in start..end - 7 {
        if code[at..at + poll.len()] == poll {
            code[at..at + 7].copy_from_slice(&[0x18, 0x05, 0, 0, 0, 0, 0]);
            patched += 1;
        }
    }
    assert!(patched >= 4, "found {patched} polls");
    for settings in SETTINGS.into_iter().filter(|s| !s.1) {
        let got = run_code(REAL, &code, settings, &script, true);
        assert!(
            !lcd_equals(&frame, &got.lcd),
            "{settings:?}: no write was lost"
        );
    }
}

#[test]
fn blitting_twice_gives_the_same_screen() {
    let (frame, calls) = busy_frame();
    let mut script = ops(&calls);
    script.extend([Op::Blit, Op::Blit]);
    let got = run_code(REAL, REAL.code, (Speed::Mhz15, true), &script, true);
    assert!(lcd_equals(&frame, &got.lcd));
}

#[test]
fn a_blit_leaves_the_lcd_in_8_bit_mode_with_increment_mode_5() {
    // TI-OS enters an app with status 61h: on, 8 bit, increment mode 5
    let (frame, calls) = busy_frame();
    let draw = ops(&calls);
    let mut variants = vec![vec![Op::Command(0x01), Op::Command(0x05)]];
    for before in [0x07, 0x06, 0x04] {
        variants.push(vec![Op::Command(0x01), Op::Command(before)]);
    }
    variants.push(vec![Op::Command(0x00), Op::Command(0x07)]);
    for setup in variants {
        for settings in SETTINGS {
            let mut script = setup.clone();
            script.extend(draw.iter().cloned());
            script.extend([Op::Status, Op::Blit, Op::Status]);
            let got = run_code(REAL, REAL.code, settings, &script, true);
            let n = got.results.len();
            let status: Vec<u8> = got.results[n - 2..]
                .iter()
                .map(|&r| r as u8 & 0x7F)
                .collect();
            assert_eq!(status[1], 0x61, "{setup:?} {settings:?}");
            if let Op::Command(5) = setup[1] {
                assert_eq!(status[0], status[1], "{settings:?}: mode changed");
            }
            assert!(lcd_equals(&frame, &got.lcd));
        }
    }
}

/// Replaces the port number the polls read, `ld c,10h`, in the assembled blit; returns how many.
fn patch_blit_port(code: &mut [u8], port: u8) -> usize {
    let start = usize::from(label(REAL, "ui_blit")) - 0x4000;
    let end = usize::from(label(REAL, "ui_blit_end")) - 0x4000;
    let mut patched = 0;
    for at in start..end - 1 {
        if code[at..at + 2] == [0x0E, 0x10] {
            code[at + 1] = port;
            patched += 1;
        }
    }
    patched
}

#[test]
fn a_busy_flag_stuck_set_costs_pixels_not_a_hang() {
    let (_, calls) = busy_frame();
    let mut script = ops(&calls);
    script.push(Op::Blit);
    let mut code = REAL.code.to_vec();
    // port 2 reads with bit 7 set, always
    assert!(patch_blit_port(&mut code, 0x02) >= 2);
    for settings in SETTINGS {
        let got = run_code(REAL, &code, settings, &script, true);
        let blit = got.tstates - run_code(REAL, &code, settings, &script, false).tstates;
        // about 800 waits of at most 255 polls of 35 T-states, with room for the writes between
        assert!(blit < 10_000_000, "{settings:?}: {blit} T-states");
        assert!(
            blit > 1_000_000,
            "{settings:?}: the waits did not run: {blit}"
        );
    }
}

/// The stub with an interrupt handler that also sends the LCD command that puts the row counter
/// back at the top, as a handler that draws a cursor or an indicator would.
fn lcd_writing_rom() -> Vec<u8> {
    let mut rom = STUB_ROM.to_vec();
    assert_eq!(rom[0x38], 0xC3, "jp isr");
    assert!(
        rom[0x3B..0x50].iter().all(|&b| b == 0xFF),
        "room for a handler"
    );
    rom[0x39..0x3B].copy_from_slice(&[0x3B, 0x00]);
    let isr = [
        0xF5, 0xDB, 0x03, 0xF5, 0xAF, 0xD3, 0x03, 0xF1, 0xD3,
        0x03, // acknowledge as the stub does
        0x3E, 0x80, 0xD3, 0x10, // ld a,80h; out (10h),a
        0xF1, 0xFB, 0xED, 0x4D,
    ];
    rom[0x3B..0x3B + isr.len()].copy_from_slice(&isr);
    rom
}

#[test]
fn a_handler_that_writes_the_lcd_cannot_tear_the_frame_during_a_blit() {
    let (frame, calls) = busy_frame();
    let mut script = ops(&calls);
    script.extend([Op::Interrupts(true), Op::Blit, Op::Interrupts(false)]);
    let rom = lcd_writing_rom();
    for settings in SETTINGS {
        let got = run_code_on(&rom, REAL, REAL.code, settings, &script, true);
        assert!(lcd_equals(&frame, &got.lcd), "{settings:?}");
    }

    // the same run with the blit's di removed tears it, so the handler does reach the LCD
    let mut code = REAL.code.to_vec();
    let start = usize::from(label(REAL, "ui_blit")) - 0x4000;
    let di = (start..start + 24)
        .find(|&at| code[at] == 0xF3)
        .expect("the di");
    code[di] = 0x00;
    let got = run_code_on(&rom, REAL, &code, SETTINGS[0], &script, true);
    assert!(
        !lcd_equals(&frame, &got.lcd),
        "the handler never wrote the LCD"
    );
}

#[test]
fn a_blit_gives_back_the_interrupt_state_it_was_called_with() {
    let (_, calls) = busy_frame();
    let draw = ops(&calls);
    for on in [true, false] {
        let mut script = draw.clone();
        script.extend([Op::Interrupts(on), Op::Blit, Op::Iff, Op::Interrupts(false)]);
        for settings in SETTINGS {
            let got = run_code(REAL, REAL.code, settings, &script, true);
            let iff = got.results.last().unwrap() & 0x04 != 0;
            assert_eq!(iff, on, "{settings:?}: called with interrupts on: {on}");
        }
    }
}
