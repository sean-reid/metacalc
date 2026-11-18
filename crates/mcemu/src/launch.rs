//! Installing an app and entering it the way TI-OS does.

use crate::{AppError, FLASH_PAGE_SIZE, Machine, Regs, Run, Speed, Stop, decode_app, stub};

/// Where an app is entered, and the stack TI-OS gives it.
pub const APP_ENTRY: u16 = 0x4080;
pub const APP_STACK: u16 = 0xFFE3;
/// The flags pointer TI-OS leaves in IY.
pub const FLAGS: u16 = 0x89F0;

/// Flash pages below this one are never protected from execution.
const FIRST_PROTECTED: u8 = 0x08;
const RAM_PAGE_BANK_B: u8 = 0x81;
const INIT_LIMIT: u32 = 100_000;

impl Machine {
    fn check_base(&self, base: u8) -> Result<(), AppError> {
        if base <= FIRST_PROTECTED {
            return Err(AppError("the base page is in the OS's range"));
        }
        if usize::from(base) >= self.calc.rom_size() / FLASH_PAGE_SIZE - 1 {
            return Err(AppError("the base page is past the user flash"));
        }
        Ok(())
    }

    /// Writes page k of the app file to flash page `base - k`.
    pub fn install_app(&mut self, file: &[u8], base: u8) -> Result<(), AppError> {
        self.check_base(base)?;
        let pages = decode_app(file)?;
        if pages.len() > usize::from(base - FIRST_PROTECTED) {
            return Err(AppError("the pages do not fit below the base page"));
        }
        for (k, page) in pages.iter().enumerate() {
            self.load_flash(base - k as u8, 0, page);
        }
        Ok(())
    }

    /// Leaves the machine as TI-OS does at the first instruction of an app installed with `base`,
    /// ready for `run_app`.
    pub fn launch(&mut self, base: u8) -> Result<(), AppError> {
        self.check_base(base)?;
        let init = self.run(stub::OS_INIT, INIT_LIMIT);
        assert_eq!(init.stop, Stop::Halted, "the stub's launch setup finishes");
        self.set_speed(Speed::Mhz6);
        let ports: [(u8, u8); 19] = [
            (0x03, 0x0B),
            (0x04, 0x06),
            (0x05, 0x00),
            (0x06, base),
            (0x07, RAM_PAGE_BANK_B),
            (0x21, 0x01),
            (0x22, FIRST_PROTECTED),
            (0x23, base - 1),
            (0x25, 0x10),
            (0x26, 0x20),
            (0x27, 0x00),
            (0x28, 0x00),
            (0x29, 0x17),
            (0x2A, 0x27),
            (0x2B, 0x2F),
            (0x2C, 0x3B),
            (0x2D, 0x02),
            (0x2E, 0x45),
            (0x2F, 0x4B),
        ];
        for (port, value) in ports {
            self.calc.out_port_unlocked(port, value);
        }
        let [lo, hi] = stub::APP_RETURN.to_le_bytes();
        self.load_ram(APP_STACK, &[lo, hi]);
        self.calc.set_regs(&Regs {
            af: 0x0044,
            bc: 0x0000,
            de: 0x8599,
            hl: 0x843C,
            ix: 0,
            iy: FLAGS,
            sp: APP_STACK,
            pc: APP_ENTRY,
            af2: 0x0B5C,
            bc2: 0x00FE,
            de2: 0x0900,
            hl2: 0x8448,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        Ok(())
    }

    /// Runs the launched app on from where it is, as `resume` does.
    pub fn run_app(&mut self, limit: u32) -> Run {
        self.resume(limit)
    }
}
