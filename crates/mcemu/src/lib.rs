//! A TI-84 Plus SE on the TilEm core, for tests. The C core is built from the TilEm
//! source named by `TILEM_SRC`.

mod ffi;

/// Free stub ROM: boots to a known state and jumps to `ENTRY`.
pub const STUB_ROM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/stub.rom"));

/// Where the stub jumps after boot, in flash page `FLASH_TEST_PAGE`.
pub const ENTRY: u16 = 0x4000;
pub const FLASH_TEST_PAGE: u8 = 0x20;

pub const LCD_WIDTH: usize = 96;
pub const LCD_HEIGHT: usize = 64;

const PORT_SPEED: u8 = 0x20;
const FLASH_PAGE_SIZE: usize = 0x4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Mhz6,
    Mhz15,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Halted,
    Limit,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub stop: Stop,
    pub tstates: u32,
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
}

impl Machine {
    /// Loads `rom`, resets, and runs the boot code up to `ENTRY`.
    pub fn new(rom: &[u8]) -> Result<Machine, BootError> {
        ffi::Calc::new(rom)
            .map(|calc| Machine { calc })
            .ok_or(BootError)
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

    pub fn load_ram(&mut self, addr: u16, bytes: &[u8]) {
        for (a, &b) in (addr..=u16::MAX).zip(bytes) {
            let p = self.calc.logical_to_physical(a);
            self.calc.mem_mut()[p] = b;
        }
    }

    pub fn read_ram(&mut self, addr: u16, len: usize) -> Vec<u8> {
        (addr..=u16::MAX)
            .take(len)
            .map(|a| {
                let p = self.calc.logical_to_physical(a);
                self.calc.mem_mut()[p]
            })
            .collect()
    }

    pub fn out_port(&mut self, port: u8, value: u8) {
        self.calc.out_port(port, value);
    }

    pub fn set_speed(&mut self, speed: Speed) {
        self.out_port(PORT_SPEED, u8::from(speed == Speed::Mhz15));
    }

    pub fn press_key(&mut self, scancode: u8) {
        self.calc.key(scancode, true);
    }

    pub fn release_key(&mut self, scancode: u8) {
        self.calc.key(scancode, false);
    }

    /// Runs from `pc` with interrupts off until HALT executes or `limit` T-states pass.
    /// The count includes the HALT.
    pub fn run(&mut self, pc: u16, limit: u32) -> Run {
        let limit = i32::try_from(limit).expect("limit fits in i32");
        let (reason, tstates) = self.calc.run(pc, limit);
        let stop = match reason {
            0 => Stop::Limit,
            1 => Stop::Halted,
            _ => Stop::Fault,
        };
        Run { stop, tstates }
    }

    pub fn lcd(&mut self) -> Lcd {
        let mut raw = vec![0u8; LCD_WIDTH * LCD_HEIGHT];
        assert!(
            self.calc.lcd(&mut raw, LCD_WIDTH, LCD_HEIGHT),
            "unexpected LCD size"
        );
        Lcd(raw.into_iter().map(|p| p != 0).collect())
    }
}
