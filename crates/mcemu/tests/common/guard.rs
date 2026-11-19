use crate::support::*;
use std::ops::Range;

const STACK_BYTES: usize = 64;

pub fn pattern(page: u8, at: usize) -> u8 {
    let page = page.min(2);
    ((at.wrapping_mul(31) ^ (at >> 8).wrapping_mul(7)) as u8) ^ page.wrapping_mul(0x55) ^ 0xA7
}

/// Where a run may write: the reader's state block and its bounce buffer and spare page, as the
/// image under test places them, and the caller's own variables and buffers in bank B.
pub struct Allowed {
    pub state: u16,
    pub bounce: u16,
    pub spare: u8,
    /// More places a run may write, as (page, address, length): bank B addresses are in page
    /// 1 and bank C addresses in the spare page.
    pub extra: Vec<(u8, u16, usize)>,
}

impl Allowed {
    pub fn of(image: Image) -> Allowed {
        Allowed {
            state: label_in(image, "LIB_STATE"),
            bounce: label_in(image, "LIB_BOUNCE"),
            spare: 3,
            extra: Vec::new(),
        }
    }
}

pub fn prime(rig: &mut Rig) {
    for page in 0..8u8 {
        for (at, byte) in rig.m.ram_page(page).iter_mut().enumerate() {
            *byte = pattern(page, at);
        }
    }
}

pub fn offset(addr: u16, base: u16) -> usize {
    usize::from(addr - base)
}

/// Every byte of RAM the calls changed that they had no right to, as (page, address).
pub fn stray(rig: &mut Rig, allowed: &Allowed) -> Vec<(u8, u16)> {
    let size = usize::from(label_in(DEFAULT, "LIB_STATE_SIZE"));
    let bank_b: Vec<Range<usize>> = [
        (allowed.state, size),
        (label_in(DEFAULT, "TEST_TARGET"), 0x20),
        (BUF, 0x200),
        (SCRATCH, 0x100),
    ]
    .iter()
    .map(|&(start, len)| offset(start, 0x8000)..offset(start, 0x8000) + len)
    .collect();
    let bounce = offset(allowed.bounce, 0xC000)..offset(allowed.bounce, 0xC000) + 256;
    let stack = 0x4000 - STACK_BYTES..0x4000;
    let mut found = Vec::new();
    for page in [OS_PAGE, APP_PAGE, allowed.spare] {
        let bytes = rig.m.ram_page(page).to_vec();
        for (at, &byte) in bytes.iter().enumerate() {
            if byte == pattern(page, at) {
                continue;
            }
            let fine = match page {
                OS_PAGE => stack.contains(&at),
                APP_PAGE => bank_b.iter().any(|r| r.contains(&at)),
                p if p == allowed.spare => bounce.contains(&at),
                _ => false,
            } || allowed.extra.iter().any(|&(p, start, len)| {
                let base = if p == APP_PAGE { 0x8000 } else { 0xC000 };
                p == page && (offset(start, base)..offset(start, base) + len).contains(&at)
            });
            if !fine {
                let base = if page == APP_PAGE { 0x8000 } else { 0xC000 };
                found.push((page, base + at as u16));
            }
        }
    }
    found
}
