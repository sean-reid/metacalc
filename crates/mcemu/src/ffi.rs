//! The only module with unsafe code: a safe handle over the C harness.

use std::ptr::NonNull;

#[repr(C)]
struct Raw {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn mcemu_new(rom: *const u8, len: usize) -> *mut Raw;
    fn mcemu_new_os(rom: *const u8, len: usize) -> *mut Raw;
    fn mcemu_run_time(calc: *mut Raw, microseconds: i32) -> u32;
    fn mcemu_free(calc: *mut Raw);
    fn mcemu_mem(calc: *mut Raw) -> *mut u8;
    fn mcemu_rom_size(calc: *const Raw) -> usize;
    fn mcemu_ram_size(calc: *const Raw) -> usize;
    fn mcemu_ltop(calc: *mut Raw, addr: u32) -> u32;
    fn mcemu_out(calc: *mut Raw, port: u8, value: u8);
    fn mcemu_in(calc: *mut Raw, port: u8) -> u8;
    fn mcemu_banks(calc: *mut Raw, out: *mut [[u8; 32]; 3]);
    fn mcemu_stick_lcd_busy(calc: *mut Raw, on: i32);
    fn mcemu_khz(calc: *const Raw) -> i32;
    fn mcemu_clock(calc: *const Raw) -> u32;
    fn mcemu_run_to(calc: *mut Raw, physical: u32, limit: i32) -> i32;
    fn mcemu_out_unlocked(calc: *mut Raw, port: u8, value: u8);
    fn mcemu_get_regs(calc: *const Raw, out: *mut Regs);
    fn mcemu_set_regs(calc: *mut Raw, regs: *const Regs);
    fn mcemu_resume(calc: *mut Raw, limit: i32, elapsed: *mut u32) -> u32;
    fn mcemu_key(calc: *mut Raw, scancode: i32, down: i32);
    fn mcemu_run(calc: *mut Raw, pc: u16, limit: i32, elapsed: *mut u32) -> u32;
    fn mcemu_pc(calc: *const Raw) -> u16;
    fn mcemu_watch(calc: *mut Raw, start: u16, len: u32);
    fn mcemu_unwatch(calc: *mut Raw);
    fn mcemu_watch_len(calc: *mut Raw) -> usize;
    fn mcemu_watch_take(calc: *mut Raw, out: *mut Write, cap: usize, lost: *mut usize) -> usize;
    fn mcemu_lcd(calc: *mut Raw, out: *mut u8, width: i32, height: i32) -> i32;
    fn mcemu_lcd_ram(calc: *mut Raw, out: *mut u8, width: i32, height: i32) -> i32;
}

/// The Z80 registers the tests set and read.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub pc: u16,
    pub af2: u16,
    pub bc2: u16,
    pub de2: u16,
    pub hl2: u16,
    pub i: u8,
    pub iff1: u8,
    pub iff2: u8,
    pub im: u8,
}

/// One write the CPU made into a watched range.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Write {
    /// The CPU clock in T-states when the write ran, before the writing instruction's own cost
    /// is added.
    pub clock: u32,
    /// The flat memory offset of `pc` under the bank mapping at the time of the write, flash
    /// first and then RAM.
    pub pc_physical: u32,
    pub addr: u16,
    /// The logical address of the writing instruction's first byte; an interrupt's pushes carry
    /// the instruction that completed before it.
    pub pc: u16,
    /// The byte read back at `addr` after the write, so a write to flash or ROM records the byte
    /// that stays.
    pub value: u8,
}

pub struct Calc(NonNull<Raw>);

impl Calc {
    pub fn new(rom: &[u8]) -> Option<Calc> {
        // SAFETY: the pointer and length describe `rom`, which the C side copies and does not keep.
        NonNull::new(unsafe { mcemu_new(rom.as_ptr(), rom.len()) }).map(Calc)
    }

    /// Resets a real OS image without running it.
    pub fn new_os(rom: &[u8]) -> Option<Calc> {
        // SAFETY: the pointer and length describe `rom`, which the C side copies and does not keep.
        NonNull::new(unsafe { mcemu_new_os(rom.as_ptr(), rom.len()) }).map(Calc)
    }

    pub fn run_time(&mut self, microseconds: i32) -> u32 {
        // SAFETY: the handle is live.
        unsafe { mcemu_run_time(self.0.as_ptr(), microseconds) }
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

    /// Reads a port that has no side effects, such as 5 and 7.
    pub fn in_port(&mut self, port: u8) -> u8 {
        // SAFETY: the handle is live.
        unsafe { mcemu_in(self.0.as_ptr(), port) }
    }

    /// Ports 5, 6 and 7 with the values written to them since the last run began.
    pub fn banks_written(&mut self) -> Vec<(u8, u8)> {
        let mut sets = [[0u8; 32]; 3];
        // SAFETY: the C side writes exactly the 96 bytes of `sets`.
        unsafe { mcemu_banks(self.0.as_ptr(), &mut sets) };
        let mut found = Vec::new();
        for (port, set) in (5u8..).zip(sets) {
            for value in 0..=255u8 {
                if set[usize::from(value >> 3)] >> (value & 7) & 1 == 1 {
                    found.push((port, value));
                }
            }
        }
        found
    }

    pub fn stick_lcd_busy(&mut self, on: bool) {
        // SAFETY: the handle is live.
        unsafe { mcemu_stick_lcd_busy(self.0.as_ptr(), on.into()) }
    }

    pub fn khz(&self) -> i32 {
        // SAFETY: the handle is live.
        unsafe { mcemu_khz(self.0.as_ptr()) }
    }

    /// 1 when the run reached the address, 2 when it reached 0038h first, 0 at the limit.
    pub fn run_to(&mut self, physical: u32, limit: i32) -> i32 {
        // SAFETY: the handle is live.
        unsafe { mcemu_run_to(self.0.as_ptr(), physical, limit) }
    }

    pub fn clock(&self) -> u32 {
        // SAFETY: the handle is live.
        unsafe { mcemu_clock(self.0.as_ptr()) }
    }

    pub fn out_port_unlocked(&mut self, port: u8, value: u8) {
        // SAFETY: the handle is live.
        unsafe { mcemu_out_unlocked(self.0.as_ptr(), port, value) }
    }

    pub fn regs(&self) -> Regs {
        let mut regs = Regs::default();
        // SAFETY: the handle is live and `regs` has the layout the C side writes.
        unsafe { mcemu_get_regs(self.0.as_ptr(), &mut regs) };
        regs
    }

    pub fn set_regs(&mut self, regs: &Regs) {
        // SAFETY: the handle is live and `regs` has the layout the C side reads.
        unsafe { mcemu_set_regs(self.0.as_ptr(), regs) }
    }

    /// Runs on from the current state; returns the core's stop reason and the T-states elapsed.
    pub fn resume(&mut self, limit: i32) -> (u32, u32) {
        let mut elapsed = 0;
        // SAFETY: the handle is live and `elapsed` outlives the call.
        let reason = unsafe { mcemu_resume(self.0.as_ptr(), limit, &mut elapsed) };
        (reason, elapsed)
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

    pub fn pc(&self) -> u16 {
        // SAFETY: the handle is live.
        unsafe { mcemu_pc(self.0.as_ptr()) }
    }

    pub fn watch(&mut self, start: u16, len: u32) {
        // SAFETY: the handle is live.
        unsafe { mcemu_watch(self.0.as_ptr(), start, len) }
    }

    pub fn unwatch(&mut self) {
        // SAFETY: the handle is live.
        unsafe { mcemu_unwatch(self.0.as_ptr()) }
    }

    /// Returns the writes recorded since the last call and the count the record had no room for.
    pub fn take_writes(&mut self) -> (Vec<Write>, usize) {
        // SAFETY: the handle is live.
        let len = unsafe { mcemu_watch_len(self.0.as_ptr()) };
        let mut out = vec![Write::default(); len];
        let mut lost = 0;
        // SAFETY: `out` has room for `len` records and `lost` outlives the call.
        let n = unsafe { mcemu_watch_take(self.0.as_ptr(), out.as_mut_ptr(), len, &mut lost) };
        out.truncate(n);
        (out, lost)
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

    pub fn lcd_ram(&mut self, out: &mut [u8], width: usize, height: usize) {
        assert_eq!(out.len(), width * height);
        // SAFETY: `out` holds width * height bytes, which is all the C side writes.
        let status = unsafe {
            mcemu_lcd_ram(
                self.0.as_ptr(),
                out.as_mut_ptr(),
                width as i32,
                height as i32,
            )
        };
        assert_eq!(
            status, 0,
            "the core's LCD size or read length is not what the harness assumes"
        );
    }
}

impl Drop for Calc {
    fn drop(&mut self) {
        // SAFETY: the handle is live and never used again.
        unsafe { mcemu_free(self.0.as_ptr()) }
    }
}
