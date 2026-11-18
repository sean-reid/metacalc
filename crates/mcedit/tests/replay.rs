use mcedit::Catalog;
use mcedit::oracle::{Failure, replay, replay_database, verify_spliced};
use mcpack::mm::{self, Grammar};
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
    let catalog = Catalog::new(&db, &grammar);
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
        let thin = name == "ring";
        let check = |n: usize, done: &mcedit::oracle::Replayed| {
            !thin || n.is_multiple_of(5) || done.dummies > 0
        };
        let r = replay_database(&src, |_| true, check, true).unwrap();
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
            "{name}: rebuilt {} verified {} failures {} skipped {} ops {} assigns {} refines {} with dummies {} with dv {} nodes (median, p99, max) {:?} metas {:?} goals {:?} saved bytes {:?} peak {:?} at {} in {:?} (rebuild {:?}, verify {:?})",
            r.rebuilt,
            r.verified,
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
    }
}

#[test]
#[ignore = "needs the exported libraries; run through scripts/replay-libraries.sh"]
fn exported_libraries_replay() {
    libraries(&["prop", "pred", "ring"]);
}
