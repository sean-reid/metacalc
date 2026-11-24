use crate::rig::*;

/// The times, in microseconds from the press, at which app_key returns while Down is held for
/// 2.5 s, and the keys it gives, with `calls` calls before each app_key; or with the routine
/// polled after every interrupt until it answers NZ when `poll` is set.
fn repeat_run(mk: fn() -> Rig, calls: u8, poll: bool) -> (Vec<u32>, Vec<u8>) {
    let mut rig = mk();
    rig.start_round(0, true, LOOP | if poll { POLL } else { 0 }, calls);
    rig.step(50_000);
    let base = rig.done();
    rig.m().press_key(DOWN);
    let mut times = vec![];
    let mut seen = base;
    for t in (STEP_US..=2_500_000).step_by(STEP_US as usize) {
        rig.step(STEP_US);
        let d = rig.done();
        for _ in seen..d {
            times.push(t);
        }
        seen = d;
    }
    rig.m().release_key(DOWN);
    let keys = rig.log()[usize::from(base)..].to_vec();
    (times, keys)
}

fn assert_repeat(mk: fn() -> Rig, at_least: usize) {
    let (without, _) = repeat_run(mk, 0, false);
    let (with, _) = repeat_run(mk, 1, false);
    println!("returns without the call: {without:?}");
    assert!(without.len() >= at_least, "{without:?}");
    assert_eq!(with.len(), without.len(), "{with:?} against {without:?}");
    for (a, b) in with.iter().zip(&without) {
        assert!(a.abs_diff(*b) <= PERIOD_US, "{with:?} against {without:?}");
    }
}

#[test]
fn a_held_key_repeats_at_the_same_times_with_the_call_before_every_app_key() {
    assert_repeat(Rig::stub, 1);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_held_arrow_repeats_at_the_same_times_with_the_call_before_every_app_key() {
    assert_repeat(Rig::real, 15);
}

/// The routine polled every interrupt across the gaps between repeats gives the same keys at the
/// same times as app_key left to wait: each return may come one poll (one interrupt period) and
/// the interrupt that sets the code later, so two periods.
fn assert_polled_repeat(mk: fn() -> Rig, at_least: usize) {
    let (without, keys) = repeat_run(mk, 0, false);
    let (with, polled_keys) = repeat_run(mk, 0, true);
    println!("returns without polling: {without:?}\nreturns polled: {with:?}");
    assert!(without.len() >= at_least, "{without:?}");
    assert_eq!(with.len(), without.len(), "{with:?} against {without:?}");
    assert_eq!(polled_keys, keys);
    assert!(keys.iter().all(|&k| k == CODE_DOWN), "{keys:?}");
    for (a, b) in with.iter().zip(&without) {
        assert!(
            a.abs_diff(*b) <= 2 * PERIOD_US,
            "{with:?} against {without:?}"
        );
    }
}

#[test]
fn polling_across_the_gaps_between_repeats_gives_the_same_keys_at_the_same_times() {
    assert_polled_repeat(Rig::stub, 1);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_polling_across_the_gaps_between_repeats_gives_the_same_keys_at_the_same_times()
 {
    assert_polled_repeat(Rig::real, 15);
}
