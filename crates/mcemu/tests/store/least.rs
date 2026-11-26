//! store_least_not_one against a scan of the list by hand, one rule written from the header, over
//! tables the tests lay out themselves.

use super::layout::{self, Table};
use super::rig::*;

/// The OS's operand registers OP3 to OP6, of which the routine destroys none past OP2.
const OP3: u16 = 0x848E;

fn leak(list: &[u16]) -> &'static [u16] {
    Box::leak(list.to_vec().into_boxed_slice())
}

/// The rule: the least number of the list with start <= n < limit whose value is not 1, else the
/// limit; ST_RANGE for a limit above the count or a start above the limit.
fn least(values: &[u8], list: &[u16], start: u16, limit: u16) -> Result<u16, u8> {
    if usize::from(limit) > values.len() || start > limit {
        return Err(ST_RANGE);
    }
    Ok(list
        .iter()
        .copied()
        .filter(|&n| n >= start && n < limit && values[usize::from(n)] != 1)
        .min()
        .unwrap_or(limit))
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
    hit: usize,
    limit: usize,
    range: usize,
}

/// Runs `cases` of (list, start, limit) over `values` in scripts that fit the probe's 640 bytes
/// and checks each against the rule.
fn run_cases(rig: &mut Rig, values: &[u8], cases: &[(Vec<u16>, u16, u16)], counts: &mut Counts) {
    let mut part: Vec<Cmd> = Vec::new();
    let mut wants: Vec<Result<u16, u8>> = Vec::new();
    let mut size = 0;
    let flush = |rig: &mut Rig, part: &mut Vec<Cmd>, wants: &mut Vec<Result<u16, u8>>| {
        if part.is_empty() {
            return;
        }
        let log = rig.run(part);
        assert_eq!(log.len(), part.len());
        for ((cmd, rec), want) in part.iter().zip(&log).zip(wants.iter()) {
            match want {
                Ok(n) => {
                    assert_eq!((rec.a, rec.word(0)), (ST_OK, *n), "{cmd:?} over {values:?}");
                }
                Err(status) => assert_eq!(rec.a, *status, "{cmd:?} over {values:?}"),
            }
        }
        part.clear();
        wants.clear();
    };
    for (list, start, limit) in cases {
        let bytes = 6 + 2 * list.len();
        if size + bytes > 600 {
            flush(rig, &mut part, &mut wants);
            size = 0;
        }
        size += bytes;
        let want = least(values, list, *start, *limit);
        match want {
            Ok(n) if n == *limit => counts.limit += 1,
            Ok(_) => counts.hit += 1,
            Err(_) => counts.range += 1,
        }
        part.push(Cmd::LeastNotOne(*start, *limit, leak(list)));
        wants.push(want);
    }
    flush(rig, &mut part, &mut wants);
}

fn lists_for(count: usize, pos: usize) -> Vec<Vec<u16>> {
    let up: Vec<u16> = (0..count as u16).collect();
    let down: Vec<u16> = up.iter().rev().copied().collect();
    vec![
        up,
        down,
        vec![pos as u16],
        vec![count as u16 + 5, 0xFFFF, pos as u16, 3, pos as u16],
        vec![],
    ]
}

fn grid(count: usize, lists: &[Vec<u16>]) -> Vec<(Vec<u16>, u16, u16)> {
    let mut cases = Vec::new();
    for list in lists {
        for start in 0..=count as u16 + 1 {
            for limit in 0..=count as u16 + 1 {
                cases.push((list.clone(), start, limit));
            }
        }
    }
    cases
}

#[test]
fn every_value_at_every_position_of_a_byte_is_found_for_every_list_start_and_limit() {
    let mut counts = Counts::default();
    for count in [9usize, 13] {
        for pos in 0..count {
            for value in [0u8, 2, 3] {
                let mut values = vec![1u8; count];
                values[pos] = value;
                let mut rig = table(&values, 0xFF);
                let before = rig.raw();
                run_cases(
                    &mut rig,
                    &values,
                    &grid(count, &lists_for(count, pos)),
                    &mut counts,
                );
                assert_eq!(rig.raw(), before, "a scan writes nothing");
            }
        }
        let ones = vec![1u8; count];
        for padding in [0xFFu8, 0] {
            let mut rig = table(&ones, padding);
            run_cases(
                &mut rig,
                &ones,
                &grid(count, &lists_for(count, 0)),
                &mut counts,
            );
        }
    }
    println!("{counts:?}");
    assert!(counts.hit > 1000, "{counts:?}");
    assert!(counts.limit > 3000, "{counts:?}");
    assert!(counts.range > 3000, "{counts:?}");
}

#[test]
fn a_table_of_mixed_values_and_a_count_that_is_not_a_multiple_of_four_agree_with_the_rule() {
    let mut seed = 0x0A0B_0C0D;
    let mut counts = Counts::default();
    for count in [1usize, 2, 3, 5, 6, 7, 10, 30] {
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
            let mut rig = table(&values, 0xFF);
            let mut lists = lists_for(count, count - 1);
            let mut shuffled: Vec<u16> = (0..count as u16).collect();
            for i in (1..shuffled.len()).rev() {
                shuffled.swap(i, xorshift(&mut seed) as usize % (i + 1));
            }
            lists.push(shuffled);
            run_cases(&mut rig, &values, &grid(count, &lists), &mut counts);
        }
    }
    println!("{counts:?}");
    assert!(
        counts.hit > 800 && counts.limit > 1000 && counts.range > 1000,
        "{counts:?}"
    );
}

#[test]
fn a_list_of_one_and_a_list_of_127_over_the_ring_table_are_the_rules_answer() {
    let count = usize::from(COUNT);
    let mut seed = 0x5EED_0002;
    let values: Vec<u8> = (0..count)
        .map(|_| {
            if xorshift(&mut seed).is_multiple_of(6) {
                (xorshift(&mut seed) & 3) as u8
            } else {
                1
            }
        })
        .collect();
    let mut rig = table(&values, 0xFF);
    let mut counts = Counts::default();
    let mut random = |n: usize| -> Vec<u16> {
        (0..n)
            .map(|_| (xorshift(&mut seed) as usize % count) as u16)
            .collect()
    };
    let mut cases = Vec::new();
    for _ in 0..6 {
        let one = random(1);
        cases.push((one.clone(), 0, COUNT));
        cases.push((one.clone(), one[0], one[0] + 1));
        cases.push((one.clone(), one[0] + 1, COUNT));
        cases.push((one, 0, COUNT));
        let many = random(127);
        cases.push((many.clone(), 0, COUNT));
        cases.push((many.clone(), 500, 2500));
        cases.push((many.clone(), 1000, 1001));
        cases.push((many, 3000, COUNT));
    }
    run_cases(&mut rig, &values, &cases, &mut counts);
    println!("{counts:?}");
    assert!(counts.hit > 15 && counts.limit > 8, "{counts:?}");
    let ones = vec![1u8; count];
    let mut rig = table(&ones, 0);
    let mut counts = Counts::default();
    let cases: Vec<_> = (0..4)
        .map(|i| {
            (
                (0..127).map(|k| (k * 25 + i) as u16).collect::<Vec<_>>(),
                0,
                COUNT,
            )
        })
        .collect();
    run_cases(&mut rig, &ones, &cases, &mut counts);
    assert_eq!(
        (counts.hit, counts.limit, counts.range),
        (0, 4, 0),
        "an all-1 table answers the limit"
    );
}

#[test]
fn a_number_below_the_start_at_it_at_the_limit_and_above_is_skipped_or_taken_as_the_rule_says() {
    let mut values = vec![1u8; 20];
    for n in [4usize, 9, 15] {
        values[n] = 0;
    }
    let mut rig = table(&values, 0xFF);
    let list = [15u16, 9, 4, 9, 20, 3];
    let mut counts = Counts::default();
    let cases: Vec<_> = [
        (0u16, 20u16),
        (4, 20),
        (5, 20),
        (4, 5),
        (4, 4),
        (5, 9),
        (5, 10),
        (9, 10),
        (10, 15),
        (10, 16),
        (16, 20),
        (20, 20),
    ]
    .into_iter()
    .map(|(s, l)| (list.to_vec(), s, l))
    .collect();
    run_cases(&mut rig, &values, &cases, &mut counts);
    let log = rig.run(&[
        Cmd::LeastNotOne(0, 20, leak(&list)),
        Cmd::LeastNotOne(5, 20, leak(&list)),
        Cmd::LeastNotOne(10, 15, leak(&list)),
        Cmd::LeastNotOne(0, 4, leak(&list)),
        Cmd::LeastNotOne(0, 20, leak(&[])),
    ]);
    let got: Vec<(u8, u16)> = log.iter().map(|r| (r.a, r.word(0))).collect();
    assert_eq!(
        got,
        [(ST_OK, 4), (ST_OK, 9), (ST_OK, 15), (ST_OK, 4), (ST_OK, 20)]
    );
}

#[test]
fn a_missing_or_damaged_table_gives_the_status_store_next_not_one_gives() {
    let list = leak(&[1, 2, 3]);
    let cmds = [Cmd::LeastNotOne(0, 0, list), Cmd::NextNotOne(0, 0)];
    let mut rig = Rig::new();
    let log = rig.run(&cmds);
    assert_eq!(log.iter().map(|r| r.a).collect::<Vec<_>>(), [ST_ABSENT; 2]);
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
        assert_eq!(got, [want; 2], "{bytes:?}");
        assert_eq!(rig.raw().unwrap(), bytes, "a refused call writes nothing");
    }
}

fn operands(rig: &mut Rig) -> Vec<u8> {
    rig.m.read_ram(OP3, 4 * 11)
}

#[test]
fn the_table_the_list_and_the_operand_registers_after_op2_are_as_they_were() {
    let mut seed = 0x5EED_0003;
    let values: Vec<u8> = (0..300)
        .map(|_| {
            if xorshift(&mut seed).is_multiple_of(4) {
                (xorshift(&mut seed) & 3) as u8
            } else {
                1
            }
        })
        .collect();
    let list: Vec<u16> = (0..40).map(|i| (i * 7 + 3) as u16).collect();
    let cmds = [
        Cmd::LeastNotOne(0, 300, leak(&list)),
        Cmd::LeastNotOne(17, 299, leak(&list[5..30])),
    ];
    let mut rig = table(&values, 0xFF);
    rig.m.load_ram(OP3, &[0xC3; 44]);
    let raw = rig.raw();
    let log = rig.run(&[Cmd::Nop]);
    assert_eq!(log[0].a, ST_OK);
    let quiet = operands(&mut rig);
    assert_eq!(
        quiet,
        vec![0xC3; 44],
        "a run with no call leaves the marks alone"
    );
    rig.m.load_ram(OP3, &[0xC3; 44]);
    let log = rig.run(&cmds);
    assert!(log.iter().all(|r| r.a == ST_OK));
    assert!(log.iter().any(|r| r.word(0) != 300 && r.word(0) != 299));
    assert_eq!(operands(&mut rig), quiet, "OP3 to OP6");
    assert_eq!(rig.raw(), raw);
    let mut script: Vec<u8> = cmds.iter().flat_map(|c| c.bytes()).collect();
    script.push(0);
    assert_eq!(
        rig.m.read_ram(label("SCRIPT"), script.len()),
        script,
        "the lists are as they were"
    );
}

/// The numbers of the assertions of ring whose label starts with `prefix`, in label order, from the
/// built library.
fn ring_numbers(prefix: &[u8]) -> Vec<u16> {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/ring.mclib")).unwrap();
    let library = mcpack::library::decode(&bytes).unwrap();
    let labels: Vec<&[u8]> = library
        .assertions
        .iter()
        .map(|a| a.label.as_bytes())
        .collect();
    let mut order: Vec<u16> = (0..labels.len() as u16).collect();
    order.sort_by_key(|&n| labels[usize::from(n)]);
    order
        .into_iter()
        .filter(|&n| labels[usize::from(n)].starts_with(prefix))
        .collect()
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_call_costs_over_the_runs_of_s_on_ring_is_printed_and_pinned() {
    let numbers = ring_numbers(b"s");
    assert_eq!(numbers.len(), 375);
    let runs: Vec<&'static [u16]> = numbers.chunks(127).map(leak).collect();
    assert_eq!(
        runs.iter().map(|r| r.len()).collect::<Vec<_>>(),
        [127, 127, 121]
    );
    let count = usize::from(COUNT);
    let mut total_ones = 0;
    let mut total_none = 0;
    for (name, values) in [
        ("every assertion 1", vec![1u8; count]),
        ("no assertion 1", vec![0u8; count]),
    ] {
        let mut rig = table(&values, 0);
        let mut limit = COUNT;
        for run in &runs {
            let took = rig.window(
                &[Cmd::LeastNotOne(0, limit, run)],
                label("STORE_LEAST_NOT_ONE"),
                0,
            );
            let want = least(&values, run, 0, limit).unwrap();
            println!(
                "{name}: a run of {} with limit {limit}: {took} T-states, answer {want}",
                run.len()
            );
            if values[0] == 1 {
                total_ones += took;
            } else {
                total_none += took;
            }
            limit = want;
        }
    }
    println!("375 tests, no hit: {total_ones} T-states; none is 1: {total_none}");
    for (what, got, measured) in [
        ("every assertion 1, three runs", total_ones, 259_854u32),
        ("no assertion 1, three runs", total_none, 107_015),
    ] {
        let (low, high) = (measured - measured / 50, measured + measured / 100);
        assert!(
            (low..=high).contains(&got),
            "{what}: {got} T-states, measured {measured}"
        );
    }
}
