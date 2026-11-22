use super::rig::*;

#[test]
fn timings_stay_inside_their_budgets() {
    let mut rig = Rig::new();
    let open = [Cmd::Open(CRC, COUNT)];
    let open_found = rig.cost(open[0], &open);
    rig.m.delete_var(VAR);
    let (_, bare) = rig.run_timed(&[Cmd::Nop], None);
    let (_, created) = rig.run_timed(&open, None);
    let open_created = created - bare;
    let get = rig.cost(Cmd::Get(1663), &open);
    let set = rig.cost(Cmd::Set(1663, 2), &open);
    let count = rig.cost(Cmd::Count, &open);
    let to = [779, 1171, 3326].map(|n| rig.cost(Cmd::CountTo(n), &open));
    let pass = rig.cost(Cmd::CountEnds(&[779, 1171, 3326]), &open);
    let three: u32 = to.iter().sum();
    eprintln!(
        "T-states on the stub at 6 MHz from flash: open found {open_found}, open created {open_created}, get {get}, set {set}, count of {COUNT} {count}, prefixes {to:?}, three prefixes {three}, one pass over the three ends {pass}"
    );
    assert!(open_found < 20_000, "{open_found}");
    assert!(open_created < 60_000, "{open_created}");
    assert!(get < 10_000, "{get}");
    assert!(set < 10_000, "{set}");
    assert!(count < 250_000, "{count}");
    assert!(pass < 250_000, "{pass}");
    assert!(
        pass * 3 < three * 2,
        "one pass {pass} against three calls {three}"
    );
}
