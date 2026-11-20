//! The statechk probe on the reference OS: the screens it draws, and what the OS does to the
//! regions it fills through keys, contrast, ON, auto power down and an ONSCRPT that runs.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/realos.rs"]
mod realos;
#[path = "common/statecheck.rs"]
mod statecheck;

use keytables::{ALPHA, PLAIN, SECOND};
use mcemu::Machine;
use realos::*;
use statecheck::*;

const SCAN_UP: u8 = 0x04;
const SCAN_DOWN: u8 = 0x01;
const SCAN_LAST: u8 = 0x38;
const NO_KEY: [u8; 7] = [5, 6, 7, 8, 0x10, 0x18, 0x29];
const CONTRAST: u16 = 0x8447;

/// Taps every key after `modifier`, the keys that leave excepted, and returns how many `_GetKey`
/// returns that is and the last code.
fn sweep(rig: &mut Rig, modifier: &[u8], table: &[u8; 56]) -> (u16, u8) {
    let (mut returns, mut last) = (0, 0);
    for scan in 1..=SCAN_LAST {
        if NO_KEY.contains(&scan) || [SCAN_2ND, SCAN_ALPHA, KEY_CLEAR].contains(&scan) {
            continue;
        }
        for &m in modifier {
            rig.tap(m);
        }
        rig.tap(scan);
        let code = table[usize::from(scan) - 1];
        returns += u16::from(code != 0);
        last = if code != 0 { code } else { last };
    }
    (returns, last)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_start_screen_shows_the_version_the_flag_byte_the_script_vars_and_a_good_fill() {
    let mut rig = probe();
    rig.assert_screen("start", &start_screen(VER, 0, "none", "none", false));
    rig.assert_patterns("after the fill");
    assert_eq!(rig.keys(), 0);
    rig.assert_confined("the start screen");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn keys_modifiers_contrast_and_on_leave_vars_stack_and_appdata_whole() {
    let mut rig = probe();
    rig.tap(SCAN_5);
    assert_keys(&mut rig, "one key", 1, CODE_5, &Report::ok());
    let mut count = 1;
    for (name, modifier, table) in [
        ("every key plain", vec![], &PLAIN),
        ("every key after 2nd", vec![SCAN_2ND], &SECOND),
        ("every key after ALPHA", vec![SCAN_ALPHA], &ALPHA),
    ] {
        let (returns, last) = sweep(&mut rig, &modifier, table);
        count += returns;
        assert!(returns > 20, "{name}: {returns} keys");
        assert_keys(&mut rig, name, count, last, &Report::ok());
    }
    rig.tap(SCAN_2ND);
    rig.tap(SCAN_ALPHA);
    let (returns, last) = sweep(&mut rig, &[], &ALPHA);
    rig.tap(SCAN_ALPHA);
    count += returns;
    assert_keys(&mut rig, "alpha lock", count, last, &Report::ok());

    let contrast = rig.peek(CONTRAST, 1)[0];
    for _ in 0..4 {
        rig.tap(SCAN_2ND);
        rig.tap(SCAN_UP);
    }
    assert!(
        rig.peek(CONTRAST, 1)[0] > contrast,
        "2nd+up raised the contrast"
    );
    for _ in 0..4 {
        rig.tap(SCAN_2ND);
        rig.tap(SCAN_DOWN);
    }
    assert_eq!(rig.peek(CONTRAST, 1)[0], contrast);
    assert_eq!(rig.keys(), count, "the contrast keys return nothing");
    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(500);
    assert_keys(&mut rig, "after ON", count, last, &Report::ok());
    rig.tap(SCAN_5);
    let f = flags(&mut rig);
    assert_eq!(
        f & ON_SEEN,
        ON_SEEN,
        "a tap of ON leaves bit 3 of iy+33h set"
    );
    assert_keys_with(
        &mut rig,
        "key after contrast and ON",
        count + 1,
        CODE_5,
        f,
        &Report::ok(),
    );
    rig.assert_patterns("after every key");
    rig.assert_confined("after every key, contrast and ON");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn second_quit_is_counted_as_a_key_and_does_not_leave() {
    let mut rig = probe();
    rig.tap(SCAN_2ND);
    rig.tap(0x37);
    assert_keys(&mut rig, "2nd+QUIT", 1, 0x40, &Report::ok());
    rig.tap(SCAN_5);
    assert_keys(&mut rig, "a key after 2nd+QUIT", 2, CODE_5, &Report::ok());
    rig.assert_confined("2nd+QUIT");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_auto_power_down_and_wake_leave_vars_and_stack_ok_and_the_save_screen_tail_changed_once() {
    let mut rig = probe();
    rig.tap(SCAN_5);
    power_cycle(&mut rig);
    assert_keys(&mut rig, "awake, no key yet", 1, CODE_5, &Report::ok());
    rig.tap(SCAN_5);
    let changed_tail = Report {
        ssc: "chg",
        ..Report::ok()
    };
    assert_keys(&mut rig, "first key after waking", 2, CODE_5, &changed_tail);
    rig.tap(SCAN_5);
    assert_keys(
        &mut rig,
        "second key after waking",
        3,
        CODE_5,
        &Report::ok(),
    );
    rig.assert_confined("an auto power down and wake");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_onscrpt_in_ram_with_bit_0_set_damages_appdata_and_nothing_else_the_probe_reports() {
    let mut rig = with_scripts(&["MAKE_ON"]);
    rig.assert_screen(
        "start with ONSCRPT",
        &start_screen(VER, 0, "ram", "none", false),
    );
    set_flags(&mut rig, 1);
    rig.tap(SCAN_5);
    power_cycle(&mut rig);
    assert_eq!(rig.peek(MARK, 1), [0x5A], "the script ran");
    // The OS copies 64 bytes from the size word's high byte: a zero, then the script less its last.
    let mut script = vec![0x3E, 0x5A, 0x32, 0x00, 0xA8, 0xC9];
    script.resize(64, 0xC9);
    let mut want = pattern(256, 0x44);
    want[0] = 0;
    want[1..64].copy_from_slice(&script[..63]);
    let diff: Vec<usize> = (0..256)
        .filter(|&i| want[i] != pattern(256, 0x44)[i])
        .collect();
    assert_eq!(rig.peek(0x8000, 256), want);
    let report = Report {
        appd: changed(diff[0], diff.len()),
        ssc: "chg",
        ..Report::ok()
    };
    rig.tap(SCAN_5);
    assert_keys(&mut rig, "ONSCRPT ran", 2, CODE_5, &report);
    for region in ["SCR_VARS", "SCR_STACK"] {
        let (_, addr, len, seed) = rig.regions().into_iter().find(|r| r.0 == region).unwrap();
        assert_eq!(rig.peek(addr, len), pattern(len, seed), "{region}");
    }
    rig.tap(SCAN_5);
    let still = Report {
        appd: changed(diff[0], diff.len()),
        ..Report::ok()
    };
    assert_keys(&mut rig, "ONSCRPT damage stays", 3, CODE_5, &still);
    rig.assert_confined("an ONSCRPT that ran");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn scripts_that_are_archived_or_have_no_flag_set_leave_appdata_whole_after_a_power_down() {
    for (defines, flag, on, off) in [
        (
            &["MAKE_ON", "MAKE_OFF", "ARCHIVE_SCRIPTS"][..],
            3,
            "arc",
            "arc",
        ),
        (&["MAKE_ON", "MAKE_OFF"][..], 0, "ram", "ram"),
    ] {
        let mut rig = with_scripts(defines);
        rig.assert_screen("start with scripts", &start_screen(VER, 0, on, off, false));
        set_flags(&mut rig, flag);
        rig.tap(SCAN_5);
        power_cycle(&mut rig);
        rig.tap(SCAN_5);
        assert_eq!(rig.peek(MARK, 2), [0, 0], "{defines:?}: no script ran");
        let report = Report {
            ssc: "chg",
            ..Report::ok()
        };
        assert_keys(&mut rig, "scripts that do not run", 2, CODE_5, &report);
        rig.assert_confined("scripts that do not run");
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_changed_byte_in_any_region_is_reported_by_the_next_key_and_the_owner_data_stays_whole() {
    for (region, offset) in [
        ("SCR_VARS", 0),
        ("SCR_VARS", 107),
        ("SCR_STACK", 0),
        ("SCR_STACK", 127),
    ] {
        let mut rig = probe();
        let (_, addr, _, _) = rig.regions().into_iter().find(|r| r.0 == region).unwrap();
        let at = addr + offset as u16;
        let was = rig.peek(at, 1)[0];
        rig.poke(at, &[!was]);
        rig.tap(SCAN_5);
        let hit = changed(offset, 1);
        let mut report = Report::ok();
        if region == "SCR_VARS" {
            report.vars = hit;
        } else {
            report.stack = hit;
        }
        let name = format!("{region} at {offset}");
        assert_keys(&mut rig, &name, 1, CODE_5, &report);
        rig.assert_confined(&name);
    }
    for end in [0, STAT_VARS.1 - 1] {
        let mut rig = probe();
        rig.poke(STAT_VARS.0 + end as u16, &[0xEE]);
        rig.tap(SCAN_5);
        let report = Report {
            stat: "chg",
            ..Report::ok()
        };
        assert_keys(&mut rig, &format!("statVars at {end}"), 1, CODE_5, &report);
        rig.assert_confined(&format!("statVars at {end}"));
    }
    let mut rig = probe();
    let tail = OTHERS[2].1;
    rig.poke(tail, &[0xEE]);
    rig.poke(0x8000 + 200, &[0]);
    rig.tap(SCAN_5);
    assert_ne!(pattern(256, 0x44)[200], 0);
    let report = Report {
        appd: changed(200, 1),
        ssc: "chg",
        ..Report::ok()
    };
    assert_keys(&mut rig, "appData and the tail at 0", 1, CODE_5, &report);
    rig.tap(SCAN_5);
    let report = Report {
        appd: changed(200, 1),
        ..Report::ok()
    };
    assert_keys(
        &mut rig,
        "the tail is not reported twice",
        2,
        CODE_5,
        &report,
    );
    rig.assert_confined("appData and the tail");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_whole_region_changed_reports_offset_0_and_the_full_count() {
    let mut rig = probe();
    let vars = rig.label("SCR_VARS");
    let flipped: Vec<u8> = pattern(108, 0x77).iter().map(|b| !b).collect();
    rig.poke(vars, &flipped);
    let data: Vec<u8> = pattern(256, 0x44).iter().map(|b| !b).collect();
    rig.poke(0x8000, &data);
    rig.tap(SCAN_5);
    let report = Report {
        vars: changed(0, 108),
        appd: changed(0, 256),
        ..Report::ok()
    };
    assert_keys(&mut rig, "whole regions", 1, CODE_5, &report);
    rig.assert_confined("whole regions");
}
