use crate::fit::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcpack::library::Library;
use mcui::canvas::{CONTENT_ROWS, STATEMENT_WIDTH};

/// Milliseconds at 6 MHz.
fn ms(tstates: u32) -> f64 {
    f64::from(tstates) / 6000.0
}

/// T-states ceilings, a little above what the pass measured: the median, the 99th percentile and
/// the worst fm_fit over every assertion, the tables built by the call, and the worst with the
/// tables already built.
pub struct Ceilings {
    pub median: u32,
    pub p99: u32,
    pub worst: u32,
    pub warm_worst: u32,
}

pub struct Costs {
    pub median: u32,
    pub p99: u32,
    pub worst: (u32, String),
    pub warm_worst: (u32, String),
}

/// fm_fit of every assertion of `library` at six rows and the screen's width: with the node tables
/// still to build, and again with them built. Refused assertions are left out.
pub fn costs(name: &str, bytes: &[u8]) -> Costs {
    let library = decoded(bytes);
    let mut fm = Fm::new(bytes);
    let zero = fm.rig.call("CALL_NOP", Regs::default()).tstates;
    let (mut cold, mut warm) = (Vec::new(), Vec::new());
    for n in 0..library.assertions.len() as u16 {
        if fm.assertion(n).status() != 0 {
            continue;
        }
        let hyps = essentials(&library, n);
        let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
        let label = library.assertions[usize::from(n)].label.clone();
        let out = fm.fit(CONTENT_ROWS, hyp, concl);
        if out.status() != 0 {
            continue;
        }
        cold.push((out.tstates - zero, label.clone()));
        let again = fm.fit(CONTENT_ROWS, hyp, concl);
        assert_eq!(again.status(), 0);
        warm.push((again.tstates - zero, label));
    }
    cold.sort();
    warm.sort();
    let at =
        |all: &[(u32, String)], per_cent: usize| all[(all.len() * per_cent).div_ceil(100) - 1].0;
    let costs = Costs {
        median: at(&cold, 50),
        p99: at(&cold, 99),
        worst: cold.last().cloned().unwrap(),
        warm_worst: warm.last().cloned().unwrap(),
    };
    println!(
        "{name}: fm_fit over {} assertions: median {} ({:.1} ms), 99th percentile {} ({:.1} ms), worst {} ({:.1} ms) on {}; with the tables built, median {}, worst {} on {}",
        cold.len(),
        costs.median,
        ms(costs.median),
        costs.p99,
        ms(costs.p99),
        costs.worst.0,
        ms(costs.worst.0),
        costs.worst.1,
        at(&warm, 50),
        costs.warm_worst.0,
        costs.warm_worst.1,
    );
    costs
}

fn within(name: &str, got: &Costs, ceilings: &Ceilings) {
    for (what, value, ceiling) in [
        ("median", got.median, ceilings.median),
        ("99th percentile", got.p99, ceilings.p99),
        ("worst", got.worst.0, ceilings.worst),
        (
            "worst with the tables built",
            got.warm_worst.0,
            ceilings.warm_worst,
        ),
    ] {
        assert!(
            value <= ceiling,
            "{name}: fm_fit {what} took {value}, ceiling {ceiling}"
        );
    }
}

const FIXTURE: Ceilings = Ceilings {
    median: 22000,
    p99: 6330000,
    worst: 28700000,
    warm_worst: 27800000,
};

#[test]
fn fm_fit_costs_over_every_fixture_assertion_stay_under_their_ceilings() {
    let bytes = fixture::nodes_library();
    within("fixture", &costs("fixture", &bytes), &FIXTURE);
}

/// What a pass over a shipped library found: how many assertions the model calls too large, and
/// that fm refuses exactly those.
fn library_pass(name: &str, bytes: &[u8], frames_every: usize) -> Tally {
    let model = Model::new(bytes);
    let library: Library = decoded(bytes);
    let mut fm = Fm::new(bytes);
    let mut tally = Tally::default();
    for n in 0..library.assertions.len() {
        let check = if n % frames_every == 0 { FULL } else { FLAGS };
        compare(
            &mut fm,
            &model,
            &library,
            (n as u16, CONTENT_ROWS, STATEMENT_WIDTH),
            check,
            &mut tally,
        );
    }
    println!(
        "{name}: {} assertions, {} too large for the model and refused by fm ({} of them at fm_assertion), {} unfolded, {} fitted, {} alone, {} folded nodes, {} frames",
        tally.assertions,
        tally.too_large,
        tally.refused_at_load,
        tally.unfolded,
        tally.fitted,
        tally.alone,
        tally.folded_nodes,
        tally.frames
    );
    tally
}

const SHIPPED: [(&str, Ceilings, usize, (usize, usize)); 3] = [
    (
        "prop",
        Ceilings {
            median: 90000,
            p99: 222000,
            worst: 402000,
            warm_worst: 58500,
        },
        5,
        (3, 1),
    ),
    (
        "pred",
        Ceilings {
            median: 87500,
            p99: 226000,
            worst: 540000,
            warm_worst: 58500,
        },
        10,
        (7, 3),
    ),
    (
        "ring",
        Ceilings {
            median: 80000,
            p99: 437000,
            worst: 1340000,
            warm_worst: 505000,
        },
        25,
        (300, 40),
    ),
];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_fit_as_the_model_does_and_within_the_budget() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for (name, ceilings, frames_every, (nodes, fitted)) in SHIPPED {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let tally = library_pass(name, &bytes, frames_every);
        assert!(
            tally.folded_nodes >= nodes && tally.fitted >= fitted && tally.frames > 20,
            "{name}: {tally:?}"
        );
        let got = costs(name, &bytes);
        if name == "ring" {
            assert!(got.worst.0 <= 3_000_000, "ring worst {}", got.worst.0);
            assert!(got.median < 300_000, "ring median {}", got.median);
        }
        within(name, &got, &ceilings);
    }
}
