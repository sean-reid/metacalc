//! How many times the reference OS 2.43 lets an installed app launch after a boot. The OS keeps
//! a launch budget for each app slot and deletes an app whose budget is spent, with a
//! "Defragmenting..." screen. `install_app` writes the budget the OS's own install writes for an
//! app without a trial limit; every test here is ignored without the ROM image.

#[path = "common/realos.rs"]
mod realos;

use realos::*;

/// The OS's text shadow in RAM page 81h.
const TEXT_SHADOW: usize = 0x8508 - 0x8000;
/// The launch budget of the app on `BASE`: flash page 7Eh, 5E50h.
const BUDGET: (u8, usize) = (0x7E, 0x1E50);

fn first_row(os: &mut RealOs) -> String {
    String::from_utf8_lossy(&os.ram(TEXT_SHADOW, 16)).into_owned()
}

fn budget(os: &mut RealOs) -> Vec<u8> {
    os.m.read_flash(BUDGET.0, BUDGET.1, 2)
}

/// Opens the APPS menu and, unless the OS is defragmenting, starts the app and leaves it with
/// CLEAR. Returns whether the app ran.
fn launch_and_quit(os: &mut RealOs) -> bool {
    os.m.load_ram(EXIT_MARK, &[0]);
    os.tap(KEY_APPS, 150, 600);
    if first_row(os).starts_with("Defragmenting") {
        return false;
    }
    os.tap(KEY_2, 150, 300);
    os.run_ms(300);
    os.tap(KEY_CLEAR, 150, 300);
    os.m.read_ram(EXIT_MARK, 1)[0] == 0xA5
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_installed_app_launches_forty_times_without_a_defragment() {
    let mut os = RealOs::boot(&assemble_routine("wait_app.z80"));
    assert_eq!(
        budget(&mut os),
        [0x80, 0x00],
        "install_app marks the app unlimited"
    );
    for n in 1..=40 {
        assert!(launch_and_quit(&mut os), "launch {n} ran");
        assert_eq!(
            budget(&mut os),
            [0x80, 0x00],
            "launch {n} leaves the budget alone"
        );
    }
    assert_eq!(
        os.m.read_flash(BASE, 0, 2),
        [0x80, 0x0F],
        "the app is still installed"
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_app_on_the_default_budget_is_deleted_after_sixteen_launches() {
    let mut os = RealOs::boot(&assemble_routine("wait_app.z80"));
    os.m.load_flash(BUDGET.0, BUDGET.1, &[0xFF, 0xFF]);
    for n in 1..=16 {
        assert!(launch_and_quit(&mut os), "launch {n} ran");
        let left = budget(&mut os);
        if n < 16 {
            assert_eq!(
                u16::from_le_bytes([left[0], left[1]]),
                0xFFFFu16 >> n,
                "launch {n} spends one bit"
            );
        }
    }
    assert_eq!(
        os.m.read_flash(BASE, 0, 1),
        [0x80],
        "the app is installed after its 16th launch"
    );
    assert!(!launch_and_quit(&mut os), "the 17th APPS press defragments");
    os.run_ms(30_000);
    assert_eq!(os.m.read_flash(BASE, 0, 1), [0xFF], "the OS erased the app");
}
