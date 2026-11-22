use mcemu::{FLASH_TEST_PAGE, Machine, STUB_ROM, Speed, Stop};

pub type Frame = [u8; 16];

#[derive(Clone, Copy)]
pub struct Image {
    pub code: &'static [u8],
    pub labels: &'static str,
}

pub const REAL: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/scr_test.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/scr_test.lab")),
};

pub const MOVED: Image = Image {
    code: include_bytes!(concat!(env!("OUT_DIR"), "/scr_moved.bin")),
    labels: include_str!(concat!(env!("OUT_DIR"), "/scr_moved.lab")),
};

pub const SP: u16 = 0xB800;
pub const MARK: u8 = 0xA5;
pub const GUARD: u16 = 16;
pub const RUN_LIMIT: u32 = 100_000_000;
pub const IY: u16 = 0xC0DE;
pub const MAX_DEPTH: usize = 8;

pub const STAY: u8 = 0;
pub const REDRAW: u8 = 1;
pub const PUSH: u8 = 2;
pub const POP: u8 = 3;
pub const JUMP: u8 = 4;

pub const IDLE: u8 = 0;
pub const DRAW: u8 = 1;
pub const LEAVE: u8 = 2;
pub const FAILED: u8 = 3;

/// The kinds the fake routines serve, and the fake each one's row of the table points at.
pub const KINDS: [(u8, u8); 5] = [(1, 1), (2, 2), (3, 3), (200, 4), (255, 5)];

pub fn label_in(image: Image, name: &str) -> u16 {
    image
        .labels
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name.to_uppercase()).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
}

pub struct Out {
    pub af: u16,
    pub bc: u16,
    pub hl: u16,
    pub tstates: u32,
    /// Bytes of stack the call reached below its starting point, its return address included.
    pub stack: usize,
}

impl Out {
    pub fn a(&self) -> u8 {
        (self.af >> 8) as u8
    }

    pub fn b(&self) -> u8 {
        (self.bc >> 8) as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Routine {
    Enter,
    Draw,
    Key,
    Land,
}

/// One call of a fake routine, as it logged itself. `a` and `b` mean something for a key, `de` for a land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub routine: Routine,
    pub kind: u8,
    pub at: u16,
    pub frame: Frame,
    pub de: u16,
    pub a: u8,
    pub b: u8,
}

impl Rec {
    /// The record with the registers the routine was not given set to 0.
    pub fn given(mut self) -> Rec {
        if self.routine != Routine::Key {
            (self.a, self.b) = (0, 0);
        }
        if self.routine != Routine::Land {
            self.de = 0;
        }
        self
    }
}

/// What a key routine returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Stay,
    Redraw,
    Push(Frame),
    Pop,
    Jump(u16),
    Other(u8),
}

impl Act {
    pub fn code(self) -> u8 {
        match self {
            Act::Stay => STAY,
            Act::Redraw => REDRAW,
            Act::Push(_) => PUSH,
            Act::Pop => POP,
            Act::Jump(_) => JUMP,
            Act::Other(code) => code,
        }
    }
}

/// A frame of kind `kind` whose other bytes tell it apart from every frame with another `tag`.
pub fn frame(kind: u8, tag: u8) -> Frame {
    let mut f = [0; 16];
    f[0] = kind;
    for (i, b) in f.iter_mut().enumerate().skip(1) {
        *b = tag
            .wrapping_mul(17)
            .wrapping_add((i as u8).wrapping_mul(29))
            | 0x01;
    }
    f[1] = tag;
    f
}

pub struct Rig {
    pub m: Machine,
    pub image: Image,
    pub ei: bool,
    pub spent: u32,
    stack_mark: u8,
}

impl Rig {
    pub fn new(image: Image) -> Rig {
        Rig::with_ei(image, true)
    }

    pub fn with_ei(image: Image, ei: bool) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.out_port(0x29, 0x17);
        m.out_port(0x2A, 0x27);
        m.out_port(0x2E, 0x45);
        m.set_speed(Speed::Mhz6);
        m.load_flash(FLASH_TEST_PAGE, 0, image.code);
        let mut rig = Rig {
            m,
            image,
            ei,
            spent: 0,
            stack_mark: 0,
        };
        rig.write_table();
        for (at, len) in rig.guarded() {
            rig.poke(at, &vec![MARK; len]);
        }
        rig
    }

    pub fn label(&self, name: &str) -> u16 {
        label_in(self.image, name)
    }

    pub fn poke(&mut self, addr: u16, bytes: &[u8]) {
        self.m.load_ram(addr, bytes);
    }

    pub fn peek(&mut self, addr: u16, len: usize) -> Vec<u8> {
        self.m.read_ram(addr, len)
    }

    pub fn table(&self) -> u16 {
        self.label("FK_TABLE")
    }

    pub fn stack(&self) -> u16 {
        self.label("SCR_STACK")
    }

    fn write_table(&mut self) {
        let table = self.table();
        self.poke(table, &[0; 255 * 8]);
        for (kind, fake) in KINDS {
            let row: Vec<u8> = ["enter", "draw", "key", "land"]
                .iter()
                .flat_map(|r| self.label(&format!("fk{fake}_{r}")).to_le_bytes())
                .collect();
            self.poke(table + u16::from(kind - 1) * 8, &row);
        }
    }

    /// The ranges the framework must not change: the bytes around SCR_STACK, the bytes of SCR_VARS
    /// it has not claimed, and the bytes after SCR_VARS.
    pub fn guarded(&self) -> Vec<(u16, usize)> {
        let (stack, vars) = (self.stack(), self.label("SCR_VARS"));
        let used = self.label("SCR_USED");
        let size = self.label("SCR_VARS_SIZE");
        let g = usize::from(GUARD);
        vec![
            (stack - GUARD, g),
            (stack + self.label("SCR_STACK_SIZE"), g),
            (vars - GUARD, g),
            (vars + used, usize::from(size - used)),
            (vars + size, g),
        ]
    }

    /// Panics when the framework has written outside its own bytes.
    pub fn check_guards(&mut self) {
        for (at, len) in self.guarded() {
            let got = self.peek(at, len);
            assert!(
                got.iter().all(|&b| b == MARK),
                "bytes outside the framework's changed at {at:04X}: {got:02X?}"
            );
        }
    }

    pub fn call(&mut self, target: &str, regs: Regs) -> Out {
        self.run(target, regs, None).0
    }

    /// Puts the interrupt handler in RAM and points the interrupt vector at it.
    pub fn install_handler(&mut self) {
        let org = 0x4000;
        let start = usize::from(self.label("IRQ_BODY_START") - org);
        let end = usize::from(self.label("IRQ_BODY_END") - org);
        let code = self.image.code[start..end].to_vec();
        self.poke(self.label("IRQ_HANDLER"), &code);
        let at = self.label("IRQ_HANDLER").to_le_bytes();
        self.m.load_flash(0, 0x38, &[0xC3, at[0], at[1]]);
    }

    fn run(&mut self, target: &str, regs: Regs, after: Option<u32>) -> (Out, u8) {
        let word = |v: u16| v.to_le_bytes().to_vec();
        let at = |rig: &Rig, name: &str| rig.label(name);
        let writes = [
            ("TEST_TARGET", word(self.label(target))),
            ("TEST_SP", word(SP)),
            ("TEST_EI", vec![u8::from(self.ei)]),
            ("TEST_MASK", vec![u8::from(after.is_some())]),
            ("IRQ_COUNT", vec![0]),
            ("IN_A", vec![regs.a]),
            ("IN_BC", word(regs.bc)),
            ("IN_DE", word(regs.de)),
            ("IN_HL", word(regs.hl)),
            ("IN_IX", word(0x1D1D)),
            ("IN_IY", word(IY)),
        ];
        for (name, bytes) in writes {
            let to = at(self, name);
            self.poke(to, &bytes);
        }
        self.stack_mark = self.stack_mark.wrapping_add(0x3B) | 1;
        let mark = self.stack_mark;
        let window: Vec<u8> = (0..0x80u8)
            .map(|i| mark.wrapping_add(i.wrapping_mul(7)))
            .collect();
        self.poke(SP - 0x80, &window);
        let entry = self.label("CALL_ENTRY");
        let run = match after {
            None => self.m.run(entry, RUN_LIMIT),
            Some(limit) => {
                let first = self.m.run(entry, limit);
                if first.stop == Stop::Limit {
                    self.m.press_key(Machine::KEY_ON);
                    let more = self.m.run_app(RUN_LIMIT);
                    self.m.release_key(Machine::KEY_ON);
                    more
                } else {
                    first
                }
            }
        };
        assert_eq!(run.stop, Stop::Halted, "{target} did not halt");
        self.spent = run.tstates;
        let tstates = run.tstates;
        let out = self.peek(self.label("OUT_AF"), 0x0F);
        let w = |i: usize| u16::from_le_bytes([out[i], out[i + 1]]);
        let (iy, sp, iff) = (w(10), self.peek(self.label("OUT_SP"), 2), out[12]);
        assert_eq!(iy, IY, "{target} changed IY");
        assert_eq!(
            u16::from_le_bytes([sp[0], sp[1]]),
            SP,
            "{target} left the stack unbalanced"
        );
        assert_eq!(iff == 1, self.ei, "{target} changed the interrupt state");
        let after_window = self.peek(SP - 0x80, 0x80);
        let reached = window
            .iter()
            .zip(&after_window)
            .position(|(a, b)| a != b)
            .map_or(0, |at| 0x80 - at);
        self.check_guards();
        let over = self.peek(self.label("FK_OVER"), 1)[0];
        assert_eq!(over, 0, "{target}: the fake kinds' log overflowed");
        let taken = self.peek(self.label("IRQ_COUNT"), 1)[0];
        (
            Out {
                af: w(0),
                bc: w(2),
                hl: w(6),
                tstates,
                stack: reached,
            },
            taken,
        )
    }

    /// T-states a call takes beyond the driver's own.
    pub fn cost(&mut self, target: &str, regs: Regs) -> u32 {
        let base = self.call("CALL_NOP", regs).tstates;
        self.call(target, regs).tstates - base
    }

    /// Clears the log and the queue of statuses `enter` returns, and loads the queue.
    pub fn arm(&mut self, statuses: &[u8]) {
        assert!(statuses.len() <= 16);
        let log = self.label("FK_LOG");
        self.poke(self.label("FK_LOGP"), &log.to_le_bytes());
        self.poke(self.label("FK_OVER"), &[0]);
        let queue = self.label("FK_STATQ");
        self.poke(self.label("FK_STATQ_P"), &queue.to_le_bytes());
        let mut padded = statuses.to_vec();
        padded.resize(16, 0);
        self.poke(queue, &padded);
    }

    pub fn script(&mut self, act: Act) {
        let mut entry = vec![act.code(), 0];
        let (de, frame) = match act {
            Act::Jump(de) => (de, [0; 16]),
            Act::Push(frame) => (0, frame),
            _ => (0, [0; 16]),
        };
        entry.extend(de.to_le_bytes());
        entry.extend(frame);
        self.poke(self.label("FK_SCRIPT"), &entry);
    }

    pub fn log(&mut self) -> Vec<Rec> {
        let start = self.label("FK_LOG");
        let end = self.peek(self.label("FK_LOGP"), 2);
        let end = u16::from_le_bytes([end[0], end[1]]);
        let bytes = self.peek(start, usize::from(end - start));
        bytes
            .chunks(24)
            .map(|r| Rec {
                routine: match r[0] {
                    1 => Routine::Enter,
                    2 => Routine::Draw,
                    3 => Routine::Key,
                    4 => Routine::Land,
                    other => panic!("a log record of routine {other}"),
                },
                kind: r[1],
                at: u16::from_le_bytes([r[2], r[3]]),
                frame: r[4..20].try_into().unwrap(),
                de: u16::from_le_bytes([r[20], r[21]]),
                a: r[22],
                b: r[23],
            })
            .collect()
    }

    pub fn depth(&mut self) -> usize {
        usize::from(self.call("SCR_DEPTH", Regs::default()).a())
    }

    /// The open frames, bottom first.
    pub fn frames(&mut self) -> Vec<Frame> {
        let depth = self.depth();
        let stack = self.stack();
        self.peek(stack, depth * 16)
            .chunks(16)
            .map(|f| f.try_into().unwrap())
            .collect()
    }

    pub fn top(&mut self) -> (u16, Frame) {
        let hl = self.call("SCR_TOP", Regs::default()).hl;
        let frame = self.peek(hl, 16).try_into().unwrap();
        (hl, frame)
    }

    pub fn init(&mut self, first: Frame, statuses: &[u8]) -> (u8, Vec<Rec>) {
        let at = self.label("FK_SCRIPT") + 0x100;
        self.poke(at, &first);
        self.arm(statuses);
        let regs = Regs {
            hl: self.table(),
            de: at,
            ..Regs::default()
        };
        let out = self.call("SCR_INIT", regs);
        (out.a(), self.log())
    }

    /// One `scr_push_frame` of `new`, which sits outside the stack, while the enters return
    /// `statuses` in turn.
    pub fn push_frame(&mut self, new: Frame, statuses: &[u8]) -> Out {
        let at = self.label("FK_SCRIPT") + 0x100;
        self.poke(at, &new);
        self.arm(statuses);
        let regs = Regs {
            hl: at,
            ..Regs::default()
        };
        self.call("SCR_PUSH_FRAME", regs)
    }

    /// One `scr_key`: the fake key routine returns `act`, and the enters return `statuses` in turn.
    pub fn key(&mut self, key: u8, ch: u8, act: Act, statuses: &[u8]) -> (Out, Vec<Rec>) {
        let (out, log, _) = self.key_at(key, ch, act, statuses, None);
        (out, log)
    }

    /// `key`, with an interrupt after `after` T-states when given; also returns how many the
    /// handler took.
    pub fn key_at(
        &mut self,
        key: u8,
        ch: u8,
        act: Act,
        statuses: &[u8],
        after: Option<u32>,
    ) -> (Out, Vec<Rec>, u8) {
        self.arm(statuses);
        self.script(act);
        let regs = Regs {
            a: key,
            bc: u16::from(ch) << 8,
            ..Regs::default()
        };
        let (out, taken) = self.run("SCR_KEY", regs, after);
        (out, self.log(), taken)
    }
}
