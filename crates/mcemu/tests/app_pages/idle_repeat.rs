//! Key repeat while the top screen has no work, or short work after each key, on the reference OS,
//! which is the one that repeats. Times are milliseconds from the press.

use crate::app::main_tree;
use crate::builds::*;
use crate::idlerig::*;
use crate::shellrig::*;

/// What the plain wait measured on the reference OS with a held Down, from the first return: the
/// next three returns, then one every 92.5 ms. The first return is 1.5 to 10 ms after the press,
/// by the phase of the OS's scan.
const PLAIN: [f64; 4] = [0.0, 463.0, 556.0, 648.5];
const GAP: f64 = 92.5;
/// One interrupt period and one step of the rig, in milliseconds.
const TOLERANCE: f64 = 1.953 + 0.5;

fn ms(us: u32) -> f64 {
    f64::from(us) / 1000.0
}

/// Holds Down for `hold` ms and returns when scr_key was called and when idle slices began, from
/// the press, then lets go and settles.
fn held_down(rig: &mut Rig, hold: u32) -> (Vec<f64>, Vec<f64>) {
    let mut keys_seen = rig.shell.scr_keys();
    let mut slices_seen = rig.state().logged;
    let start = rig.t;
    rig.m().press_key(DOWN);
    let (mut keys, mut slices) = (Vec::new(), Vec::new());
    rig.watch(hold * 1000, |r| {
        let now = r.shell.scr_keys();
        while keys_seen < now {
            keys.push(ms(r.t - start));
            keys_seen += 1;
        }
        let logged = r.state().logged;
        while slices_seen < logged {
            slices.push(ms(r.t - start));
            slices_seen += 1;
        }
    });
    rig.m().release_key(DOWN);
    rig.step(300_000);
    (keys, slices)
}

fn tap(rig: &mut Rig, scan: u8) {
    rig.tap_when_taken(scan);
    rig.step(300_000);
}

/// The screen of each shipped kind, reached from the Libraries screen with three stages.
fn on_kind(rig: &mut Rig, kind: u8) {
    match kind {
        KIND_LIBRARIES => {}
        KIND_NOTICE => {
            tap(rig, F1);
            tap(rig, F2);
        }
        KIND_STAGE => tap(rig, F3),
        crate::theorem_rig::KIND_THEOREM => {
            tap(rig, F1);
            tap(rig, F1);
        }
        crate::dist_rig::KIND_DIST => {
            tap(rig, F1);
            tap(rig, F1);
            tap(rig, F2);
        }
        KIND_ABOUT => {
            for _ in 0..4 {
                tap(rig, DOWN);
            }
            tap(rig, F3);
        }
        _ => unreachable!(),
    }
    assert_eq!(
        rig.shell.top_kind(),
        kind,
        "the screen of kind {kind} is open"
    );
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_holding_down_repeats_at_the_plain_wait_times_on_every_shipped_kind() {
    for (name, kind) in [
        ("libraries", KIND_LIBRARIES),
        ("notice", KIND_NOTICE),
        ("about", KIND_ABOUT),
        ("stage", KIND_STAGE),
        ("theorem", crate::theorem_rig::KIND_THEOREM),
        ("dist", crate::dist_rig::KIND_DIST),
    ] {
        let mut rig = Rig::real(three_stages(), Enter::Real, &script(&[], &[]));
        rig.step(600_000);
        on_kind(&mut rig, kind);
        let before = (rig.shell.key_waiting_calls(), rig.shell.idle_calls());
        let (keys, _) = held_down(&mut rig, 1_100);
        println!("Down held on {name}: scr_key at {keys:?} ms");
        assert!(keys.len() >= 8, "{name}: {} keys in 1.1 s", keys.len());
        assert!(
            (1.5..=10.0 + TOLERANCE).contains(&keys[0]),
            "{name}: the first key at {} ms",
            keys[0]
        );
        for (got, want) in keys.iter().zip(PLAIN) {
            assert!(
                (got - keys[0] - want).abs() <= TOLERANCE,
                "{name}: a key at {got} ms, {want} ms after the first expected"
            );
        }
        for pair in keys[3..].windows(2) {
            let gap = pair[1] - pair[0];
            assert!((gap - GAP).abs() <= TOLERANCE, "{name}: repeat gap {gap}");
        }
        let after = (rig.shell.key_waiting_calls(), rig.shell.idle_calls());
        let n = keys.len() as u16;
        assert_eq!(
            (after.0 - before.0, after.1 - before.1),
            (n, n),
            "{name}: one app_key_waiting and one scr_idle for each of {n} keys"
        );
    }
}

/// Down held on a screen whose top kind does 100 ms of work after each key, with interrupts on: the
/// repeat still comes, a code the OS sets during the work is taken as the work ends, and each key
/// is followed by one slice, so the repeat runs at the pace of the work.
#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_holding_down_with_100_ms_of_work_after_each_key_still_repeats() {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(100)]);
    sc.restart = true;
    let mut rig = Rig::real(three_stages(), Enter::Real, &sc);
    rig.until("the first slice", 2_000_000, |r| r.state().logged == 1);
    rig.step(600_000);
    let (keys, slices) = held_down(&mut rig, 1_500);
    println!("Down held with 100 ms of work after each key: keys at {keys:?}");
    println!("slices began at {slices:?}");
    assert!(keys.len() >= 10, "{} keys in 1.5 s", keys.len());
    assert!(
        (1.5..=10.0 + TOLERANCE).contains(&keys[0]),
        "first key {}",
        keys[0]
    );
    assert!(
        (keys[1] - keys[0] - PLAIN[1]).abs() <= TOLERANCE,
        "the first repeat at {} ms",
        keys[1]
    );
    let gaps: Vec<f64> = keys[1..].windows(2).map(|p| p[1] - p[0]).collect();
    println!("gaps between the keys: {gaps:?}");
    for gap in &gaps {
        assert!((110.0..=140.0).contains(gap), "repeat gap {gap} ms");
    }
    for gap in &gaps[gaps.len() - 5..] {
        assert!((110.0..=115.0).contains(gap), "steady repeat gap {gap} ms");
    }
    assert_eq!(slices.len(), keys.len(), "one slice for each key");
    for (key, slice) in keys.iter().zip(&slices) {
        assert!(
            (0.0..=45.0).contains(&(slice - key)),
            "the slice began {} ms after its key",
            slice - key
        );
    }
}

/// Down held for 1.1 s on the Libraries screen of the app built from the commit the branch was cut
/// from (`MAIN`), and of this branch's: main's shell blocks in app_key after every key, so its times
/// are the plain wait's, and the branch's with no work pending are the same within one interrupt
/// period.
#[test]
#[ignore = "needs the reference ROM image and the history of the commit the branch was cut from"]
fn on_the_reference_os_holding_down_repeats_at_the_times_main_does() {
    let main = three_stages_from(main_tree(), "pages-main-three");
    let mut times = Vec::new();
    for (name, built) in [("main", main), ("branch", three_stages())] {
        let mut rig = Rig::real(built, Enter::Real, &script(&[], &[]));
        rig.step(600_000);
        let (keys, _) = held_down(&mut rig, 1_100);
        println!("Down held on {name}: scr_key at {keys:?} ms");
        assert!(keys.len() >= 8, "{name}: {} keys in 1.1 s", keys.len());
        times.push(keys);
    }
    let (main, branch) = (&times[0], &times[1]);
    let n = main.len().min(branch.len());
    assert!(
        main.len().abs_diff(branch.len()) <= 1,
        "keys in 1.1 s: main {}, branch {}",
        main.len(),
        branch.len()
    );
    for i in 0..n {
        let (m, b) = (main[i] - main[0], branch[i] - branch[0]);
        println!("key {i}: main {m:.1} ms after its first, branch {b:.1}");
        assert!(
            (m - b).abs() <= TOLERANCE,
            "key {i}: main {m} ms, branch {b} ms after the first"
        );
    }
}
