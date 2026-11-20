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
    ("SCR_STACK", 0x20),
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

/// The probe on the stub with `contents` in each of its restored regions before launch, patterns
/// in statVars and prevData, and a guard byte in place beside every region the probe must not
/// write past.
fn stub_with(build: &Build, contents: &[(&str, Vec<u8>)]) -> Rig {
    let (stat, prev) = (pattern(STAT_VARS.1, 0x33), pattern(PREV_DATA.1, 0x66));
    let guards = guards(&build.lab);
    Rig::stub(build, |m| {
        let page = m.ram_page(1);
        let at = |a: u16| usize::from(a) - RAM_BASE;
        for (name, bytes) in contents {
            let addr = match *name {
                "textShadow" => TEXT_SHADOW,
                "appData" => APP_DATA,
                n => realos::label_in(&build.lab, n),
            };
            page[at(addr)..][..bytes.len()].copy_from_slice(bytes);
        }
        page[at(STAT_VARS.0)..][..stat.len()].copy_from_slice(&stat);
        page[at(PREV_DATA.0)..][..prev.len()].copy_from_slice(&prev);
        for g in guards {
            page[at(g)] = 0xEE;
        }
    })
}

fn found() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("textShadow", varied(128, 0x51)),
        ("SCR_STACK", varied(128, 0xC3)),
        ("SCR_VARS", varied(108, 0x3C)),
        ("appData", varied(256, 0xA7)),
    ]
}

/// What the OS holds in each region at an entry from a cleared home screen.
const FRESH: [(&str, u8); 4] = [
    ("SCR_VARS", 0),
    ("SCR_STACK", 0x20),
    ("appData", 0),
    ("textShadow", 0x20),
];

/// The byte before and after each region the probe writes, which it must leave alone: by name
/// and offset from the label, or by address.
fn guards(lab: &std::path::Path) -> Vec<u16> {
    let l = |n: &str| realos::label_in(lab, n);
    vec![
        l("SCR_VARS") - 1,
        l("SCR_VARS") + l("SCR_VARS_SIZE"),
        l("SCR_STACK") - 1,
        l("SCR_STACK") + l("SCR_STACK_SIZE"),
        APP_DATA + 256,
        TEXT_SHADOW - 1,
        TEXT_SHADOW + 128,
        OTHERS[2].1 - 1,
        OTHERS[2].1 + 128,
        l("S_BASE") - 1,
        l("S_BASE") + l("S_SIZE"),
    ]
}

fn assert_guards(rig: &mut Rig, lab: &std::path::Path, when: &str) {
    for g in guards(lab) {
        assert_eq!(rig.peek(g, 1), [0xEE], "the byte at {g:04X} {when}");
    }
}

/// Presses CLEAR and runs until the probe leaves.
fn clear(rig: &mut Rig) {
    rig.os.m.press_key(realos::KEY_CLEAR);
    assert_eq!(rig.os.m.run_app(3_000_000).stop, Stop::Exited);
}

#[test]
fn clear_puts_the_four_regions_back_as_found_and_leaves_the_tail_and_the_owner_data() {
    let build = assemble(PROBE, &[]);
    let contents = found();
    let mut rig = stub_with(&build, &contents);
    assert_guards(&mut rig, &build.lab, "before the first key");
    assert_eq!(rig.byte("S_STALE"), 0);
    for ((name, bytes), (_, save)) in contents.iter().zip(SAVES_BY_FOUND) {
        let (a, s) = (rig.addr(name), rig.label(save));
        assert_eq!(
            rig.peek(s, bytes.len()),
            *bytes,
            "{save} holds what was found"
        );
        assert_ne!(rig.peek(a, bytes.len()), *bytes, "{name} is filled");
    }
    rig.tap(SCAN_5);
    assert_eq!(rig.byte("S_SAVE_NOW"), 0, "the copies are whole");
    assert_guards(&mut rig, &build.lab, "after a key");
    clear(&mut rig);
    assert_eq!(rig.keys(), 1, "CLEAR is not counted");
    for ((name, bytes), (_, save)) in contents.iter().zip(SAVES_BY_FOUND) {
        let (a, s) = (rig.addr(name), rig.label(save));
        assert_eq!(rig.peek(a, bytes.len()), *bytes, "{name} is put back");
        assert_eq!(rig.peek(s, bytes.len()), *bytes, "{save} is as it was");
    }
    assert_guards(&mut rig, &build.lab, "after CLEAR");
    let tail = OTHERS[2];
    assert_eq!(
        rig.peek(tail.1, tail.2),
        pattern(tail.2, tail.3),
        "the tail is left"
    );
    assert_eq!(
        rig.peek(STAT_VARS.0, STAT_VARS.1),
        pattern(STAT_VARS.1, 0x33)
    );
    assert_eq!(
        rig.peek(PREV_DATA.0, PREV_DATA.1),
        pattern(PREV_DATA.1, 0x66)
    );
}

/// `found()` is in the order of these.
const SAVES_BY_FOUND: [(&str, &str); 4] = [
    ("textShadow", "S_SAVE_TEXT"),
    ("SCR_STACK", "S_SAVE_STACK"),
    ("SCR_VARS", "S_SAVE_VARS"),
    ("appData", "S_SAVE_DATA"),
];

#[test]
fn a_saved_copy_damaged_during_the_wait_makes_clear_write_what_a_cleared_home_screen_holds() {
    let build = assemble(PROBE, &[]);
    let contents = found();
    for (_, save) in SAVES_BY_FOUND {
        let len = region_len(SAVES.iter().find(|s| s.1 == save).unwrap().0);
        for (offset, late) in [
            (0, false),
            (len / 2, false),
            (len - 1, false),
            (len - 1, true),
        ] {
            let mut rig = stub_with(&build, &contents);
            let at = rig.label(save) + offset as u16;
            let was = rig.peek(at, 1)[0];
            if late {
                rig.tap(SCAN_5);
            }
            rig.poke(at, &[!was]);
            if !late {
                rig.tap(SCAN_5);
                assert_eq!(rig.byte("S_SAVE_NOW"), 1, "{save}+{offset} is seen");
            }
            clear(&mut rig);
            for (region, fill) in FRESH {
                let len = region_len(region);
                assert_eq!(
                    rig.peek(rig.addr(region), len),
                    vec![fill; len],
                    "{save}+{offset}: {region}"
                );
            }
            assert_guards(
                &mut rig,
                &build.lab,
                &format!("after CLEAR, {save}+{offset}"),
            );
            let tail = OTHERS[2];
            assert_eq!(rig.peek(tail.1, tail.2), pattern(tail.2, tail.3));
            assert_eq!(
                rig.peek(STAT_VARS.0, STAT_VARS.1),
                pattern(STAT_VARS.1, 0x33)
            );
            assert_eq!(
                rig.peek(PREV_DATA.0, PREV_DATA.1),
                pattern(PREV_DATA.1, 0x66)
            );
        }
    }
}

/// The pattern of a region with its first `k` bytes inverted.
fn off_pattern(len: usize, seed: u8, k: usize) -> Vec<u8> {
    let mut bytes = pattern(len, seed);
    bytes[..k].iter_mut().for_each(|b| *b = !*b);
    bytes
}

/// Region, length, seed and, for a run of k bytes off the pattern, whether the region is stale.
type StaleCase = (&'static str, usize, u8, &'static [(usize, bool)]);
const STALE_CASES: [StaleCase; 4] = [
    (
        "SCR_VARS",
        108,
        0x77,
        &[(0, true), (1, true), (53, true), (54, false), (108, false)],
    ),
    (
        "SCR_STACK",
        128,
        0x22,
        &[(0, true), (1, true), (63, true), (64, false), (128, false)],
    ),
    (
        "appData",
        256,
        0x44,
        &[
            (0, true),
            (1, true),
            (64, true),
            (127, true),
            (128, false),
            (256, false),
        ],
    ),
    (
        "textShadow",
        128,
        0x11,
        &[(0, true), (1, true), (63, true), (64, false)],
    ),
];

#[test]
fn a_region_is_stale_when_fewer_than_half_its_bytes_are_off_its_pattern_and_only_it_is_restored_to_a_constant()
 {
    let build = assemble(PROBE, &[]);
    for (region, len, seed, cases) in STALE_CASES {
        for &(k, stale) in cases {
            let mut contents = found();
            let held = off_pattern(len, seed, k);
            contents.iter_mut().find(|c| c.0 == region).unwrap().1 = held.clone();
            let mut rig = stub_with(&build, &contents);
            let name = format!("{region} with {k} bytes off");
            let flags = rig.peek(rig.label("S_STALE_R"), 4);
            let want: Vec<u8> = SAVES
                .iter()
                .map(|s| u8::from(s.0 == region && stale))
                .collect();
            assert_eq!(flags, want, "{name}: stale flags");
            assert_eq!(rig.byte("S_STALE"), u8::from(stale), "{name}");
            rig.tap(SCAN_5);
            clear(&mut rig);
            for (other, bytes) in &contents {
                let (addr, len) = (rig.addr(other), bytes.len());
                let fresh = FRESH.iter().find(|f| f.0 == *other).unwrap().1;
                let want = if *other == region && stale {
                    vec![fresh; len]
                } else {
                    bytes.clone()
                };
                assert_eq!(rig.peek(addr, len), want, "{name}: {other} after CLEAR");
            }
            assert_guards(&mut rig, &build.lab, &format!("after CLEAR, {name}"));
        }
    }
}

#[test]
fn regions_holding_their_whole_patterns_are_all_stale_and_clear_writes_what_a_cleared_home_screen_holds()
 {
    let build = assemble(PROBE, &[]);
    let whole = [
        ("SCR_VARS", pattern(108, 0x77)),
        ("SCR_STACK", pattern(128, 0x22)),
        ("appData", pattern(256, 0x44)),
    ];
    let mut cases: Vec<Vec<(&str, Vec<u8>)>> = whole.iter().map(|w| vec![w.clone()]).collect();
    cases.push(whole.to_vec());
    for patterns in cases {
        let mut contents = found();
        for (name, bytes) in &patterns {
            contents.iter_mut().find(|c| c.0 == *name).unwrap().1 = bytes.clone();
        }
        let names: Vec<_> = patterns.iter().map(|c| c.0).collect();
        let mut rig = stub_with(&build, &contents);
        assert_eq!(rig.byte("S_STALE"), 1, "{names:?}");
        rig.tap(SCAN_5);
        clear(&mut rig);
        for (name, bytes) in &contents {
            let len = bytes.len();
            let fresh = FRESH.iter().find(|f| f.0 == *name).unwrap().1;
            let want = if names.contains(name) {
                vec![fresh; len]
            } else {
                bytes.clone()
            };
            assert_eq!(rig.peek(rig.addr(name), len), want, "{names:?}: {name}");
        }
    }
}

#[test]
fn a_whole_region_changed_reports_offset_0_and_a_count_over_99() {
    let mut rig = probe();
    let vars = rig.label("SCR_VARS");
    let flipped =
        |len: usize, seed: u8| -> Vec<u8> { pattern(len, seed).iter().map(|b| !b).collect() };
    rig.poke(vars, &flipped(108, 0x77));
    rig.poke(0x8000, &flipped(256, 0x44));
    rig.tap(SCAN_5);
    assert_eq!(result(&mut rig, 0), (0, 108));
    assert_eq!(result(&mut rig, 1), (0, 0));
    assert_eq!(result(&mut rig, 2), (0, 256));
}

#[test]
fn statvars_changed_at_either_end_and_the_tail_changed_at_its_first_byte_are_seen() {
    for end in [0, STAT_VARS.1 - 1] {
        let mut rig = probe();
        rig.poke(STAT_VARS.0 + end as u16, &[0xEE]);
        rig.tap(SCAN_5);
        assert_eq!(rig.byte("S_STAT_NOW"), 1, "statVars at {end}");
    }
    let mut rig = probe();
    rig.poke(OTHERS[2].1, &[0xEE]);
    rig.tap(SCAN_5);
    assert_eq!(rig.byte("S_SSC_NOW"), 1, "the tail at offset 0");
}
