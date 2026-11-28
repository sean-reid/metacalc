use mcemu::{
    ENTRY, FLASH_TEST_PAGE, LCD_HEIGHT, LCD_WIDTH, LcdAccess, Machine, STUB_ROM, Speed, Stop,
};
use mcglyph::Bitmap;
use mcglyph::z80::Table;
use mcui::canvas::{Call, Canvas};
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub struct Image {
    pub code: &'static [u8],
    pub labels: &'static str,
    pub frame: u16,
    pub vars: u16,
}

pub const REAL: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/ui_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/ui_test.lab")),
    frame: 0x9340,
    vars: 0x86EC,
};

pub const MOVED: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/ui_moved.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/ui_moved.lab")),
    frame: 0xA000,
    vars: 0xA500,
};

pub const FRAME_BYTES: usize = 768;
const SCRIPT: u16 = 0xA800;
const DATA: u16 = 0xE000;
const RESULTS: u16 = 0xF800;
const STACK: u16 = 0xFF00;
const LIMIT: u32 = 50_000_000;

pub fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(Table::builtin)
}

/// The key font's table: soft key labels are encoded with it, everything else with `table()`.
pub fn key_table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(Table::keys)
}

pub fn label(image: Image, name: &str) -> u16 {
    image
        .labels
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// One call of the canvas routines, strings as glyph codes.
#[derive(Clone, Debug)]
pub enum Op {
    Text {
        x: usize,
        row: usize,
        codes: Vec<u8>,
    },
    Right {
        row: usize,
        codes: Vec<u8>,
    },
    Centered {
        row: usize,
        codes: Vec<u8>,
    },
    Width(Vec<u8>),
    Invert {
        x: u8,
        y: u8,
        w: u8,
        h: u8,
    },
    InvertRow(u8),
    Title {
        left: Vec<u8>,
        right: Vec<u8>,
    },
    Keys([Vec<u8>; 5]),
    List {
        row: u8,
        left: Vec<u8>,
        right: Vec<u8>,
        selected: bool,
    },
    Marker(u8),
    Bar {
        row: u8,
        done: u16,
        total: u16,
    },
    Blit,
    Clear,
    ClearRows {
        first: u8,
        count: u8,
    },
    /// `dir` is the C register of the call: 0 moves up, anything else down.
    ShiftRows {
        first: u8,
        count: u8,
        dir: u8,
    },
    /// The 768 bytes of the frame, put there directly, as another routine's drawing would leave
    /// them.
    SetFrame(Vec<u8>),
    /// One byte of the variable block, `ui_dirty` and `ui_sent` among them, at an offset.
    SetVar(u8, u8),
    /// Forgets the LCD writes recorded so far.
    ClearLcdWrites,
    /// An LCD command, after a delay that lets the controller take it.
    Command(u8),
    /// The LCD status byte, read from port 10h.
    Status,
    /// `ei` or `di`. The timer interrupt source is on only in a run made with `run_code_on` and
    /// its `timer` set.
    Interrupts(bool),
    /// The interrupt flip-flop as `ld a,i` reports it, in bit 2 of the result.
    Iff,
    SetFont(Vec<u8>),
    BuiltinFont,
    /// Zeroes the whole variable block, standing in for the OS overwriting saveSScreen when auto
    /// power down fires.
    ClearVars,
    /// Pixels only the host draws, ORed into the frame between two Z80 calls.
    Ink(Bitmap),
    /// Loads IX, IY and the shadow registers with `SHADOWS`; `Op::CheckShadows` keeps what they
    /// hold, as six words after every other result.
    Shadows,
    CheckShadows,
    /// `lines` calls of a fitting routine, `ui_truncate`, `ui_tail` or `ui_wrap`, each into its
    /// own buffer of `cap` bytes between `FIT_GUARD` bytes of `MARK` on each side; every call
    /// after the first starts where the one before it left HL.
    Fit {
        routine: &'static str,
        text: Vec<u8>,
        cap: u8,
        limit: u8,
        lines: usize,
    },
}

/// IX, IY, BC', DE', HL' and AF' as `Op::Shadows` loads them.
pub const SHADOWS: [u16; 6] = [0x1234, 0x5678, 0x9ABC, 0xDEF0, 0x1357, 0x2468];

/// Bytes of `MARK` before and after each buffer of an `Op::Fit`.
pub const FIT_GUARD: usize = 4;

/// Where an `Op::Fit` put its text and its buffers, guard bytes included.
pub struct Placed {
    pub text: u16,
    pub buffers: Vec<u16>,
}

/// `value` as a byte; a model value that does not fit is a bug in the test.
pub fn byte(value: usize) -> u8 {
    u8::try_from(value).unwrap_or_else(|_| panic!("{value} does not fit a byte"))
}

/// A count as the 16 bits the Z80 takes; one past them is a bug in the test.
pub fn count(value: usize) -> u16 {
    u16::try_from(value).unwrap_or_else(|_| panic!("{value} does not fit 16 bits"))
}

pub fn ops(calls: &[Call]) -> Vec<Op> {
    let t = table();
    let enc = |s: &str| t.encode(s);
    calls
        .iter()
        .map(|call| match call {
            Call::Text { x, row, text } => Op::Text {
                x: *x,
                row: *row,
                codes: enc(text),
            },
            Call::TextRight { row, text } => Op::Right {
                row: *row,
                codes: enc(text),
            },
            Call::TextCentered { row, text } => Op::Centered {
                row: *row,
                codes: enc(text),
            },
            Call::Invert {
                x,
                y,
                width,
                height,
            } => Op::Invert {
                x: byte(*x),
                y: byte(*y),
                w: byte(*width),
                h: byte(*height),
            },
            Call::InvertRow(row) => Op::InvertRow(byte(*row)),
            Call::Title { left, right } => Op::Title {
                left: enc(left),
                right: enc(right),
            },
            Call::SoftKeys(labels) => Op::Keys(labels.each_ref().map(|l| key_table().encode(l))),
            Call::ListRow {
                row,
                left,
                right,
                selected,
            } => Op::List {
                row: byte(*row),
                left: enc(left),
                right: enc(right),
                selected: *selected,
            },
            Call::Marker(row) => Op::Marker(byte(*row)),
            Call::ClearRows { first, count } => Op::ClearRows {
                first: byte(*first),
                count: byte(*count),
            },
            Call::ShiftRows { first, count, up } => Op::ShiftRows {
                first: byte(*first),
                count: byte(*count),
                dir: u8::from(!*up),
            },
            Call::Bar { row, done, total } => Op::Bar {
                row: byte(*row),
                done: count(*done),
                total: count(*total),
            },
            Call::Ink(layer) => Op::Ink(layer.clone()),
        })
        .collect()
}

/// What the host does when the script halts for it.
enum Pause {
    Ink(Bitmap),
    ClearLcdWrites,
}

struct Script {
    code: Vec<u8>,
    data: Vec<u8>,
    image: Image,
    results: u16,
    pauses: Vec<(u16, Pause)>,
    placed: Vec<Placed>,
}

impl Script {
    fn put(&mut self, bytes: &[u8]) -> u16 {
        let at = DATA + u16::try_from(self.data.len()).expect("data fits 16 bits");
        assert!(
            usize::from(at) + bytes.len() <= usize::from(RESULTS),
            "the data reaches the results area"
        );
        self.data.extend_from_slice(bytes);
        at
    }

    fn word(&mut self, opcode: &[u8], value: u16) {
        self.code.extend_from_slice(opcode);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn call(&mut self, routine: &str) {
        self.word(&[0xCD], label(self.image, routine));
    }

    fn keep(&mut self, store: &[u8]) {
        self.word(store, RESULTS + self.results);
        self.results += 2;
    }

    fn keep_a(&mut self) {
        self.word(&[0x32], RESULTS + self.results);
        self.results += 2;
    }

    fn emit(&mut self, op: &Op, call: bool) {
        match op {
            Op::Text { x, row, codes } => {
                let s = self.put(codes);
                self.word(&[0x21], s);
                self.word(&[0x11], u16::try_from(*x).expect("x fits 16 bits"));
                self.code.extend_from_slice(&[0x06, byte(*row)]);
                self.run("ui_text", &[0xED, 0x53], call);
            }
            Op::Right { row, codes } | Op::Centered { row, codes } => {
                let s = self.put(codes);
                self.word(&[0x21], s);
                self.code.extend_from_slice(&[0x06, byte(*row)]);
                let name = if matches!(op, Op::Right { .. }) {
                    "ui_text_right"
                } else {
                    "ui_text_centered"
                };
                self.run(name, &[0xED, 0x53], call);
            }
            Op::Width(codes) => {
                let s = self.put(codes);
                self.word(&[0x21], s);
                self.run("ui_text_width", &[0x22], call);
            }
            Op::Invert { x, y, w, h } => {
                self.word(&[0x01], u16::from_be_bytes([*x, *y]));
                self.word(&[0x11], u16::from_be_bytes([*w, *h]));
                self.run("ui_invert", &[], call);
            }
            Op::InvertRow(row) | Op::Marker(row) => {
                self.code.extend_from_slice(&[0x3E, *row]);
                let name = if matches!(op, Op::Marker(_)) {
                    "ui_marker"
                } else {
                    "ui_invert_row"
                };
                self.run(name, &[], call);
            }
            Op::Title { left, right } => {
                let (l, r) = (self.put(left), self.put(right));
                self.word(&[0x21], l);
                self.word(&[0x11], r);
                self.run("ui_title", &[], call);
            }
            Op::Keys(labels) => {
                let at: Vec<u16> = labels.iter().map(|l| self.put(l)).collect();
                let table = self.put(&at.iter().flat_map(|a| a.to_le_bytes()).collect::<Vec<_>>());
                self.word(&[0x21], table);
                self.run("ui_soft_keys", &[], call);
            }
            Op::List {
                row,
                left,
                right,
                selected,
            } => {
                let (l, r) = (self.put(left), self.put(right));
                self.word(&[0x21], l);
                self.word(&[0x11], r);
                self.word(&[0x01], u16::from_be_bytes([*row, u8::from(*selected)]));
                self.run("ui_list_row", &[], call);
            }
            Op::Bar { row, done, total } => {
                self.code.extend_from_slice(&[0x3E, *row]);
                self.word(&[0x21], *done);
                self.word(&[0x11], *total);
                self.run("ui_bar", &[], call);
            }
            Op::Blit => self.run("ui_blit", &[], call),
            Op::Clear => self.run("ui_clear", &[], call),
            Op::ClearRows { first, count } => {
                self.code.extend_from_slice(&[0x3E, *first, 0x06, *count]);
                self.run("ui_clear_rows", &[], call);
            }
            Op::ShiftRows { first, count, dir } => {
                self.code
                    .extend_from_slice(&[0x3E, *first, 0x06, *count, 0x0E, *dir]);
                self.run("ui_shift_rows", &[], call);
            }
            Op::SetFrame(bytes) => {
                assert_eq!(bytes.len(), FRAME_BYTES);
                let at = self.put(bytes);
                self.word(&[0x21], at);
                self.word(&[0x11], self.image.frame);
                self.word(&[0x01], FRAME_LEN);
                self.code.extend_from_slice(&[0xED, 0xB0]);
            }
            Op::SetVar(offset, value) => {
                self.word(&[0x21], self.image.vars + u16::from(*offset));
                self.code.extend_from_slice(&[0x36, *value]);
            }
            Op::Command(value) if call => {
                self.code
                    .extend_from_slice(&[0x06, 0x00, 0x10, 0xFE, 0x3E, *value, 0xD3, 0x10]);
            }
            Op::Command(_) => {}
            Op::Status => {
                if call {
                    self.code.extend_from_slice(&[0xDB, 0x10]);
                    self.keep_a();
                }
            }
            Op::Interrupts(on) if call => self.code.push(if *on { 0xFB } else { 0xF3 }),
            Op::Interrupts(_) => {}
            Op::Iff if call => {
                self.code.extend_from_slice(&[0xED, 0x57, 0xF5, 0xE1]);
                self.keep(&[0x22]);
            }
            Op::Iff => {}
            Op::Ink(layer) => self.pause(Pause::Ink(layer.clone())),
            Op::ClearLcdWrites => self.pause(Pause::ClearLcdWrites),
            Op::SetFont(table) => {
                let at = self.put(table);
                self.word(&[0x21], at);
                self.run("ui_set_font", &[], call);
            }
            Op::ClearVars => {
                if call {
                    self.word(&[0x21], self.image.vars);
                    self.word(&[0x11], self.image.vars + 1);
                    self.word(&[0x01], VARS_LEN - 1);
                    self.code.extend_from_slice(&[0x36, 0x00, 0xED, 0xB0]);
                }
            }
            Op::BuiltinFont => {
                self.word(&[0x21], label(self.image, "ui_font"));
                self.run("ui_set_font", &[], call);
            }
            Op::Shadows if call => self.code.extend_from_slice(&[
                0xDD, 0x21, 0x34, 0x12, 0xFD, 0x21, 0x78, 0x56, 0x01, 0xBC, 0x9A, 0x11, 0xF0, 0xDE,
                0x21, 0x57, 0x13, 0xD9, 0x01, 0x68, 0x24, 0xC5, 0xF1, 0x08,
            ]),
            Op::CheckShadows if call => {
                self.keep(&[0xDD, 0x22]);
                self.keep(&[0xFD, 0x22]);
                self.code.push(0xD9);
                self.keep(&[0xED, 0x43]);
                self.keep(&[0xED, 0x53]);
                self.keep(&[0x22]);
                self.code.extend_from_slice(&[0xD9, 0x08, 0xF5, 0xC1]);
                self.keep(&[0xED, 0x43]);
                self.code.push(0x08);
            }
            Op::Shadows | Op::CheckShadows => {}
            Op::Fit {
                routine,
                text,
                cap,
                limit,
                lines,
            } => {
                let text = self.put(text);
                let region = vec![MARK; usize::from(*cap) + 2 * FIT_GUARD];
                let buffers: Vec<u16> = (0..*lines).map(|_| self.put(&region)).collect();
                self.word(&[0x21], text);
                for &buffer in &buffers {
                    self.word(&[0x11], buffer + FIT_GUARD as u16);
                    self.word(&[0x01], u16::from_be_bytes([*cap, *limit]));
                    self.run(routine, &[], call);
                    if call {
                        self.keep_a();
                        self.keep(&[0x22]);
                    }
                }
                self.placed.push(Placed { text, buffers });
            }
        }
    }

    fn pause(&mut self, pause: Pause) {
        self.code.push(0x76);
        self.pauses.push((SCRIPT + self.code.len() as u16, pause));
    }

    fn run(&mut self, routine: &str, store: &[u8], call: bool) {
        if !call {
            return;
        }
        self.call(routine);
        if !store.is_empty() {
            self.keep(store);
        }
    }
}

pub struct Outcome {
    pub frame: Vec<u8>,
    pub results: Vec<u16>,
    pub tstates: u32,
    pub lcd: Vec<bool>,
    /// The controller's memory as the screen would show it, whatever its on state.
    pub lcd_ram: Vec<bool>,
    /// The writes to the LCD's ports 10h and 11h since the start or the last `Op::ClearLcdWrites`.
    pub lcd_writes: Vec<LcdAccess>,
    /// The variable block, `VARS_LEN` bytes.
    pub vars: Vec<u8>,
    /// Where the last run stopped.
    pub pc: u16,
    /// Bytes of stack the run reached below its starting point, calls from the script included.
    pub stack: usize,
    /// The data the script's strings and buffers were put in, from `DATA`.
    pub data: Vec<u8>,
    pub placed: Vec<Placed>,
}

impl Outcome {
    /// `len` bytes of the script's data at `at`.
    pub fn bytes(&self, at: u16, len: usize) -> &[u8] {
        let from = usize::from(at - DATA);
        &self.data[from..from + len]
    }
}

pub const MARK: u8 = 0xA5;
pub const FRAME_LEN: u16 = 768;
pub const VARS_LEN: u16 = 0x1A;
/// RAM guarded below the two blocks, and above them up to the script, at most `GUARD_ABOVE`.
const GUARD: u16 = 0x200;
const GUARD_ABOVE: u16 = 0x800;
/// Marked bytes below the stack, to measure how deep it goes.
const STACK_WINDOW: u16 = 0x80;

/// Flash fetch and RAM write wait states and LCD delays as TI-OS programs them.
pub fn os_ports(m: &mut Machine) {
    m.out_port(0x29, 0x17);
    m.out_port(0x2A, 0x27);
    m.out_port(0x2E, 0x45);
}

/// Runs `ops` on the canvas code in `code`, from flash, after setting the font and clearing the
/// frame; with `call` false only the register loads run, which gives the cost of the harness.
pub fn run_code(
    image: Image,
    code: &[u8],
    settings: (Speed, bool),
    ops: &[Op],
    call: bool,
) -> Outcome {
    run_code_on(STUB_ROM, false, image, code, settings, ops, call)
}

/// `run_code` on another OS image than the stub as it is built, and with the timer interrupt
/// source on when `timer` is set.
pub fn run_code_on(
    rom: &[u8],
    timer: bool,
    image: Image,
    code: &[u8],
    settings: (Speed, bool),
    ops: &[Op],
    call: bool,
) -> Outcome {
    run_marked(rom, timer, image, code, settings, ops, call, MARK, &[])
}

/// The deepest the stack goes in a run of `ops`, as the larger of two measures with different
/// marks, so a byte the run writes that equals one mark is still seen.
pub fn stack_depth(image: Image, ops: &[Op]) -> usize {
    [MARK, !MARK]
        .into_iter()
        .map(|mark| {
            let settings = (Speed::Mhz6, true);
            run_marked(
                STUB_ROM,
                false,
                image,
                image.code,
                settings,
                ops,
                true,
                mark,
                &[],
            )
            .stack
        })
        .max()
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn run_marked(
    rom: &[u8],
    timer: bool,
    image: Image,
    code: &[u8],
    (speed, ports): (Speed, bool),
    ops: &[Op],
    call: bool,
    stack_mark: u8,
    watch: &[(u16, u16)],
) -> Outcome {
    let mut script = Script {
        code: vec![],
        data: vec![],
        image,
        results: 0,
        pauses: vec![],
        placed: vec![],
    };
    script.word(&[0x31], STACK);
    script.word(&[0x21], label(image, "ui_font"));
    script.call("ui_set_font");
    script.call("ui_clear");
    for op in ops {
        script.emit(op, call);
    }
    script.code.push(0x76);
    assert!(
        script.code.len() < 0x1700,
        "the script reaches the end of bank B"
    );
    let mut m = Machine::new(rom).expect("the ROM boots");
    if ports {
        os_ports(&mut m);
    }
    if timer {
        let mask = m.in_port(0x03);
        m.out_port(0x03, mask | 0x02);
    }
    m.out_port(0x10, 0x03);
    m.load_flash(FLASH_TEST_PAGE, 0, code);
    let (lo, hi) = (
        image.frame.min(image.vars) - GUARD,
        ((image.frame + FRAME_LEN).max(image.vars + VARS_LEN) + GUARD_ABOVE).min(SCRIPT),
    );
    m.load_ram(lo, &vec![MARK; usize::from(hi - lo)]);
    for &(at, len) in watch {
        m.load_ram(at, &vec![MARK; usize::from(len)]);
    }
    m.load_ram(
        STACK - STACK_WINDOW,
        &vec![stack_mark; usize::from(STACK_WINDOW)],
    );
    m.load_ram(SCRIPT, &script.code);
    m.load_ram(DATA, &script.data);
    m.set_speed(speed);
    m.trace_lcd(true);
    let mut run = m.run(SCRIPT, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    for (next, pause) in &script.pauses {
        match pause {
            Pause::Ink(layer) => {
                let mut frame = m.read_ram(image.frame, FRAME_BYTES);
                for (byte, ink) in frame.iter_mut().zip(pack(layer)) {
                    *byte |= ink;
                }
                m.load_ram(image.frame, &frame);
            }
            Pause::ClearLcdWrites => drop(m.take_lcd_trace()),
        }
        let more = m.run(*next, LIMIT);
        assert_eq!(more.stop, Stop::Halted);
        run.tstates += more.tstates;
    }
    assert_eq!(ENTRY, 0x4000);
    let frame = m.read_ram(image.frame, FRAME_BYTES);
    let data = m.read_ram(DATA, script.data.len());
    let raw = m.read_ram(RESULTS, usize::from(script.results));
    let results = raw
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    let in_block = |a: u16| {
        (image.frame..image.frame + FRAME_LEN).contains(&a)
            || (image.vars..image.vars + VARS_LEN).contains(&a)
    };
    let guarded = m.read_ram(lo, usize::from(hi - lo));
    for (at, &b) in (lo..hi).zip(&guarded) {
        assert!(
            in_block(at) || b == MARK,
            "RAM outside the two blocks changed at {at:04X}"
        );
    }
    for &(at, len) in watch {
        assert!(
            m.read_ram(at, usize::from(len)).iter().all(|&b| b == MARK),
            "RAM watched at {at:04X} changed"
        );
    }
    let window = m.read_ram(STACK - STACK_WINDOW, usize::from(STACK_WINDOW));
    let stack = window
        .iter()
        .position(|&b| b != stack_mark)
        .map_or(0, |at| window.len() - at);
    let pc = m.regs().pc;
    let pixels = |lcd: mcemu::Lcd| -> Vec<bool> {
        (0..LCD_HEIGHT)
            .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| lcd.pixel(x, y))
            .collect()
    };
    let lcd = pixels(m.lcd());
    let lcd_ram = pixels(m.lcd_ram());
    let lcd_writes = m
        .take_lcd_trace()
        .into_iter()
        .filter(|a| a.is_write() && matches!(a.port, 0x10 | 0x11))
        .collect();
    let vars = m.read_ram(image.vars, usize::from(VARS_LEN));
    Outcome {
        frame,
        results,
        tstates: run.tstates,
        lcd,
        lcd_ram,
        lcd_writes,
        vars,
        pc,
        stack,
        data,
        placed: script.placed,
    }
}

/// `run` that also marks the `(address, length)` ranges in `watch` and fails when a run changes
/// one; the ranges must lie clear of the script, the data and the stack.
pub fn run_watched(image: Image, ops: &[Op], watch: &[(u16, u16)]) -> Outcome {
    let settings = (Speed::Mhz6, true);
    run_marked(
        STUB_ROM, false, image, image.code, settings, ops, true, MARK, watch,
    )
}

pub fn run(image: Image, ops: &[Op]) -> Outcome {
    run_code(image, image.code, (Speed::Mhz6, true), ops, true)
}

/// The frame as the Z80 lays it out: 12 bytes a row, leftmost pixel in bit 7.
pub fn pack(bitmap: &Bitmap) -> Vec<u8> {
    let mut out = vec![0u8; FRAME_BYTES];
    for y in 0..64 {
        for x in 0..96 {
            if bitmap.get(x, y) {
                out[y * 12 + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

fn art(frame: &[u8], y: usize) -> String {
    (0..96)
        .map(|x| {
            if frame[y * 12 + x / 8] & (0x80 >> (x % 8)) != 0 {
                '#'
            } else {
                '.'
            }
        })
        .collect()
}

/// Panics with both frames as text art, differing rows marked, unless they are equal.
pub fn same_frame(what: &str, got: &[u8], want: &[u8]) {
    if got == want {
        return;
    }
    let mut text = format!("{what}: the Z80 frame (left) differs from the model (right)\n");
    for y in 0..64 {
        let (g, w) = (art(got, y), art(want, y));
        text += &format!("{y:2} {g} {w} {}\n", if g == w { ' ' } else { '!' });
    }
    panic!("{text}");
}

/// Runs `calls` on the model and the Z80 and compares the frames.
pub fn agree(what: &str, f: impl FnOnce(&mut Canvas)) {
    let mut canvas = Canvas::recording();
    f(&mut canvas);
    let (bitmap, calls) = canvas.into_parts();
    let got = run(REAL, &ops(&calls));
    same_frame(what, &got.frame, &pack(&bitmap));
}
