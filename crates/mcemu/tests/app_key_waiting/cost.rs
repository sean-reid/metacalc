use crate::realos::ON_INTERRUPT;
use crate::rig::*;
use mcemu::{Machine, Speed};

/// The most T-states a call may take, with the call, on the path each name gives, 2 percent over the
/// count the emulator prints. Both OSes and both speeds give the same counts.
const COST_CODE: u32 = 52;
const COST_FLAG: u32 = 101;
const COST_NONE: u32 = 274;
const COST_KEY: u32 = 287;
const COST_ON: u32 = 284;
const COST_BOTH: u32 = 287;
/// The least each port path takes, a little under what the emulator prints (nothing 269, ON down
/// 279, a key down 282, both 282). The ON floor is over the nothing path's ceiling, so ON cannot
/// pass by answering as nothing does.
const FLOOR_NONE: u32 = 266;
const FLOOR_KEY: u32 = 279;
const FLOOR_ON: u32 = 276;
const FLOOR_BOTH: u32 = 279;

/// `with_on` measures the paths with ON down, which only the stub does: with ON down the cost
/// harness's mask of every interrupt source leaves the reference OS in its own code, so the call is
/// never reached in 100,000 T-states, though outputs.rs reaches it there with the sources unmasked.
fn assert_cost(mut rig: Rig, name: &str, with_on: bool) {
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
        rig.m().release_key(DOWN);
        rig.step(100_000);
        let (on, both) = if with_on {
            let on = rig.cost_late(
                speed,
                |r| r.poke(KBD_SCAN, &[0]),
                |r| r.m().press_key(Machine::KEY_ON),
            );
            rig.m().release_key(Machine::KEY_ON);
            rig.step(200_000);
            let both = rig.cost_late(
                speed,
                |r| r.poke(KBD_SCAN, &[0]),
                |r| {
                    r.m().press_key(Machine::KEY_ON);
                    r.m().press_key(DOWN);
                },
            );
            for key in [DOWN, Machine::KEY_ON] {
                rig.m().release_key(key);
            }
            rig.step(200_000);
            (on, both)
        } else {
            (0, 0)
        };
        println!(
            "{name} {speed:?} T-states: nothing {none}, scan code {code}, ON flag {flag}, \
             key down {held}, ON down {on}, ON and key down {both}"
        );
        assert!((49..=COST_CODE).contains(&code), "scan code path {code}");
        assert!((97..=COST_FLAG).contains(&flag), "break flag path {flag}");
        let mut paths = vec![
            ("nothing", none, FLOOR_NONE, COST_NONE),
            ("key", held, FLOOR_KEY, COST_KEY),
        ];
        if with_on {
            paths.extend([
                ("ON", on, FLOOR_ON, COST_ON),
                ("both", both, FLOOR_BOTH, COST_BOTH),
            ]);
        }
        for (path, t, floor, ceiling) in paths {
            assert!(t > COST_FLAG, "the port path was not taken: {path} {t}");
            assert!(t <= ceiling, "{path} {t} over {ceiling}");
            assert!(
                t >= floor,
                "{path} {t} under {floor}: the path or the settle delay is missing"
            );
        }
    }
}

#[test]
fn the_call_stays_under_its_cost_ceilings_on_every_path_on_the_stub() {
    assert_cost(Rig::stub(), "stub", true);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_call_stays_under_its_cost_ceilings_on_every_path_on_the_reference_os() {
    assert_cost(Rig::real(), "ROM", false);
}
