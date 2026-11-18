use mcedit::Catalog;
use mcedit::oracle::{Failure, replay, replay_as_new, replay_database, splice, verify_spliced};
use mcpack::library;
use mcpack::mm::{self, Grammar, Verifier};
use std::cell::RefCell;
use std::path::Path;
use std::time::Instant;

fn corpus(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcpack/tests/corpus")
        .join(name);
    std::fs::read_to_string(path).expect("corpus file reads")
}

#[test]
fn corpus_theorems_replay_into_proofs_the_verifier_accepts() {
    for (name, theorems) in [("demo0.mm", 1), ("big-unifier.mm", 2)] {
        let report = replay_database(&corpus(name), |_| true, |_, _| true, false).unwrap();
        assert_eq!(report.failures, [], "{name}");
        assert_eq!(
            (report.rebuilt, report.verified),
            (theorems, theorems),
            "{name}"
        );
    }
}

#[test]
fn corpus_databases_the_grammar_cannot_read_are_refused_up_front() {
    for name in ["miu.mm", "anatomy.mm"] {
        let report = replay_database(&corpus(name), |_| true, |_, _| true, false);
        assert!(report.is_err(), "{name}");
    }
}

#[test]
fn a_damaged_export_is_rejected_by_the_verifier() {
    let src = corpus("demo0.mm");
    let db = mm::parse(src.clone().into_bytes()).unwrap();
    let grammar = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let catalog = Catalog::new(&db, &grammar, 0);
    let theorem = db.find("th1").unwrap();
    let mut done = replay(&catalog, theorem, false).unwrap();
    assert_eq!(verify_spliced(&src, &db, &done.finished), Ok(()));
    done.finished.proof.pop();
    assert!(matches!(
        verify_spliced(&src, &db, &done.finished),
        Err(Failure::Rejected(_))
    ));
}

fn libraries(names: &[&str]) {
    let dir =
        std::env::var("MCEDIT_LIBRARIES").expect("MCEDIT_LIBRARIES names the exported libraries");
    for &name in names {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mm")).unwrap();
        let started = Instant::now();
        let counts = format!("{dir}/{name}.proofs");
        let want: usize = std::fs::read_to_string(&counts)
            .unwrap_or_else(|e| panic!("{counts} is written by scripts/build-libraries.sh: {e}"))
            .trim()
            .parse()
            .unwrap_or_else(|e| {
                panic!("{counts} is written by scripts/build-libraries.sh and holds no count: {e}")
            });
        let db = mm::parse(src.clone().into_bytes()).unwrap();
        let exports = RefCell::new(Vec::new());
        let r = replay_database(
            &src,
            |_| true,
            |_, done| {
                let label = db.stmt(done.finished.theorem).label.clone();
                exports
                    .borrow_mut()
                    .push((label, done.finished.statement(&db)));
                true
            },
            true,
        )
        .unwrap();
        if let Ok(out) = std::env::var("MCEDIT_SPLICED") {
            let mut all = src.clone();
            for (label, statement) in exports.into_inner() {
                all = splice(&all, &label, &statement)
                    .unwrap_or_else(|e| panic!("{name} {label}: {e:?}"));
            }
            std::fs::write(format!("{out}/{name}.mm"), all).unwrap();
        }
        let median = |mut v: Vec<usize>| {
            v.sort_unstable();
            (v[v.len() / 2], v[v.len() * 99 / 100], v[v.len() - 1])
        };
        let (nodes, metas, goals) = (
            median(r.samples.iter().map(|s| s.terms.nodes).collect()),
            median(r.samples.iter().map(|s| s.terms.metas).collect()),
            median(r.samples.iter().map(|s| s.goals).collect()),
        );
        let bytes = median(r.saved.clone());
        println!(
            "{name}: verified {} of {want} rebuilt {} failures {} skipped {} ops {} assigns {} refines {} with dummies {} with dv {} nodes (median, p99, max) {:?} metas {:?} goals {:?} saved bytes {:?} peak {:?} at {} in {:?} (rebuild {:?}, verify {:?})",
            r.verified,
            r.rebuilt,
            r.failures.len(),
            r.skipped,
            r.ops,
            r.assigns,
            r.refines,
            r.with_dummies,
            r.with_dv,
            nodes,
            metas,
            goals,
            bytes,
            r.peak,
            r.peak_theorem,
            started.elapsed(),
            r.rebuild_time,
            r.verify_time
        );
        let strict = replay_database(&src, |_| true, |_, _| false, false).unwrap();
        let needing: Vec<&str> = strict.failures.iter().map(|f| f.0.as_str()).collect();
        println!("  without refine, {} need it: {needing:?}", needing.len());
        for (label, why) in &r.failures {
            println!("  {label}: {why:?}");
        }
        assert!(r.failures.is_empty(), "{name}");
        assert_eq!(r.skipped, 0, "{name}");
        assert_eq!((r.rebuilt, r.verified), (want, want), "{name}");
    }
}

#[test]
#[ignore = "needs the exported libraries; run through scripts/replay-libraries.sh"]
fn exported_libraries_replay() {
    libraries(&["prop", "pred", "ring"]);
}

const SMALL: &str = "
$c ( ) -> wff |- A. setvar $.
$v ph ps x y z $.
wph $f wff ph $.
wps $f wff ps $.
vx $f setvar x $.
vy $f setvar y $.
wal $a wff A. x ph $.
${
  gen.1 $e |- ph $.
  ax-gen $a |- A. x ph $.
$}
${
  $d x ph $.
  free.1 $e |- A. x ph $.
  ax-free $a |- ph $.
$}
${
  $d ps z $.
  th.1 $e |- ps $.
  vz $f setvar z $.
  th $p |- ps $= wps vz wps vz th.1 ax-gen ax-free $.
$}
";

fn small_library() -> String {
    let db = mm::parse(SMALL.as_bytes().to_vec()).unwrap();
    let everything: Vec<u32> = (0..db.stmts.len() as u32).collect();
    let bytes = library::pack_kernel(&db, &everything).unwrap();
    library::check(&bytes).expect("the kernel accepts the library");
    library::export(&bytes).unwrap()
}

/// Whether every dummy of the rebuilt proof is floated at the top of the database, in scope for any theorem.
fn dummies_are_global(done: &mcedit::oracle::Replayed, rebuilt: &mm::Database) -> bool {
    done.finished.dummies.iter().all(|d| {
        let float = rebuilt.find(&d.label).map(|id| rebuilt.stmt(id));
        !d.declared && float.is_some_and(|f| f.scope_end == mm::StmtId::MAX)
    })
}

/// The database `rebuilt` repacked as a library, which the kernel model checks.
fn kernel_accepts(rebuilt: &mm::Database) -> Result<(), library::CheckError> {
    let everything: Vec<u32> = (0..rebuilt.stmts.len() as u32).collect();
    library::check(&library::pack_kernel(rebuilt, &everything).unwrap()).map(drop)
}

#[test]
fn a_question_mark_theorem_is_rebuilt_with_the_dummy_its_proof_names() {
    let text = small_library();
    assert!(text.contains("\nf.4 $f setvar z $.\n"), "{text}");
    let (done, rebuilt) = replay_as_new(&text, "th", false).unwrap();
    assert_eq!(done.dummies, 1);
    assert_eq!(done.assigns, 1);
    assert!(dummies_are_global(&done, &rebuilt));
    let id = rebuilt.find("th").unwrap();
    assert_eq!(Verifier::new(&rebuilt).verify(id), Ok(()));
    assert_eq!(kernel_accepts(&rebuilt), Ok(()));
}

/// The theorems the editor could not rebuild while a dummy's float came after the theorem.
const LATE_FLOATS: [&str; 7] = ["ax7", "cbvaev", "aev", "oprabidw", "tfrlem5", "1re", "00id"];

#[test]
#[ignore = "needs the exported libraries; run through scripts/require-tests.sh"]
fn theorems_with_a_late_float_are_rebuilt_as_new_proofs() {
    let dir =
        std::env::var("MCEDIT_LIBRARIES").expect("MCEDIT_LIBRARIES names the exported libraries");
    let mut rebuilt_labels = Vec::new();
    for name in ["pred", "ring"] {
        let src = std::fs::read_to_string(format!("{dir}/{name}.mm")).unwrap();
        let db = mm::parse(src.clone().into_bytes()).unwrap();
        for label in LATE_FLOATS.iter().filter(|l| db.find(l).is_some()) {
            let (done, rebuilt) = replay_as_new(&src, label, true)
                .unwrap_or_else(|e| panic!("{name} {label}: {e:?}"));
            assert!(done.dummies > 0, "{name} {label} needs no dummy");
            assert!(dummies_are_global(&done, &rebuilt), "{name} {label}");
            let id = rebuilt.find(label).unwrap();
            assert_eq!(Verifier::new(&rebuilt).verify(id), Ok(()), "{name} {label}");
            assert_eq!(kernel_accepts(&rebuilt), Ok(()), "{name} {label}");
            println!("{name} {label}: rebuilt with {} dummies", done.dummies);
            rebuilt_labels.push(*label);
        }
    }
    for label in LATE_FLOATS {
        assert!(rebuilt_labels.contains(&label), "{label} was not rebuilt");
    }
}
