//! What the store's routines cost on the reference OS, beside the stub's figures.

use crate::harness::*;
use crate::rig::*;

const ENDS: &[u16] = &[779, 1171, 3326];

/// T-states from the record before `cmd` to its own, with `cmd` run `reps` times after `setup`
/// on a machine booted for it: the OS defragments its apps at the sixteenth launch after a boot.
fn interval(setup: &[Cmd], cmd: Cmd, reps: u16) -> u32 {
    let script = [setup, &[Cmd::Nop, Cmd::Repeat(reps), cmd]].concat();
    launch_timed(&mut boot(), &script)[setup.len() + 1]
}

/// T-states of one `cmd` after `setup`, less the same for a command that does nothing.
fn cost(setup: &[Cmd], cmd: Cmd, reps: u16) -> u32 {
    (interval(setup, cmd, reps) - interval(setup, Cmd::Nop, reps)) / u32::from(reps)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_routines_cost_on_the_reference_os_stays_inside_its_budgets() {
    let open = [Cmd::Open(CRC, COUNT)];

    // The probe's loop makes no OS call, so a turn costs more than on the stub only if the OS's
    // interrupt runs during a script.
    let spin = (interval(&[], Cmd::Nop, 20_001) - interval(&[], Cmd::Nop, 1)) / 20_000;
    let stub_spin = {
        let mut rig = Rig::new();
        let mut run = |n| rig.run_timed(&[Cmd::Repeat(n), Cmd::Nop], None).1;
        (run(20_001) - run(1)) / 20_000
    };
    println!("the probe's loop: {spin} T-states a turn on OS 2.43, {stub_spin} on the stub");
    assert_eq!(spin, stub_spin, "no interrupt runs during a script");

    let real = [
        cost(&[], open[0], 1),
        cost(&open, Cmd::Locate, 100),
        cost(&open, open[0], 100),
        cost(&open, Cmd::Get(1663), 100),
        cost(&open, Cmd::Set(1663, 2), 100),
        cost(&open, Cmd::Count, 10),
        cost(&open, Cmd::CountEnds(ENDS), 10),
    ];
    let mut rig = Rig::new();
    let (_, bare) = rig.run_timed(&[Cmd::Nop], None);
    let (_, made) = rig.run_timed(&open, None);
    let stub = [
        made - bare,
        rig.cost(Cmd::Locate, &open),
        rig.cost(open[0], &open),
        rig.cost(Cmd::Get(1663), &open),
        rig.cost(Cmd::Set(1663, 2), &open),
        rig.cost(Cmd::Count, &open),
        rig.cost(Cmd::CountEnds(ENDS), &open),
    ];
    let rows = [
        ("open created", 70_000),
        ("one _ChkFindSym", 6_300),
        ("open found", 8_000),
        ("get", 7_500),
        ("set", 7_700),
        ("count", 215_000),
        ("one pass over three ends", 220_000),
    ];
    for (((name, ceiling), real), stub) in rows.into_iter().zip(real).zip(stub) {
        println!("{name}: {real} T-states on OS 2.43, {stub} on the stub");
        assert!(real < ceiling, "{name}: {real}");
    }

    // The OS walks the named entries from the oldest, so a table created after other variables
    // is found later.
    let mut crowd: Vec<Cmd> = (b'A'..=b'T').map(|c| Cmd::Make(char::from(c), 1)).collect();
    crowd.push(open[0]);
    let behind = cost(&crowd, Cmd::Get(1663), 100);
    println!("get with 20 AppVars created before the table: {behind} T-states");
    assert!(behind > real[3] && behind < 13_000, "{behind}");
}
