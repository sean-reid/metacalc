//! What the lcdcheck tests share: the probe images, the pattern it fills, a stub machine to run
//! it on and the reading of an LCD trace. The figures are written out here, not read from the probe.

#![allow(dead_code)]

use super::realos;
use mcemu::{LcdAccess, Machine, Reached, STUB_ROM, Speed, Stop};
use std::path::PathBuf;

pub const BASE: u8 = 0x69;
pub const PROBE: &str = "lcdcheck_probe.z80";
pub const REF: &str = "lcdcheck_ref.z80";
pub const UI_FRAME: u16 = 0x9340;
pub const SCAN_2ND: u8 = 0x36;
pub const SCAN_ALPHA: u8 = 0x30;
pub const SCAN_5: u8 = 0x1B;
/// The frame is 12 bytes a row, 64 rows. A pass sends 26 commands: two modes and a column and a
/// row for each of the 12 columns; and 768 data bytes.
pub const STRIDE: usize = 12;
pub const FRAME: usize = 768;
pub const WRITES: usize = 26 + FRAME;
/// The rows the comparison covers: pixel rows 0 to 55.
pub const COMPARED: usize = 56 * STRIDE;
pub const LCD_CMD: u8 = 0x10;
pub const LCD_DATA: u8 = 0x11;
pub const BUSY: u8 = 0x80;
pub const PORT_SPEED: u8 = 0x20;
/// The OS flag byte with graphDraw, bit 0, and curRow and curCol.
pub const GRAPH_ADDR: u16 = 0x89F3;
pub const CUR_ROW: u16 = 0x844B;

/// Byte `i` of the pattern the probe fills: 5Ch + 37 * (i mod 256), where 00h reads as 5Ah and
/// FFh as A5h.
pub fn pat(i: usize) -> u8 {
    match 0x5Cu8.wrapping_add(37u8.wrapping_mul(i as u8)) {
        0x00 => 0x5A,
        0xFF => 0xA5,
        b => b,
    }
}

pub fn pattern() -> Vec<u8> {
    (0..FRAME).map(pat).collect()
}

pub struct Image {
    pub app: Vec<u8>,
    lab: PathBuf,
}

pub fn image(file: &str) -> Image {
    Image {
        app: realos::assemble_routine(file),
        lab: realos::routine_labels(file),
    }
}

impl Image {
    pub fn label(&self, name: &str) -> u16 {
        realos::label_in(&self.lab, name)
    }
}

/// A result slot: the count of differing bytes and the row and column of the first.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Slot {
    pub count: u16,
    pub row: u8,
    pub col: u8,
}

pub const NONE: Slot = Slot {
    count: 0,
    row: 0xFF,
    col: 0xFF,
};

impl Slot {
    pub fn at(count: u16, row: u8, col: u8) -> Slot {
        Slot { count, row, col }
    }

    pub fn from(b: &[u8]) -> Slot {
        Slot {
            count: u16::from_le_bytes([b[0], b[1]]),
            row: b[2],
            col: b[3],
        }
    }
}

/// The probe's counters: the four buckets of writes with 0, 1, 2 and 3 or more busy reads, and the
/// busy reads in all.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Counts {
    pub hist: [u16; 4],
    pub total: u16,
}

/// One write of a pass and the reads of the status port before it.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Wr {
    pub port: u8,
    pub value: u8,
    /// Clocks from the write before it to the first status read after that one, when there was one.
    pub gap: Option<u32>,
    pub polls: u32,
    pub busy: u32,
}

/// The first pass's writes, from a trace that starts before it: the first `WRITES` writes, each
/// with the status reads that came before it.
pub fn pass(trace: &[LcdAccess]) -> Vec<Wr> {
    let trace = lcd_only(trace);
    let mut out = Vec::new();
    let (mut last, mut gap, mut polls, mut busy) = (None, None, 0, 0);
    for a in &trace {
        if a.is_write() {
            out.push(Wr {
                port: a.port,
                value: a.value,
                gap,
                polls,
                busy,
            });
            if out.len() == WRITES {
                break;
            }
            (last, gap, polls, busy) = (Some(a.clock), None, 0, 0);
        } else if a.port == LCD_CMD {
            polls += 1;
            busy += u32::from(a.value & BUSY != 0);
            if gap.is_none() {
                gap = last.map(|c: u32| a.clock.wrapping_sub(c));
            }
        }
    }
    assert_eq!(out.len(), WRITES, "the trace holds a whole pass");
    out
}

/// The accesses to the LCD ports, without the writes to the speed port.
pub fn lcd_only(trace: &[LcdAccess]) -> Vec<LcdAccess> {
    trace
        .iter()
        .copied()
        .filter(|a| a.port != PORT_SPEED)
        .collect()
}

/// What the probe's counters must say for the writes to `port`, by the busy reads in the trace.
pub fn tally(writes: &[Wr], port: u8) -> Counts {
    let mut c = Counts {
        hist: [0; 4],
        total: 0,
    };
    let mut total = 0u32;
    for w in writes.iter().filter(|w| w.port == port) {
        c.hist[w.busy.min(3) as usize] += 1;
        total += w.busy;
    }
    c.total = total.min(0xFFFF) as u16;
    c
}

/// Writes one LCD byte as a driver does: 8 bit mode, increment on the row, the column and the row,
/// then the data, with a millisecond of emulated time for the controller after each write.
pub fn poke_lcd(m: &mut Machine, row: u8, col: u8, value: u8) {
    for (port, v) in [
        (LCD_CMD, 0x01),
        (LCD_CMD, 0x05),
        (LCD_CMD, 0x20 + col),
        (LCD_CMD, 0x80 + row),
        (LCD_DATA, value),
    ] {
        m.out_port(port, v);
        let run = m.run_time(1000);
        assert_eq!(run.stop, Stop::Limit);
    }
}

/// A stub machine running the probe, with the LCD recorded when asked.
pub struct Stub {
    pub m: Machine,
    img: Image,
}

const SLICE: u32 = 60_000;
const SLICES: usize = 400;

impl Stub {
    pub fn new(file: &str, speed: Speed, window: u32, trace: bool) -> Stub {
        let img = image(file);
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&img.app, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        m.set_speed(speed);
        m.set_lcd_busy_window(window);
        m.trace_lcd(trace);
        Stub { m, img }
    }

    pub fn label(&self, name: &str) -> u16 {
        self.img.label(name)
    }

    pub fn peek(&mut self, addr: u16, len: usize) -> Vec<u8> {
        self.m.read_ram(addr, len)
    }

    pub fn byte(&mut self, name: &str) -> u8 {
        self.peek(self.label(name), 1)[0]
    }

    pub fn slot(&mut self, name: &str) -> Slot {
        Slot::from(&self.peek(self.label(name), 4))
    }

    pub fn counts(&mut self, hist: &str, total: &str) -> Counts {
        let h = self.peek(self.label(hist), 8);
        let t = self.peek(self.label(total), 2);
        Counts {
            hist: std::array::from_fn(|i| u16::from_le_bytes([h[2 * i], h[2 * i + 1]])),
            total: u16::from_le_bytes([t[0], t[1]]),
        }
    }

    pub fn waits(&mut self) -> u8 {
        self.byte("S_WAITS")
    }

    /// Runs in slices of T-states until the probe has made `n` waits for a key, which each end
    /// by a slice that is only the wait.
    pub fn until_wait(&mut self, n: u8) {
        for _ in 0..SLICES {
            if self.waits() >= n {
                let run = self.m.resume(SLICE);
                assert_eq!(run.stop, Stop::Limit, "the probe waits for its key");
                assert_eq!(self.waits(), n, "the probe is waiting for wait {n}");
                return;
            }
            let run = self.m.resume(SLICE);
            assert_eq!(run.stop, Stop::Limit, "the probe runs on to wait {n}");
        }
        panic!(
            "no wait {n} within {} T-states, the probe is at {}",
            SLICE as usize * SLICES,
            self.waits()
        );
    }

    /// Runs to the instruction at `label` in the app's page, through interrupts.
    pub fn run_to_label(&mut self, name: &str) {
        let at = self.label(name);
        for _ in 0..200 {
            match self.m.run_to(BASE, at, 100_000) {
                Reached::Target => return,
                Reached::Vector38 | Reached::Limit => {}
            }
        }
        panic!("{name} not reached");
    }

    /// Presses `scan` and runs until the probe has left, and returns how it stopped.
    pub fn press_to_exit(&mut self, scan: u8) -> Stop {
        self.m.press_key(scan);
        for _ in 0..20 {
            let stop = self.m.resume(100_000).stop;
            if stop != Stop::Limit {
                return stop;
            }
        }
        panic!("the probe did not leave");
    }

    /// What the probe printed on its screen since the last time it cleared: the characters it
    /// passed to `putchar` by row, in the order printed, until it has made `n` waits. The stub's
    /// `_PutC` draws nothing and keeps no text, so this is read at the probe's own call.
    pub fn printed_until_wait(&mut self, n: u8) -> Vec<String> {
        let put = self.label("PUTCHAR");
        let mut grid = vec![[b' '; 16]; 8];
        let (mut col, mut last) = (0usize, 0xFFu8);
        for _ in 0..4000 {
            if self.waits() >= n {
                break;
            }
            if self.m.run_to(BASE, put, 100_000) != Reached::Target {
                continue;
            }
            let ch = (self.m.regs().af >> 8) as u8;
            let row = self.peek(CUR_ROW, 1)[0];
            if row != last {
                (col, last) = (0, row);
            }
            if row == 0 && col == 0 {
                grid = vec![[b' '; 16]; 8];
            }
            grid[usize::from(row)][col] = ch;
            col += 1;
            let run = self.m.resume(1);
            assert_eq!(run.stop, Stop::Limit);
        }
        assert!(self.waits() >= n, "the probe made {n} waits");
        grid.iter()
            .map(|r| String::from_utf8_lossy(r).trim_end().to_string())
            .collect()
    }

    pub fn run_ms(&mut self, ms: u32) {
        let run = self.m.run_time(ms * 1000);
        assert_eq!(run.stop, Stop::Limit, "the machine ran for {ms} ms");
    }

    /// A key down for 150 ms and up for 300, which is longer than the OS's 42 ms debounce.
    pub fn tap(&mut self, scan: u8) {
        self.m.press_key(scan);
        self.run_ms(150);
        self.m.release_key(scan);
        self.run_ms(300);
        assert!(!self.m.key_is_down(scan));
    }

    pub fn poke_lcd(&mut self, row: u8, col: u8, value: u8) {
        poke_lcd(&mut self.m, row, col, value);
    }

    pub fn poke_frame(&mut self, offset: usize, value: u8) {
        self.m.load_ram(UI_FRAME + offset as u16, &[value]);
    }

    pub fn lcd_byte(&mut self, row: usize, col: usize) -> u8 {
        let lcd = self.m.lcd_ram();
        (0..8).fold(0u8, |b, bit| {
            (b << 1) | u8::from(lcd.pixel(col * 8 + bit, row))
        })
    }
}
