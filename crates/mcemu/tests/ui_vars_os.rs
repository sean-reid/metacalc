//! What the reference OS does to saveSScreen, where the UI keeps its variables, while an app runs.

#[path = "common/realos.rs"]
mod realos;

use realos::*;

const RAM_BASE: usize = 0x8000;
const VARS: usize = 0x86EC - RAM_BASE;
const VARS_SIZE: usize = 0x1A;
const SAVE_SCREEN_SIZE: usize = 768;
const RESULT: usize = 0xA800 - RAM_BASE;
/// Bit 4 of port 3, the link interrupt mask the OS writes; it reads set at the step where the OS
/// has run its auto power down, and is clear before.
const POWER_DOWN: u8 = 0x10;
const STEP_MS: u32 = 30_000;
/// The _GetKey test requires auto power down by `BEFORE_APD_MS + HUNT_MS`.
const BEFORE_APD_MS: u32 = 240_000;
const HUNT_MS: u32 = 150_000;
const EXITED: u8 = 0xEE;
const SEED: u8 = 0xA5;
const STEP: u8 = 37;

fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| SEED.wrapping_add(STEP.wrapping_mul(i as u8)))
        .collect()
}

/// Starts the app from the APPS menu. `launch` cannot be used: the app is inside the OS's
/// `_GetKey` by the time it would read the flash page.
fn start(os: &mut RealOs) {
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(500);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_calls_an_app_makes_leave_the_ui_variables_in_save_screen_alone() {
    let app = assemble_routine("ui_vars_app.z80");
    let mut os = RealOs::boot(&app);
    start(&mut os);
    assert_eq!(
        os.ram(RESULT, 2),
        [5, 0],
        "five calls returned, none changed the bytes"
    );
    os.run_ms(10_000);
    os.tap(KEY_CLEAR, 150, 3000);
    assert_eq!(
        os.ram(RESULT, 2),
        [8, 0],
        "_GetCSC, _PutC, _ClrLCDFull, _GrBufCpy, _NewLine, _GetKey, _CreateAppVar and _Arc_Unarc \
         returned, none changed the bytes"
    );
    assert_eq!(os.ram(VARS, VARS_SIZE), pattern(VARS_SIZE));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn auto_power_down_in_get_key_copies_the_screen_into_save_screen() {
    let app = assemble_routine("ui_vars_app.z80");
    let mut os = RealOs::boot(&app);
    start(&mut os);
    assert_eq!(os.ram(RESULT, 1), [5], "the app is waiting in _GetKey");
    let screen = os.packed_screen();
    assert!(screen.iter().any(|&b| b != 0), "the screen shows text");
    assert_eq!(screen.len(), SAVE_SCREEN_SIZE);
    let fill = pattern(SAVE_SCREEN_SIZE);
    os.m.ram_page(1)[VARS..VARS + SAVE_SCREEN_SIZE].copy_from_slice(&fill);
    let powered_down = |os: &mut RealOs| os.m.in_port(3) & POWER_DOWN != 0;
    for _ in 0..BEFORE_APD_MS / STEP_MS {
        os.run_ms(STEP_MS);
        assert_eq!(
            os.ram(VARS, SAVE_SCREEN_SIZE),
            fill,
            "before auto power down"
        );
        assert!(!powered_down(&mut os), "port 3 reports a power down early");
    }
    let mut copied = false;
    for _ in 0..HUNT_MS / 1_000 {
        os.run_ms(1_000);
        let changed = os.ram(VARS, SAVE_SCREEN_SIZE) != fill;
        let down = powered_down(&mut os);
        if changed || down {
            assert!(
                changed && down,
                "changed {changed}, port 3 powered down {down}"
            );
            assert_eq!(
                os.ram(VARS, SAVE_SCREEN_SIZE),
                screen,
                "saveSScreen holds the screen's pixels"
            );
            copied = true;
            break;
        }
    }
    assert!(copied, "auto power down never changed saveSScreen");
    assert_eq!(os.ram(RESULT, 1), [5], "_GetKey has not returned");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_wait_that_polls_get_csc_between_halts_keeps_the_ui_variables_past_the_apd_time() {
    let app = assemble_routine("ui_vars_csc_app.z80");
    let mut os = RealOs::boot(&app);
    os.launch();
    os.run_ms(500);
    assert_eq!(os.ram(VARS, VARS_SIZE), pattern(VARS_SIZE));
    for _ in 0..(BEFORE_APD_MS + HUNT_MS) / STEP_MS + 1 {
        os.run_ms(STEP_MS);
        assert_eq!(os.ram(VARS, VARS_SIZE), pattern(VARS_SIZE), "after a wait");
        assert_eq!(
            os.m.in_port(3) & POWER_DOWN,
            0,
            "port 3 reports a power down"
        );
    }
    assert_eq!(os.ram(RESULT, 1), [0], "the app has not exited");
    os.tap(KEY_CLEAR, 150, 1500);
    assert_eq!(os.ram(RESULT, 1), [EXITED], "CLEAR reached the app's loop");
}
