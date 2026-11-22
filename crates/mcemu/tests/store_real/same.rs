//! One script on the stub and on the reference OS: the logs match byte for byte.

use crate::harness::*;
use crate::layout::{self, Table};
use crate::rig::*;

fn by_hand(values: &[u8], end: usize) -> [u16; 4] {
    let mut counts = [0u16; 4];
    for v in &values[..end] {
        counts[usize::from(*v)] += 1;
    }
    counts
}

/// The records with what a refused get or get_next left in B and HL taken out: the store's header
/// defines neither.
fn defined(mut log: Vec<Rec>) -> Vec<Rec> {
    for rec in &mut log {
        if rec.a != ST_OK && matches!(rec.cmd, 2 | 4) {
            rec.rest.clear();
        }
    }
    log
}

fn on_both(what: &str, cmds: &[Cmd]) -> Vec<Rec> {
    let stub = defined(Rig::new().run(cmds));
    let mut os = boot();
    let real = defined(launch(&mut os, cmds));
    assert_eq!(real, stub, "{what}: the reference OS against the stub");
    real
}

#[test]
#[ignore = "needs the reference ROM image"]
fn every_routine_answers_as_on_the_stub_with_no_table() {
    let log = on_both(
        "no table",
        &[
            Cmd::Get(0),
            Cmd::Set(0, 1),
            Cmd::GetNext,
            Cmd::SetNext(1),
            Cmd::Count,
            Cmd::CountTo(1),
            Cmd::CountEnds(&[1]),
            Cmd::Reset,
        ],
    );
    assert!(log.iter().all(|r| r.a == ST_ABSENT), "{log:?}");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn get_set_and_the_next_number_answer_as_on_the_stub() {
    let mut script = vec![Cmd::Open(CRC, COUNT)];
    for (i, n) in [0, 1663, 3325].into_iter().enumerate() {
        script.extend([Cmd::Set(n, 1 + i as u8), Cmd::Get(n)]);
    }
    for v in [3, 2, 1, 0] {
        script.extend([Cmd::Set(1662, v), Cmd::Get(1662), Cmd::Get(1663)]);
    }
    script.extend([
        Cmd::Get(3326),
        Cmd::Set(3326, 1),
        Cmd::Set(5, 4),
        Cmd::Get(5),
        Cmd::GetNext,
        Cmd::SetNext(3326),
        Cmd::GetNext,
        Cmd::SetNext(3327),
        Cmd::GetNext,
        Cmd::Count,
        Cmd::CountTo(1664),
        Cmd::CountEnds(&[1, 1663, 1664, 3326]),
        Cmd::Reset,
        Cmd::Count,
        Cmd::GetNext,
        Cmd::Open(CRC, COUNT),
    ]);
    let log = on_both("get and set", &script);
    assert_eq!((log[2].b(), log[4].b(), log[6].b()), (1, 2, 3));
    assert_eq!(
        (log[8].b(), log[9].b()),
        (3, 2),
        "a set leaves its neighbour"
    );
    assert_eq!(log[19].a, ST_RANGE, "get past the end");
    assert_eq!(log[25].word(0), 3326);
    assert_eq!(log[26].a, ST_RANGE, "a next number past the count");
    assert_eq!(log[28].counts(), [3323, 1, 1, 1]);
    assert_eq!(log[32].counts(), [3326, 0, 0, 0], "after the reset");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_counts_over_a_full_table_match_the_stub_and_a_count_by_hand() {
    const ENDS: &[u16] = &[779, 1171, 3326];
    const ODD: &[u16] = &[0, 1, 2, 3, 4, 5, 251, 3325];
    let mut seed = 0x6EC0_FFEE;
    let mut t = Table::new(CRC, COUNT);
    t.values = (0..COUNT)
        .map(|_| (xorshift(&mut seed) & 3) as u8)
        .collect();
    let bytes = layout::encode_padded(&t, xorshift(&mut seed) as u8);
    let script = [
        Cmd::Open(CRC, COUNT),
        Cmd::Count,
        Cmd::CountTo(779),
        Cmd::CountTo(3326),
        Cmd::CountTo(3327),
        Cmd::CountEnds(ENDS),
        Cmd::CountEnds(ODD),
        Cmd::CountEnds(&[1171, 779]),
        Cmd::Get(3325),
    ];
    let mut rig = Rig::new();
    rig.m.create_var(VAR, &bytes).unwrap();
    let stub = defined(rig.run(&script));

    let mut os = boot();
    launch(&mut os, &[Cmd::Open(CRC, COUNT)]);
    leave(&mut os);
    poke_table(&mut os, 0, &bytes);
    let real = defined(launch(&mut os, &script));
    assert_eq!(real, stub, "the reference OS against the stub");

    let all = |n: u16| by_hand(&t.values, n.into());
    assert_eq!(
        (real[0].a, real[0].b()),
        (ST_OK, 0),
        "the poked table is kept"
    );
    assert_eq!(real[1].counts(), all(3326));
    assert_eq!(real[2].counts(), all(779));
    assert_eq!((real[4].a, real[4].counts()), (ST_RANGE, [0xA5A5; 4]));
    let ends = |list: &[u16]| list.iter().map(|&e| all(e)).collect::<Vec<_>>();
    assert_eq!(real[5].ends(), ends(ENDS));
    assert_eq!(real[6].ends(), ends(ODD));
    assert_eq!((real[7].a, real[7].untouched()), (ST_RANGE, true));
    assert_eq!(
        table(&mut os),
        Some((bytes, false)),
        "counting writes nothing"
    );
}
