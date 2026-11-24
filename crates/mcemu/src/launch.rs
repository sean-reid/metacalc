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
/// The OS keeps one two-byte launch budget per app slot in flash page 7Eh, from 5E50h, slot 0
/// being the app on page 69h and each lower base page the next slot. A budget the OS left at
/// FFFFh allows 16 launches, after which it deletes the app and defragments; 80h, 00h marks an
/// app with no limit, which is what its own install writes for an app without a trial field.
const BUDGET_PAGE: u8 = 0x7E;
const BUDGET_TABLE: usize = 0x1E50;
const TOP_APP_PAGE: u8 = 0x69;
const UNLIMITED: [u8; 2] = [0x80, 0x00];

/// The offset in page 7Eh of the launch budget of the app on `base`, which is 69h or below.
fn budget_slot(base: u8) -> usize {
    BUDGET_TABLE + 2 * usize::from(TOP_APP_PAGE - base)
}

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

    /// Writes page k of the app file to flash page `base - k`, and remembers how many pages that
    /// is for `launch`, per base page. On a real OS image it also marks the app unlimited, as the
    /// OS's own install does, and refuses a base above 69h, which has no slot in the OS's table.
    /// The table address is the SE image's with that half of the sector active; a slot holding
    /// anything but FFh FFh or the mark is refused, and nothing is written.
    pub fn install_app(&mut self, file: &[u8], base: u8) -> Result<(), AppError> {
        self.check_base(base)?;
        if self.os && base > TOP_APP_PAGE {
            return Err(AppError(
                "the OS's launch table has no slot for the base page",
            ));
        }
        let pages = decode_app(file)?;
        if pages.len() > usize::from(base - FIRST_PROTECTED) {
            return Err(AppError("the pages do not fit below the base page"));
        }
        if self.os {
            let slot = budget_slot(base);
            let held = self.read_flash(BUDGET_PAGE, slot, 2);
            if held != [0xFF, 0xFF] && held != UNLIMITED {
                return Err(AppError(
                    "the launch table is not where this image keeps it",
                ));
            }
        }
        for (k, page) in pages.iter().enumerate() {
            self.load_flash(base - k as u8, 0, page);
        }
        if self.os {
            self.load_flash(BUDGET_PAGE, budget_slot(base), &UNLIMITED);
        }
        self.apps.insert(base, pages.len());
        Ok(())
    }

    /// Leaves the machine as TI-OS does at the first instruction of an app installed with `base`,
    /// ready for `run_app`. Port 23h ends the protected range at the page below the app's last
    /// one, so the pages of an app of any length can run; an app that was not installed is one
    /// page.
    pub fn launch(&mut self, base: u8) -> Result<(), AppError> {
        self.check_base(base)?;
        let pages = self.apps.get(&base).copied().unwrap_or(1);
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
            (0x23, base - pages as u8),
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
