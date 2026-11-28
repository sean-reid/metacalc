//! The lcdcheck probe on the stub OS: what it counts, against a trace of every access to the LCD
//! ports, the copy of ui_blit against ui_blit, the frame's cost, the pattern and a controller that
//! never clears. The reports across a key wait are in lcdcheck_cases.rs, the run, the exit and the
//! screens in lcdcheck_exit.rs, and what the reference OS draws in lcdcheck_os.rs, which is ignored
//! because it needs the reference ROM.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/lcdcheck.rs"]
mod kit;
#[path = "common/realos.rs"]
mod realos;

use kit::*;
use mcemu::{LcdAccess, Speed, Stop};
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
            ((768, 0, 0, 0, 0), (25, 1, 0, 0, 1)),
            ((0, 84, 684, 0, 1452), (13, 12, 0, 1, 15)),
        ],
    ),
    (
        60,
        [
            ((84, 684, 0, 0, 684), (25, 1, 0, 0, 1)),
            ((0, 0, 84, 684, 2220), (13, 0, 12, 1, 27)),
        ],
    ),
    (
        95,
        [
            ((84, 684, 0, 0, 684), (13, 12, 1, 0, 14)),
            ((0, 0, 0, 768, 2988), (2, 0, 11, 13, 75)),
        ],
    ),
    (
        250,
        [
            ((0, 0, 0, 768, 3756), (2, 0, 11, 13, 76)),
            ((0, 0, 0, 768, 9132), (2, 0, 0, 24, 267)),
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
    let gaps = [BTreeSet::from([62, 110]), BTreeSet::from([70, 118])];
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
        (real[0].clocks, 74_000),
        (real[1].clocks, 151_000),
        (sixty[0].clocks, 104_400),
    ] {
        assert!(clocks > ceiling * 98 / 100 && clocks < ceiling, "{clocks}");
    }
}

#[test]
fn the_pattern_is_the_documented_one_and_fills_plotsscreen() {
    let mut s = Stub::new(PROBE, Speed::Mhz6, 0, false);
    s.until_wait(1);
    let frame = s.peek(s.frame(), FRAME);
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
