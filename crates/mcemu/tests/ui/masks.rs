//! The masked blit: which rows of the frame reach the LCD, and the exact writes that take them there.

use crate::support::*;
use mcemu::{LcdAccess, Speed};

const CMD: u8 = 0x10;
const DATA: u8 = 0x11;

/// The offset of a UI variable in the variable block, from the build's label file.
pub fn var(image: Image, name: &str) -> usize {
    usize::from(label(image, name) - image.vars)
}

pub fn dirty_at() -> u8 {
    byte(var(REAL, "ui_dirty"))
}

pub fn sent_at() -> u8 {
    byte(var(REAL, "ui_sent"))
}

/// 768 bytes of a repeatable noise.
fn noise(seed: u32) -> Vec<u8> {
    let mut x = seed;
    (0..FRAME_BYTES)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

/// Two frames that share no byte at the same place and whose eight text rows are all different, in
/// the frame and between the frames, so a row sent from the wrong place or not sent shows.
pub fn frames() -> (Vec<u8>, Vec<u8>) {
    let a = noise(1);
    let mut b = noise(2);
    for (x, y) in a.iter().zip(b.iter_mut()) {
        if x == y {
            *y = !*x;
        }
    }
    assert!(a.iter().zip(&b).all(|(x, y)| x != y));
    let rows: Vec<&[u8]> = a.chunks(96).chain(b.chunks(96)).collect();
    for (i, r) in rows.iter().enumerate() {
        assert!(rows[i + 1..].iter().all(|s| s != r), "two rows alike");
    }
    (a, b)
}

/// A frame's bytes as the pixels the LCD shows.
pub fn pixels(frame: &[u8]) -> Vec<bool> {
    (0..64 * 96)
        .map(|i| {
            let (y, x) = (i / 96, i % 96);
            frame[y * 12 + x / 8] & (0x80 >> (x % 8)) != 0
        })
        .collect()
}

/// What the LCD shows after the frame `new` is blitted with `mask` over what it showed of `old`.
fn mirror(old: &[u8], new: &[u8], mask: u8) -> Vec<bool> {
    let (old, new) = (pixels(old), pixels(new));
    (0..64 * 96)
        .map(|i| {
            if mask & (1 << (i / 96 / 8)) != 0 {
                new[i]
            } else {
                old[i]
            }
        })
        .collect()
}

/// The LCD seeded by a full blit of `a`, then `b` in the frame, the dirty byte `mask` and a sent
/// byte that is not the mask, and a masked blit.
pub fn partial(settings: (Speed, bool), mask: u8) -> Outcome {
    let (a, b) = frames();
    let script = [
        Op::SetFrame(a),
        Op::SetVar(dirty_at(), 0xFF),
        Op::Blit,
        Op::SetFrame(b),
        Op::SetVar(dirty_at(), mask),
        Op::SetVar(sent_at(), mask ^ 0xA5),
        Op::ClearLcdWrites,
        Op::Blit,
    ];
    run_code(REAL, REAL.code, settings, &script, true)
}

fn check_partial(settings: (Speed, bool), mask: u8) {
    let (a, b) = frames();
    let got = partial(settings, mask);
    assert!(
        got.lcd_ram == mirror(&a, &b, mask),
        "{settings:?} mask {mask:02X}: the LCD is not the old rows outside the mask and the new inside"
    );
    assert_eq!(got.frame, b, "{settings:?} mask {mask:02X}: the frame");
    assert_eq!(
        got.vars[usize::from(dirty_at())],
        0,
        "{settings:?} mask {mask:02X}: ui_dirty"
    );
    assert_eq!(
        got.vars[usize::from(sent_at())],
        mask,
        "{settings:?} mask {mask:02X}: ui_sent"
    );
}

#[test]
fn the_lcd_holds_the_new_rows_in_the_mask_and_the_old_ones_elsewhere_for_every_mask() {
    let mut sent = [0usize; 8];
    for mask in 0..=255u8 {
        check_partial((Speed::Mhz6, true), mask);
        for (r, n) in sent.iter_mut().enumerate() {
            *n += usize::from(mask & (1 << r) != 0);
        }
    }
    assert_eq!(sent, [128; 8], "each row is in half of the masks");
}

#[test]
fn a_partial_mask_blits_alike_at_both_speeds_with_and_without_the_wait_states() {
    let mut run = 0;
    for settings in [
        (Speed::Mhz15, true),
        (Speed::Mhz6, false),
        (Speed::Mhz15, false),
    ] {
        for mask in (0..=255u8).filter(|m| m.count_ones() <= 1 || m % 23 == 0) {
            check_partial(settings, mask);
            run += 1;
        }
    }
    assert!(run >= 3 * (9 + 10), "{run} runs");
}

/// What the blit writes, from its rules: the modes 01h and 05h, then for each of the 12 columns
/// the column command 20h + c, and for each run of adjacent rows in the mask the row command
/// 80h + 8r once at its head and the eight bytes of each of its rows from the frame, a byte every
/// 12 for the 8 scanlines.
pub fn expected(frame: &[u8], mask: u8) -> Vec<(u8, u8)> {
    let mut out = vec![(CMD, 0x01), (CMD, 0x05)];
    if mask == 0 {
        return out;
    }
    for c in 0..12 {
        out.push((CMD, 0x20 + c as u8));
        let mut previous = false;
        for r in 0..8 {
            let on = mask & (1 << r) != 0;
            if on && !previous {
                out.push((CMD, 0x80 + 8 * r as u8));
            }
            if on {
                for y in 8 * r..8 * r + 8 {
                    out.push((DATA, frame[y * 12 + c]));
                }
            }
            previous = on;
        }
    }
    out
}

pub fn writes(got: &Outcome) -> Vec<(u8, u8)> {
    got.lcd_writes
        .iter()
        .map(|w: &LcdAccess| (w.port, w.value))
        .collect()
}

/// The mask FFh's writes written out by hand: 26 commands and 768 data bytes.
#[test]
fn a_full_mask_writes_the_two_modes_then_for_each_column_its_command_row_zero_and_64_bytes() {
    let (_, b) = frames();
    let got = partial((Speed::Mhz6, true), 0xFF);
    let mut want = vec![(CMD, 0x01), (CMD, 0x05)];
    for c in 0..12usize {
        want.push((CMD, 0x20 + c as u8));
        want.push((CMD, 0x80));
        for y in 0..64usize {
            want.push((DATA, b[y * 12 + c]));
        }
    }
    assert_eq!(want.len(), 2 + 12 * 66);
    assert_eq!(want.iter().filter(|w| w.0 == CMD).count(), 26);
    assert_eq!(want.iter().filter(|w| w.0 == DATA).count(), 768);
    assert_eq!(writes(&got), want);
}

#[test]
fn each_one_row_mask_writes_its_row_command_and_eight_bytes_in_every_column() {
    let (_, b) = frames();
    for r in 0..8usize {
        let got = partial((Speed::Mhz6, true), 1 << r);
        let mut want = vec![(CMD, 0x01), (CMD, 0x05)];
        for c in 0..12usize {
            want.push((CMD, 0x20 + c as u8));
            want.push((CMD, 0x80 + 8 * r as u8));
            for y in 8 * r..8 * r + 8 {
                want.push((DATA, b[y * 12 + c]));
            }
        }
        assert_eq!(want.len(), 2 + 12 * 10);
        assert_eq!(writes(&got), want, "row {r}");
    }
}

#[test]
fn a_mask_with_runs_sends_the_row_command_once_at_the_head_of_each_run() {
    let (_, b) = frames();
    let mut shapes = [0usize; 5];
    for mask in 0..=255u8 {
        let got = partial((Speed::Mhz6, true), mask);
        assert_eq!(writes(&got), expected(&b, mask), "mask {mask:02X}");
        let runs = (0..8)
            .filter(|&r| mask & (1 << r) != 0 && (r == 0 || mask & (1 << (r - 1)) == 0))
            .count();
        let row_commands = got
            .lcd_writes
            .iter()
            .filter(|w| w.port == CMD && w.value >= 0x80)
            .count();
        assert_eq!(row_commands, 12 * runs, "mask {mask:02X}");
        shapes[runs] += 1;
    }
    assert_eq!(shapes, [1, 36, 126, 84, 9], "masks by their number of runs");
}

#[test]
fn the_empty_mask_writes_the_two_modes_and_nothing_else() {
    for settings in [(Speed::Mhz6, true), (Speed::Mhz15, true)] {
        let got = partial(settings, 0);
        assert_eq!(writes(&got), [(CMD, 0x01), (CMD, 0x05)], "{settings:?}");
    }
}

/// The stub with an interrupt handler that sends the LCD command putting the row counter at the
/// top, rewrites ui_dirty and a frame byte with the values they hold and counts itself in
/// `ui_dst`. Its one effect on a blit is the LCD command, which tears a column it lands in.
fn writing_rom() -> Vec<u8> {
    let mut rom = mcemu::STUB_ROM.to_vec();
    assert_eq!(rom[0x38], 0xC3, "jp isr");
    let at = (0x100..0x3F00)
        .find(|&a| rom[a..a + 48].iter().all(|&b| b == 0xFF))
        .expect("room for a handler");
    let word = |a: u16| a.to_le_bytes();
    let (dirty, count) = (REAL.vars + u16::from(dirty_at()), REAL.vars + 2);
    let probe = REAL.frame + 0x2FF;
    let mut isr = vec![0xF5, 0xDB, 0x03, 0xF5, 0xAF, 0xD3, 0x03, 0xF1, 0xD3, 0x03];
    isr.extend([
        0x3E,
        0x80,
        0xD3,
        0x10,
        0x3A,
        word(dirty)[0],
        word(dirty)[1],
        0x32,
        word(dirty)[0],
        word(dirty)[1],
    ]);
    isr.extend([
        0x3A,
        word(probe)[0],
        word(probe)[1],
        0x32,
        word(probe)[0],
        word(probe)[1],
    ]);
    isr.extend([
        0x3A,
        word(count)[0],
        word(count)[1],
        0x3C,
        0x32,
        word(count)[0],
        word(count)[1],
    ]);
    isr.extend([0xF1, 0xFB, 0xED, 0x4D]);
    assert!(isr.len() <= 48);
    rom[at..at + isr.len()].copy_from_slice(&isr);
    rom[0x39..0x3B].copy_from_slice(&word(at as u16));
    rom
}

fn handled(rom: &[u8], code: &[u8], mask: u8) -> Outcome {
    let (a, b) = frames();
    let script = [
        Op::Interrupts(true),
        Op::Iff,
        Op::Iff,
        Op::Interrupts(false),
        Op::SetFrame(a),
        Op::SetVar(dirty_at(), 0xFF),
        Op::Blit,
        Op::SetFrame(b),
        Op::SetVar(dirty_at(), mask),
        Op::SetVar(sent_at(), mask ^ 0xA5),
        Op::SetVar(2, 0),
        Op::Interrupts(true),
        Op::Blit,
        Op::Interrupts(false),
    ];
    run_code_on(rom, true, REAL, code, (Speed::Mhz6, true), &script, true)
}

#[test]
fn a_handler_cannot_reach_a_partial_blit_and_does_when_the_blit_does_not_mask_interrupts() {
    let rom = writing_rom();
    let (a, b) = frames();
    for mask in [0x7E, 0x18, 0x81, 0xFF] {
        let got = handled(&rom, REAL.code, mask);
        assert!(
            got.lcd_ram == mirror(&a, &b, mask),
            "mask {mask:02X}: the LCD"
        );
        assert_eq!(got.frame, b, "mask {mask:02X}: the frame");
        assert_eq!(
            got.vars[usize::from(sent_at())],
            mask,
            "mask {mask:02X}: ui_sent"
        );
        assert!(got.vars[2] >= 1, "mask {mask:02X}: the handler ran");
        assert_eq!(
            got.vars[usize::from(dirty_at())],
            0,
            "mask {mask:02X}: ui_dirty"
        );
    }
    let mut code = REAL.code.to_vec();
    let di = usize::from(label(REAL, "bl_mask")) - 0x4000;
    assert_eq!(code[di], 0xF3, "the di");
    code[di] = 0x00;
    let got = handled(&rom, &code, 0x7E);
    assert!(
        got.lcd_ram != mirror(&a, &b, 0x7E),
        "the handler never reached the blit"
    );
}

#[test]
fn a_blit_gives_back_the_interrupt_state_it_was_called_with_for_every_kind_of_mask() {
    for mask in [0xFF, 0x18, 0x00] {
        for on in [true, false] {
            let script = [
                Op::SetVar(dirty_at(), mask),
                Op::Interrupts(on),
                Op::Blit,
                Op::Iff,
                Op::Interrupts(false),
            ];
            let got = run(REAL, &script);
            assert_eq!(got.results.len(), 1);
            assert_eq!(got.results[0] & 0x04 != 0, on, "mask {mask:02X}, on {on}");
        }
    }
}

#[test]
fn the_blit_and_the_row_routines_need_exactly_the_stack_their_headers_give() {
    let depth = |op: Op| stack_depth(REAL, &[Op::SetVar(dirty_at(), 0x18), op]);
    assert_eq!(depth(Op::Blit), 2, "ui_blit");
    assert_eq!(
        depth(Op::ClearRows { first: 1, count: 4 }),
        6,
        "ui_clear_rows"
    );
    for dir in [0, 1] {
        let shift = Op::ShiftRows {
            first: 1,
            count: 4,
            dir,
        };
        assert_eq!(depth(shift), 10, "ui_shift_rows, dir {dir}");
    }
}
