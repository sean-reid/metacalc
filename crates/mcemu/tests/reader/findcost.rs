use crate::findrig::*;
use crate::support::*;

fn shipped(name: &str) -> (Model, Vec<u8>) {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/{name}.mclib")).unwrap();
    let library = decoded(&bytes);
    let labels: Vec<String> = library.assertions.iter().map(|a| a.label.clone()).collect();
    let bits: Vec<bool> = library.assertions.iter().map(|a| a.theorem).collect();
    (Model::new(&labels, Some(bits)), bytes)
}

fn range_cost(rig: &mut Rig, model: &Model, prefix: &[u8]) -> (u32, (u16, u16)) {
    rig.poke(PREFIX_AT, prefix);
    let regs = range_regs(PREFIX_AT, prefix.len() as u8);
    let base = rig.call("CALL_NOP", regs).tstates;
    let out = rig.call(RANGE, regs);
    let want = model.range(prefix).unwrap();
    assert_eq!((out.status(), out.hl, out.bc), (ST_OK, want.0, want.1));
    (out.tstates - base, want)
}

fn walk_cost(rig: &mut Rig, model: &Model, range: (u16, u16), s: u16, b: u16, kind: u8) -> u32 {
    rig.poke(BLOCK_AT, &block(range.0, range.1, s, b, kind));
    let regs = walk_regs(BLOCK_AT);
    let base = rig.call("CALL_NOP", regs).tstates;
    let out = rig.call(WALK, regs);
    let (want, _) = model.walk(range, s, b, kind).unwrap();
    assert_eq!((out.status(), out.hl), (ST_OK, want));
    out.tstates - base
}

/// A measured cost is within 2% under and 1% over what it was when its pin was set.
fn pinned(costs: &[(String, u32, u32)]) {
    for (what, took, measured) in costs {
        let (low, high) = (measured - measured / 50, measured + measured / 100);
        assert!(
            (low..=high).contains(took),
            "{what}: {took} T-states, measured {measured}"
        );
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_find_calls_cost_over_ring_is_printed_and_pinned() {
    let (model, bytes) = shipped("ring");
    let mut rig = opened(&bytes);
    let n = model.len();
    let sorted_label = |i: usize| model.labels[usize::from(model.order[i])].clone();
    let eight = (0..model.order.len())
        .map(sorted_label)
        .find(|l| l.len() == 8)
        .expect("a label of 8 characters");
    let first = sorted_label(0);
    let last = sorted_label(model.order.len() - 1);
    let absent = (b'a'..=b'z')
        .find(|&c| model.range(&[c]).unwrap().1 == 0)
        .expect("a letter no label starts with");
    assert_eq!(
        (&first[..], &last[..], absent),
        (&b"00id"[..], &b"zfpair2"[..], b'k')
    );
    let cut = |l: &[u8]| l[..l.len().min(8)].to_vec();
    let mut costs: Vec<(String, u32, u32)> = Vec::new();
    let cases: Vec<(&str, Vec<u8>, u32)> = vec![
        ("s", b"s".to_vec(), 95_420),
        ("sg", b"sg".to_vec(), 92_275),
        ("a whole 8 character label", eight, 117_261),
        ("the first label", cut(&first), 112_217),
        ("the last label", cut(&last), 58_567),
        ("below the first label", vec![b'-' - 1], 54_427),
        ("above the last label", vec![b'~'], 50_046),
        ("a letter no label starts with", vec![absent], 54_447),
    ];
    for (what, prefix, pin) in &cases {
        let (took, got) = range_cost(&mut rig, &model, prefix);
        println!(
            "range, {what} ({:?}): {took} T-states, {got:?}",
            String::from_utf8_lossy(prefix)
        );
        costs.push((format!("range, {what}"), took, *pin));
    }
    let (mut worst, mut worst_at, mut worst_none, mut none_at) = (0, Vec::new(), 0, Vec::new());
    let (mut total, mut calls) = (0u64, 0u64);
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for label in &model.labels {
        for p in 1..=label.len().min(8) {
            let prefix = label[..p].to_vec();
            if seen.contains(&prefix) {
                continue;
            }
            seen.push(prefix.clone());
            let (took, got) = range_cost(&mut rig, &model, &prefix);
            assert!(got.1 > 0);
            total += u64::from(took);
            calls += 1;
            if took > worst {
                (worst, worst_at) = (took, prefix);
            }
        }
    }
    println!(
        "range over every prefix of 1 to 8 characters of ring's labels: {calls} prefixes, worst {worst} T-states at {:?}, mean {}",
        String::from_utf8_lossy(&worst_at),
        total / calls
    );
    assert_eq!((calls, &worst_at[..]), (6338, &b"eleqtrrd"[..]));
    costs.push(("range, the worst of every prefix".into(), worst, 120_295));
    costs.push((
        "range, the mean of every prefix".into(),
        (total / calls) as u32,
        107_669,
    ));
    for label in model.labels.iter().step_by(7) {
        let mut prefix = label[..label.len().min(8)].to_vec();
        let last = prefix.len() - 1;
        prefix[last] = prefix[last].wrapping_add(1);
        if model.range(&prefix).unwrap().1 > 0 {
            continue;
        }
        let (took, _) = range_cost(&mut rig, &model, &prefix);
        if took > worst_none {
            (worst_none, none_at) = (took, prefix);
        }
    }
    println!(
        "range that ends in no match, over every seventh label with its last byte one up: worst {worst_none} T-states at {:?}",
        String::from_utf8_lossy(&none_at)
    );
    costs.push(("range, no match, the worst".into(), worst_none, 60_693));
    let s = model.range(b"s").unwrap();
    let df = model.range(b"df").unwrap();
    assert_eq!((s.1, df.1), (375, 192));
    for (what, range, bound, kind, reads, pin) in [
        ("s, ring's end, FFh", s, n, EVERY, 0, 172_167),
        ("s, ring's end, theorems", s, n, THEOREM, 8, 183_027),
        ("s, ring's end, axioms", s, n, AXIOM, 375, 690_436),
        ("df, ring's end, theorems", df, n, THEOREM, 111, 241_808),
        ("s, bound 779, FFh", s, 779, EVERY, 0, 172_014),
        ("s, bound 779, axioms", s, 779, AXIOM, 131, 352_984),
        ("s, bound 1171, FFh", s, 1171, EVERY, 0, 172_167),
        ("s, bound 1171, axioms", s, 1171, AXIOM, 189, 433_198),
        ("df, bound 779, theorems", df, 779, THEOREM, 8, 99_359),
        ("df, bound 1171, theorems", df, 1171, THEOREM, 13, 106_274),
    ] {
        let took = walk_cost(&mut rig, &model, range, 0, bound, kind);
        let (_, got_reads) = model.walk(range, 0, bound, kind).unwrap();
        println!("walk {what}, start 0: {took} T-states, {got_reads} bitmap reads");
        assert_eq!(got_reads, reads, "{what}");
        costs.push((format!("walk {what}"), took, pin));
    }
    let every = walk_cost(&mut rig, &model, s, 0, n, EVERY);
    let axioms = walk_cost(&mut rig, &model, s, 0, n, AXIOM);
    let test = (axioms - every) / 375;
    println!("one test of the kinds bitmap inside the walk: {test} T-states");
    costs.push(("one test of the kinds bitmap".into(), test, 1_382));
    let read1 = rig.cost(
        "LIB_READ",
        Regs {
            bc: 1,
            de: BUF,
            ..Regs::default()
        },
    );
    println!("lib_read of one byte, the reader's own: {read1} T-states");
    costs.push(("lib_read of one byte".into(), read1, 1_169));
    let mut copied = 0;
    let (mut first, mut count) = (s.0, s.1);
    let pins = [14_155, 14_179, 13_600];
    let mut run = 0;
    while count > 0 {
        rig.poke(COPY_AT, &[MARK; 0x100]);
        let regs = copy_regs(first, count, COPY_AT);
        let base = rig.call("CALL_NOP", regs).tstates;
        let out = rig.call(COPY, regs);
        assert_eq!(out.status(), ST_OK);
        let took = out.tstates - base;
        println!("copy of {} entries: {took} T-states", out.hl);
        costs.push((format!("copy of {} entries", out.hl), took, pins[run]));
        copied += took;
        first += out.hl;
        count -= out.hl;
        run += 1;
    }
    println!("copy of s in its runs: {copied} T-states");
    costs.push(("copy of s in its runs".into(), copied, 41_934));
    pinned(&costs);
}
