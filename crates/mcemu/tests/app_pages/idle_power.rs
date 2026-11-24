//! Auto power down on the reference OS while a screen has work pending. The OS fires it only
//! inside `_GetKey`, so work that never ends keeps the calculator on, and when it ends the power
//! down comes and the next key refreshes the screen as it does after any power down.

use crate::builds::*;
use crate::idlerig::*;
use crate::shellrig::*;
use mcemu::Machine;

/// Bit 4 of port 3 reads set once the OS has powered down.
const POWERED_DOWN: u8 = 0x10;
const STEP_S: u32 = 5;

fn powered_down(rig: &mut Rig) -> bool {
    rig.m().in_port(3) & POWERED_DOWN != 0
}

/// Runs in steps of 5 s until the OS powers down or `max` seconds pass. Returns the seconds it took
/// and the OS's two power down timer bytes after each step.
fn until_power_down(rig: &mut Rig, max: u32) -> (u32, Vec<(u8, u8)>) {
    let mut timers = vec![rig.apd_timers()];
    let mut seconds = 0;
    while !powered_down(rig) {
        assert!(seconds < max, "no power down in {max} s");
        rig.step(STEP_S * 1_000_000);
        seconds += STEP_S;
        timers.push(rig.apd_timers());
    }
    (seconds, timers)
}

fn show(what: &str, timers: &[(u8, u8)]) {
    let head: Vec<_> = timers.iter().take(5).collect();
    let tail: Vec<_> = timers.iter().rev().take(3).rev().collect();
    println!("{what}: timer bytes (8448h, 8449h) every {STEP_S} s, first {head:?}, last {tail:?}");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_work_that_never_ends_holds_off_the_power_down_until_it_ends() {
    let built = three_stages();
    let mut idle = Rig::real(built, Enter::Real, &script(&[], &[]));
    let (apd, control) = until_power_down(&mut idle, 900);
    show("waiting for a key", &control);
    println!("control: the OS powers down {apd} s after the app started waiting");
    assert!(apd >= 200, "{apd} s");
    drop(idle);

    let mut rig = Rig::real(
        built,
        Enter::Real,
        &Script {
            slices: vec![Slice::ms(100).more()],
            forever: true,
            restart: false,
            kinds: vec![KIND_LIBRARIES],
        },
    );
    let run = apd * 2 + 60;
    let mut timers = vec![rig.apd_timers()];
    let mut calls = Vec::new();
    for second in (STEP_S..=run).step_by(STEP_S as usize) {
        rig.step(STEP_S * 1_000_000);
        assert!(
            !powered_down(&mut rig),
            "powered down at {second} s with work pending, the control did at {apd} s"
        );
        timers.push(rig.apd_timers());
        calls.push(rig.state().calls);
    }
    show("work pending forever", &timers);
    let (first, last) = (calls[0], *calls.last().unwrap());
    println!(
        "{run} s with work pending, no power down; scr_idle calls {first} after 5 s, {last} at the end"
    );
    assert!(
        u32::from(last) >= run * 8,
        "the work ran: {last} calls in {run} s"
    );
    assert_eq!((rig.shell.blits(), rig.shell.scr_keys()), (1, 0));

    rig.m().load_ram(STATE + 2, &[0]);
    let (after, _) = until_power_down(&mut rig, 900);
    println!("the work ended, and the OS powered down {after} s later");
    let (sentinel, frame) = rig.shell.sentinel();
    let (enters, blits) = (rig.shell.enters(KIND_LIBRARIES), rig.shell.blits());
    rig.m().press_key(Machine::KEY_ON);
    rig.step(150_000);
    rig.m().release_key(Machine::KEY_ON);
    rig.step(3_000_000);
    assert_ne!(
        rig.ram(sentinel, 1)[0],
        !rig.ram(frame, 1)[0],
        "the power down wrote the pixels over the sentinel"
    );
    rig.m().load_ram(crate::app::UI_FRAME, &[0xAA; 768]);
    rig.m().press_key(F4);
    rig.step(150_000);
    rig.m().release_key(F4);
    rig.step(500_000);
    assert_eq!(
        rig.shell.enters(KIND_LIBRARIES),
        enters + 1,
        "the key refreshed the screen once"
    );
    assert_eq!(rig.shell.blits(), blits + 1, "and drew it once");
    let want = model::libraries(built);
    rig.shell
        .shows("the screen after the wake and a key", &want);
    assert_eq!(
        rig.ram(sentinel, 1)[0],
        !rig.ram(frame, 1)[0],
        "the sentinel is stored again"
    );
}
