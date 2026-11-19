//! What the reference OS does to saveSScreen, where the UI keeps its variables, while an app runs.

#[path = "common/realos.rs"]
mod realos;

use mcemu::Machine;
use realos::*;

const RAM_BASE: usize = 0x8000;
const VARS: usize = 0x86EC - RAM_BASE;
const VARS_SIZE: usize = 0x1A;
const SAVE_SCREEN_SIZE: usize = 768;
const RESULT: usize = 0xA800 - RAM_BASE;
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
fn auto_power_down_in_get_key_zeroes_save_screen() {
    let app = assemble_routine("ui_vars_app.z80");
    let mut os = RealOs::boot(&app);
    start(&mut os);
    assert_eq!(os.ram(RESULT, 1), [5], "the app is waiting in _GetKey");
    let fill = pattern(SAVE_SCREEN_SIZE);
    os.m.ram_page(1)[VARS..VARS + SAVE_SCREEN_SIZE].copy_from_slice(&fill);
    for _ in 0..8 {
        os.run_ms(30_000);
        assert_eq!(
            os.ram(VARS, SAVE_SCREEN_SIZE),
            fill,
            "before auto power down"
        );
    }
    let mut zeroed = false;
    for _ in 0..30 {
        os.run_ms(5_000);
        if os.ram(VARS, SAVE_SCREEN_SIZE).iter().all(|&b| b == 0) {
            zeroed = true;
            break;
        }
    }
    assert!(zeroed, "auto power down never zeroed saveSScreen");
    assert_eq!(os.ram(RESULT, 1), [5], "_GetKey has not returned");
    os.tap(Machine::KEY_ON, 150, 1500);
}
