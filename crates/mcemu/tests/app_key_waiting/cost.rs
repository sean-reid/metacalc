use crate::realos::ON_INTERRUPT;
use crate::rig::*;
use mcemu::Speed;

/// The most T-states a call may take, with the call, on the path each name gives. Both OSes and both
/// speeds give the same counts, which each test prints.
const COST_CODE: u32 = 55;
const COST_FLAG: u32 = 85;
const COST_FULL: u32 = 250;
/// The port path with its settle delay, which the emulator cannot otherwise check: without the
/// delay it is 50 T-states shorter.
const COST_FULL_MIN: u32 = 235;

fn assert_cost(mut rig: Rig, name: &str) {
    for speed in [Speed::Mhz6, Speed::Mhz15] {
        let none = rig.cost(speed, |r| r.poke(KBD_SCAN, &[0]));
        let code = rig.cost(speed, |r| r.poke(KBD_SCAN, &[DOWN]));
        let flag = rig.cost(speed, |r| {
            r.poke(KBD_SCAN, &[0]);
            r.poke(ON_FLAGS_AT, &[ON_INTERRUPT]);
        });
        let held = rig.cost(speed, |r| {
            r.poke(ON_FLAGS_AT, &[0]);
            r.m().press_key(DOWN);
            r.step(100_000);
            r.poke(KBD_SCAN, &[0]);
        });
        println!(
            "{name} {speed:?} T-states: nothing {none}, scan code {code}, ON flag {flag}, key down {held}"
        );
        assert!(code <= COST_CODE, "scan code path {code}");
        assert!(flag <= COST_FLAG, "break flag path {flag}");
        assert!(
            none > COST_FLAG && held > COST_FLAG,
            "the port path was not taken: {none} {held}"
        );
        assert!(none <= COST_FULL && held <= COST_FULL, "{none} {held}");
        assert!(
            none >= COST_FULL_MIN && held >= COST_FULL_MIN,
            "the settle delay is missing: {none} {held}"
        );
        rig.m().release_key(DOWN);
        rig.step(100_000);
    }
}

#[test]
fn the_call_stays_under_its_cost_ceilings_on_every_path_on_the_stub() {
    assert_cost(Rig::stub(), "stub");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_call_stays_under_its_cost_ceilings_on_every_path_on_the_reference_os() {
    assert_cost(Rig::real(), "ROM");
}
