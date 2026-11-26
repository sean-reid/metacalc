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

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_find_calls_cost_over_ring_is_printed() {
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
        .map(|c| vec![c]);
    println!("first label {first:?}, last label {last:?}, a letter with no label {absent:?}");
    let cut = |l: &[u8]| l[..l.len().min(8)].to_vec();
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("s".into(), b"s".to_vec()),
        ("sg".into(), b"sg".to_vec()),
        ("a whole 8 character label".into(), eight.clone()),
        ("the first label".into(), cut(&first)),
        ("the last label".into(), cut(&last)),
        ("below the first label".into(), vec![b'-' - 1]),
        ("above the last label".into(), vec![b'~']),
    ];
    if let Some(letter) = absent {
        cases.push(("a letter no label starts with".into(), letter));
    }
    for (what, prefix) in &cases {
        let (took, got) = range_cost(&mut rig, &model, prefix);
        println!("range, {what} ({:?}): {took} T-states, {got:?}", String::from_utf8_lossy(prefix));
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
        "range that ends in no match: worst {worst_none} T-states at {:?}",
        String::from_utf8_lossy(&none_at)
    );
    let s = model.range(b"s").unwrap();
    let df = model.range(b"df").unwrap();
    assert_eq!((s.1, df.1), (375, 192));
    for (what, range, start, bound, kind) in [
        ("s, start 0, bound ring's end, FFh", s, 0, n, EVERY),
        ("s, start 0, bound ring's end, theorems", s, 0, n, THEOREM),
        ("s, start 0, bound ring's end, axioms", s, 0, n, AXIOM),
        ("df, start 0, bound ring's end, theorems", df, 0, n, THEOREM),
        ("s, start 0, bound 779, FFh", s, 0, 779, EVERY),
        ("s, start 0, bound 779, axioms", s, 0, 779, AXIOM),
        ("s, start 0, bound 1171, FFh", s, 0, 1171, EVERY),
        ("s, start 0, bound 1171, axioms", s, 0, 1171, AXIOM),
        ("df, start 0, bound 779, theorems", df, 0, 779, THEOREM),
        ("df, start 0, bound 1171, theorems", df, 0, 1171, THEOREM),
    ] {
        let took = walk_cost(&mut rig, &model, range, start, bound, kind);
        let (_, reads) = model.walk(range, start, bound, kind).unwrap();
        println!("walk {what}: {took} T-states, {reads} bitmap reads");
    }
    let mut copied = 0u64;
    let (mut first, mut count) = (s.0, s.1);
    while count > 0 {
        rig.poke(COPY_AT, &[MARK; 0x100]);
        let regs = copy_regs(first, count, COPY_AT);
        let base = rig.call("CALL_NOP", regs).tstates;
        let out = rig.call(COPY, regs);
        assert_eq!(out.status(), ST_OK);
        println!("copy of {} entries: {} T-states", out.hl, out.tstates - base);
        copied += u64::from(out.tstates - base);
        first += out.hl;
        count -= out.hl;
    }
    println!("copy of s in its runs: {copied} T-states");
}
