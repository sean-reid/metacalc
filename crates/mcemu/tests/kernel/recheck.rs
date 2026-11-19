use crate::support::*;
use mcpack::library::{CACHE_IDS, Kernel, KernelError, WORK_LIMIT};
use mcpack::mm::Database;

/// Filler axioms that put the first heavy axiom, `P1`, at id `CACHE_IDS - 1` and `P2` at `CACHE_IDS`.
const FILLERS: usize = CACHE_IDS - 4;

/// What `check_record` does on `P1` and `P2` with `vars` variables, counted from the source:
/// the pairs, then a unit per hypothesis (`vars` floats and one essential), the three symbols of
/// the essential hypothesis and the `vars + 1` symbols of the conclusion.
fn record_units(vars: usize) -> u32 {
    (vars * (vars - 1) / 2 + vars + 1 + 3 + vars + 1) as u32
}

/// The pieces of a library around the heavy axioms: `declared` variables with floats (at least
/// `vars`), `low` for the ids just below `P1` and `tail` after everything else.
struct Shape<'a> {
    vars: usize,
    declared: usize,
    ks: &'a [usize],
    low: String,
    low_items: usize,
    tail: String,
}

impl<'a> Shape<'a> {
    fn new(vars: usize, ks: &'a [usize]) -> Self {
        Shape {
            vars,
            declared: vars,
            ks,
            low: String::new(),
            low_items: 0,
            tail: String::new(),
        }
    }
}

/// `E` for each float and the essential hypothesis, then `p`: an apply of a heavy axiom whose
/// variables are all bound to nothing.
fn apply_of(p: &str, vars: usize) -> String {
    format!("{}E {p} ", "E ".repeat(vars))
}

/// A record with `floats` variables, `m` essential hypotheses of `len` symbols and a conclusion of
/// `conc`, every symbol past the typecode a variable, so each binds to nothing; applied by
/// `E` for each float and hypothesis.
fn long_record(name: &str, floats: usize, m: usize, len: usize, conc: usize) -> String {
    let expr = |len: usize| {
        format!(
            "wff {}",
            (0..len - 1)
                .map(|i| format!("v{}", i % floats))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let hyps: String = (0..m)
        .map(|i| format!("{name}.{i} $e {} $. ", expr(len)))
        .collect();
    format!("${{ {hyps}{name} $a {} $. $}}\n", expr(conc))
}

fn long_units(floats: usize, m: usize, len: usize, conc: usize) -> u32 {
    (floats + m * (1 + len) + conc) as u32
}

/// A theorem of `wff` that names `n` variables only as dummies, each bound by `Wk` to nothing,
/// and declares them all disjoint: its pairs are all extra ones.
fn extra_record(name: &str, n: usize) -> String {
    let vars: Vec<String> = (0..n).map(|i| format!("v{i}")).collect();
    let step = |v: &String| format!("f{v} f{v} Wk ");
    let proof = format!(
        "{}{}",
        step(&vars[0]),
        vars[1..]
            .iter()
            .map(|v| format!("{}J ", step(v)))
            .collect::<String>()
    );
    format!(
        "${{ $d {} $. {name} $p wff $= {proof}$. $}}\n",
        vars.join(" ")
    )
}

fn extra_units(n: usize) -> u32 {
    (n * (n - 1) / 2 + 1) as u32
}

/// Axioms `E` and `J` (a join), `FILLERS` axioms with `low` in the last of them, then `P1` and
/// `P2` with the same shape: `vars` pairwise disjoint variables, one essential hypothesis.
/// Theorems `lo{k}` and `hi{k}` prove `wff` by `k` applies of `P1` and `P2`, joined; every
/// variable is bound to nothing.
fn recheck_source(shape: &Shape) -> String {
    let names: Vec<String> = (0..shape.declared).map(|i| format!("v{i}")).collect();
    let all = names[..shape.vars].join(" ");
    let mut src = format!("$c wff $. $v {} $.\nE $a wff $.\n", names.join(" "));
    src += "${ J.1 $e wff $. J.2 $e wff $. J $a wff $. $}\n";
    for v in &names {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    src += "${ Wk.1 $e wff v0 $. Wk $a wff $. $}\n";
    for i in 0..FILLERS - shape.low_items {
        src += &format!("n{i} $a wff $.\n");
    }
    src += &shape.low;
    for p in ["P1", "P2"] {
        src += &format!("${{ $d {all} $. {p}.1 $e wff v0 v1 $. {p} $a wff {all} $. $}}\n");
    }
    for &k in shape.ks {
        for (name, p) in [("lo", "P1"), ("hi", "P2")] {
            let one = apply_of(p, shape.vars);
            let proof = format!("{one}{}", format!("{one}J ").repeat(k - 1));
            src += &format!("{name}{k} $p wff $= {proof}$.\n");
        }
    }
    src + &shape.tail
}

struct Library {
    db: Database,
    bytes: Vec<u8>,
}

fn library(vars: usize, ks: &[usize]) -> Library {
    library_of(&Shape::new(vars, ks))
}

fn library_of(shape: &Shape) -> Library {
    let db = parse(&recheck_source(shape));
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

/// Measured at 286.1; the window is 20 percent either side.
const T_STATES_PER_UNIT: std::ops::RangeInclusive<f64> = 229.0..=343.0;

#[test]
fn the_record_check_takes_about_286_t_states_for_each_unit_an_apply_charges_for_it() {
    let vars = 100;
    let lib = library(vars, &[1, 3]);
    let run = |label: &str| {
        let got = lib.device(label);
        assert_eq!(got.0, 0, "{label}");
        got
    };
    let (hi1, hi3, lo1, lo3) = (run("hi1"), run("hi3"), run("lo1"), run("lo3"));
    let units = record_units(vars);
    assert_eq!(hi3.1 - lo3.1, 3 * units, "the units the device charges");
    assert_eq!(hi1.1 - lo1.1, units);
    // Each extra apply costs the same on either side, and past 2000h it also repeats the check.
    let check = f64::from((hi3.2 - hi1.2) - (lo3.2 - lo1.2)) / 2.0;
    let per_unit = check / f64::from(units);
    println!("record check: {units} units, {check} T-states, {per_unit:.1} per unit");
    assert!(T_STATES_PER_UNIT.contains(&per_unit), "{per_unit}");
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
    let ks: Vec<usize> = (1..=30).collect();
    let lib = library(vars, &ks);
    let fits = limit_in_model(&lib, &ks, "hi");
    assert!(fits < 30 && fits > 1, "{fits}");
    assert_eq!(
        limit_in_model(&lib, &ks, "lo"),
        30,
        "no apply below the boundary hits the limit"
    );
    let most = lib.model("lo30").1;
    let margin = f64::from(WORK_LIMIT - most) / f64::from(WORK_LIMIT);
    println!(
        "30 applies below 2000h: {most} units, {:.0} percent under the limit",
        100.0 * margin
    );
    assert!(
        margin >= 0.10,
        "{most} units is within 10 percent of the limit"
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

/// Charges `lo` below and `hi` past 2000h from records of one shape, as device counts, model
/// counts and the units counted from the source.
fn charges_exactly(lib: &Library, lo: &str, hi: &str, units: u32) {
    let (lo_model, hi_model) = (lib.model(lo), lib.model(hi));
    assert_eq!((lo_model.0, hi_model.0), (Ok(()), Ok(())), "{hi}");
    let (lo_dev, hi_dev) = (lib.device(lo), lib.device(hi));
    assert_eq!((lo_dev.0, hi_dev.0), (0, 0), "{hi}");
    assert_eq!(
        (lo_dev.1, hi_dev.1),
        (lo_model.1, hi_model.1),
        "{hi}: device and model"
    );
    assert_eq!(hi_dev.1 - lo_dev.1, units, "{hi}: units from the source");
}

#[test]
fn the_extra_pairs_of_a_theorem_past_2000h_are_charged() {
    let n = 20;
    let mut shape = Shape::new(30, &[1]);
    shape.declared = n.max(30);
    shape.low = extra_record("exlo", n);
    shape.low_items = 1;
    shape.tail =
        extra_record("exhi", n) + "use-exlo $p wff $= exlo $.\nuse-exhi $p wff $= exhi $.\n";
    let lib = library_of(&shape);
    assert!(lib.id("exlo") < CACHE_IDS && lib.id("exhi") > CACHE_IDS);
    charges_exactly(&lib, "use-exlo", "use-exhi", extra_units(n));
}

#[test]
fn a_charge_of_hypotheses_and_symbols_past_255_and_past_511_carries() {
    let (a, b) = ((3, 2, 100, 100), (3, 4, 120, 120));
    let mut shape = Shape::new(30, &[1]);
    let mut tail = String::new();
    for (name, (floats, m, len, conc)) in [("q1", a), ("q2", b)] {
        shape.low += &long_record(&format!("{name}lo"), floats, m, len, conc);
        tail += &long_record(&format!("{name}hi"), floats, m, len, conc);
        for side in ["lo", "hi"] {
            let es = "E ".repeat(floats + m);
            tail += &format!("use-{name}{side} $p wff $= {es}{name}{side} $.\n");
        }
    }
    shape.low_items = 2;
    shape.tail = tail;
    let lib = library_of(&shape);
    for (name, (floats, m, len, conc)) in [("q1", a), ("q2", b)] {
        let units = long_units(floats, m, len, conc);
        println!("{name}: {units} units");
        charges_exactly(
            &lib,
            &format!("use-{name}lo"),
            &format!("use-{name}hi"),
            units,
        );
    }
    assert_eq!(long_units(3, 2, 100, 100), 305);
    assert_eq!(long_units(3, 4, 120, 120), 607);
}

/// P2, and a bad copy of it at a higher id: 120 variables, so its charge is more than what 21
/// applies of P2 leave of the limit.
fn order_library(break_record: bool) -> Library {
    let heavy = 120;
    let names: Vec<String> = (0..heavy).map(|i| format!("v{i}")).collect();
    let all = names.join(" ");
    let prefix = format!(
        "{}{}",
        apply_of("P2", 100),
        format!("{}J ", apply_of("P2", 100)).repeat(20)
    );
    let mut shape = Shape::new(100, &[21]);
    shape.declared = heavy;
    shape.tail = format!(
        "${{ $d {all} $. PB.1 $e wff v0 v1 $. PB $a wff {all} $. $}}\n\
         ord-spent $p wff $= {prefix}{}J $.\n\
         ord-fresh $p wff $= {}$.\n\
         under-spent $p wff $= {prefix}PB $.\n\
         under-fresh $p wff $= PB $.\n",
        apply_of("PB", heavy),
        apply_of("PB", heavy),
    );
    let mut lib = library_of(&shape);
    if break_record {
        let at = fields(&lib.bytes, lib.id("PB")).dv;
        lib.bytes[at + 1] = lib.bytes[at];
        reseal(&mut lib.bytes);
    }
    lib
}

#[test]
fn the_record_charge_comes_before_the_record_check_and_before_the_stack_check() {
    use KernelError::{BadRecord, StackUnderflow, WorkLimit};
    let (good, broken) = (order_library(false), order_library(true));
    assert!(good.id("PB") > CACHE_IDS);
    let left = WORK_LIMIT - good.model("hi21").1;
    assert!(left < record_units(120), "{left} units left");
    for (lib, label, want) in [
        (&good, "ord-spent", Err(WorkLimit)),
        (&good, "ord-fresh", Ok(())),
        (&good, "under-spent", Err(WorkLimit)),
        (&good, "under-fresh", Err(StackUnderflow)),
        (&broken, "ord-spent", Err(WorkLimit)),
        (&broken, "ord-fresh", Err(BadRecord)),
        (&broken, "under-spent", Err(WorkLimit)),
        (&broken, "under-fresh", Err(BadRecord)),
    ] {
        assert_eq!(lib.model(label).0, want, "{label}: the model");
        assert_eq!(lib.device(label).0, status(want), "{label}: the device");
    }
}
