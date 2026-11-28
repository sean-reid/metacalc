//! The lcdcheck probe on the stub OS: what it counts, against a trace of every access to the LCD
//! ports, and what it reports for the LCD and plotSScreen across a key wait, against bytes the test
//! writes. The stub's `_PutC` draws nothing and keeps no text shadow, so the screens are read from
//! the characters the probe passes to its own `putchar`, in order, with the row from curRow, and the
//! figures by label; what the OS draws from them is checked in lcdcheck_os.rs, which is ignored
//! because it needs the reference ROM.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/lcdcheck.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;

use kit::*;
use mcemu::{LcdAccess, Reached, Speed, Stop};
use std::collections::BTreeSet;

/// One pass at one speed: the writes, and the probe's own counters for them.
struct Pass {
    writes: Vec<Wr>,
    clocks: u32,
    data: Counts,
    cmd: Counts,
}

/// Runs `file` through both passes with the LCD's busy window `window` (0 for the emulator's own)
/// and returns the 6 MHz pass and the 15 MHz pass.
fn passes(file: &str, window: u32) -> [Pass; 2] {
    let mut s = Stub::new(file, Speed::Mhz6, window, true);
    let mut out = Vec::new();
    for n in 1..=2 {
        s.until_wait(n);
        let trace = s.m.take_lcd_trace();
        out.push(Pass {
            writes: pass(&trace),
            clocks: frame_clocks(&trace),
            data: s.counts("LCD_HIST_DATA", "LCD_TOTAL_DATA"),
            cmd: s.counts("LCD_HIST_CMD", "LCD_TOTAL_CMD"),
        });
        s.tap(SCAN_5);
    }
    let second = out.pop().unwrap();
    [out.pop().unwrap(), second]
}

/// CPU clocks from the first write of a pass to its last.
fn frame_clocks(trace: &[LcdAccess]) -> u32 {
    let writes: Vec<u32> = lcd_only(trace)
        .iter()
        .filter(|a| a.is_write())
        .map(|a| a.clock)
        .take(WRITES)
        .collect();
    writes[WRITES - 1] - writes[0]
}

/// What the emulator gives, by the window in 6 MHz clocks (0 is its own, 50) and the pass: the
/// data writes' buckets and busy reads, then the command writes'.
type Expect = ((u16, u16, u16, u16, u16), (u16, u16, u16, u16, u16));
const EXPECT: [(u32, [Expect; 2]); 4] = [
    (
        0,
        [
            ((768, 0, 0, 0, 0), (13, 13, 0, 0, 13)),
            ((0, 84, 684, 0, 1452), (12, 0, 1, 13, 41)),
        ],
    ),
    (
        60,
        [
            ((96, 672, 0, 0, 672), (13, 13, 0, 0, 13)),
            ((0, 0, 96, 672, 2208), (12, 0, 1, 13, 41)),
        ],
    ),
    (
        95,
        [
            ((84, 684, 0, 0, 684), (12, 1, 13, 0, 27)),
            ((0, 0, 0, 768, 2988), (1, 0, 11, 14, 91)),
        ],
    ),
    (
        250,
        [
            ((0, 0, 0, 768, 3744), (1, 0, 0, 25, 116)),
            ((0, 0, 0, 768, 9132), (1, 0, 0, 25, 291)),
        ],
    ),
];

fn expected(e: (u16, u16, u16, u16, u16)) -> Counts {
    Counts {
        hist: [e.0, e.1, e.2, e.3],
        total: e.4,
    }
}

#[test]
fn the_counts_are_what_the_emulator_gives_and_agree_with_the_status_reads_it_traced() {
    for (window, want) in EXPECT {
        let got = passes(PROBE, window);
        for (n, (pass, want)) in got.iter().zip(want).enumerate() {
            let at = format!("window {window}, pass {n}");
            let (data, cmd) = (expected(want.0), expected(want.1));
            println!(
                "{at}: data {:?} {}, cmd {:?} {}",
                data.hist, data.total, cmd.hist, cmd.total
            );
            assert_eq!(
                tally(&pass.writes, LCD_DATA),
                data,
                "{at}: data, from the trace"
            );
            assert_eq!(
                tally(&pass.writes, LCD_CMD),
                cmd,
                "{at}: commands, from the trace"
            );
            assert_eq!(pass.data, data, "{at}: the probe's data counters");
            assert_eq!(pass.cmd, cmd, "{at}: the probe's command counters");
            let writes = pass.writes.iter().filter(|w| w.port == LCD_DATA).count();
            assert_eq!(writes, FRAME, "{at}: data writes");
            assert_eq!(pass.writes.len() - writes, 26, "{at}: command writes");
        }
    }
}

/// A write and its first poll's distance, as `(port, value, gap)`.
fn shape(writes: &[Wr]) -> Vec<(u8, u8, Option<u32>)> {
    writes.iter().map(|w| (w.port, w.value, w.gap)).collect()
}

#[test]
fn the_copy_writes_what_ui_blit_writes_with_the_same_gap_from_a_write_to_its_first_poll() {
    let gaps = [BTreeSet::from([62, 85, 107]), BTreeSet::from([70, 93, 115])];
    for window in [0, 60, 250] {
        let copy = passes(PROBE, window);
        let real = passes(REF, window);
        for n in 0..2 {
            let at = format!("window {window}, pass {n}");
            assert_eq!(shape(&copy[n].writes), shape(&real[n].writes), "{at}");
            assert_eq!(copy[n].writes[0].gap, None);
            let polled = copy[n].writes.iter().filter(|w| w.gap.is_some()).count();
            assert_eq!(
                polled,
                WRITES - 1,
                "{at}: every write after the first follows a poll"
            );
            let data: BTreeSet<u32> = copy[n]
                .writes
                .iter()
                .filter(|w| w.port == LCD_DATA)
                .filter_map(|w| w.gap)
                .collect();
            assert_eq!(data, gaps[n], "{at}: the gaps after a data write");
            assert_eq!(real[n].data.hist, [0; 4], "{at}: ui_blit counts nothing");
            assert!(
                copy[n].clocks > real[n].clocks,
                "{at}: the count costs clocks"
            );
        }
    }
}

#[test]
fn the_data_the_probe_sends_is_the_pattern_down_each_column() {
    let frame = pattern();
    let copy = passes(PROBE, 0);
    for (n, one) in copy.iter().enumerate() {
        let data: Vec<u8> = one
            .writes
            .iter()
            .filter(|w| w.port == LCD_DATA)
            .map(|w| w.value)
            .collect();
        let want: Vec<u8> = (0..FRAME)
            .map(|k| frame[(k % 64) * STRIDE + k / 64])
            .collect();
        assert_eq!(data, want, "pass {n}");
        let commands: Vec<u8> = one
            .writes
            .iter()
            .filter(|w| w.port == LCD_CMD)
            .map(|w| w.value)
            .collect();
        let mut want = vec![0x01, 0x05];
        for c in 0..STRIDE as u8 {
            want.extend([0x20 + c, 0x80]);
        }
        assert_eq!(commands, want, "pass {n}");
    }
}

#[test]
fn the_frame_costs_what_the_header_figures_say() {
    let real = passes(REF, 0);
    let sixty = passes(REF, 60);
    println!(
        "ui_blit frame: {} clocks at 6 MHz, {} at 15 MHz; {} at 6 MHz with a 60 clock window",
        real[0].clocks, real[1].clocks, sixty[0].clocks
    );
    for (clocks, ceiling) in [
        (real[0].clocks, 73_100),
        (real[1].clocks, 150_700),
        (sixty[0].clocks, 102_700),
    ] {
        assert!(clocks > ceiling * 98 / 100 && clocks < ceiling, "{clocks}");
    }
}

#[test]
fn the_pattern_is_the_documented_one_and_fills_plotsscreen() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.until_wait(1);
    let frame = s.peek(UI_FRAME, FRAME);
    let raw = |i: usize| 0x5Cu8.wrapping_add(37u8.wrapping_mul(i as u8));
    assert_eq!(frame, pattern());
    assert_eq!(&frame[..4], &[0x5C, 0x81, 0xA6, 0xCB]);
    let zero: Vec<usize> = (0..FRAME).filter(|&i| raw(i) == 0).collect();
    let ones: Vec<usize> = (0..FRAME).filter(|&i| raw(i) == 0xFF).collect();
    assert_eq!((zero.len(), ones.len()), (3, 3));
    assert!(zero.iter().all(|&i| frame[i] == 0x5A));
    assert!(ones.iter().all(|&i| frame[i] == 0xA5));
}

#[test]
fn a_controller_that_never_clears_counts_every_write_at_the_limit_and_saturates_the_total() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.m.stick_lcd_busy(true);
    let mut stop = Stop::Limit;
    for _ in 0..40 {
        stop = s.m.resume(1_000_000).stop;
        if stop != Stop::Limit {
            break;
        }
    }
    assert_eq!(
        stop,
        Stop::LcdStuck,
        "the stub's own clear gives up after the pass"
    );
    let data = s.counts("LCD_HIST_DATA", "LCD_TOTAL_DATA");
    let cmd = s.counts("LCD_HIST_CMD", "LCD_TOTAL_CMD");
    assert_eq!(
        data,
        Counts {
            hist: [0, 0, 0, 768],
            total: 0xFFFF
        }
    );
    assert_eq!(
        cmd,
        Counts {
            hist: [0, 0, 0, 26],
            total: 26 * 255
        }
    );
}

/// One thing the test does to the machine during a key wait.
#[derive(Clone, Copy)]
enum Poke {
    Lcd(u8, u8, u8),
    Frame(usize, u8),
}

/// What the probe reports after one key.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    key: u8,
    pre: Slot,
    lcd: Slot,
    plt: Slot,
    r7: Slot,
}

fn none() -> Report {
    Report {
        key: 0,
        pre: NONE,
        lcd: NONE,
        plt: NONE,
        r7: NONE,
    }
}

/// The key the stub's `_GetKey` returns for the key the probe asks for in each case.
fn keys() -> [u8; 3] {
    let i = usize::from(SCAN_5) - 1;
    [
        keytables::PLAIN[i],
        keytables::SECOND[i],
        keytables::ALPHA[i],
    ]
}

/// A byte of the pattern different from `pat(i)`.
fn other(i: usize) -> u8 {
    !pat(i)
}

/// Runs the three cases, doing `pokes[k]` during case k's wait, and returns what each reported.
fn run_cases(pokes: [&[Poke]; 3]) -> (Stub, Vec<Report>) {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.until_wait(1);
    s.tap(SCAN_5);
    s.until_wait(2);
    s.tap(SCAN_5);
    let mut reports = Vec::new();
    for case in 0..3u8 {
        s.until_wait(3 + 2 * case);
        assert_eq!(s.byte("S_CASE"), case);
        assert_eq!(
            s.slot("S_PRE"),
            NONE,
            "case {case}: the read-back matches before the wait"
        );
        for poke in pokes[usize::from(case)] {
            match *poke {
                Poke::Lcd(row, col, v) => {
                    s.poke_lcd(row, col, v);
                    assert_eq!(
                        s.lcd_byte(usize::from(row), usize::from(col)),
                        v,
                        "the poke took"
                    );
                }
                Poke::Frame(offset, v) => s.poke_frame(offset, v),
            }
        }
        match case {
            1 => s.tap(SCAN_2ND),
            2 => s.tap(SCAN_ALPHA),
            _ => {}
        }
        assert_eq!(s.waits(), 3 + 2 * case, "a modifier is not a key");
        s.tap(SCAN_5);
        s.until_wait(4 + 2 * case);
        reports.push(Report {
            key: s.byte("S_KEY"),
            pre: s.slot("S_PRE"),
            lcd: s.slot("S_LCD"),
            plt: s.slot("S_PLT"),
            r7: s.slot("S_R7"),
        });
        if case < 2 {
            s.tap(SCAN_5);
        }
    }
    (s, reports)
}

fn expect_keys(reports: &mut [Report]) {
    for (r, k) in reports.iter_mut().zip(keys()) {
        assert_eq!(r.key, k, "the key _GetKey returned");
        r.key = 0;
    }
}

#[test]
fn nothing_written_during_the_waits_reports_no_difference() {
    let (_, mut reports) = run_cases([&[], &[], &[]]);
    expect_keys(&mut reports);
    assert_eq!(reports, [none(), none(), none()]);
}

fn lcd_at(row: usize, col: usize) -> Poke {
    Poke::Lcd(row as u8, col as u8, other(row * STRIDE + col))
}

fn slot(count: u16, row: u8, col: u8) -> Slot {
    Slot::at(count, row, col)
}

#[test]
fn bytes_written_to_the_lcd_during_a_wait_are_counted_and_located() {
    let same = Poke::Lcd(20, 4, pat(20 * STRIDE + 4));
    let (_, mut reports) = run_cases([
        &[lcd_at(10, 3), same, lcd_at(0, 5)],
        &[lcd_at(40, 2), lcd_at(7, 9), lcd_at(7, 4)],
        &[lcd_at(55, 11), lcd_at(56, 0), lcd_at(63, 11)],
    ]);
    expect_keys(&mut reports);
    let want = [
        Report {
            lcd: slot(2, 0, 5),
            ..none()
        },
        Report {
            lcd: slot(3, 7, 4),
            ..none()
        },
        Report {
            lcd: slot(1, 55, 11),
            r7: slot(2, 56, 0),
            ..none()
        },
    ];
    assert_eq!(reports, want);
}

#[test]
fn bytes_written_to_plotsscreen_during_a_wait_are_counted_and_located() {
    let at = |row: usize, col: usize| Poke::Frame(row * STRIDE + col, other(row * STRIDE + col));
    let (mut s, mut reports) = run_cases([
        &[at(5, 7)],
        &[at(0, 0), at(55, 11)],
        &[at(56, 0), at(63, 11)],
    ]);
    expect_keys(&mut reports);
    let want = [
        Report {
            plt: slot(1, 5, 7),
            ..none()
        },
        Report {
            plt: slot(2, 0, 0),
            ..none()
        },
        none(),
    ];
    assert_eq!(reports, want);
    assert_eq!(s.byte("S_WAITS"), 8);
}

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
    s.m.load_ram(GRAPH_ADDR, &[0]);
    let leave = s.label("LEAVE");
    s.m.press_key(SCAN_5);
    let mut reached = Reached::Vector38;
    for _ in 0..20 {
        reached = s.m.run_to(BASE, leave, 100_000);
        if reached != Reached::Vector38 {
            break;
        }
    }
    assert_eq!(reached, Reached::Target, "the probe reaches its exit");
    assert_eq!(s.m.regs().iff1, 1, "interrupts are on when it leaves");
    let mut stop = Stop::Limit;
    for _ in 0..20 {
        stop = s.m.resume(100_000).stop;
        if stop != Stop::Limit {
            break;
        }
    }
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
            s.peek(GRAPH_ADDR, 1)[0] & 1,
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
fn every_write_of_the_speed_is_made_with_interrupts_off_from_a_call_made_with_them_on() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    for call in 0..4 {
        s.run_to_label("SET_SPEED");
        assert_eq!(s.m.regs().iff1, 1, "call {call} is made with interrupts on");
        s.run_to_label("SS_OUT");
        assert_eq!(
            s.m.regs().iff1,
            0,
            "call {call} writes port 20h with interrupts off"
        );
        let run = s.m.resume(1);
        assert_eq!(run.stop, Stop::Limit);
        match call {
            1 => {
                s.until_wait(1);
                s.m.press_key(SCAN_5);
            }
            2 => s.m.release_key(SCAN_5),
            _ => {}
        }
    }
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
    "c01 00013 00013",
    "c23 00000 00000",
    "ctot 00013",
    "any key in20=00",
];
const PASS_15: [&str; 8] = [
    "lcdchk v1 15MHz",
    "d01 00000 00084",
    "d23 00684 00000",
    "dtot 01452",
    "c01 00012 00000",
    "c23 00001 00013",
    "ctot 00041",
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
fn clear_at_any_wait_leaves_through_the_exit_with_the_speed_back_and_the_flag_set() {
    for (speed, waits) in [
        (Speed::Mhz6, 1),
        (Speed::Mhz15, 2),
        (Speed::Mhz6, 3),
        (Speed::Mhz6, 4),
        (Speed::Mhz15, 7),
    ] {
        let mut s = Stub::new(PROBE, speed, 0, false);
        for n in 1..waits {
            s.until_wait(n);
            s.tap(SCAN_5);
        }
        s.until_wait(waits);
        s.m.load_ram(GRAPH_ADDR, &[0]);
        let stop = s.press_to_exit(0x0F);
        assert_eq!(left_at(stop), speed, "wait {waits}");
        assert_eq!(s.peek(GRAPH_ADDR, 1)[0] & 1, 1, "wait {waits}: graphDraw");
    }
}

#[test]
fn the_probe_writes_only_the_memory_it_names() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    let before = s.peek(0x8000, 0x4000);
    let size = s.label("S_SIZE");
    let (base, buf) = (s.label("S_BASE"), s.label("LCD_BUF"));
    let spans = [
        (base, size),
        (0x843F, 8),
        (buf, 768),
        (UI_FRAME, 768),
        (0x86EC, 26),
        (CUR_ROW, 2),
        (GRAPH_ADDR, 1),
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
    assert_eq!(size, 153);
}
