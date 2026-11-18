use crate::support::*;
use mcemu::Speed;
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
    c.soft_keys(["See", "Run", "Info", "Find", "Up"]);
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
    let start = usize::from(label(REAL, "ui_blit")) - 0x4000;
    let end = usize::from(label(REAL, "ui_blit_end")) - 0x4000;
    let mut code = REAL.code.to_vec();
    // in f,(c); jp m,back become five NOPs
    let poll = [0xED, 0x70, 0xFA];
    let mut patched = 0;
    for at in start..end - 5 {
        if code[at..at + poll.len()] == poll {
            code[at..at + 5].fill(0);
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
