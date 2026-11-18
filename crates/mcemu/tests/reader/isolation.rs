use crate::support::*;
use std::ops::Range;

const STACK_BYTES: usize = 64;

fn pattern(page: u8, at: usize) -> u8 {
    ((at.wrapping_mul(31) ^ (at >> 8).wrapping_mul(7)) as u8) ^ page.wrapping_mul(0x55) ^ 0xA7
}

/// Where a run may write: the reader's state block and its bounce buffer and spare page, as the
/// image under test places them, and the caller's own variables and buffers in bank B.
pub struct Allowed {
    pub state: u16,
    pub bounce: u16,
    pub spare: u8,
}

impl Allowed {
    pub fn of(image: Image) -> Allowed {
        Allowed {
            state: label_in(image, "LIB_STATE"),
            bounce: label_in(image, "LIB_BOUNCE"),
            spare: 3,
        }
    }
}

fn prime(rig: &mut Rig) {
    for page in 0..8u8 {
        for (at, byte) in rig.m.ram_page(page).iter_mut().enumerate() {
            *byte = pattern(page, at);
        }
    }
}

fn offset(addr: u16, base: u16) -> usize {
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
    for page in 0..8u8 {
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
            };
            if !fine {
                let base = if page == APP_PAGE { 0x8000 } else { 0xC000 };
                found.push((page, base + at as u16));
            }
        }
    }
    found
}

/// A long run of every kind of call, including refused ones and reads across chunks.
fn workout(rig: &mut Rig, bytes: &[u8]) {
    rig.open_ok();
    let count = u16::from_le_bytes([bytes[14], bytes[15]]);
    for split in [0usize, 1, 100, 255, 256] {
        let offset = CHUNK - split;
        assert_eq!(rig.read(offset as u32, 256).0, ST_OK);
    }
    for len in [1u16, 17, 256] {
        assert_eq!(rig.read(3, len).0, ST_OK);
    }
    assert_eq!(rig.read(bytes.len() as u32, 1).0, ST_RANGE);
    assert_eq!(rig.read_call(0, 0, BUF).status(), ST_ARG);
    for id in 0..(bytes[12] + bytes[13]).min(12) {
        assert_eq!(rig.symbol_name(id).0, ST_OK);
    }
    for n in (0..count).step_by(97).chain([count - 1]) {
        assert_eq!(rig.label_of(n).0, ST_OK);
        assert_eq!(rig.record(n), ST_OK);
        assert_eq!(rig.conclusion(n).0, ST_OK);
        rig.hypothesis(n, 0);
    }
    assert_eq!(rig.label_of(count).0, ST_INDEX);
    assert_eq!(rig.counts().0, ST_OK);
}

#[test]
fn a_long_run_of_reader_calls_writes_only_the_bounce_buffer_the_state_and_the_callers_own_bytes() {
    let bytes = big_library();
    for (image, name) in [(DEFAULT, "default"), (MOVED, "moved")] {
        let mut rig = Rig::with_image(image);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.load(&bytes);
        workout(&mut rig, &bytes);
        let mut allowed = Allowed::of(image);
        if image.code == MOVED.code {
            allowed.spare = 5;
        }
        let found = stray(&mut rig, &allowed);
        assert!(found.is_empty(), "{name}: {found:x?}");
        let bounce = offset(allowed.bounce, 0xC000);
        let written = rig.m.ram_page(allowed.spare)[bounce..bounce + 256].to_vec();
        let untouched: Vec<u8> = (0..256)
            .map(|i| pattern(allowed.spare, bounce + i))
            .collect();
        assert_ne!(
            written, untouched,
            "{name}: the reader used its bounce buffer"
        );
    }
}

#[test]
fn the_reader_uses_at_most_16_bytes_of_the_callers_stack() {
    let bytes = big_library();
    let mut rig = Rig::with_library(&bytes);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    let page = rig.m.ram_page(OS_PAGE).to_vec();
    let lowest = (0..page.len())
        .find(|&at| page[at] != pattern(OS_PAGE, at))
        .unwrap();
    let used = page.len() - lowest;
    println!("stack used, return address of the driver included: {used} bytes");
    assert!(used > 2 && used <= 2 + 16, "{used} bytes");
}

#[test]
fn the_isolation_check_fails_for_a_bounce_buffer_that_runs_past_the_end_of_the_region() {
    let bytes = big_library();
    let mut rig = Rig::with_image(STRADDLE);
    prime(&mut rig);
    rig.load(&bytes);
    workout(&mut rig, &bytes);
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert!(
        found
            .iter()
            .any(|&(page, addr)| page == SPARE_PAGE && (0xFF00..0xFF80).contains(&addr)),
        "{found:x?}"
    );
    assert!(
        found.iter().all(|&(page, _)| page == SPARE_PAGE),
        "{found:x?}"
    );
}

#[test]
fn a_stray_write_to_the_caller_stack_page_is_caught() {
    let bytes = big_library();
    let mut rig = Rig::with_library(&bytes);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    rig.m.ram_page(OS_PAGE)[0x1234] ^= 1;
    rig.m.ram_page(APP_PAGE)[0x0100] ^= 1;
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert_eq!(found, [(OS_PAGE, 0xD234), (APP_PAGE, 0x8100)]);
}
