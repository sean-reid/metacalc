use crate::support::*;
use mcpack::library::{CACHE_IDS, Kernel, KernelError, WORK_LIMIT};
use mcpack::mm::Database;

/// Filler axioms that put the first heavy axiom, `P1`, at id `CACHE_IDS - 1` and `P2` at `CACHE_IDS`.
const FILLERS: usize = CACHE_IDS - 3;

/// What `check_record` does on `P1` and `P2` with `vars` variables, counted from the source:
/// the pairs, then a unit per hypothesis (`vars` floats and one essential), the three symbols of
/// the essential hypothesis and the `vars + 1` symbols of the conclusion.
fn record_units(vars: usize) -> u32 {
    (vars * (vars - 1) / 2 + vars + 1 + 3 + vars + 1) as u32
}

/// Axioms `E` and `J` (a join), `FILLERS` axioms, then `P1` and `P2` with the same shape: `vars`
/// pairwise disjoint variables, one essential hypothesis. Theorems `lo{k}` and `hi{k}` prove
/// `wff` by `k` applies of `P1` and `P2`, joined; every variable is bound to nothing.
fn recheck_source(vars: usize, ks: &[usize]) -> String {
    let names: Vec<String> = (0..vars).map(|i| format!("v{i}")).collect();
    let all = names.join(" ");
    let mut src = format!("$c wff $. $v {all} $.\nE $a wff $.\n");
    src += "${ J.1 $e wff $. J.2 $e wff $. J $a wff $. $}\n";
    for v in &names {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    for i in 0..FILLERS {
        src += &format!("n{i} $a wff $.\n");
    }
    for p in ["P1", "P2"] {
        src += &format!("${{ $d {all} $. {p}.1 $e wff v0 v1 $. {p} $a wff {all} $. $}}\n");
    }
    let apply = |p: &str| format!("{}E {p} ", "E ".repeat(vars));
    for &k in ks {
        for (name, p) in [("lo", "P1"), ("hi", "P2")] {
            let proof = format!("{}{}", apply(p), format!("{}J ", apply(p)).repeat(k - 1));
            src += &format!("{name}{k} $p wff $= {proof}$.\n");
        }
    }
    src
}

struct Library {
    db: Database,
    bytes: Vec<u8>,
}

fn library(vars: usize, ks: &[usize]) -> Library {
    let db = parse(&recheck_source(vars, ks));
    let bytes = pack_all(&db);
    Library { db, bytes }
}

impl Library {
    fn id(&self, label: &str) -> usize {
        number(&self.db, label)
    }

    /// The model's verdict and count for `label` on a fresh start.
    fn model(&self, label: &str) -> (Result<(), KernelError>, u32) {
        let mut kernel = Kernel::new(&self.bytes, clamped(ASKED_UNBOUNDED)).unwrap();
        let result = kernel.verify(self.id(label));
        (result, kernel.work())
    }

    /// The device's status, count and T-states for `label` on a fresh start.
    fn device(&self, label: &str) -> (u8, u32, u32) {
        let mut device = Device::with_library(&self.bytes);
        let got = device.verify(self.id(label), ASKED_UNBOUNDED);
        (got.verify, device.work_charged(), got.tstates)
    }
}

#[test]
fn the_kernel_and_the_model_share_one_cache_boundary() {
    assert_eq!(constant("CACHE_IDS") as usize, CACHE_IDS);
}

#[test]
fn an_apply_charges_the_record_check_from_id_2000h_and_not_below() {
    let vars = 30;
    let lib = library(vars, &[1]);
    assert_eq!(lib.id("P1"), CACHE_IDS - 1);
    assert_eq!(lib.id("P2"), CACHE_IDS);
    let (lo_model, hi_model) = (lib.model("lo1"), lib.model("hi1"));
    assert_eq!(lo_model.0, Ok(()));
    assert_eq!(hi_model.0, Ok(()));
    let (lo, hi) = (lib.device("lo1"), lib.device("hi1"));
    assert_eq!((lo.0, hi.0), (0, 0));
    assert_eq!(
        lo.1, lo_model.1,
        "below the boundary, the device against the model"
    );
    assert_eq!(
        hi.1, hi_model.1,
        "at the boundary, the device against the model"
    );
    let units = record_units(vars);
    assert_eq!(hi_model.1 - lo_model.1, units, "the model's charge");
    assert_eq!(hi.1 - lo.1, units, "the device's charge");
    println!(
        "record of {vars} variables: {units} units, work {} below and {} at 2000h",
        lo.1, hi.1
    );
}

#[test]
fn the_record_charge_costs_about_as_many_t_states_a_unit_as_the_charges_beside_it() {
    let vars = 100;
    let lib = library(vars, &[1, 3]);
    let t_states = |label: &str| {
        let got = lib.device(label);
        assert_eq!(got.0, 0, "{label}");
        got.2
    };
    // Each extra apply below the boundary costs the same as one past it, less its record check.
    let checks = (t_states("hi3") - t_states("hi1")) - (t_states("lo3") - t_states("lo1"));
    let check = f64::from(checks) / 2.0;
    let units = f64::from(record_units(vars));
    let per_unit = check / units;
    println!("record check: {units} units, {check} T-states, {per_unit:.1} per unit");
    assert!((12.0..=2232.0).contains(&per_unit), "{per_unit}");
}

/// The most applies of the heavy record that the limit allows, by the model, at each side.
fn limit_in_model(lib: &Library, ks: &[usize], side: &str) -> usize {
    ks.iter()
        .copied()
        .take_while(|k| lib.model(&format!("{side}{k}")).0 == Ok(()))
        .last()
        .expect("one apply fits")
}

#[test]
fn applies_past_2000h_stop_at_the_work_limit_in_the_step_the_record_charge_crosses_it() {
    let vars = 100;
    let ks: Vec<usize> = (1..=40).collect();
    let lib = library(vars, &ks);
    let fits = limit_in_model(&lib, &ks, "hi");
    assert!(fits < 40 && fits > 1, "{fits}");
    assert_eq!(
        limit_in_model(&lib, &ks, "lo"),
        40,
        "no apply below the boundary hits the limit"
    );
    let (hi, over) = (format!("hi{fits}"), format!("hi{}", fits + 1));
    let (lo_over, lo_fits) = (format!("lo{}", fits + 1), format!("lo{fits}"));
    assert_eq!(lib.model(&hi).0, Ok(()));
    assert_eq!(lib.model(&over).0, Err(KernelError::WorkLimit));
    let (fit, refused) = (lib.device(&hi), lib.device(&over));
    assert_eq!(fit.0, status(Ok(())), "the device accepts {fits} applies");
    assert_eq!(fit.1, lib.model(&hi).1, "and counts what the model counts");
    assert_eq!(
        refused.0,
        status(Err(KernelError::WorkLimit)),
        "and refuses one more"
    );
    let (low_fit, low_over) = (lib.device(&lo_fits), lib.device(&lo_over));
    assert_eq!(
        low_over.0,
        status(Ok(())),
        "the same proof below 2000h passes"
    );
    assert_eq!(low_over.1, lib.model(&lo_over).1);
    let units = record_units(vars);
    assert_eq!(fit.1 - low_fit.1, fits as u32 * units);
    assert!(low_over.1 + (fits as u32 + 1) * units > WORK_LIMIT);
    assert!(fit.1 <= WORK_LIMIT && WORK_LIMIT - fit.1 < low_over.1 - low_fit.1 + units);
    println!(
        "{fits} applies fit: {} units, {} below 2000h; {} do not: {} units below",
        fit.1,
        low_fit.1,
        fits + 1,
        low_over.1
    );
}

#[test]
fn the_work_of_a_proof_is_the_same_on_every_run_and_init_past_2000h_and_below() {
    let lib = library(30, &[3]);
    let axiom = 0;
    for label in ["hi3", "lo3"] {
        let (id, (result, want)) = (lib.id(label), lib.model(label));
        assert_eq!(result, Ok(()), "{label}");
        // An axiom has no work to do, so the count of the call before it stays in the kernel.
        let mut device = Device::with_library(&lib.bytes);
        assert_eq!(
            device.verify_two(id, axiom, ASKED_UNBOUNDED),
            (0, 0, 0),
            "{label}"
        );
        assert_eq!(
            device.work_charged(),
            want,
            "{label}: the first call of an init"
        );
        let mut device = Device::with_library(&lib.bytes);
        assert_eq!(
            device.verify_two(id, id, ASKED_UNBOUNDED),
            (0, 0, 0),
            "{label}"
        );
        assert_eq!(
            device.work_charged(),
            want,
            "{label}: the second call of an init"
        );
        let mut device = Device::with_library(&lib.bytes);
        device.verify(id, ASKED_UNBOUNDED);
        assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, 0, "{label}");
        assert_eq!(
            device.work_charged(),
            want,
            "{label}: after another kernel_init"
        );
    }
}
