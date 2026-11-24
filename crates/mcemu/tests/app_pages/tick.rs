//! app_tick: what it keeps, the interrupt state it leaves, and the length of a tick on the stub OS
//! and on the reference OS while the app is in its loop (the idle routine of idlerig.rs calls it).

use crate::builds::*;
use crate::idlerig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use mcemu::{Reached, Regs, Speed};

const HEADER_BYTES: [u8; 3] = [0xFB, 0x76, 0xC9];
const MARK: Regs = Regs {
    af: 0x5AD7,
    bc: 0x1234,
    de: 0x5678,
    hl: 0x9ABC,
    ix: 0x1357,
    iy: 0x89F0,
    sp: 0,
    pc: 0,
    af2: 0x2222,
    bc2: 0x3333,
    de2: 0x4444,
    hl2: 0x5555,
    i: 0,
    iff1: 0,
    iff2: 0,
    im: 1,
};

/// Waits for the instruction at `at` to be next, going through every interrupt on the way.
fn reach(rig: &mut Rig, at: u16, limit: u32) -> u32 {
    let start = rig.m().clock();
    let mut interrupts = 0;
    loop {
        match rig.m().run_to(BASE, at, limit) {
            Reached::Target => return interrupts,
            Reached::Vector38 => interrupts += 1,
            Reached::Limit => panic!("{at:04X} was not reached in {limit} T-states"),
        }
        let spent = rig.m().clock().wrapping_sub(start);
        assert!(
            spent < limit,
            "{at:04X} was not reached in {limit} T-states"
        );
    }
}

/// Calls app_tick with every register a marker and interrupts as `iff` says, the return address
/// the entry page's `screen_ret`, and returns what it left.
fn call(rig: &mut Rig, iff: u8) -> (Regs, u32) {
    let (tick, ret) = (
        rig.shell.app.label("app_tick"),
        rig.shell.app.label("screen_ret"),
    );
    let sp = rig.shell.app.stack;
    rig.m().load_ram(sp, &ret.to_le_bytes());
    rig.m().out_port(6, BASE);
    let regs = Regs {
        sp,
        pc: tick,
        iff1: iff,
        iff2: iff,
        ..MARK
    };
    rig.m().set_regs(&regs);
    let start = rig.m().clock();
    reach(rig, ret, 200_000);
    let took = rig.m().clock().wrapping_sub(start);
    (rig.m().regs(), took)
}

fn keeps_what_it_must_and_leaves_interrupts_on(real: bool) {
    let mut rig = Rig::new(real, three_stages(), Enter::Real, &script(&[], &[]));
    rig.step(300_000);
    let at = rig.shell.app.label("app_tick");
    let code = rig.m().read_flash(BASE, usize::from(at - 0x4000), 3);
    assert_eq!(
        code, HEADER_BYTES,
        "ei, halt, ret: no push, no call, no write"
    );
    for iff in [0, 1] {
        let (regs, took) = call(&mut rig, iff);
        let sp = rig.shell.app.stack;
        println!("interrupts {iff} at the entry: returned after {took} T-states");
        assert_eq!(
            regs.sp,
            sp + 2,
            "interrupts {iff}: the return address is the only stack it uses"
        );
        assert_eq!(
            [regs.af, regs.bc, regs.de, regs.hl, regs.ix, regs.iy],
            [MARK.af, MARK.bc, MARK.de, MARK.hl, MARK.ix, MARK.iy],
            "interrupts {iff}: the registers and flags"
        );
        assert_eq!(
            (regs.iff1, regs.iff2),
            (1, 1),
            "interrupts {iff}: on at its exit"
        );
    }
}

twin!(
    app_tick_keeps_the_registers_and_flags_and_returns_with_interrupts_on,
    on_the_reference_os_app_tick_keeps_the_registers_and_flags_and_returns_with_interrupts_on,
    keeps_what_it_must_and_leaves_interrupts_on
);

/// The T-states between the ends of `n` consecutive ticks the app's loop waits for, and the
/// interrupts taken during each. The first tick's end is the starting point.
fn ticks(rig: &mut Rig, n: usize) -> (Vec<u32>, Vec<u32>) {
    let tick = rig.shell.app.label("app_tick");
    reach(rig, tick + 2, 2_000_000);
    let mut last = rig.m().clock();
    let (mut periods, mut taken) = (Vec::new(), Vec::new());
    for _ in 0..n {
        reach(rig, tick + 1, 100_000);
        taken.push(reach(rig, tick + 2, 2_000_000));
        assert_eq!(rig.m().regs().iff1, 1, "interrupts are on when a tick ends");
        let now = rig.m().clock();
        periods.push(now.wrapping_sub(last));
        last = now;
    }
    (periods, taken)
}

/// The range of a tick in T-states, a little outside the 55,659 to 55,665 and 139,152 to 139,156
/// the stub and the reference OS print over 298 ticks.
const TICK_6: (u32, u32) = (55_650, 55_675);
const TICK_15: (u32, u32) = (139_140, 139_170);

struct Spread {
    min: u32,
    max: u32,
    mean: f64,
    interrupts: (u32, u32),
}

fn spread(periods: &[u32], taken: &[u32]) -> Spread {
    Spread {
        min: *periods.iter().min().unwrap(),
        max: *periods.iter().max().unwrap(),
        mean: periods.iter().map(|&p| f64::from(p)).sum::<f64>() / periods.len() as f64,
        interrupts: (*taken.iter().min().unwrap(), *taken.iter().max().unwrap()),
    }
}

fn the_length_of_a_tick(real: bool) {
    let slice = Slice::ms(10).ticks(700);
    let mut rig = Rig::new(
        real,
        three_stages(),
        Enter::Real,
        &script(&[KIND_LIBRARIES], &[slice]),
    );
    rig.until("the ticks", 2_000_000, |r| r.state().in_slice);
    rig.step(30_000);
    for (speed, mhz) in [(Speed::Mhz6, 6.0), (Speed::Mhz15, 15.0)] {
        rig.m().set_speed(speed);
        let (mut periods, mut taken) = ticks(&mut rig, 300);
        periods.drain(..2);
        taken.drain(..2);
        let s = spread(&periods, &taken);
        println!(
            "{} OS at {mhz} MHz, {} consecutive ticks: min {} max {} mean {:.1} T-states, \
             {:.1} to {:.1} us, mean {:.1} us; interrupts taken per tick {} to {}; \
             ticks in 100 ms: {:.1}",
            if real { "reference" } else { "stub" },
            periods.len(),
            s.min,
            s.max,
            s.mean,
            f64::from(s.min) / mhz,
            f64::from(s.max) / mhz,
            s.mean / mhz,
            s.interrupts.0,
            s.interrupts.1,
            100_000.0 * mhz / s.mean
        );
        let (low, high) = if mhz == 6.0 {
            (TICK_6.0, TICK_6.1)
        } else {
            (TICK_15.0, TICK_15.1)
        };
        assert!(
            s.min >= low && s.max <= high,
            "{} to {} T-states at {mhz} MHz, expected within {low} to {high}",
            s.min,
            s.max
        );
        assert_eq!(s.interrupts, (1, 1), "one interrupt ends each tick");
        let eleven_ms = 11.0 * s.mean / mhz / 1_000.0;
        assert!(
            (100.0..=105.0).contains(&eleven_ms),
            "11 ticks take {eleven_ms:.2} ms"
        );
    }
    rig.m().set_speed(Speed::Mhz6);
}

twin!(
    the_length_of_a_tick_on_the_stub_is_printed_over_hundreds_of_ticks,
    on_the_reference_os_the_length_of_a_tick_is_printed_over_hundreds_of_ticks,
    the_length_of_a_tick
);
