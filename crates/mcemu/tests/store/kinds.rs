//! store_next_not_one and store_prev_not_one against a scan by hand, one rule written from the
//! header, over tables the tests lay out themselves.

use super::layout::{self, Table};
use super::rig::*;

const NO_MATCH: u16 = 0xFFFF;
/// The OS's OP4 to OP6, which the two routines do not name and must leave alone.
const OP4: u16 = 0x8499;

fn next_not_one(values: &[u8], from: usize, end: usize) -> u16 {
    (from..end).find(|&i| values[i] != 1).unwrap_or(end) as u16
}

fn prev_not_one(values: &[u8], from: usize) -> u16 {
    (0..from)
        .rev()
        .find(|&i| values[i] != 1)
        .map_or(NO_MATCH, |i| i as u16)
}

fn table(values: &[u8], padding: u8) -> Rig {
    let mut rig = Rig::new();
    let mut t = Table::new(CRC, values.len() as u16);
    t.values = values.to_vec();
    rig.m
        .create_var(VAR, &layout::encode_padded(&t, padding))
        .unwrap();
    rig
}

#[derive(Default, Debug)]
struct Counts {
    matched: usize,
    none: usize,
    range: usize,
}

/// Every call of both routines over `values` for every from and end up to two past the count,
/// run in scripts of at most `PART` commands, against the rule.
fn sweep(values: &[u8], padding: u8, counts: &mut Counts) {
    const PART: usize = 40;
    let count = values.len();
    let mut script = Vec::new();
    let mut want: Vec<(u8, u16)> = Vec::new();
    for from in 0..=count + 1 {
        for end in 0..=count + 1 {
            script.push(Cmd::NextNotOne(from as u16, end as u16));
            if end > count || from > end {
                want.push((ST_RANGE, 0));
                counts.range += 1;
            } else {
                let n = next_not_one(values, from, end);
                want.push((ST_OK, n));
                if usize::from(n) == end {
                    counts.none += 1;
                } else {
                    counts.matched += 1;
                }
            }
        }
        script.push(Cmd::PrevNotOne(from as u16));
        if from > count {
            want.push((ST_RANGE, 0));
            counts.range += 1;
        } else {
            let n = prev_not_one(values, from);
            want.push((ST_OK, n));
            if n == NO_MATCH {
                counts.none += 1;
            } else {
                counts.matched += 1;
            }
        }
    }
    let mut rig = table(values, padding);
    let before = rig.raw();
    for (cmds, wants) in script.chunks(PART).zip(want.chunks(PART)) {
        let log = rig.run(cmds);
        assert_eq!(log.len(), cmds.len());
        for ((cmd, rec), want) in cmds.iter().zip(&log).zip(wants) {
            assert_eq!(rec.a, want.0, "{cmd:?} over {values:?}");
            if want.0 == ST_OK {
                assert_eq!(rec.word(0), want.1, "{cmd:?} over {values:?}");
            }
        }
    }
    assert_eq!(rig.raw(), before, "a scan writes nothing");
}

#[test]
fn every_value_at_every_position_of_a_byte_is_found_from_every_start_to_every_end() {
    let mut counts = Counts::default();
    for count in [9usize, 13] {
        for pos in 0..count {
            for value in [0u8, 2, 3] {
                let mut values = vec![1u8; count];
                values[pos] = value;
                sweep(&values, 0xFF, &mut counts);
            }
        }
        sweep(&vec![1u8; count], 0xFF, &mut counts);
        sweep(&vec![1u8; count], 0, &mut counts);
    }
    println!("{counts:?}");
    assert!(counts.matched > 2200, "{counts:?}");
    assert!(counts.none > 4400, "{counts:?}");
    assert!(counts.range > 6800, "{counts:?}");
}

#[test]
fn a_table_of_mixed_values_is_scanned_as_the_rule_says() {
    let mut seed = 0x0A0B_0C0D;
    let mut counts = Counts::default();
    for count in [1usize, 2, 3, 4, 5, 8, 17, 30] {
        for ones in [50, 90] {
            let values: Vec<u8> = (0..count)
                .map(|_| {
                    if xorshift(&mut seed) % 100 < ones {
                        1
                    } else {
                        (xorshift(&mut seed) & 3) as u8
                    }
                })
                .collect();
            sweep(&values, 0xFF, &mut counts);
        }
    }
    println!("{counts:?}");
    assert!(
        counts.matched > 850 && counts.none > 750 && counts.range > 1650,
        "{counts:?}"
    );
}

#[test]
fn bytes_of_four_equal_values_other_than_one_are_never_passed() {
    let mut counts = Counts::default();
    for value in [0u8, 2, 3] {
        for count in [13usize, 16, 25] {
            sweep(&vec![value; count], 0xFF, &mut counts);
            for (from, to) in [(4, 8), (4, 12), (8, 16)] {
                if to <= count {
                    let mut values = vec![1u8; count];
                    values[from..to].fill(value);
                    sweep(&values, 0xFF, &mut counts);
                    let mut values = vec![value; count];
                    values[from..to].fill(1);
                    sweep(&values, 0, &mut counts);
                }
            }
        }
    }
    println!("{counts:?}");
    assert!(
        counts.matched > 9800 && counts.none > 3300 && counts.range > 13300,
        "{counts:?}"
    );
}

#[test]
fn whole_bytes_of_ones_are_passed_and_the_scan_stops_inside_the_byte_that_is_not() {
    let count = usize::from(COUNT);
    for pos in [
        0usize,
        1,
        3,
        4,
        7,
        8,
        11,
        12,
        255,
        256,
        1000,
        count - 2,
        count - 1,
    ] {
        for value in [0u8, 2, 3] {
            let mut values = vec![1u8; count];
            values[pos] = value;
            let mut rig = table(&values, 0xFF);
            let (p, c) = (pos as u16, count as u16);
            let log = rig.run(&[
                Cmd::NextNotOne(0, c),
                Cmd::NextNotOne(0, p),
                Cmd::NextNotOne(0, p + 1),
                Cmd::NextNotOne(p, c),
                Cmd::NextNotOne(p + 1, c),
                Cmd::PrevNotOne(c),
                Cmd::PrevNotOne(p + 1),
                Cmd::PrevNotOne(p),
                Cmd::PrevNotOne(0),
            ]);
            let got: Vec<u16> = log.iter().map(|r| r.word(0)).collect();
            assert!(log.iter().all(|r| r.a == ST_OK));
            assert_eq!(
                got,
                [p, p, p, p, c, p, p, NO_MATCH, NO_MATCH],
                "position {pos} value {value}"
            );
        }
    }
    let mut rig = table(&vec![1u8; count], 0xFF);
    let log = rig.run(&[
        Cmd::NextNotOne(0, COUNT),
        Cmd::NextNotOne(COUNT, COUNT),
        Cmd::PrevNotOne(COUNT),
        Cmd::PrevNotOne(0),
    ]);
    let got: Vec<u16> = log.iter().map(|r| r.word(0)).collect();
    assert_eq!(got, [COUNT, COUNT, NO_MATCH, NO_MATCH]);
}

#[test]
fn an_end_in_the_middle_of_a_byte_takes_the_assertion_before_it_and_not_the_one_at_it() {
    for pos in [9usize, 10, 11] {
        let mut values = vec![1u8; 20];
        values[pos] = 3;
        let mut rig = table(&values, 0xFF);
        let log = rig.run(&[
            Cmd::NextNotOne(8, 10),
            Cmd::NextNotOne(8, 11),
            Cmd::PrevNotOne(10),
            Cmd::PrevNotOne(11),
        ]);
        let got: Vec<u16> = log.iter().map(|r| r.word(0)).collect();
        let want = match pos {
            9 => [9, 9, 9, 9],
            10 => [10, 10, NO_MATCH, 10],
            _ => [10, 11, NO_MATCH, NO_MATCH],
        };
        assert_eq!(got, want, "not 1 at {pos}");
    }
}

#[test]
fn a_missing_or_damaged_table_gives_the_status_store_next_zero_gives() {
    let cmds = [
        Cmd::NextNotOne(0, 0),
        Cmd::PrevNotOne(0),
        Cmd::NextZero(0, 0),
    ];
    let mut rig = Rig::new();
    let log = rig.run(&cmds);
    assert_eq!(log.iter().map(|r| r.a).collect::<Vec<_>>(), [ST_ABSENT; 3]);
    assert!(rig.raw().is_none());

    let mut t = Table::new(CRC, 10);
    t.values.fill(1);
    let good = layout::encode(&t);
    let mut magic = good.clone();
    magic[0] = 0x4E;
    let mut version = good.clone();
    version[1] = 2;
    let long = [good.clone(), vec![0]].concat();
    let short = good[..good.len() - 1].to_vec();
    for (bytes, want) in [
        (magic, ST_BAD_MAGIC),
        (version, ST_BAD_MAGIC),
        (long, ST_BAD_SIZE),
        (short, ST_BAD_SIZE),
    ] {
        let mut rig = Rig::new();
        rig.m.create_var(VAR, &bytes).unwrap();
        let log = rig.run(&cmds);
        let got: Vec<u8> = log.iter().map(|r| r.a).collect();
        assert_eq!(got, [want; 3], "{bytes:?}");
        assert_eq!(rig.raw().unwrap(), bytes, "a refused call writes nothing");
    }
}

/// The RAM after OP3, where the OS keeps its other operand registers.
fn operands(rig: &mut Rig) -> Vec<u8> {
    rig.m.read_ram(OP4, 3 * 11)
}

#[test]
fn the_table_and_the_operand_registers_after_op3_are_as_they_were() {
    let mut seed = 0x5EED_0001;
    let values: Vec<u8> = (0..300)
        .map(|_| {
            if xorshift(&mut seed).is_multiple_of(4) {
                (xorshift(&mut seed) & 3) as u8
            } else {
                1
            }
        })
        .collect();
    let cmds = [
        Cmd::NextNotOne(0, 300),
        Cmd::NextNotOne(17, 299),
        Cmd::PrevNotOne(300),
        Cmd::PrevNotOne(123),
    ];
    let mut rig = table(&values, 0xFF);
    rig.m.load_ram(OP4, &[0xC3; 33]);
    let raw = rig.raw();
    let log = rig.run(&[Cmd::Nop]);
    assert_eq!(log[0].a, ST_OK);
    let quiet = operands(&mut rig);
    assert_eq!(
        quiet,
        vec![0xC3; 33],
        "a run with no scan leaves the marks alone"
    );
    rig.m.load_ram(OP4, &[0xC3; 33]);
    let log = rig.run(&cmds);
    assert!(log.iter().all(|r| r.a == ST_OK));
    assert!(
        log.iter()
            .any(|r| r.word(0) != 300 && r.word(0) != NO_MATCH)
    );
    assert_eq!(operands(&mut rig), quiet, "OP4 to OP6");
    assert_eq!(rig.raw(), raw);
}

#[test]
fn what_they_cost_over_the_ring_table_is_printed_and_pinned() {
    let count = usize::from(COUNT);
    let mut values = vec![1u8; count];
    let mut rig = table(&values, 0);
    let c = COUNT;
    let measure = |rig: &mut Rig, name: &str, cmd: Cmd| {
        let took = rig.window(&[cmd], label(name), 0);
        println!("{name} {cmd:?}: {took} T-states");
        took
    };
    let whole_next = measure(&mut rig, "STORE_NEXT_NOT_ONE", Cmd::NextNotOne(0, c));
    let whole_prev = measure(&mut rig, "STORE_PREV_NOT_ONE", Cmd::PrevNotOne(c));
    let one_next = measure(&mut rig, "STORE_NEXT_NOT_ONE", Cmd::NextNotOne(1, 2));
    let one_prev = measure(&mut rig, "STORE_PREV_NOT_ONE", Cmd::PrevNotOne(1));
    values[0] = 0;
    values[count - 1] = 0;
    let mut rig = table(&values, 0);
    let row_next = measure(&mut rig, "STORE_NEXT_NOT_ONE", Cmd::NextNotOne(1, c));
    let row_prev = measure(&mut rig, "STORE_PREV_NOT_ONE", Cmd::PrevNotOne(c - 1));
    println!("{whole_next} {whole_prev} {one_next} {one_prev} {row_next} {row_prev}");
    for (what, got, measured) in [
        ("next, all of ring's table", whole_next, 57_232),
        ("prev, all of ring's table", whole_prev, 55_989),
        ("next, one assertion", one_next, 5_010),
        ("prev, one assertion", one_prev, 4_391),
        ("next, 1 to the last", row_next, 59_744),
        ("prev, the last to 0", row_prev, 57_543),
    ] {
        let (low, high) = (measured - measured / 50, measured + measured / 100);
        assert!(
            (low..=high).contains(&got),
            "{what}: {got} T-states, measured {measured}"
        );
    }
}
