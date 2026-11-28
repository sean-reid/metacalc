//! The lcdcheck probe on the stub OS from launch to exit: the speeds it writes, interrupts, the LCD
//! commands, the shadow registers and stack of the blit, the memory it writes, CLEAR and what it
//! prints. The stub's `_PutC` draws nothing and keeps no text shadow, so the screens are read from
//! the characters the probe passes to its own `putchar`, in order, with the row from curRow and the
//! column counted by the test (the stub does not advance curCol); what the OS draws from them is
//! checked in lcdcheck_os.rs.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/lcdcheck.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;

use kit::*;
use mcemu::{LcdAccess, Speed, Stop};
use std::collections::BTreeSet;

/// Runs all eight waits to the exit with a key each, 2nd and ALPHA before the keys of cases 1 and
/// 2, and returns the trace of the whole run, how the probe stopped and the machine.
fn whole_run(speed: Speed) -> (Vec<LcdAccess>, Stop, Stub) {
    let mut s = Stub::new(PROBE, speed, 0, true);
    let mut trace = Vec::new();
    for n in 1..=8 {
        s.until_wait(n);
        trace.extend(s.m.take_lcd_trace());
        if n == 8 {
            break;
        }
        match n {
            5 => s.tap(SCAN_2ND),
            7 => s.tap(SCAN_ALPHA),
            _ => {}
        }
        s.tap(SCAN_5);
    }
    s.m.load_ram(s.graph_flags(), &[0]);
    s.m.press_key(SCAN_5);
    s.run_to_label("LEAVE_GO");
    assert_eq!(s.m.regs().iff1, 1, "interrupts are on when it leaves");
    let stop = s.finish();
    trace.extend(s.m.take_lcd_trace());
    (trace, stop, s)
}

fn left_at(stop: Stop) -> Speed {
    match stop {
        Stop::Exited => Speed::Mhz6,
        Stop::BadExit(left) => left.speed,
        other => panic!("the probe did not leave: {other:?}"),
    }
}

#[test]
fn the_probe_writes_the_speed_only_to_set_a_pass_and_to_put_it_back_and_leaves_as_it_found_it() {
    for (speed, found) in [(Speed::Mhz6, 0u8), (Speed::Mhz15, 1)] {
        let (trace, stop, mut s) = whole_run(speed);
        let speeds: Vec<u8> = trace
            .iter()
            .filter(|a| a.is_write() && a.port == PORT_SPEED)
            .map(|a| a.value)
            .collect();
        assert_eq!(speeds, [0, found, 1, found], "the speeds written, in order");
        assert_eq!(left_at(stop), speed, "the speed it found");
        assert_eq!(
            s.peek(s.graph_flags(), 1)[0] & 1,
            1,
            "graphDraw is set at the exit"
        );
        let commands: BTreeSet<u8> = trace
            .iter()
            .filter(|a| a.is_write() && a.port == LCD_CMD)
            .map(|a| a.value)
            .collect();
        assert!(
            commands.iter().all(|&c| c < 0xC0),
            "no contrast write: {commands:x?}"
        );
    }
}

#[test]
fn a_pass_runs_with_interrupts_off_from_before_its_speed_write_until_after_the_restore() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    for pass in 0..2 {
        s.run_to_label("SET_SPEED");
        assert_eq!(
            s.m.regs().iff1,
            0,
            "pass {pass}: the speed is written with interrupts off"
        );
        s.run_to_label("PROBE_BLIT");
        assert_eq!(
            s.m.regs().iff1,
            0,
            "pass {pass}: the blit is entered with them off"
        );
        s.run_to_label("SET_SPEED");
        assert_eq!(
            s.m.regs().iff1,
            0,
            "pass {pass}: the speed is restored with them off"
        );
        s.run_to_label("SHOW_PASS");
        assert_eq!(
            s.m.regs().iff1,
            1,
            "pass {pass}: they are on again after the restore"
        );
        if pass == 0 {
            s.until_wait(1);
            s.m.press_key(SCAN_5);
        }
    }
    s.m.release_key(SCAN_5);
}

#[test]
fn the_only_commands_the_probe_writes_are_the_ones_ui_blit_writes() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, true);
    s.run_to_label("PASS_BLIT_DONE");
    let first = s.m.take_lcd_trace();
    s.until_wait(1);
    s.tap(SCAN_5);
    s.until_wait(2);
    s.m.trace_lcd(true);
    s.m.press_key(SCAN_5);
    s.run_to_label("CASE_BLIT_DONE");
    let blit = s.m.take_lcd_trace();
    s.m.release_key(SCAN_5);
    s.run_to_label("PRE_READ_DONE");
    let pre = s.m.take_lcd_trace();
    s.until_wait(3);
    s.run_ms(300);
    s.m.trace_lcd(true);
    s.m.press_key(SCAN_5);
    s.run_to_label("POST_READ_DONE");
    let post = s.m.take_lcd_trace();
    let mut want: BTreeSet<u8> = BTreeSet::from([0x01, 0x05, 0x80]);
    want.extend(0x20..=0x2B);
    let mut got = BTreeSet::new();
    for (name, span) in [
        ("pass", &first),
        ("blit", &blit),
        ("pre", &pre),
        ("post", &post),
    ] {
        let commands: Vec<u8> = span
            .iter()
            .filter(|a| a.is_write() && a.port == LCD_CMD)
            .map(|a| a.value)
            .collect();
        assert_eq!(commands.len(), 26, "{name}: command writes");
        got.extend(commands);
        let data_writes = span
            .iter()
            .filter(|a| a.is_write() && a.port == LCD_DATA)
            .count();
        let data_reads = span
            .iter()
            .filter(|a| !a.is_write() && a.port == LCD_DATA)
            .count();
        let other = span
            .iter()
            .filter(|a| !matches!(a.port, 0x10 | 0x11 | PORT_SPEED))
            .count();
        let sends = name == "pass" || name == "blit";
        assert_eq!(
            data_writes,
            if sends { FRAME } else { 0 },
            "{name}: data writes"
        );
        assert_eq!(
            data_reads,
            if sends { 0 } else { 12 * 65 },
            "{name}: data reads"
        );
        assert_eq!(other, 0, "{name}: other ports");
    }
    assert_eq!(got, want);
}

#[test]
fn the_blit_returns_with_the_shadow_registers_it_was_called_with() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    for k in 0..5u8 {
        s.run_to_label("PB_MASKED");
        let mut r = s.m.regs();
        (r.af2, r.bc2, r.de2, r.hl2) = (0x1234 + u16::from(k), 0x5678, 0x9ABC, 0xDEF0);
        s.m.set_regs(&r);
        s.run_to_label("PB_UNMASKED");
        let after = s.m.regs();
        assert_eq!(
            (after.af2, after.bc2, after.de2, after.hl2),
            (r.af2, r.bc2, r.de2, r.hl2),
            "blit {k}"
        );
        s.m.release_key(SCAN_5);
        let waits = if k < 2 { k + 1 } else { 3 + 2 * (k - 2) };
        s.until_wait(waits);
        s.run_ms(300);
        if k >= 2 {
            s.m.press_key(SCAN_5);
            s.until_wait(waits + 1);
            s.m.release_key(SCAN_5);
            s.run_ms(300);
        }
        if k < 4 {
            s.m.press_key(SCAN_5);
        }
    }
}

const PASS_6: [&str; 8] = [
    "lcdchk v1 6MHz",
    "d01 00768 00000",
    "d23 00000 00000",
    "dtot 00000",
    "c01 00025 00001",
    "c23 00000 00000",
    "ctot 00001",
    "any key in20=00",
];
const PASS_15: [&str; 8] = [
    "lcdchk v1 15MHz",
    "d01 00000 00084",
    "d23 00684 00000",
    "dtot 01452",
    "c01 00013 00012",
    "c23 00000 00001",
    "ctot 00015",
    "any key in20=01",
];

#[test]
fn the_screens_print_the_figures_the_probe_holds() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    assert_eq!(s.printed_until_wait(1), PASS_6);
    s.m.press_key(SCAN_5);
    assert_eq!(s.printed_until_wait(2), PASS_15);
    s.m.release_key(SCAN_5);
    s.run_ms(300);
    s.m.press_key(SCAN_5);
    let prompt = s.printed_until_wait(3);
    s.m.release_key(SCAN_5);
    s.run_ms(300);
    assert_eq!(prompt[7], "press 5");
    assert!(prompt[..7].iter().all(String::is_empty));
    s.poke_lcd(10, 3, other(10 * STRIDE + 3));
    s.m.press_key(SCAN_5);
    let rows = s.printed_until_wait(4);
    let key = format!("key plain k={:02X}", keys()[0]);
    let want = [
        &key[..],
        "pre 000 r-- c--",
        "lcd 001 r10 c03",
        "plt 000 r-- c--",
        "r7  000 r-- c--",
        "",
        "",
        "any key",
    ];
    assert_eq!(rows, want);
}

#[test]
fn clear_at_each_of_the_eight_waits_leaves_through_the_exit_and_nothing_follows() {
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        for waits in 1..=8u8 {
            let at = format!("{speed:?}, wait {waits}");
            let mut s = Stub::new(PROBE, speed, 0, true);
            for n in 1..waits {
                s.until_wait(n);
                s.tap(SCAN_5);
            }
            s.until_wait(waits);
            let history = s.m.take_lcd_trace();
            let speeds = history
                .iter()
                .filter(|a| a.is_write() && a.port == PORT_SPEED)
                .count();
            assert_eq!(
                speeds,
                if waits == 1 { 2 } else { 4 },
                "{at}: speed writes so far"
            );
            let flags = s.graph_flags();
            s.m.load_ram(flags, &[0]);
            s.m.press_key(0x0F);
            s.run_to_label("LEAVE");
            let mut r = s.m.regs();
            r.iff1 = 0;
            r.iff2 = 0;
            s.m.set_regs(&r);
            s.run_to_label("LEAVE_GO");
            assert_eq!(s.m.regs().iff1, 1, "{at}: interrupts are on at the exit");
            let stop = s.finish();
            assert_eq!(left_at(stop), speed, "{at}: the speed found");
            assert_eq!(s.waits(), waits, "{at}: no wait after the CLEAR");
            assert_eq!(s.peek(flags, 1)[0] & 1, 1, "{at}: graphDraw");
            let after = s.m.take_lcd_trace();
            assert!(
                after.iter().all(|a| !a.is_write()),
                "{at}: a write followed the CLEAR: {after:?}"
            );
        }
    }
}

/// The bytes the stub OS keeps around kbdScanCode, which its key scan changes.
const STUB_KEYS: u16 = 8;

#[test]
fn the_probe_writes_only_the_memory_it_names_and_leaves_a_marker_everywhere_else() {
    const MARKER: u8 = 0xA7;
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    let (base, size) = (s.label("S_BASE"), s.label("S_SIZE"));
    let start = s.label("APPDATA") + 153;
    let past = start..s.label("APPDATA") + s.label("APP_DATA_SIZE");
    let marked = [
        (past.start, past.len() as u16),
        (s.label("TEXTSHADOW"), s.label("TEXT_SHADOW_SIZE")),
        (s.label("STATVARS"), s.label("STAT_VARS_SIZE")),
        (s.label("PREVDATA"), s.label("PREV_DATA_SIZE")),
        (s.label("SMALLEDITRAM"), s.label("SCR_VARS_SIZE")),
        (s.label("CMDSHADOW"), s.label("SCR_STACK_SIZE")),
    ];
    for (at, len) in marked {
        s.m.load_ram(at, &vec![MARKER; usize::from(len)]);
    }
    let before = s.peek(0x8000, 0x4000);
    assert!(past.len() > 90, "appData has room past the scratch");
    assert!(
        past.clone()
            .all(|a| before[usize::from(a - 0x8000)] == MARKER),
        "the marker is in appData past the scratch before the run"
    );
    let spans = [
        (base, size),
        (s.label("LCD_BUF"), s.label("UI_FRAME_SIZE")),
        (s.frame(), s.label("UI_FRAME_SIZE")),
        (s.label("UI_VARS"), s.label("UI_VARS_SIZE")),
        (s.label("CURROW"), 2),
        (s.graph_flags(), 1),
        (s.label("KBDSCANCODE"), STUB_KEYS),
    ];
    for n in 1..=8 {
        s.until_wait(n);
        if n < 8 {
            s.tap(SCAN_5);
        }
    }
    s.press_to_exit(SCAN_5);
    let after = s.peek(0x8000, 0x4000);
    let strays: Vec<u16> = (0..0x4000u16)
        .filter(|&i| before[usize::from(i)] != after[usize::from(i)])
        .map(|i| 0x8000 + i)
        .filter(|a| !spans.iter().any(|&(at, len)| (at..at + len).contains(a)))
        .collect();
    assert!(
        strays.is_empty(),
        "bytes changed outside the probe's regions: {strays:04X?}"
    );
    assert!(
        past.clone()
            .all(|a| after[usize::from(a - 0x8000)] == MARKER),
        "the marker past the scratch is intact"
    );
    assert_eq!(size, 153);
}

#[test]
fn the_blit_uses_the_stack_its_header_states_and_gives_back_the_stack_pointer() {
    // The header says 8 bytes below the return address, 10 with it.
    const BELOW: u16 = 8;
    let mut changed = BTreeSet::new();
    for mark in [0x00u8, 0xFF] {
        let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
        s.run_to_label("PROBE_BLIT");
        let sp = s.m.regs().sp;
        let window = sp - 32;
        s.m.load_ram(window, &[mark; 32]);
        s.run_to_label("PB_UNMASKED");
        assert_eq!(s.m.regs().sp, sp, "the stack pointer is as at the entry");
        let now = s.peek(window, 32);
        for (i, &b) in now.iter().enumerate() {
            if b != mark {
                changed.insert(window + i as u16);
            }
        }
    }
    let sp_low = *changed.iter().next().expect("the blit pushes");
    let top = *changed.iter().last().unwrap();
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.run_to_label("PROBE_BLIT");
    let sp = s.m.regs().sp;
    assert_eq!(
        changed,
        (sp - BELOW..sp).collect(),
        "{sp_low:04X} to {top:04X}"
    );
}

#[test]
fn the_speed_found_is_masked_to_its_low_bit_before_it_is_written_back() {
    for (junk, found) in [(0xF2u16, 0u8), (0xFF, 1)] {
        let mut s = Stub::new(PROBE, Speed::Mhz6, 0, true);
        s.run_to_label("SPEED_FOUND");
        let mut r = s.m.regs();
        r.af = (junk << 8) | (r.af & 0xFF);
        s.m.set_regs(&r);
        s.until_wait(1);
        let speeds: Vec<u8> =
            s.m.take_lcd_trace()
                .iter()
                .filter(|a| a.is_write() && a.port == PORT_SPEED)
                .map(|a| a.value)
                .collect();
        assert_eq!(speeds, [0, found], "found {junk:02X}");
    }
}
