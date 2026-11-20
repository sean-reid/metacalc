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
    rig.os.run_ms(1000);
    rig.tap(SCAN_2ND);
    rig.os.m.press_key(Machine::KEY_ON);
    rig.os.run_ms(300);
    rig.os.m.release_key(Machine::KEY_ON);
    rig.os.run_ms(2000);
    assert_ne!(rig.os.m.in_port(3) & POWER_DOWN, 0, "2nd+OFF powered down");
    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(3000);
}

fn lit(rig: &mut Rig) -> Vec<usize> {
    let lcd = rig.os.screen();
    (0..lcd.len()).filter(|&i| lcd[i]).collect()
}

/// The home screen is blank but for its cursor in the first cell.
fn assert_blank_home(rig: &mut Rig, name: &str) {
    let lcd = rig.os.screen();
    rig.dump(name, &lcd);
    let pixels = lit(rig);
    assert!(
        pixels.iter().all(|&i| i / 96 < 8 && i % 96 < 6),
        "{name}: blank but for the cursor"
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn clear_leaves_the_home_screen_and_puts_vars_and_appdata_back_with_the_owner_data_whole() {
    let vars = varied(108, 0x3C);
    let data = varied(256, 0xA7);
    let build = assemble(PROBE, &[]);
    let (stat, prev) = (pattern(STAT_VARS.1, 0x33), pattern(PREV_DATA.1, 0x66));
    let mut rig = Rig::real_with(&build, |m| {
        let lab = &build.lab;
        let page = m.ram_page(1);
        let at = |a: u16| usize::from(a) - RAM_BASE;
        page[at(label_in(lab, "SCR_VARS"))..][..108].copy_from_slice(&vars);
        page[at(0x8000)..][..256].copy_from_slice(&data);
        page[at(STAT_VARS.0)..][..stat.len()].copy_from_slice(&stat);
        page[at(PREV_DATA.0)..][..prev.len()].copy_from_slice(&prev);
    });
    let (v, d) = (rig.label("S_SAVE_VARS"), rig.label("S_SAVE_DATA"));
    assert_eq!(rig.peek(v, 108), vars, "the saved SCR_VARS");
    assert_eq!(rig.peek(d, 256), data, "the saved appData");
    assert_eq!(rig.byte("S_STALE"), 0);
    rig.tap(SCAN_5);
    rig.tap(KEY_CLEAR);
    rig.os.run_ms(1000);
    assert_blank_home(&mut rig, "clear");
    assert_eq!(
        rig.peek(rig.label("SCR_VARS"), 108),
        vars,
        "SCR_VARS is put back"
    );
    assert_eq!(rig.peek(0x8000, 256), data, "appData is put back");
    assert_eq!(
        rig.peek(v, 108),
        vars,
        "and the saved copies are as they were"
    );
    assert_eq!(rig.peek(d, 256), data);
    rig.assert_confined("CLEAR");
    rig.tap(SCAN_5);
    assert_eq!(rig.keys(), 1, "CLEAR is not counted and the probe is gone");
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
    rig.os.run_ms(500);
    assert_blank_home(&mut rig, "after-off-clear");
    rig.tap(0x09);
    rig.os.run_ms(500);
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
fn a_second_run_after_a_wrong_exit_says_stale_and_clear_writes_what_a_fresh_entry_holds() {
    let mut rig = probe();
    user_power_off(&mut rig);
    rig.tap(KEY_APPS);
    rig.os.run_ms(600);
    rig.tap(KEY_2);
    rig.os.run_ms(1500);
    assert_eq!(rig.byte("S_STALE"), 1);
    rig.assert_screen("stale", &start_screen(VER, 0, "none", "none", true));
    rig.tap(SCAN_5);
    rig.tap(KEY_CLEAR);
    rig.os.run_ms(1000);
    assert_blank_home(&mut rig, "stale-clear");
    assert_eq!(
        rig.peek(rig.label("SCR_STACK"), 128),
        [0x20; 128],
        "cmdShadow as a fresh entry has it"
    );
    assert_eq!(
        rig.peek(rig.label("SCR_VARS"), 108),
        [0; 108],
        "smallEditRAM"
    );
    assert_eq!(rig.peek(0x8000, 256), [0; 256], "appData");
}
