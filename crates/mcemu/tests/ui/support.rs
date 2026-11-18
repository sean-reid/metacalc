use mcemu::{ENTRY, FLASH_TEST_PAGE, LCD_HEIGHT, LCD_WIDTH, Machine, STUB_ROM, Speed, Stop};
use mcglyph::Bitmap;
use mcglyph::z80::Table;
use mcui::canvas::{Call, Canvas};
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub struct Image {
    pub code: &'static [u8],
    pub labels: &'static str,
    pub frame: u16,
}

pub const REAL: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/ui_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/ui_test.lab")),
    frame: 0x9000,
};
pub const MOVED: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/ui_moved.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/ui_moved.lab")),
    frame: 0xA000,
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
    Blit,
    Clear,
    SetFont(Vec<u8>),
    BuiltinFont,
    /// Pixels only the host draws, ORed into the frame between two Z80 calls.
    Ink(Bitmap),
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
                x: *x as u8,
                y: *y as u8,
                w: *width as u8,
                h: *height as u8,
            },
            Call::InvertRow(row) => Op::InvertRow(*row as u8),
            Call::Title { left, right } => Op::Title {
                left: enc(left),
                right: enc(right),
            },
            Call::SoftKeys(labels) => Op::Keys(labels.each_ref().map(|l| enc(l))),
            Call::ListRow {
                row,
                left,
                right,
                selected,
            } => Op::List {
                row: *row as u8,
                left: enc(left),
                right: enc(right),
                selected: *selected,
            },
            Call::Marker(row) => Op::Marker(*row as u8),
            Call::Ink(layer) => Op::Ink(layer.clone()),
        })
        .collect()
}

struct Script {
    code: Vec<u8>,
    data: Vec<u8>,
    image: Image,
    results: u16,
    inks: Vec<(u16, Bitmap)>,
}

impl Script {
    fn put(&mut self, bytes: &[u8]) -> u16 {
        let at = DATA + self.data.len() as u16;
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

    fn emit(&mut self, op: &Op, call: bool) {
        match op {
            Op::Text { x, row, codes } => {
                let s = self.put(codes);
                self.word(&[0x21], s);
                self.word(&[0x11], *x as u16);
                self.code.extend_from_slice(&[0x06, *row as u8]);
                self.run("ui_text", &[0xED, 0x53], call);
            }
            Op::Right { row, codes } | Op::Centered { row, codes } => {
                let s = self.put(codes);
                self.word(&[0x21], s);
                self.code.extend_from_slice(&[0x06, *row as u8]);
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
            Op::Blit => self.run("ui_blit", &[], call),
            Op::Clear => self.run("ui_clear", &[], call),
            Op::Ink(layer) => {
                self.code.push(0x76);
                self.inks
                    .push((SCRIPT + self.code.len() as u16, layer.clone()));
            }
            Op::SetFont(table) => {
                let at = self.put(table);
                self.word(&[0x21], at);
                self.run("ui_set_font", &[], call);
            }
            Op::BuiltinFont => {
                self.word(&[0x21], label(self.image, "ui_font"));
                self.run("ui_set_font", &[], call);
            }
        }
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
    /// The 0x100 bytes below the block and the 0x100 above it, filled with MARK before the run.
    pub around: Vec<u8>,
}

pub const MARK: u8 = 0xA5;
const BLOCK: u16 = 0x330;

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
    (speed, ports): (Speed, bool),
    ops: &[Op],
    call: bool,
) -> Outcome {
    let mut script = Script {
        code: vec![],
        data: vec![],
        image,
        results: 0,
        inks: vec![],
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
    let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
    if ports {
        os_ports(&mut m);
    }
    m.out_port(0x10, 0x03);
    m.load_flash(FLASH_TEST_PAGE, 0, code);
    m.load_ram(image.frame - 0x100, &[MARK; 0x100]);
    m.load_ram(image.frame + BLOCK, &[MARK; 0x100]);
    m.load_ram(SCRIPT, &script.code);
    m.load_ram(DATA, &script.data);
    m.set_speed(speed);
    let mut run = m.run(SCRIPT, LIMIT);
    assert_eq!(run.stop, Stop::Halted);
    for (next, layer) in &script.inks {
        let mut frame = m.read_ram(image.frame, FRAME_BYTES);
        for (byte, ink) in frame.iter_mut().zip(pack(layer)) {
            *byte |= ink;
        }
        m.load_ram(image.frame, &frame);
        let more = m.run(*next, LIMIT);
        assert_eq!(more.stop, Stop::Halted);
        run.tstates += more.tstates;
    }
    assert_eq!(ENTRY, 0x4000);
    let frame = m.read_ram(image.frame, FRAME_BYTES);
    let raw = m.read_ram(RESULTS, usize::from(script.results));
    let results = raw
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    let mut around = m.read_ram(image.frame - 0x100, 0x100);
    around.extend(m.read_ram(image.frame + BLOCK, 0x100));
    let lcd = m.lcd();
    let lcd = (0..LCD_HEIGHT)
        .flat_map(|y| (0..LCD_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| lcd.pixel(x, y))
        .collect();
    Outcome {
        frame,
        results,
        tstates: run.tstates,
        lcd,
        around,
    }
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
