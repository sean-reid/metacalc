//! The statechk probe on the stub OS: the state it keeps and the results it computes. The stub
//! draws no text and cannot archive a variable, so the screens and the archived case are checked on
//! the reference OS in statecheck_os.rs.

#[path = "common/keytables.rs"]
#[allow(dead_code)]
mod keytables;
#[path = "common/realos.rs"]
mod realos;
#[path = "common/statecheck.rs"]
mod statecheck;

use mcemu::{Machine, Stop};
use statecheck::*;

const SCAN_5: u8 = 0x1B;
const KEY_5: u8 = keytables::PLAIN[SCAN_5 as usize - 1];
const POKED: [(&str, usize); 8] = [
    ("SCR_VARS", 0),
    ("SCR_VARS", 53),
    ("SCR_VARS", 107),
    ("SCR_STACK", 0),
    ("SCR_STACK", 64),
    ("SCR_STACK", 127),
    ("appData", 0),
    ("appData", 255),
];

fn probe() -> Rig {
    Rig::stub(&assemble(PROBE, &[]), |_| {})
}

/// The probe's result for compared region `n`: the first offset and the count.
fn result(rig: &mut Rig, n: u16) -> (u8, u16) {
    let r = rig.peek(rig.label("S_RES") + 3 * n, 3);
    (r[0], u16::from_le_bytes([r[1], r[2]]))
}

#[test]
fn the_probe_fills_every_region_with_its_pattern_and_finds_them_whole() {
    let mut rig = probe();
    rig.assert_patterns("after the fill");
    assert_eq!(rig.byte("S_FILL"), 0);
    assert_eq!((rig.byte("S_VER_A"), rig.byte("S_VER_B")), (2, 0x2B));
    assert_eq!((rig.byte("S_ON"), rig.byte("S_OFF")), (0, 0));
    let flags = rig.peek(SCRIPT_FLAGS as u16, 1)[0];
    assert_eq!(rig.byte("S_FLAG"), flags);
    assert_eq!(rig.keys(), 0);
}

#[test]
fn a_key_counts_once_and_leaves_every_region_whole_and_on_is_not_a_key() {
    let mut rig = probe();
    rig.tap(SCAN_5);
    assert_eq!((rig.keys(), rig.byte("S_KEY")), (1, KEY_5));
    for n in 0..3 {
        assert_eq!(result(&mut rig, n), (0, 0), "region {n}");
    }
    assert_eq!((rig.byte("S_STAT_NOW"), rig.byte("S_SSC_NOW")), (0, 0));
    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(300);
    assert_eq!(rig.keys(), 1, "ON is not a key");
    rig.tap(SCAN_5);
    assert_eq!(rig.keys(), 2);
    rig.assert_patterns("after keys");
}

#[test]
fn a_changed_byte_is_reported_at_its_offset_by_the_next_key_and_by_nothing_before() {
    for (region, offset) in POKED {
        let mut rig = probe();
        let (addr, len, seed) = rig
            .regions()
            .into_iter()
            .find(|r| r.0 == region)
            .map(|r| (r.1, r.2, r.3))
            .unwrap();
        let at = addr + offset as u16;
        let was = rig.peek(at, 1)[0];
        rig.poke(at, &[!was]);
        assert_eq!(
            result(&mut rig, 0),
            (0, 0),
            "nothing is reported before a key"
        );
        rig.tap(SCAN_5);
        let n = ["SCR_VARS", "SCR_STACK", "appData"]
            .iter()
            .position(|&r| r == region)
            .unwrap();
        for m in 0..3u16 {
            let want = if usize::from(m) == n {
                (offset as u8, 1)
            } else {
                (0, 0)
            };
            assert_eq!(result(&mut rig, m), want, "{region}+{offset}: region {m}");
        }
        assert_eq!(rig.peek(addr, len)[offset], !was);
        assert_eq!(rig.peek(addr, len)[..offset], pattern(len, seed)[..offset]);
    }
}

#[test]
fn a_run_of_changed_bytes_counts_them_and_starts_at_the_first() {
    let mut rig = probe();
    let vars = rig.label("SCR_VARS");
    rig.poke(vars + 40, &[0; 10]);
    rig.tap(SCAN_5);
    let first = pattern(108, 0x77)[40..50]
        .iter()
        .position(|&b| b != 0)
        .unwrap();
    let count = pattern(108, 0x77)[40..50]
        .iter()
        .filter(|&&b| b != 0)
        .count();
    assert_eq!(result(&mut rig, 0), (40 + first as u8, count as u16));
}

#[test]
fn a_changed_statvars_byte_and_a_changed_save_screen_tail_are_reported_once_each() {
    let mut rig = probe();
    rig.poke(STAT_VARS.0 + 7, &[0xEE]);
    rig.tap(SCAN_5);
    assert_eq!(rig.byte("S_STAT_NOW"), 1, "statVars differs from the start");
    assert_eq!(rig.byte("S_SSC_NOW"), 0);
    rig.poke(OTHERS[2].1 + 100, &[0xEE]);
    rig.tap(SCAN_5);
    assert_eq!(
        rig.byte("S_SSC_NOW"),
        1,
        "the tail differs from the last key"
    );
    rig.tap(SCAN_5);
    assert_eq!(rig.byte("S_SSC_NOW"), 0, "and not from the key after");
    assert_eq!(
        rig.byte("S_STAT_NOW"),
        1,
        "statVars is compared with the start"
    );
}

#[test]
fn the_probe_reads_statvars_and_prev_data_and_never_writes_them() {
    let (stat, prev) = (pattern(STAT_VARS.1, 0x33), pattern(PREV_DATA.1, 0x66));
    let mut rig = Rig::stub(&assemble(PROBE, &[]), |m| {
        let page = m.ram_page(1);
        page[usize::from(STAT_VARS.0) - RAM_BASE..][..stat.len()].copy_from_slice(&stat);
        page[usize::from(PREV_DATA.0) - RAM_BASE..][..prev.len()].copy_from_slice(&prev);
    });
    for _ in 0..3 {
        rig.tap(SCAN_5);
    }
    assert_eq!(rig.peek(STAT_VARS.0, stat.len()), stat);
    assert_eq!(rig.peek(PREV_DATA.0, prev.len()), prev);
    assert_eq!(rig.byte("S_STAT_NOW"), 0, "the checksum matches the start");
}

#[test]
fn script_appvars_are_found_in_ram_and_a_missing_one_is_none() {
    for (defines, want) in [
        (&["MAKE_ON"][..], (1, 0)),
        (&["MAKE_OFF"][..], (0, 1)),
        (&["MAKE_ON", "MAKE_OFF"][..], (1, 1)),
    ] {
        let mut rig = Rig::stub(&assemble(WITH_SCRIPTS, defines), |_| {});
        assert_eq!((rig.byte("S_ON"), rig.byte("S_OFF")), want, "{defines:?}");
        rig.assert_patterns(&format!("{defines:?}"));
    }
}

#[test]
fn clear_leaves_the_app_with_text_shadow_and_the_stack_region_as_the_probe_found_them() {
    let build = assemble(PROBE, &[]);
    let stack = usize::from(realos::label_in(&build.lab, "SCR_STACK"));
    let size = usize::from(realos::label_in(&build.lab, "SCR_STACK_SIZE"));
    let mut rig = Rig::stub(&build, |m| {
        m.ram_page(1)[usize::from(TEXT_SHADOW) - RAM_BASE..][..128].copy_from_slice(&[0x20; 128]);
        m.ram_page(1)[stack - RAM_BASE..][..size].fill(0x5A);
    });
    assert_eq!(
        rig.peek(TEXT_SHADOW, 128),
        pattern(128, 0x11),
        "the probe filled it"
    );
    rig.tap(SCAN_5);
    assert_eq!(
        rig.peek(stack as u16, size),
        pattern(size, 0x22),
        "keys leave it filled"
    );
    rig.os.m.press_key(realos::KEY_CLEAR);
    assert_eq!(rig.os.m.run_app(3_000_000).stop, Stop::Exited);
    assert_eq!(rig.keys(), 1, "CLEAR is not counted");
    assert_eq!(
        rig.peek(TEXT_SHADOW, 128),
        [0x20; 128],
        "textShadow is as it was found"
    );
    assert_eq!(
        rig.peek(stack as u16, size),
        vec![0x5A; size],
        "so is SCR_STACK"
    );
}
