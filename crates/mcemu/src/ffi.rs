//! The only module with unsafe code: a safe handle over the C harness.

use std::ptr::NonNull;

#[repr(C)]
struct Raw {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn mcemu_new(rom: *const u8, len: usize) -> *mut Raw;
    fn mcemu_free(calc: *mut Raw);
    fn mcemu_mem(calc: *mut Raw) -> *mut u8;
    fn mcemu_rom_size(calc: *const Raw) -> usize;
    fn mcemu_ram_size(calc: *const Raw) -> usize;
    fn mcemu_ltop(calc: *mut Raw, addr: u32) -> u32;
    fn mcemu_out(calc: *mut Raw, port: u8, value: u8);
    fn mcemu_key(calc: *mut Raw, scancode: i32, down: i32);
    fn mcemu_run(calc: *mut Raw, pc: u16, limit: i32, elapsed: *mut u32) -> u32;
    fn mcemu_lcd(calc: *mut Raw, out: *mut u8, width: i32, height: i32) -> i32;
}

pub struct Calc(NonNull<Raw>);

impl Calc {
    pub fn new(rom: &[u8]) -> Option<Calc> {
        // SAFETY: the pointer and length describe `rom`, which the C side copies and does not keep.
        NonNull::new(unsafe { mcemu_new(rom.as_ptr(), rom.len()) }).map(Calc)
    }

    /// Flash followed by RAM, as the core lays them out.
    pub fn mem_mut(&mut self) -> &mut [u8] {
        // SAFETY: the core allocates rom_size + ram_size bytes at `mem` and owns them until free.
        unsafe {
            let len = mcemu_rom_size(self.0.as_ptr()) + mcemu_ram_size(self.0.as_ptr());
            std::slice::from_raw_parts_mut(mcemu_mem(self.0.as_ptr()), len)
        }
    }

    pub fn rom_size(&self) -> usize {
        // SAFETY: the handle is live.
        unsafe { mcemu_rom_size(self.0.as_ptr()) }
    }

    pub fn logical_to_physical(&mut self, addr: u16) -> usize {
        // SAFETY: the handle is live; the core maps any 16-bit address.
        unsafe { mcemu_ltop(self.0.as_ptr(), addr.into()) as usize }
    }

    pub fn out_port(&mut self, port: u8, value: u8) {
        // SAFETY: the handle is live.
        unsafe { mcemu_out(self.0.as_ptr(), port, value) }
    }

    pub fn key(&mut self, scancode: u8, down: bool) {
        // SAFETY: the handle is live.
        unsafe { mcemu_key(self.0.as_ptr(), scancode.into(), down.into()) }
    }

    /// Returns the core's stop reason and the T-states elapsed.
    pub fn run(&mut self, pc: u16, limit: i32) -> (u32, u32) {
        let mut elapsed = 0;
        // SAFETY: the handle is live and `elapsed` outlives the call.
        let reason = unsafe { mcemu_run(self.0.as_ptr(), pc, limit, &mut elapsed) };
        (reason, elapsed)
    }

    pub fn lcd(&mut self, out: &mut [u8], width: usize, height: usize) -> bool {
        assert_eq!(out.len(), width * height);
        // SAFETY: `out` holds width * height bytes, which is all the C side writes.
        unsafe {
            mcemu_lcd(
                self.0.as_ptr(),
                out.as_mut_ptr(),
                width as i32,
                height as i32,
            ) == 0
        }
    }
}

impl Drop for Calc {
    fn drop(&mut self) {
        // SAFETY: the handle is live and never used again.
        unsafe { mcemu_free(self.0.as_ptr()) }
    }
}
