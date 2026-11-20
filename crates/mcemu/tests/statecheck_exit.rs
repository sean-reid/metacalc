//! How the statechk probe leaves: CLEAR puts back what it found, 2nd+OFF cannot, and a second run
//! after the OS cut the first short recognises the patterns and writes what the OS expects.

#[path = "common/realos.rs"]
mod realos;
#[path = "common/statecheck.rs"]
mod statecheck;

use mcemu::Machine;
use realos::*;
use statecheck::*;

/// Presses 2nd+OFF with an OFFSCRPT or not, waits for the power down and wakes it with ON.
fn user_power_off(rig: &mut Rig) {
    rig.tap(SCAN_5);
    rig.run_ms(1000);
    rig.tap(SCAN_2ND);
    rig.press(Machine::KEY_ON);
    rig.run_ms(300);
    rig.release(Machine::KEY_ON);
    rig.run_ms(2000);
    assert_ne!(rig.os.m.in_port(3) & POWER_DOWN, 0, "2nd+OFF powered down");
    rig.tap(Machine::KEY_ON);
    rig.run_ms(3000);
}

fn lit(rig: &mut Rig) -> Vec<usize> {
    let lcd = rig.os.screen();
    (0..lcd.len()).filter(|&i| lcd[i]).collect()
}

#[test]
#[ignore = "needs the reference ROM image"]
fn clear_leaves_the_home_screen_and_puts_vars_and_appdata_back_with_the_owner_data_whole() {
    let mut rig = probe();
    assert_eq!(rig.byte("S_STALE"), 0);
    assert_eq!(rig.peek(rig.label("S_SAVE_VARS"), 108), varied(108, 0x3C));
    assert_eq!(rig.peek(rig.label("S_SAVE_DATA"), 256), varied(256, 0xA7));
    rig.tap(SCAN_5);
    assert_clear_restores(&mut rig, "clear");
    assert_eq!(rig.peek(rig.label("SCR_VARS"), 108), varied(108, 0x3C));
    assert_eq!(rig.peek(APP_DATA, 256), varied(256, 0xA7));
    rig.tap(SCAN_5);
    assert_eq!(rig.keys(), 2, "CLEAR is not counted and the probe is gone");
}

fn read(os: &mut RealOs, addr: u16, len: usize) -> Vec<u8> {
    os.m.ram_page(1)[usize::from(addr) - RAM_BASE..][..len].to_vec()
}

/// What the home screen holds in cmdShadow, textShadow, smallEditRAM and appData before the probe
/// is launched: after CLEAR, and after CLEAR then ENTER, which draws Done.
#[test]
#[ignore = "needs the reference ROM image"]
fn a_clean_home_screen_holds_zeros_in_cmdshadow_smalleditram_and_appdata_and_the_screen_text() {
    let build = assemble(PROBE, &[]);
    let mut os = RealOs::boot(&build.app);
    assert_eq!(label_in(&build.lab, "SCR_VARS"), SCR_VARS);
    assert_eq!(label_in(&build.lab, "SCR_STACK"), SCR_STACK);
    os.tap(KEY_CLEAR, 150, 500);
    assert_eq!(read(&mut os, TEXT_SHADOW, 128), [0x20; 128], "after CLEAR");
    os.tap(0x09, 150, 1000);
    let mut shadow = [0x20u8; 128];
    shadow[12..16].copy_from_slice(b"Done");
    let text = read(&mut os, TEXT_SHADOW, 128);
    assert_eq!(text[..16], shadow[..16], "the first row after ENTER");
    assert!([0x20, 0xE0].contains(&text[16]), "the cursor cell");
    assert_eq!(text[17..], shadow[17..], "the rest of the screen");
    assert_eq!(read(&mut os, SCR_STACK, 128), [0; 128], "cmdShadow");
    assert_eq!(read(&mut os, SCR_VARS, 108), [0; 108], "smallEditRAM");
    assert_eq!(read(&mut os, APP_DATA, 256), [0; 256], "appData");
}

/// The constants CLEAR writes after a wrong exit are what the OS holds when the probe is entered
/// from a cleared home screen, which is not what the home screen holds at rest: cmdShadow is zeros
/// there, and writing zeros back leaves the home screen drawing 128 dots.
#[test]
#[ignore = "needs the reference ROM image"]
fn the_constants_are_what_the_os_holds_at_an_entry_from_a_cleared_home_screen() {
    let build = assemble(PROBE, &[]);
    let mut os = RealOs::boot(&build.app);
    os.tap(KEY_CLEAR, 150, 500);
    assert_eq!(read(&mut os, SCR_STACK, 128), [0; 128], "cmdShadow at rest");
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(1500);
    let lab = &build.lab;
    for (save, len, want) in [
        ("S_SAVE_STACK", 128, 0x20),
        ("S_SAVE_TEXT", 128, 0x20),
        ("S_SAVE_VARS", 108, 0),
        ("S_SAVE_DATA", 256, 0),
    ] {
        let found = read(&mut os, label_in(lab, save), len);
        assert_eq!(found, vec![want; len], "{save}");
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_plain_2nd_off_leaves_the_probe_and_the_home_screen_is_back_after_on() {
    let mut rig = probe();
    user_power_off(&mut rig);
    assert_left_the_probe(&mut rig);
}

/// After waking the OS runs the home screen: a key echoes there and does not reach the probe.
fn assert_left_the_probe(rig: &mut Rig) {
    let keys = rig.keys();
    let before = rig.os.screen();
    rig.tap(SCAN_5);
    assert_eq!(rig.keys(), keys, "no key reaches the probe");
    assert_ne!(rig.os.screen(), before, "the home screen echoes the key");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_offscrpt_runs_at_2nd_off_and_the_os_leaves_the_probe_so_it_cannot_report() {
    let mut rig = with_scripts(&["MAKE_OFF"]);
    set_flags(&mut rig, 2);
    user_power_off(&mut rig);
    assert_eq!(rig.peek(MARK + 1, 1), [0xA5], "the script ran");
    assert_left_the_probe(&mut rig);
    assert_saves_intact(&mut rig, "after OFFSCRPT");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn after_2nd_off_the_home_screen_shows_the_patterns_and_clear_then_enter_repairs_it() {
    let mut rig = probe();
    user_power_off(&mut rig);
    for (name, addr, len, seed) in rig.regions() {
        if name != "textShadow" {
            assert_eq!(
                rig.peek(addr, len),
                pattern(len, seed),
                "{name} keeps its pattern"
            );
        }
    }
    let garbage = lit(&mut rig).len();
    assert!(
        garbage > 1000,
        "the home screen draws the pattern: {garbage} lit pixels"
    );
    rig.tap(KEY_CLEAR);
    rig.run_ms(500);
    assert_blank_home(&mut rig, "after-off-clear");
    rig.tap(0x09);
    rig.run_ms(500);
    let lcd = rig.os.screen();
    rig.dump("after-off-enter", &lcd);
    let done = lit(&mut rig);
    assert!(
        done.iter()
            .all(|&i| i / 96 < 16 && (i % 96 < 6 || i % 96 >= 72)),
        "CLEAR then ENTER leaves Done and the cursor"
    );
    assert!(!done.is_empty());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_second_run_after_a_wrong_exit_says_stale_and_clear_writes_what_an_entry_from_a_cleared_home_screen_holds()
 {
    let mut rig = probe();
    user_power_off(&mut rig);
    rig.tap(KEY_APPS);
    rig.run_ms(600);
    rig.tap(KEY_2);
    rig.run_ms(1500);
    assert_eq!(rig.byte("S_STALE"), 1);
    rig.assert_screen("stale", &start_screen(VER, 0, "none", "none", true));
    rig.tap(SCAN_5);
    rig.tap(KEY_CLEAR);
    rig.run_ms(1000);
    assert_blank_home(&mut rig, "stale-clear");
    assert_eq!(
        rig.peek(rig.label("SCR_STACK"), 128),
        [0x20; 128],
        "cmdShadow as an entry from a cleared home screen has it"
    );
    assert_eq!(
        rig.peek(rig.label("SCR_VARS"), 108),
        [0; 108],
        "smallEditRAM"
    );
    assert_eq!(rig.peek(0x8000, 256), [0; 256], "appData");
}
