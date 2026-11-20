// Included by statecheck.rs: what the reference OS tests share.

pub const SCAN_2ND: u8 = 0x36;
pub const SCAN_ALPHA: u8 = 0x30;
pub const SCAN_5: u8 = 0x1B;
pub const POWER_DOWN: u8 = 0x10;
pub const VER: &str = "02 2B";
/// The `_GetKey` code of 5.
pub const CODE_5: u8 = 0x93;
/// Bit 3 of iy+33h, which a tap of ON leaves set.
pub const ON_SEEN: u8 = 0x08;

/// What the owner's calculator holds before the probe starts, which it must give back: varied
/// bytes in SCR_VARS and appData, which it restores, and patterns in statVars and prevData, which
/// it must not touch.
pub fn owner_data() -> Vec<(u16, Vec<u8>)> {
    vec![
        (SCR_VARS, varied(108, 0x3C)),
        (APP_DATA, varied(256, 0xA7)),
        (STAT_VARS.0, pattern(STAT_VARS.1, 0x33)),
        (PREV_DATA.0, pattern(PREV_DATA.1, 0x66)),
    ]
}

pub fn probe() -> Rig {
    Rig::real_with(&assemble(PROBE, &[]), &owner_data(), &[])
}

pub fn with_scripts(defines: &[&str]) -> Rig {
    Rig::real_with(&assemble(WITH_SCRIPTS, defines), &owner_data(), defines)
}

pub fn flags(rig: &mut Rig) -> u8 {
    rig.peek(SCRIPT_FLAGS as u16, 1)[0]
}

pub fn set_flags(rig: &mut Rig, bits: u8) {
    let now = flags(rig);
    rig.poke(SCRIPT_FLAGS as u16, &[now | bits]);
}

/// The key screen after `count` keys, the last with `code`, and `report`.
pub fn assert_keys(rig: &mut Rig, name: &str, count: u16, code: u8, report: &Report) {
    // Bit 3 of iy+33h is set while the OS waits in _GetKey, and clear when the probe reads it.
    let f = flags(rig) & !ON_SEEN;
    assert_keys_with(rig, name, count, code, f, report);
}

/// The same with the flag byte the screen shows given.
pub fn assert_keys_with(rig: &mut Rig, name: &str, count: u16, code: u8, f: u8, report: &Report) {
    rig.assert_screen(name, &key_screen(count, code, f, report));
    assert_eq!(rig.keys(), count, "{name}: the probe's count");
}

/// Runs the OS until auto power down fires, wakes it with ON and lets it settle. The OS must
/// have put the whole screen in saveSScreen.
pub fn power_cycle(rig: &mut Rig) {
    let lcd = rig.os.packed_screen();
    let mut seconds = 0;
    while rig.os.m.in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        rig.run_ms(5_000);
        seconds += 5;
    }
    assert_eq!(
        rig.peek(SAVE_SCREEN, 768),
        lcd,
        "the power down left the screen's pixels in saveSScreen"
    );
    rig.apd_pixels = Some(lcd);
    rig.tap(mcemu::Machine::KEY_ON);
    rig.run_ms(3000);
    assert_eq!(
        rig.os.m.in_port(3) & POWER_DOWN,
        0,
        "ON woke the calculator"
    );
}

/// The four saved copies hold what they held when the probe started, and those of SCR_VARS
/// and appData are what the owner's calculator held before the app was launched. The OS
/// changes cmdShadow and textShadow when it launches an app, so only the first two can be
/// compared with that.
pub fn assert_saves_intact(rig: &mut Rig, name: &str) {
    for (i, (region, save)) in SAVES.iter().enumerate() {
        let len = region_len(region);
        assert_eq!(
            rig.peek(rig.label(save), len),
            rig.saves0[i],
            "{name}: {save} changed since the start"
        );
        if i < 3 && *region != "SCR_STACK" {
            let before_launch = rig.home_at(rig.addr(region), len);
            assert_eq!(rig.saves0[i], before_launch, "{name}: {save} at the start");
        }
    }
}

/// Ends a scenario with CLEAR: a key first, so the screen is current, which shows the copies
/// intact and leaves each saved region as it was before the app started and the owner's data whole.
pub fn assert_clear_restores(rig: &mut Rig, name: &str) {
    rig.tap(SCAN_5);
    let row = rig.text()[6].trim_end().to_string();
    assert!(row.ends_with("save ok"), "{name}: row 6 is {row:?}");
    assert_saves_intact(rig, name);
    rig.tap(realos::KEY_CLEAR);
    rig.run_ms(1000);
    assert_blank_home(rig, &format!("{name}-clear"));
    for (region, _) in SAVES {
        let (addr, len) = (rig.addr(region), region_len(region));
        if region == "textShadow" || region == "SCR_STACK" {
            continue;
        }
        let want = rig.home_at(addr, len);
        assert_eq!(rig.peek(addr, len), want, "{name}: {region} after CLEAR");
    }
    rig.assert_confined(&format!("{name} after CLEAR"));
}
