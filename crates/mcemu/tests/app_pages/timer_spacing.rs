//! The spacing of the timer interrupts at the app's idle point, on the stub OS and on the reference
//! OS, at both speeds: the gaps between interrupt entries by the emulator's clock, what each OS
//! writes to ports 3 and 4, and how long its handler runs with interrupts off.

use crate::builds::*;
use crate::idlerig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::{PortWrite, Reached, Speed};

const NEVER: u16 = 0x7FFF;

struct Entry {
    clock: u32,
    off: u32,
}

/// The next `n` interrupt entries, each with the T-states its handler ran before interrupts
/// were on again.
fn entries(rig: &mut Rig, n: usize) -> (Vec<Entry>, Vec<PortWrite>) {
    rig.m().clear_port_writes();
    let mut found = Vec::new();
    let mut ports = Vec::new();
    while found.len() < n {
        match rig.m().run_to(BASE, NEVER, 2_000_000) {
            Reached::Vector38 => {}
            other => panic!("no interrupt in 2,000,000 T-states: {other:?}"),
        }
        let clock = rig.m().clock();
        let mut off = 0;
        while rig.m().regs().iff1 == 0 {
            assert!(off < 20_000, "interrupts stayed off for {off} T-states");
            rig.m().run_to(BASE, NEVER, 4);
            off = rig.m().clock().wrapping_sub(clock);
        }
        found.push(Entry { clock, off });
        let more = rig.m().take_port_writes();
        ports.extend(more);
    }
    (found, ports)
}

fn report(what: &str, mhz: f64, found: &[Entry], ports: &[PortWrite]) {
    let mut gaps: Vec<f64> = found
        .windows(2)
        .map(|w| f64::from(w[1].clock.wrapping_sub(w[0].clock)) / mhz)
        .collect();
    gaps.sort_by(f64::total_cmp);
    let mut hist: Vec<(u32, usize)> = Vec::new();
    for g in &gaps {
        let g = (g * 10.0).round() as u32;
        match hist.iter_mut().find(|h| h.0 == g) {
            Some(h) => h.1 += 1,
            None => hist.push((g, 1)),
        }
    }
    println!("{what}: gap histogram (tenths of us, count) {hist:?}");
    let mut offs: Vec<f64> = found.iter().map(|e| f64::from(e.off) / mhz).collect();
    offs.sort_by(f64::total_cmp);
    println!(
        "{what}: {} entries, gaps us min {:.1} median {:.1} max {:.1}; interrupts off us min {:.1} \
         median {:.1} max {:.1}",
        found.len(),
        gaps[0],
        gaps[gaps.len() / 2],
        gaps[gaps.len() - 1],
        offs[0],
        offs[offs.len() / 2],
        offs[offs.len() - 1]
    );
    let first = found[0].clock;
    let shown: Vec<String> = ports
        .iter()
        .take(24)
        .map(|w| {
            format!(
                "p{}={:02X}@{:+.1}us",
                w.port,
                w.value,
                f64::from(w.clock.wrapping_sub(first) as i32) / mhz
            )
        })
        .collect();
    println!("{what}: first port writes {shown:?}");
    let mut kinds: Vec<(u8, u8, usize)> = Vec::new();
    for w in ports {
        match kinds.iter_mut().find(|k| (k.0, k.1) == (w.port, w.value)) {
            Some(k) => k.2 += 1,
            None => kinds.push((w.port, w.value, 1)),
        }
    }
    println!("{what}: (port, value, count) over the run {kinds:?}");
}

fn spacing(real: bool, slices: Script, name: &str) {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &slices);
    rig.until("the idle point", 2_000_000, |r| r.state().in_slice);
    rig.step(100_000);
    for (speed, mhz) in [(Speed::Mhz6, 6.0), (Speed::Mhz15, 15.0)] {
        rig.m().set_speed(speed);
        entries(&mut rig, 5);
        let (found, ports) = entries(&mut rig, 200);
        report(
            &format!(
                "{} OS, {name}, {mhz} MHz",
                if real { "reference" } else { "stub" }
            ),
            mhz,
            &found,
            &ports,
        );
    }
    rig.m().set_speed(Speed::Mhz6);
}

fn in_the_tick_loop(real: bool) {
    let slice = Slice::ms(10).ticks(2_000);
    spacing(real, script(&[KIND_LIBRARIES], &[slice]), "tick loop");
}

fn in_short_slices(real: bool) {
    let mut sc = script(&[KIND_LIBRARIES], &[Slice::ms(3).off().more()]);
    sc.forever = true;
    spacing(real, sc, "3 ms slices");
}

#[test]
#[ignore = "prints a measurement"]
fn stub_tick_loop() {
    in_the_tick_loop(false);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn reference_tick_loop() {
    in_the_tick_loop(true);
}

#[test]
#[ignore = "prints a measurement"]
fn stub_short_slices() {
    in_short_slices(false);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn reference_short_slices() {
    in_short_slices(true);
}
