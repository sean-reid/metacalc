//! Standing on one of the app's pages: the machine with the app installed, and calls into it.

use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs, libraries_model, libraries_pixels};
use crate::ui_support::{pack, same_frame};
use mcemu::{Machine, Regs, STUB_ROM, Stop};

pub use crate::built::*;

/// Where the driver and the callee's buffers sit, in RAM below the app's own regions.
pub const DRIVER: u16 = 0xA000;
pub const SCRATCH: u16 = 0xA800;
pub const STACK: u16 = 0xAF00;
const FLAGS: u16 = 0x89F0;
pub const UI_FRAME: u16 = 0x9340;
pub const UI_VARS: u16 = 0x86EC;
pub const FRAME_LEN: usize = 768;
/// The ranges a driver run writes: the driver, a callee's buffers, the parameter block, the
/// scratch, a stack and the strings of the frame test.
pub const DRIVER_RAM: [(u16, u16); 7] = [
    (0xA000, 0xA010),
    (0xA100, 0xA200),
    (0xA400, 0xA410),
    (0xA600, 0xA620),
    (0xA800, 0xA900),
    (0xAE00, 0xAF00),
    (0xB000, 0xB100),
];
const LIMIT: u32 = 100_000_000;
/// The RAM page the reader puts in bank C for a read window (LIB_SPARE_PAGE in lib/ram.inc).
const SPARE_PAGE: u8 = 3;
/// What port 7 reads while bank B shows the RAM the app keeps its variables in.
const RAM_BANK_B: u8 = 0x81;
const BOOT: u32 = 3_000_000;

/// The pages' numbers as pages.inc lays them out, written here and not read from the app.
pub const DRAW_PAGE: u8 = BASE - 1;
pub const DATA_PAGE: u8 = BASE - 2;
/// The library's chunk 0 follows the three code pages.
pub const LIBRARY_PAGE: u8 = BASE - 3;

/// The reference OS with the app started from the APPS menu, waiting in the OS's `_GetKey`, where
/// port 6 reads the OS's page and not the app's.
pub fn started_on_real_os(built: &'static Built) -> Machine {
    let mut os = RealOs::boot(&built.file);
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(500);
    os.m
}

/// Sends the entry page's far entry `stub` to the RAM routine at `hook` before it goes on: the
/// stub's last three bytes, its `jp far_pageN`, become `jp hook`. The hook ends with a jump to the
/// far routine itself.
pub fn hook_stub(m: &mut Machine, built: &Built, stub: &str, hook: u16) {
    let at = built.label(stub);
    let [lo, hi] = hook.to_le_bytes();
    m.load_flash(BASE, usize::from(at - 0x4000) + 4, &[0xC3, lo, hi]);
}

/// What a routine is called with.
#[derive(Clone, Copy, Default)]
pub struct In {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    /// The flags register.
    pub f: u8,
}

/// What it came back with, the T-states the driver's call and halt took included.
pub struct Done {
    pub regs: Regs,
    pub tstates: u32,
    pub page: u8,
}

impl Done {
    pub fn status(&self) -> u8 {
        (self.regs.af >> 8) as u8
    }
}

pub struct App {
    pub m: Machine,
    pub built: &'static Built,
    /// Where `call` puts its driver, its stack and the ranges it writes: `DRIVER_RAM` on the stub
    /// OS, a small place inside A800h to AA0Fh, which other real-OS tests write too, on the reference
    /// OS.
    pub driver: u16,
    pub stack: u16,
    pub scratch: Vec<(u16, u16)>,
    /// The reference OS runs the app: its banks are not the stub's, and the rest check passes.
    real: bool,
}

impl App {
    /// The app installed, launched and run until it waits for a key, on the stub OS.
    pub fn boot(built: &'static Built) -> App {
        App::boot_with(built, |_| {})
    }

    /// `boot` with `setup` run on the launched machine before the app's first instruction: it may
    /// fill RAM or patch the installed pages.
    pub fn boot_with(built: &'static Built, setup: impl FnOnce(&mut Machine)) -> App {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&built.file, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        setup(&mut m);
        assert_eq!(m.run_app(BOOT).stop, Stop::Limit, "the app waits for a key");
        App::with(m, built)
    }

    pub fn with(m: Machine, built: &'static Built) -> App {
        App {
            m,
            built,
            driver: DRIVER,
            stack: STACK,
            scratch: DRIVER_RAM.to_vec(),
            real: false,
        }
    }

    /// `boot`, and the app has drawn the Libraries screen the model draws for its library: the frame
    /// it left is the model's, so it reached its key wait by its own path.
    pub fn start(built: &'static Built) -> App {
        let mut app = App::boot(built);
        let want = pack(&libraries_model(built.shown().as_ref()));
        same_frame("the Libraries screen", &app.frame(), &want);
        app
    }

    /// The app installed on the reference OS and started from the APPS menu, left at its key wait.
    pub fn on_real_os(built: &'static Built) -> App {
        let mut app = App::on_real_os_in(started_on_real_os(built), built);
        let screen = libraries_pixels(built.shown().as_ref());
        let lcd = app.m.lcd();
        let lit: Vec<bool> = (0..64)
            .flat_map(|y| (0..96).map(move |x| (x, y)))
            .map(|(x, y)| lcd.pixel(x, y))
            .collect();
        assert!(lit == screen, "the app has not reached its key wait");
        app
    }

    /// `m`, the reference OS with `built` waiting for a key, as an App with the driver and stack
    /// `call` and `timed` use there: a driver, its callee's buffer and a stack of 64 bytes, all in
    /// A800h to AA0Fh.
    pub fn on_real_os_in(m: Machine, built: &'static Built) -> App {
        let mut app = App::with(m, built);
        app.driver = 0xA800;
        app.stack = 0xAA00;
        app.scratch = vec![(0xA800, 0xA810), (0xA9C0, 0xAA00)];
        app.real = true;
        app
    }

    pub fn label(&self, name: &str) -> u16 {
        self.built.label(name)
    }

    /// Whether the CPU is inside a reader, fm or kernel call, where bank B can show the library's
    /// flash page and bank C the spare RAM page.
    pub fn in_call(&mut self) -> bool {
        self.m.in_port(7) != RAM_BANK_B || self.m.in_port(5) == SPARE_PAGE
    }

    /// Panics, naming the banks, when the CPU is inside a reader, fm or kernel call that maps
    /// another bank. It runs no instruction, so a read or write after it sits where the test
    /// stopped. Passes on the reference OS, whose banks are its own.
    pub fn assert_at_rest(&mut self) {
        if self.real || !self.in_call() {
            return;
        }
        panic!(
            "the stop is inside a call that maps another bank: pc {:04X}, port 7 = {:02X} (the \
             app's RAM is {RAM_BANK_B:02X}), port 5 = {:02X} (the spare page is {SPARE_PAGE:02X})",
            self.m.regs().pc,
            self.m.in_port(7),
            self.m.in_port(5)
        );
    }

    /// `len` bytes of RAM at `at`, read where the machine stands once it is at rest.
    pub fn peek(&mut self, at: u16, len: usize) -> Vec<u8> {
        self.assert_at_rest();
        self.m.read_ram(at, len)
    }

    /// Writes `bytes` to RAM at `at` where the machine stands once it is at rest.
    pub fn poke(&mut self, at: u16, bytes: &[u8]) {
        self.assert_at_rest();
        self.m.load_ram(at, bytes);
    }

    /// The frame on the canvas, read where the machine stands once it is at rest.
    pub fn frame(&mut self) -> Vec<u8> {
        self.peek(UI_FRAME, FRAME_LEN)
    }

    /// Calls `target` from the page numbered `page`, as port 6 reads there, by a driver in RAM that
    /// calls it and halts, with interrupts off, and puts port 6 back on the entry page.
    pub fn call(&mut self, page: u8, target: u16, input: In) -> Done {
        self.assert_scratch_is_free();
        let [lo, hi] = target.to_le_bytes();
        self.m
            .load_ram(self.driver, &[0xCD, lo, hi, 0x76, 0x18, 0xFD]);
        self.m.out_port(0x06, page);
        self.m.set_regs(&Regs {
            af: u16::from(input.a) << 8 | u16::from(input.f),
            bc: input.bc,
            de: input.de,
            hl: input.hl,
            ix: 0x1111,
            iy: FLAGS,
            sp: self.stack,
            pc: self.driver,
            af2: 0x2222,
            bc2: 0x3333,
            de2: 0x4444,
            hl2: 0x5555,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        let run = self.m.resume(LIMIT);
        assert!(
            matches!(run.stop, Stop::Limit | Stop::Halted),
            "the call to {target:04X} ended with {:?}",
            run.stop
        );
        let regs = self.m.regs();
        assert!(
            (self.driver + 3..=self.driver + 5).contains(&regs.pc),
            "the call to {target:04X} stopped at {:04X}",
            regs.pc
        );
        let done = Done {
            regs,
            tstates: run.tstates,
            page: self.m.in_port(0x06),
        };
        self.m.out_port(0x06, BASE);
        done
    }

    /// `call` for the reference OS, where the driver's HALT with interrupts off never ends a run:
    /// the driver jumps to the entry page's `screen_ret` instead, the run stops there, and the
    /// T-states are those from the driver's first instruction to it, less what the same driver takes
    /// to call a plain `ret`. Interrupts are off, so no handler's time is in it.
    pub fn timed(&mut self, page: u8, target: u16, input: In) -> Done {
        let overhead = self
            .run_timed(BASE, self.label("screen_ret"), In::default())
            .1;
        let (mut done, tstates) = self.run_timed(page, target, input);
        done.tstates = tstates - overhead;
        done
    }

    fn run_timed(&mut self, page: u8, target: u16, input: In) -> (Done, u32) {
        self.assert_scratch_is_free();
        let stop = self.label("screen_ret");
        let [lo, hi] = target.to_le_bytes();
        let [slo, shi] = stop.to_le_bytes();
        self.m
            .load_ram(self.driver, &[0xCD, lo, hi, 0xC3, slo, shi]);
        self.m.out_port(0x06, page);
        self.m.set_regs(&Regs {
            af: u16::from(input.a) << 8 | u16::from(input.f),
            bc: input.bc,
            de: input.de,
            hl: input.hl,
            ix: 0x1111,
            iy: FLAGS,
            sp: self.stack,
            pc: self.driver,
            af2: 0x2222,
            bc2: 0x3333,
            de2: 0x4444,
            hl2: 0x5555,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        let start = self.m.clock();
        let reached = self.m.run_to(BASE, stop, LIMIT);
        assert_eq!(reached, mcemu::Reached::Target, "the call to {target:04X}");
        let tstates = self.m.clock().wrapping_sub(start);
        let regs = self.m.regs();
        let done = Done {
            regs,
            tstates,
            page: self.m.in_port(0x06),
        };
        self.m.out_port(0x06, BASE);
        (done, tstates)
    }

    /// The RAM a driver run writes, `DRIVER_RAM`, lies clear of every region of the app's RAM map.
    fn assert_scratch_is_free(&self) {
        let regions = [
            ("UI_VARS", "UI_VARS_SIZE"),
            ("UI_FRAME", "UI_FRAME_SIZE"),
            ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
            ("FAR_VARS", "FAR_VARS_SIZE"),
            ("LIB_STATE", "LIB_STATE_SIZE"),
            ("APP_BUF", "APP_BUF_SIZE"),
            ("APP_SENTINEL", "APP_SENTINEL_SIZE"),
            ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
        ];
        for (base, size) in regions {
            let (low, len) = (self.label(base), self.label(size));
            for &(from, to) in &self.scratch {
                assert!(
                    to <= low || low + len <= from,
                    "the driver's RAM {from:04X} to {to:04X} overlaps {base} at {low:04X}"
                );
            }
        }
    }

    /// The entry page's far entry named `name`, called from the entry page.
    pub fn far(&mut self, name: &str, input: In) -> Done {
        let at = self.label(name);
        self.call(BASE, at, input)
    }
}
