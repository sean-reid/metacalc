use crate::cost::{measure, report};
use crate::host::*;
use crate::libs::shipped;
use crate::rig::*;
use crate::shape::*;
use crate::slices::RECOMMENDED;
use crate::support::*;
use mcpack::library::decode;

/// A figure within 2% above and 5% below the one measured: the slow path must be the one taken.
fn near(what: &str, got: u32, measured: u32) {
    assert!(
        got <= measured + measured / 50 && got >= measured - measured / 20,
        "{what}: {got} against {measured} measured"
    );
}

/// The most T-states of one start and of one continue over every theorem of ring at `budget`,
/// the driver taken off, and how many calls that took.
fn worst_slices(bytes: &[u8], budget: u16) -> (u32, u32, usize) {
    let library = decode(bytes).unwrap();
    let tc = provable(bytes).unwrap();
    let mut rig = opened(bytes);
    let base = rig.call("CALL_NOP", Regs::default()).tstates;
    let (mut start, mut more, mut calls) = (0, 0, 0);
    for (n, a) in library
        .assertions
        .iter()
        .enumerate()
        .filter(|(_, a)| a.theorem)
    {
        let mut out = rig.call(SHAPE, regs_with(n as u16, tc, budget));
        if out.status() == SYNTAX {
            continue;
        }
        start = start.max(out.tstates - base);
        while out.status() == MORE {
            out = rig.more(budget);
            more = more.max(out.tstates - base);
            calls += 1;
        }
        assert_eq!(out.status(), ST_OK, "{}", a.label);
    }
    (start, more, calls)
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_a_unit_a_slice_and_a_find_cost_on_ring() {
    let bytes = shipped("ring");
    let rows = measure(&bytes);
    let (median, worst) = report("ring", &rows);
    println!("uncut walk: median {median}, worst {worst}");
    near("uncut median", median, 69_951);
    near("uncut worst", worst, 1_175_175);
    for (budget, start_at, more_at) in [(1, 48_596, 18_124), (RECOMMENDED, 107_321, 103_968)] {
        let (start, more, calls) = worst_slices(&bytes, budget);
        println!("budget {budget}: worst start {start}, worst continue {more}, {calls} continues");
        near(&format!("worst start at {budget}"), start, start_at);
        near(&format!("worst continue at {budget}"), more, more_at);
        assert!(calls > 0);
    }
    let library = decode(&bytes).unwrap();
    let lib = model_of("ring", &bytes);
    let tc = provable(&bytes).unwrap();
    let mut rig = opened(&bytes);
    let base = rig.call("CALL_NOP", Regs::default()).tstates;
    let (mut biggest, mut deepest) = ((0, 0), (0, 0));
    for (n, a) in library
        .assertions
        .iter()
        .enumerate()
        .filter(|(_, a)| a.theorem)
    {
        if let Some(m) = mirror(&library, &bytes, n) {
            if m.total > biggest.1 {
                biggest = (n, m.total);
            }
            let mut h = vec![1usize; m.rows.len()];
            for (g, r) in m.rows.iter().enumerate() {
                for &c in &r.subgoals {
                    h[g] = h[g].max(1 + h[usize::from(c)]);
                }
            }
            if h[h.len() - 1] > deepest.1 {
                deepest = (n, h[h.len() - 1]);
            }
        }
        let _ = a;
    }
    for (what, n) in [("the largest total", biggest.0), ("the deepest", deepest.0)] {
        let (z, goals, pairs) = walked_with_tree(&mut rig, &lib, &library, &bytes, tc, n);
        let mut worst = (0, 0u32);
        let mut deepest_seen = 0;
        for place in 1..=z.total {
            rig.seed_find();
            let out = rig.find(place);
            assert_eq!(out.status(), ST_OK);
            let f = found(&rig.find_bytes());
            deepest_seen = deepest_seen.max(f.depth);
            if out.tstates - base > worst.1 {
                worst = (place, out.tstates - base);
            }
        }
        let _ = (&goals, &pairs);
        near(
            what,
            worst.1,
            if what == "the deepest" {
                45_821
            } else {
                26_499
            },
        );
        println!(
            "{what} ({}): total {}, goals {}, deepest place seen at depth {deepest_seen}, worst find {} T-states at place {}, last place {}",
            library.assertions[n].label,
            z.total,
            z.rows.len(),
            worst.1,
            worst.0,
            {
                rig.seed_find();
                rig.find(z.total).tstates - base
            }
        );
    }
}
