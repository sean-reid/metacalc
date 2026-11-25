use crate::rig::*;

/// Releases that land at 93 different points of a scan, 100 us apart.
const PHASES: u32 = 93;
const PHASE_US: u32 = 100;
const FINE_US: u32 = 100;

/// The microseconds from the release of DOWN to the scan code reading 0, stepped by `FINE_US`,
/// with the release `phase` us after the key had been down 150 ms.
fn clear_time(rig: &mut Rig, phase: u32) -> u32 {
    rig.step(100_000);
    assert_eq!(rig.ram(KBD_SCAN, 1)[0], 0, "no key, no code");
    rig.m().press_key(DOWN);
    rig.step(150_000 + phase);
    assert_eq!(rig.ram(KBD_SCAN, 1)[0], DOWN, "the scan saw the key");
    rig.m().release_key(DOWN);
    let mut t = 0;
    while rig.ram(KBD_SCAN, 1)[0] != 0 {
        assert!(t < 100_000, "the code was not cleared in 100 ms");
        rig.step(FINE_US);
        t += FINE_US;
    }
    t
}

/// The clear time at each of `PHASES` phases, on a fresh machine for every ten, which keeps the
/// reference OS from powering down.
fn clear_times(mk: fn() -> Rig) -> Vec<u32> {
    let mut times = vec![];
    for chunk in 0..PHASES.div_ceil(10) {
        let mut rig = mk();
        for i in chunk * 10..(chunk * 10 + 10).min(PHASES) {
            times.push(clear_time(&mut rig, i * PHASE_US));
        }
    }
    times
}

/// The reference OS cleared the code 37.2 to 46.3 ms after the release over 93 phases. The bounds
/// add 0.2 ms below and 0.4 ms above, which is where the stub's scan lands it, and stay under half
/// a scan from the measurement, so a release count one off fails.
const CLEARED_AFTER_US: u32 = 37_000;
const CLEARED_BY_US: u32 = 46_700;

fn assert_clear_times(mk: fn() -> Rig) {
    let times = clear_times(mk);
    assert_eq!(times.len(), PHASES as usize);
    let (first, last) = (*times.iter().min().unwrap(), *times.iter().max().unwrap());
    println!("clear time {first} to {last} us over {PHASES} phases");
    assert!(
        first > CLEARED_AFTER_US && last <= CLEARED_BY_US,
        "the code cleared {first} to {last} us after the release"
    );
}

#[test]
fn the_stub_clears_the_scan_code_as_long_after_a_release_as_the_reference_os() {
    assert_clear_times(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_clears_the_scan_code_about_37_to_46_ms_after_a_release() {
    assert_clear_times(Rig::real);
}
