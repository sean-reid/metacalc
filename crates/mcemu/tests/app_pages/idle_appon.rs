//! APP_ON: the shell sets it when it clears the OS's ON flag at an idle point and clears it at the
//! start and after every key. Times are milliseconds from the start of the rig.

use crate::builds::*;
use crate::idle_keys::{in_slice, slices};
use crate::idlerig::*;
use crate::shellrig::*;
use mcemu::Machine;

/// APP_ON is 0 after the start whatever RAM held before: the byte is set to 0 before the first idle
/// point, so a stale 1 is never read as a press.
#[test]
fn app_on_is_clear_at_the_start() {
    let sc = script(&[KIND_LIBRARIES], &slices(2, false).slices);
    let mut rig = Rig::stub(three_stages(), Enter::Real, &sc);
    let at = rig.shell.app.label("APP_ON");
    rig.m().load_ram(at, &[0xAA]);
    assert_eq!(
        rig.ram(at, 1),
        [0xAA],
        "the byte is dirty before the app runs"
    );
    rig.run_to_label("app_idle", 6_000_000);
    assert_eq!(rig.app_on(), 0, "the start cleared it");
}

/// ON held 100 ms in slice 1 of eight: the shell sets APP_ON when it clears the break flag, the
/// scripted kind leaves it alone, and the next key app_key returns clears it.
fn on_sets_app_on_and_a_key_clears_it(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 20_000);
    assert!(rig.state().in_slice, "the slice was running at the press");
    assert_eq!(rig.app_on(), 0, "no press yet");
    rig.m().press_key(Machine::KEY_ON);
    let mut flagged = false;
    rig.watch(400_000, |r| flagged |= r.on_flag());
    rig.m().release_key(Machine::KEY_ON);
    assert!(flagged, "the OS flagged the press");
    assert_eq!(
        rig.app_on(),
        1,
        "the shell set APP_ON when it cleared the flag"
    );
    assert!(!rig.on_flag(), "and cleared the flag");
    rig.until("the work ends and the shell waits", 3_000_000, |r| {
        r.state().logged == 8 && r.whereabouts() == Where::OsKeyWait
    });
    assert_eq!(rig.app_on(), 1, "nothing but a key clears it");
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    rig.run_to_label("scr_key", 3_000_000);
    assert_eq!(
        rig.app_on(),
        0,
        "app_key returned a key and the shell cleared it"
    );
    rig.m().release_key(DOWN);
    rig.step(300_000);
    assert_eq!(rig.shell.scr_keys(), keys + 1);
}

twin!(
    on_during_work_sets_app_on_and_the_next_key_clears_it,
    on_the_reference_os_on_during_work_sets_app_on_and_the_next_key_clears_it,
    on_sets_app_on_and_a_key_clears_it
);

/// No kind but checking has an idle routine of its own, and the shared one touches nothing.
#[test]
fn the_other_kinds_leave_app_on_alone() {
    use crate::app::{App, In};
    use crate::realos::BASE;
    let built = crate::built::with_library();
    let mut app = App::boot(built);
    let page0 = &mcemu::decode_app(&built.file).unwrap()[0];
    let table = usize::from(built.label("screen_kinds") - 0x4000);
    let idle_of = |kind: usize| {
        let at = table + 10 * (kind - 1) + 8;
        u16::from_le_bytes([page0[at], page0[at + 1]])
    };
    let shared = built.label("screen_idle");
    for kind in 1..=4 {
        assert_eq!(idle_of(kind), shared, "kind {kind} has the shared idle");
    }
    assert_eq!(
        idle_of(5),
        built.label("checking_idle"),
        "checking has its own"
    );
    let on = built.label("APP_ON");
    app.m.load_ram(on, &[1]);
    let done = app.call(
        BASE,
        shared,
        In {
            a: 0xAA,
            f: 0xFF,
            ..In::default()
        },
    );
    assert_eq!(done.regs.af >> 8, 0, "SCR_STAY");
    assert_eq!(done.regs.af & 1, 0, "carry clear");
    assert_eq!(app.m.read_ram(on, 1), [1], "APP_ON is as it was");
}

/// A key held down while work is pending is not an ON: APP_ON stays 0 at every idle point while it
/// is down, after the key was taken once.
fn a_held_key_leaves_app_on_alone(real: bool) {
    let mut rig = in_slice(real, &slices(8, false), 1, 20_000);
    let keys = rig.shell.scr_keys();
    rig.m().press_key(DOWN);
    let mut seen = Vec::new();
    rig.watch(600_000, |r| seen.push(r.app_on()));
    rig.m().release_key(DOWN);
    let taken = rig.shell.scr_keys() - keys;
    assert!(
        taken == 1 || (real && taken > 1),
        "the key was taken once, or on the reference OS again at each auto repeat: {taken}"
    );
    assert!(
        seen.len() > 100,
        "{} polls while the key was down",
        seen.len()
    );
    assert!(seen.iter().all(|&b| b == 0), "APP_ON stayed 0");
    assert!(
        rig.state().logged >= 4,
        "the work went on while it was down"
    );
}

twin!(
    a_held_key_leaves_app_on_alone_while_work_is_pending,
    on_the_reference_os_a_held_key_leaves_app_on_alone_while_work_is_pending,
    a_held_key_leaves_app_on_alone
);
