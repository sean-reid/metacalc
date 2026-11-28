//! What the reference OS's interrupt handler writes below the stack pointer it is taken at, measured over
//! the Dist enter: its keypad scan, at pc 047D, writes 10 bytes counting the interrupt's own return address,
//! and its two short paths write 4. `dist_depth::handler` states the 10.

use crate::checkrig::{Run, WAIT};
use crate::dist_depth::handler;
use crate::dist_lib::{self, SEVEN};
use crate::dist_rig::*;
use crate::list_rig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::theorem_rig::*;
use std::collections::BTreeMap;

/// The keypad scan's routine, where the handler's stack is lowest on its long path.
const KEYPAD_SCAN: u16 = 0x047D;

/// One interrupt taken inside the call: the bytes it wrote below the stack pointer it was taken at, and the
/// pc of the instruction before the stack was lowest.
struct Taken {
    bytes: u16,
    lowest_at: u16,
}

/// The interrupts taken inside `label` with interrupts on, from the call's entry to its return.
fn taken_in(run: &mut Run, label: &str, speed: u8) -> Vec<Taken> {
    let at = run.shell.app.label(label);
    run.run_to_addr(label, at, WAIT);
    run.m().out_port(0x20, speed);
    let sp = run.m().regs().sp;
    let ret = {
        let b = run.m().read_ram(sp, 2);
        u16::from_le_bytes([b[0], b[1]])
    };
    let start = run.m().clock();
    let mut found = Vec::new();
    loop {
        match run.m().run_to(BASE, ret, WAIT) {
            mcemu::Reached::Target => break,
            mcemu::Reached::Vector38 => {}
            mcemu::Reached::Limit => panic!("{label}: the return was not reached"),
        }
        assert!(
            run.m().clock().wrapping_sub(start) < WAIT,
            "{label}: past the limit"
        );
        let entered = run.m().regs().sp;
        let (mut low, mut lowest_at) = (entered, run.m().regs().pc);
        for _ in 0..100_000 {
            let before = run.m().regs().pc;
            run.m().run_to(BASE, 0x4000, 1);
            let now = run.m().regs().sp;
            if now < low {
                (low, lowest_at) = (now, before);
            }
            if now >= entered + 2 {
                break;
            }
        }
        assert_eq!(run.m().regs().sp, entered + 2, "the handler returned");
        found.push(Taken {
            bytes: entered + 2 - low,
            lowest_at,
        });
    }
    run.m().out_port(0x20, 0);
    found
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_handler_writes_10_bytes_on_its_keypad_scan_and_4_on_its_short_paths() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let mut seen: BTreeMap<(u8, u16, u16), u32> = BTreeMap::new();
    for speed in [0u8, 1] {
        for phase in 0..24u32 {
            let mut list = Lister::on_rom_of(built, 0);
            list.open_on_rom();
            let mut walker = Walker::new(list, 0, built);
            walker.gap = 600_000;
            walker.mirrored = false;
            walker.goto(index_of(&lib, SEVEN));
            walker.hold_preview();
            walker.open();
            walker.list.run.run(phase * 41_113);
            walker.list.run.press(F2);
            walker.trace("theorem_key", true, speed);
            for t in taken_in(&mut walker.list.run, "dist_enter", speed) {
                *seen.entry((speed, t.bytes, t.lowest_at)).or_default() += 1;
            }
            walker.list.run.release(F2);
            walker.list.run.run(3_000_000);
        }
    }
    println!("(speed, bytes, pc where lowest): count: {seen:?}");
    let most = seen.keys().map(|&(_, bytes, _)| bytes).max().unwrap();
    assert_eq!(most, handler(true), "the most any interrupt wrote");
    let floor_scan = [220, 65];
    for speed in [0u8, 1] {
        let count = |bytes: u16| -> u32 {
            seen.iter()
                .filter(|(k, _)| k.0 == speed && k.1 == bytes)
                .map(|(_, n)| n)
                .sum()
        };
        let scans: u32 = seen
            .iter()
            .filter(|(k, _)| k.0 == speed && k.1 == 10 && k.2 == KEYPAD_SCAN)
            .map(|(_, n)| n)
            .sum();
        assert_eq!(
            scans,
            count(10),
            "speed {speed}: every 10 is the keypad scan"
        );
        assert!(
            count(10) >= floor_scan[usize::from(speed)],
            "speed {speed}: keypad scans taken"
        );
        assert!(count(4) >= 20, "speed {speed}: short paths taken");
    }
}
