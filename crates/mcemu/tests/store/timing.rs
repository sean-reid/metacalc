use super::rig::*;

/// T-states of one command, from a run of it a hundred times against a run of nothing.
fn cost(rig: &mut Rig, cmd: Cmd, setup: &[Cmd]) -> u32 {
    let mut base = setup.to_vec();
    base.push(Cmd::Repeat(100));
    base.push(Cmd::Nop);
    let mut with = setup.to_vec();
    with.push(Cmd::Repeat(100));
    with.push(cmd);
    let (_, t0) = rig.run_timed(&base, None);
    let (_, t1) = rig.run_timed(&with, None);
    (t1 - t0) / 100
}

#[test]
fn timings_stay_inside_their_budgets() {
    let mut rig = Rig::new();
    let open = [Cmd::Open(CRC, COUNT)];
    let open_found = cost(&mut rig, open[0], &open);
    rig.m.delete_var("MCSTAT");
    let (_, bare) = rig.run_timed(&[Cmd::Nop], None);
    let (_, created) = rig.run_timed(&open, None);
    let open_created = created - bare;
    let get = cost(&mut rig, Cmd::Get(1663), &open);
    let set = cost(&mut rig, Cmd::Set(1663, 2), &open);
    let count = cost(&mut rig, Cmd::Count, &open);
    eprintln!(
        "T-states on the stub at 6 MHz from flash: open found {open_found}, open created {open_created}, get {get}, set {set}, count of {COUNT} {count}"
    );
    assert!(open_found < 20_000, "{open_found}");
    assert!(open_created < 60_000, "{open_created}");
    assert!(get < 10_000, "{get}");
    assert!(set < 10_000, "{set}");
    assert!(count < 250_000, "{count}");
}
