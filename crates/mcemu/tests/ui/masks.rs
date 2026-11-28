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
fn a_mask_of_two_runs_writes_each_runs_row_command_once_a_column() {
    let (_, b) = frames();
    let got = partial((Speed::Mhz6, true), 0b0010_0110);
    let mut want = vec![(CMD, 0x01), (CMD, 0x05)];
    for c in 0..12usize {
        want.push((CMD, 0x20 + c as u8));
        want.push((CMD, 0x88));
        for y in 8..24usize {
            want.push((DATA, b[y * 12 + c]));
        }
        want.push((CMD, 0xA8));
        for y in 40..48usize {
            want.push((DATA, b[y * 12 + c]));
        }
    }
    assert_eq!(want.len(), 2 + 12 * 27);
    assert_eq!(writes(&got), want);
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

/// The row the handler marks in `ui_dirty`, row 5.
const HANDLER_BIT: u8 = 0x20;
/// The frame byte the handler sets, in row 1, and the value it sets.
const HANDLER_AT: u16 = REAL.frame + 96 + 5;
const HANDLER_BYTE: u8 = 0xC3;
/// The sweep of waits before the blit, in rounds of 26 T-states: wider than the timer's period, 11,719
/// T-states at 6 MHz, so the interrupt arrives at every point of the blit and of the calls before it.
const PHASES: std::ops::RangeInclusive<u16> = 1..=520;
/// More than the longest blit with its waits and the handlers.
const RUN_LIMIT: u32 = 3_000_000;

/// The stub with a handler that sends the LCD command that puts the row counter back at the top,
/// marks `HANDLER_BIT` in `ui_dirty` and sets one frame byte, as a handler that drew would.
fn writing_rom() -> Vec<u8> {
    let mut rom = mcemu::STUB_ROM.to_vec();
    assert_eq!(rom[0x38], 0xC3, "jp isr");
    let at = (0x100..0x3F00)
        .find(|&a| rom[a..a + 48].iter().all(|&b| b == 0xFF))
        .expect("room for a handler");
    let word = |a: u16| a.to_le_bytes();
    let dirty = REAL.vars + u16::from(dirty_at());
    let mut isr = vec![0xF5, 0xDB, 0x03, 0xF5, 0xAF, 0xD3, 0x03, 0xF1, 0xD3, 0x03];
    isr.extend([0x3E, 0x80, 0xD3, 0x10]);
    isr.extend([0x3A, word(dirty)[0], word(dirty)[1]]);
    isr.extend([0xF6, HANDLER_BIT]);
    isr.extend([0x32, word(dirty)[0], word(dirty)[1]]);
    isr.extend([0x3E, HANDLER_BYTE]);
    isr.extend([0x32, word(HANDLER_AT)[0], word(HANDLER_AT)[1]]);
    isr.extend([0xF1, 0xFB, 0xED, 0x4D]);
    assert!(isr.len() <= 48);
    rom[at..at + isr.len()].copy_from_slice(&isr);
    rom[0x39..0x3B].copy_from_slice(&word(at as u16));
    rom
}

/// A blit of `mask` over a seeded LCD, called with interrupts on after a wait of `delay` rounds, once
/// the timer's pending interrupt has been taken.
fn handled_script(mask: u8, delay: u16) -> Vec<Op> {
    let (a, b) = frames();
    vec![
        Op::SetFrame(a),
        Op::SetVar(dirty_at(), 0xFF),
        Op::Blit,
        Op::Interrupts(true),
        Op::Delay(200),
        Op::Interrupts(false),
        Op::SetFrame(b),
        Op::SetVar(dirty_at(), mask),
        Op::SetVar(sent_at(), mask ^ 0xA5),
        Op::ClearLcdWrites,
        Op::Interrupts(true),
        Op::Delay(delay),
        Op::Blit,
        Op::Interrupts(false),
    ]
}

/// The same blit with no interrupt source: its LCD writes and its LCD.
fn quiet(mask: u8) -> (Vec<(u8, u8)>, Vec<bool>) {
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
    let got = run_code(REAL, REAL.code, (Speed::Mhz6, true), &script, true);
    (writes(&got), got.lcd_ram)
}

/// What a handled run shows: `Ok` with whether the handler ran after the blit's last write, or the first way
/// the run differs from the blit with no interrupt, the handler's own writes aside.
fn judge(got: &Outcome, mask: u8) -> Result<bool, String> {
    let (a, b) = frames();
    let all = writes(got);
    let handler = (CMD, 0x80);
    let start = all
        .iter()
        .position(|&w| w == (CMD, 0x01))
        .ok_or("the blit wrote nothing")?;
    let before = start;
    if all[..start].iter().any(|&w| w != handler) {
        return Err("a write before the blit that is not the handler's".into());
    }
    let marked = if before > 0 { mask | HANDLER_BIT } else { mask };
    let (want, want_lcd) = quiet(marked);
    let end = start + want.len();
    if all.len() < end || all[start..end] != want[..] {
        return Err(format!(
            "the blit's writes differ from the blit with no interrupt (mask {marked:02X})"
        ));
    }
    let after = all.len() - end;
    if all[end..].iter().any(|&w| w != handler) {
        return Err("a write after the blit that is not the handler's".into());
    }
    let ran = before + after > 0;
    let mut frame = b.clone();
    if ran {
        frame[usize::from(HANDLER_AT - REAL.frame)] = HANDLER_BYTE;
    }
    if got.frame != frame {
        return Err("the frame".into());
    }
    let sent = got.vars[usize::from(sent_at())];
    if sent != marked {
        return Err(format!("ui_sent is {sent:02X}, not {marked:02X}"));
    }
    let dirty = got.vars[usize::from(dirty_at())];
    let left = if after > 0 { HANDLER_BIT } else { 0 };
    if dirty != left {
        return Err(format!("ui_dirty is {dirty:02X}, not {left:02X}"));
    }
    if got.lcd_ram != want_lcd || got.lcd_ram != mirror(&a, &b, marked) {
        return Err("the LCD".into());
    }
    Ok(after > 0)
}

/// Runs every phase of `mask` on `code`: the first way one differs, or one that did not finish, or `Ok`
/// with the number of phases whose handler ran after the blit.
fn sweep(rom: &[u8], code: &[u8], mask: u8) -> Result<usize, String> {
    let mut held = 0;
    for delay in PHASES {
        let Some(got) = run_bounded(rom, REAL, code, &handled_script(mask, delay), RUN_LIMIT)
        else {
            return Err(format!("phase {delay}: no end within {RUN_LIMIT} T-states"));
        };
        held += usize::from(judge(&got, mask).map_err(|e| format!("phase {delay}: {e}"))?);
    }
    Ok(held)
}

/// The blit's code with its `di` moved from the top to just after it reads the mask: the bytes from
/// after the di to the mask read move one place down, the polls' jump targets with them.
fn di_after_the_mask(code: &[u8]) -> Vec<u8> {
    let di = usize::from(label(REAL, "bl_mask")) - 0x4000;
    assert_eq!(code[di], 0xF3, "the di");
    let dirty = label(REAL, "ui_dirty").to_le_bytes();
    let read = [0x21, dirty[0], dirty[1], 0x7E];
    let end = (di..di + 120)
        .find(|&at| code[at..at + 4] == read)
        .expect("the mask read")
        + 4;
    let mut out = code.to_vec();
    out.copy_within(di + 1..end, di);
    out[end - 1] = 0xF3;
    let mut shifted = 0;
    for at in di..end - 2 {
        if out[at] == 0xF2 {
            let target = u16::from_le_bytes([out[at + 1], out[at + 2]]);
            let first = 0x4000 + di as u16;
            if (first..first + 120).contains(&target) {
                out[at + 1..at + 3].copy_from_slice(&(target - 1).to_le_bytes());
                shifted += 1;
            }
        }
    }
    assert_eq!(shifted, 2, "the two polls' jumps");
    out
}

#[test]
fn a_handler_runs_before_or_after_a_blit_never_in_it_and_each_way_of_losing_the_di_shows() {
    let rom = writing_rom();
    for mask in [0x18, 0x4C] {
        let held = sweep(&rom, REAL.code, mask)
            .unwrap_or_else(|e| panic!("mask {mask:02X}: the blit as it is: {e}"));
        println!("mask {mask:02X}: the handler ran after the blit in {held} phases");
        assert_eq!(
            held,
            PHASES.count(),
            "mask {mask:02X}: phases that held the handler to the end"
        );
    }
    let mut gone = REAL.code.to_vec();
    let di = usize::from(label(REAL, "bl_mask")) - 0x4000;
    assert_eq!(gone[di], 0xF3, "the di");
    gone[di] = 0x00;
    let late = di_after_the_mask(REAL.code);
    for (what, code) in [
        ("the di removed", gone),
        ("the di after the mask is read", late),
    ] {
        let err = sweep(&rom, &code, 0x18).expect_err(&format!("{what}: no phase showed it"));
        println!("{what}: {err}");
    }
}

#[test]
fn a_blit_gives_back_the_interrupt_state_it_was_called_with_for_every_kind_of_mask() {
    let (_, b) = frames();
    for mask in [0xFF, 0x18, 0x00] {
        for on in [true, false] {
            let script = [
                Op::SetFrame(b.clone()),
                Op::SetVar(dirty_at(), mask),
                Op::SetVar(sent_at(), mask ^ 0xA5),
                Op::Interrupts(on),
                Op::Blit,
                Op::Iff,
                Op::Interrupts(false),
            ];
            let got = run(REAL, &script);
            assert_eq!(got.results.len(), 1);
            assert_eq!(got.results[0] & 0x04 != 0, on, "mask {mask:02X}, on {on}");
            assert_eq!(got.vars[usize::from(sent_at())], mask, "ui_sent");
            assert_eq!(writes(&got), expected(&b, mask), "the blit's writes");
        }
    }
}

/// The calls of the row routines the tests below make, with the rows each changes: nothing to do,
/// a count of 1, a clear, a shift each way, and ranges cut at row 7.
fn row_calls() -> Vec<(&'static str, Op, u8)> {
    let clear = |first, count| Op::ClearRows { first, count };
    let shift = |first, count, dir| Op::ShiftRows { first, count, dir };
    vec![
        ("clear of nothing", clear(2, 0), 0x00),
        ("clear off the end", clear(8, 3), 0x00),
        ("clear of one row", clear(3, 1), 0x08),
        ("clear of four", clear(1, 4), 0x1E),
        ("clear cut at row 7", clear(6, 5), 0xC0),
        ("shift of nothing", shift(2, 0, 0), 0x00),
        ("shift off the end", shift(8, 3, 1), 0x00),
        ("shift of one row", shift(3, 1, 0), 0x08),
        ("shift up", shift(1, 4, 0), 0x1E),
        ("shift down", shift(1, 4, 1), 0x1E),
        ("shift up cut at row 7", shift(6, 5, 0), 0xC0),
        ("shift down cut at row 7", shift(6, 5, 1), 0xC0),
    ]
}

/// A row routine's run on the noise frame with `ui_dirty` at `0x01`, and what it did: the rows whose
/// bytes changed and `ui_dirty`.
fn row_run(op: &Op, on: Option<bool>) -> (Outcome, u8) {
    let (a, _) = frames();
    let mut script = vec![Op::SetFrame(a.clone()), Op::SetVar(dirty_at(), 0x01)];
    if let Some(on) = on {
        script.push(Op::Interrupts(on));
    }
    script.push(op.clone());
    if on.is_some() {
        script.extend([Op::Iff, Op::Interrupts(false)]);
    }
    let got = run(REAL, &script);
    let changed = crate::marks::rows_changed(&a, &got.frame);
    (got, changed)
}

#[test]
fn the_row_routines_give_back_the_interrupt_state_they_were_called_with_and_do_their_work() {
    for (what, op, rows) in row_calls() {
        for on in [true, false] {
            let (got, changed) = row_run(&op, Some(on));
            assert_eq!(got.results.len(), 1, "{what}");
            assert_eq!(got.results[0] & 0x04 != 0, on, "{what}, on {on}");
            assert_eq!(changed, rows, "{what}, on {on}: the rows it changed");
            assert_eq!(
                got.vars[usize::from(dirty_at())],
                0x01 | rows,
                "{what}, on {on}: ui_dirty"
            );
        }
    }
}

#[test]
fn the_blit_and_the_row_routines_need_exactly_the_stack_their_headers_give() {
    let b = frames().1;
    let seeded = |op: Op| {
        vec![
            Op::SetFrame(b.clone()),
            Op::SetVar(dirty_at(), 0x18),
            Op::SetVar(sent_at(), 0xA5),
            op,
        ]
    };
    let blit = run(REAL, &seeded(Op::Blit));
    assert_eq!(blit.vars[usize::from(sent_at())], 0x18, "the blit ran");
    assert_eq!(writes(&blit), expected(&b, 0x18), "the blit's writes");
    assert_eq!(stack_depth(REAL, &seeded(Op::Blit)), 2, "ui_blit");
    for (what, op, rows) in row_calls() {
        let (got, changed) = row_run(&op, None);
        assert_eq!(changed, rows, "{what}: the rows it changed");
        assert_eq!(got.vars[usize::from(dirty_at())], 0x01 | rows, "{what}");
        let want = match (&op, rows) {
            (_, 0) => 4,
            (Op::ClearRows { .. }, _) => 6,
            (_, rows) if rows.count_ones() == 1 => 6,
            _ => 10,
        };
        assert_eq!(stack_depth(REAL, &seeded(op)), want, "{what}");
    }
    let mut deepest = std::collections::BTreeMap::new();
    for first in 0..9u8 {
        for count in 0..10u8 {
            let ops = [
                Op::ClearRows { first, count },
                Op::ShiftRows {
                    first,
                    count,
                    dir: 0,
                },
                Op::ShiftRows {
                    first,
                    count,
                    dir: 1,
                },
            ];
            for (kind, op) in ops.into_iter().enumerate() {
                *deepest
                    .entry((kind, stack_depth(REAL, &seeded(op))))
                    .or_insert(0usize) += 1;
            }
        }
    }
    assert_eq!(
        deepest.into_iter().collect::<Vec<_>>(),
        [
            ((0, 4), 18),
            ((0, 6), 72),
            ((1, 4), 18),
            ((1, 6), 16),
            ((1, 10), 56),
            ((2, 4), 18),
            ((2, 6), 16),
            ((2, 10), 56)
        ],
        "stack depth by routine (clear, shift up, shift down) over every first row and count"
    );
}
