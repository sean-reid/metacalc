use crate::libs::{LIBRARIES, shipped};
use crate::proofs::*;
use crate::rig::*;
use crate::support::*;
use mcpack::library::{Step, decode};

/// What one theorem cost: label, steps, T-states of the walk and of lib_proof alone, the driver
/// taken off.
pub struct Row {
    pub label: String,
    pub steps: usize,
    pub walk: u32,
    pub proof: u32,
    pub status: u8,
}

pub fn measure(bytes: &[u8]) -> Vec<Row> {
    let library = decode(bytes).expect("it decodes");
    let tc = provable(bytes).expect("the provable typecode");
    let mut rig = opened(bytes);
    let base = rig.call("CALL_NOP", Regs::default()).tstates;
    let mut rows = Vec::new();
    for (n, a) in library
        .assertions
        .iter()
        .enumerate()
        .filter(|(_, a)| a.theorem)
    {
        let walk = rig.shape(n as u16, tc);
        let proof = rig.call(
            "LIB_PROOF",
            Regs {
                hl: n as u16,
                ..Regs::default()
            },
        );
        assert_eq!(proof.status(), ST_OK, "{}", a.label);
        rows.push(Row {
            label: a.label.clone(),
            steps: a
                .proof
                .iter()
                .filter(|s| matches!(s, Step::Number(_)))
                .count(),
            walk: walk.tstates - base,
            proof: proof.tstates - base,
            status: walk.status(),
        });
    }
    rows
}

fn at(sorted: &[u32], percent: usize) -> u32 {
    sorted[(sorted.len() - 1) * percent / 100]
}

/// Least squares fit of y on x: slope and intercept.
fn fit(points: &[(f64, f64)]) -> (f64, f64) {
    let n = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let sxx: f64 = points.iter().map(|p| p.0 * p.0).sum();
    let sxy: f64 = points.iter().map(|p| p.0 * p.1).sum();
    let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
    (slope, (sy - slope * sx) / n)
}

pub fn report(name: &str, rows: &[Row]) -> (u32, u32) {
    let mut by_walk: Vec<&Row> = rows.iter().collect();
    by_walk.sort_by_key(|r| r.walk);
    let walks: Vec<u32> = by_walk.iter().map(|r| r.walk).collect();
    let proofs: Vec<u32> = {
        let mut v: Vec<u32> = rows.iter().map(|r| r.proof).collect();
        v.sort_unstable();
        v
    };
    let total: u64 = rows.iter().map(|r| u64::from(r.walk)).sum();
    let steps: u64 = rows.iter().map(|r| r.steps as u64).sum();
    let median = by_walk[(by_walk.len() - 1) / 2];
    let worst = by_walk[by_walk.len() - 1];
    let (slope, intercept) = fit(&rows
        .iter()
        .map(|r| (r.steps as f64, f64::from(r.walk)))
        .collect::<Vec<_>>());
    println!(
        "{name}: {} theorems, walk {total} T-states over {steps} steps, {} a step; fit {slope:.0} a step + {intercept:.0}",
        rows.len(),
        total / steps
    );
    println!(
        "{name}: walk median {} ({} with {} steps), p90 {}, p99 {}, worst {} ({} with {} steps)",
        median.walk,
        median.label,
        median.steps,
        at(&walks, 90),
        at(&walks, 99),
        worst.walk,
        worst.label,
        worst.steps
    );
    println!(
        "{name}: lib_proof median {}, p99 {}, worst {}",
        at(&proofs, 50),
        at(&proofs, 99),
        proofs[proofs.len() - 1]
    );
    (median.walk, worst.walk)
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn what_the_walk_and_lib_proof_cost_on_the_shipped_libraries() {
    for name in LIBRARIES {
        let rows = measure(&shipped(name));
        assert!(rows.iter().all(|r| r.status == ST_OK || r.status == SYNTAX));
        report(name, &rows);
    }
}
