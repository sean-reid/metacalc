//! A TI-84 Plus SE on the TilEm core, for tests. The C core is built from the TilEm
//! source named by `TILEM_SRC`.

mod app;
mod ffi;
mod launch;
mod vars;
mod stub {
    include!(concat!(env!("OUT_DIR"), "/stub_labels.rs"));
}

pub use app::{AppError, decode as decode_app};
pub use ffi::Regs;
pub use vars::{E_BAD_CALL, E_MEMORY, NoRoom, Var};

/// Free stub ROM: boots to a known state and jumps to `ENTRY`.
pub const STUB_ROM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/stub.rom"));

/// Where the stub jumps after boot, in flash page `FLASH_TEST_PAGE`.
pub const ENTRY: u16 = 0x4000;
pub const FLASH_TEST_PAGE: u8 = 0x20;

pub const LCD_WIDTH: usize = 96;
pub const LCD_HEIGHT: usize = 64;

const PORT_SPEED: u8 = 0x20;
const FLASH_PAGE_SIZE: usize = 0x4000;
pub const RAM_PAGE_SIZE: usize = 0x4000;
/// The one spare page the calculator has: 82h to 87h all reach it.
const FIRST_SPARE_PAGE: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Mhz6,
    Mhz15,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A HALT executed with interrupts off, or one in the stub's own code. A HALT with
    /// interrupts on is the app waiting for the next one, and the run goes on.
    Halted,
    Limit,
    /// The core raised a hardware exception or stopped for another reason of its own. The
    /// core resets the machine on every exception, so after this the memory map is the
    /// reset one: `read_ram` reads flash, nothing loaded is where it was, and the machine
    /// is not to be used again.
    Fault,
    /// The app left to the home screen through `_JForceCmd` or `_JForceCmdNoChar`, the two the
    /// stub takes as an exit, with the machine as the OS needs it: port 5 at 0, port 7 at 81h,
    /// 6 MHz and interrupt mode 1.
    Exited,
    /// The app left to the home screen with the machine in another state.
    BadExit(ExitState),
    /// The app returned from its entry point instead of leaving to the home screen.
    Returned,
    /// The stub's LCD wait polled its limit without the controller becoming ready.
    LcdStuck,
    /// The app made a `bcall` the stub does not implement; the address is the one called.
    UnsupportedBcall(u16),
    /// The app made a `bjump` the stub does not implement.
    UnsupportedBjump(u16),
    /// An OS call raised an error the app did not catch, as TI-OS does with ERR:MEMORY; the
    /// code is the one in A.
    OsError(u8),
}

/// What the machine looked like when the app left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitState {
    pub port5: u8,
    pub port7: u8,
    pub speed: Speed,
    pub im: u8,
}

impl ExitState {
    /// What TI-OS needs: RAM page 0 in bank C, RAM page 1 in bank B, 6 MHz and interrupt mode 1.
    pub const CLEAN: ExitState = ExitState {
        port5: 0,
        port7: 0x81,
        speed: Speed::Mhz6,
        im: 1,
    };
}

/// What ended a `run_to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reached {
    Target,
    /// 0038h is the next instruction: an interrupt in mode 1 was taken, or an RST 38h ran. The
    /// target was not reached; another `run_to` goes on from there.
    Vector38,
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub stop: Stop,
    pub tstates: u32,
}

impl Run {
    /// What a run with a limit of 0 reports.
    const NO_TIME: Run = Run {
        stop: Stop::Limit,
        tstates: 0,
    };
}

#[derive(Debug)]
pub struct BootError;

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the ROM image did not load or did not boot to the entry point")
    }
}

impl std::error::Error for BootError {}

/// The 96x64 screen, one bool per pixel.
pub struct Lcd(Vec<bool>);

impl Lcd {
    fn from_bytes(raw: Vec<u8>) -> Lcd {
        Lcd(raw.into_iter().map(|p| p != 0).collect())
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        self.0[y * LCD_WIDTH + x]
    }

    pub fn row(&self, y: usize) -> &[bool] {
        &self.0[y * LCD_WIDTH..(y + 1) * LCD_WIDTH]
    }

    pub fn lit(&self) -> usize {
        self.0.iter().filter(|&&p| p).count()
    }
}

pub struct Machine {
    calc: ffi::Calc,
    /// The page count of each app installed, by base page, which `launch` reads.
    apps: std::collections::HashMap<u8, usize>,
}

impl Machine {
    /// Loads `rom`, resets, and runs the boot code up to `ENTRY`.
    pub fn new(rom: &[u8]) -> Result<Machine, BootError> {
        ffi::Calc::new(rom)
            .map(|calc| Machine {
                calc,
                apps: Default::default(),
            })
            .ok_or(BootError)
    }

    /// Resets a real OS image and returns before it runs, so an app can be written into its flash
    /// first. Everything else works as for the stub, except that nothing stops at a HALT and
    /// `run_time` is the way to let the OS run.
    pub fn new_os(rom: &[u8]) -> Result<Machine, BootError> {
        ffi::Calc::new_os(rom)
            .map(|calc| Machine {
                calc,
                apps: Default::default(),
            })
            .ok_or(BootError)
    }

    /// Runs on from the current state for `microseconds` of emulated time, interrupts as the code
    /// left them.
    pub fn run_time(&mut self, microseconds: u32) -> Run {
        if microseconds == 0 {
            return Run::NO_TIME;
        }
        let microseconds = i32::try_from(microseconds).expect("time fits in i32");
        let reason = self.calc.run_time(microseconds);
        Run {
            stop: if reason == 0 {
                Stop::Limit
            } else {
                Stop::Fault
            },
            tstates: 0,
        }
    }

    /// What a read of `port` returns.
    pub fn in_port(&mut self, port: u8) -> u8 {
        self.calc.in_port(port)
    }

    pub fn load_flash(&mut self, page: u8, offset: usize, bytes: &[u8]) {
        let start = usize::from(page) * FLASH_PAGE_SIZE + offset;
        assert!(
            offset + bytes.len() <= FLASH_PAGE_SIZE,
            "write crosses the flash page"
        );
        assert!(
            start + bytes.len() <= self.calc.rom_size(),
            "flash page out of range"
        );
        self.calc.mem_mut()[start..start + bytes.len()].copy_from_slice(bytes);
    }

    /// `len` bytes of flash page `page` from `offset`.
    pub fn read_flash(&mut self, page: u8, offset: usize, len: usize) -> Vec<u8> {
        let start = usize::from(page) * FLASH_PAGE_SIZE + offset;
        assert!(
            offset + len <= FLASH_PAGE_SIZE,
            "read crosses the flash page"
        );
        assert!(
            start + len <= self.calc.rom_size(),
            "flash page out of range"
        );
        self.calc.mem_mut()[start..start + len].to_vec()
    }

    /// Runs on, interrupts as the code left them, until the instruction at `addr` of flash page
    /// `page` is the next to execute, and says what ended the run. `addr` is where the page is
    /// mapped in bank A, 4000h to 7FFFh; the same address on another page does not stop the run.
    pub fn run_to(&mut self, page: u8, addr: u16, limit: u32) -> Reached {
        assert!((0x4000..0x8000).contains(&addr), "the address is in bank A");
        assert!(
            usize::from(page) < self.calc.rom_size() / FLASH_PAGE_SIZE,
            "flash page out of range"
        );
        let physical = usize::from(page) * FLASH_PAGE_SIZE + usize::from(addr - 0x4000);
        let limit = i32::try_from(limit).expect("limit fits in i32");
        match self.calc.run_to(physical as u32, limit) {
            1 => Reached::Target,
            2 => Reached::Vector38,
            _ => Reached::Limit,
        }
    }

    /// The CPU's clock in T-states. It wraps at 32 bits, so two readings are compared with
    /// `wrapping_sub`.
    pub fn clock(&self) -> u32 {
        self.calc.clock()
    }

    pub fn load_ram(&mut self, addr: u16, bytes: &[u8]) {
        assert!(
            usize::from(addr) + bytes.len() <= 0x1_0000,
            "past the end of memory"
        );
        for (a, &b) in (addr..=u16::MAX).zip(bytes) {
            let p = self.calc.logical_to_physical(a);
            self.calc.mem_mut()[p] = b;
        }
    }

    pub fn read_ram(&mut self, addr: u16, len: usize) -> Vec<u8> {
        assert!(
            usize::from(addr) + len <= 0x1_0000,
            "past the end of memory"
        );
        (addr..=u16::MAX)
            .take(len)
            .map(|a| {
                let p = self.calc.logical_to_physical(a);
                self.calc.mem_mut()[p]
            })
            .collect()
    }

    /// RAM page `n`, the one that bank C shows after a write of `n` to port 5. Pages 2 to 7
    /// are one page, as on the calculator, so any `n` from 2 to 7 returns the same bytes.
    pub fn ram_page(&mut self, n: u8) -> &mut [u8] {
        assert!(n < 8, "RAM page out of range");
        let n = n.min(FIRST_SPARE_PAGE);
        let start = self.calc.rom_size() + usize::from(n) * RAM_PAGE_SIZE;
        let mem = self.calc.mem_mut();
        &mut mem[start..start + RAM_PAGE_SIZE]
    }

    /// The (port, value) pairs written to ports 5, 6 and 7 since this machine's last `run` or
    /// `resume` began, sorted by port and then value and without repeats. It is a set: it says
    /// which values were written, not in what order or how often. Pages 82h to 87h are one
    /// page, so this is the only record of which page number the code selected.
    pub fn banks_written(&mut self) -> Vec<(u8, u8)> {
        self.calc.banks_written()
    }

    /// Forces the LCD busy bit, bit 7 of port 10h, on for every read, as a controller that never
    /// finishes would.
    pub fn stick_lcd_busy(&mut self, on: bool) {
        self.calc.stick_lcd_busy(on);
    }

    pub fn out_port(&mut self, port: u8, value: u8) {
        self.calc.out_port(port, value);
    }

    pub fn set_speed(&mut self, speed: Speed) {
        self.out_port(PORT_SPEED, u8::from(speed == Speed::Mhz15));
    }

    /// Where the next instruction is read from; after a run that hit its limit, the place to resume.
    pub fn pc(&self) -> u16 {
        self.calc.pc()
    }

    /// The scancode of the ON key, which `press_key` and `release_key` accept like any other.
    pub const KEY_ON: u8 = 0x29;

    pub fn press_key(&mut self, scancode: u8) {
        self.calc.key(scancode, true);
    }

    pub fn release_key(&mut self, scancode: u8) {
        self.calc.key(scancode, false);
    }

    /// Runs from `pc` with interrupts off until a HALT executes with them off or `limit`
    /// T-states pass. The code may turn them on, and then its HALTs wait for an interrupt
    /// instead of ending the run. The count includes the HALT. A stub service that ends the
    /// run reports its own stop. A `limit` of 0 returns `Stop::Limit` with 0 T-states and
    /// leaves the machine untouched, `pc` and interrupts included.
    ///
    /// After `Stop::Fault` the core has reset the machine: see `Stop::Fault`.
    pub fn run(&mut self, pc: u16, limit: u32) -> Run {
        if limit == 0 {
            return Run::NO_TIME;
        }
        let limit = i32::try_from(limit).expect("limit fits in i32");
        let (reason, tstates) = self.calc.run(pc, limit);
        self.stopped(reason, tstates)
    }

    /// Runs on from where the last run stopped, registers and interrupt state as the code left
    /// them, until a HALT executes with interrupts off or `limit` T-states pass. After a run
    /// that hit its limit this continues it, where `run(pc, ..)` would turn interrupts off. After
    /// `Stop::Halted` the CPU stays halted and only an interrupt wakes it, so a run that halted
    /// with interrupts off goes on with `run`. A `limit` of 0 returns `Stop::Limit` with 0
    /// T-states and runs nothing.
    pub fn resume(&mut self, limit: u32) -> Run {
        if limit == 0 {
            return Run::NO_TIME;
        }
        let limit = i32::try_from(limit).expect("limit fits in i32");
        let (reason, tstates) = self.calc.resume(limit);
        self.stopped(reason, tstates)
    }

    fn exit_state(&mut self) -> ExitState {
        ExitState {
            port5: self.calc.in_port(0x05),
            port7: self.calc.in_port(0x07),
            speed: if self.calc.khz() == 15000 {
                Speed::Mhz15
            } else {
                Speed::Mhz6
            },
            im: self.calc.regs().im,
        }
    }

    fn stopped(&mut self, reason: u32, tstates: u32) -> Run {
        let stop = match reason {
            0 => Stop::Limit,
            1 => {
                let r = self.calc.regs();
                match r.pc {
                    stub::APP_EXIT => match self.exit_state() {
                        ExitState::CLEAN => Stop::Exited,
                        left => Stop::BadExit(left),
                    },
                    stub::APP_RETURN => Stop::Returned,
                    stub::FAULT_LCD => Stop::LcdStuck,
                    stub::FAULT_BCALL => Stop::UnsupportedBcall(r.bc),
                    stub::FAULT_BJUMP => Stop::UnsupportedBjump(r.bc),
                    stub::FAULT_ERROR => Stop::OsError((r.af >> 8) as u8),
                    _ => Stop::Halted,
                }
            }
            _ => Stop::Fault,
        };
        Run { stop, tstates }
    }

    pub fn regs(&self) -> Regs {
        self.calc.regs()
    }

    /// Replaces the CPU registers, `pc` and the interrupt flip-flops included, and wakes a halted
    /// CPU. A `resume` goes on from there.
    pub fn set_regs(&mut self, regs: &Regs) {
        self.calc.set_regs(regs);
    }

    /// The screen as the core renders it: all pixels off while the controller is off (its
    /// reset state, until the program sends the on command), and also while the CPU is
    /// halted with bit 3 of port 3 clear. The CPU is halted from a HALT that executes until
    /// an interrupt is taken or the next `run`, so a run that ends at a HALT leaves it halted
    /// and a run that ends at its limit does not, unless the limit hit while the code waited
    /// in a HALT with interrupts on. A program that masks port 3 and ends at a HALT reads
    /// blank here although it drew; `lcd_ram` does not depend on either condition.
    pub fn lcd(&mut self) -> Lcd {
        let mut raw = vec![0u8; LCD_WIDTH * LCD_HEIGHT];
        assert!(
            self.calc.lcd(&mut raw, LCD_WIDTH, LCD_HEIGHT),
            "unexpected LCD size"
        );
        Lcd::from_bytes(raw)
    }

    /// The controller's memory as the screen would show it, whatever the LCD-on state, the
    /// halt state or port 3 say; the row shift (the Z address) still applies.
    pub fn lcd_ram(&mut self) -> Lcd {
        let mut raw = vec![0u8; LCD_WIDTH * LCD_HEIGHT];
        self.calc.lcd_ram(&mut raw, LCD_WIDTH, LCD_HEIGHT);
        Lcd::from_bytes(raw)
    }
}
