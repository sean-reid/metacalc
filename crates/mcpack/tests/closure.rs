use mcpack::measure::measure;
use mcpack::mm::{self, Database, ProofError, StmtId, Usage, Verifier, closure};
use std::process::Command;

const DB: &str = "
$c ( ) -> wff |- $.
$v p q r $.
wp $f wff p $.
wq $f wff q $.
wr $f wff r $.
wi $a wff ( p -> q ) $.
ax-1 $a |- ( p -> ( q -> p ) ) $.
ax-2 $a |- ( ( p -> ( q -> r ) ) -> ( ( p -> q ) -> ( p -> r ) ) ) $.
${
  min $e |- p $.
  maj $e |- ( p -> q ) $.
  ax-mp $a |- q $.
$}
${
  a1i.1 $e |- p $.
  a1i $p |- ( q -> p ) $= ( wi ax-1 ax-mp ) ABADCABEF $.
$}
th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABC $.
${
  use.1 $e |- r $.
  use $p |- ( q -> ( p -> r ) ) $= ( wi a1i ) ACEBCADFF $.
$}
bad $p |- ( p -> q ) $= ( ax-1 ) ABC $.
";

fn db() -> Database {
    mm::parse(DB.as_bytes().to_vec()).expect("database parses")
}

fn ids(db: &Database, labels: &[&str]) -> Vec<StmtId> {
    labels
        .iter()
        .map(|label| db.find(label).expect("label exists"))
        .collect()
}

fn labels(db: &Database, ids: &[StmtId]) -> Vec<String> {
    ids.iter().map(|&id| db.stmt(id).label.clone()).collect()
}

#[test]
fn closure_follows_proofs_and_hypotheses_in_database_order() {
    let db = db();
    let got = closure(&db, &ids(&db, &["use"]));
    assert_eq!(
        labels(&db, &got),
        [
            "wp", "wq", "wr", "wi", "ax-1", "min", "maj", "ax-mp", "a1i.1", "a1i", "use.1", "use"
        ]
    );
}

#[test]
fn closure_leaves_out_what_the_roots_do_not_use() {
    let db = db();
    let got = closure(&db, &ids(&db, &["th", "th"]));
    assert_eq!(labels(&db, &got), ["wp", "wq", "ax-1", "th"]);
}

#[test]
fn usage_reports_the_high_water_marks_of_a_proof() {
    let db = db();
    let mut verifier = Verifier::new(&db);
    assert_eq!(verifier.verify(db.find("th").unwrap()), Ok(()));
    assert_eq!(
        verifier.usage(),
        Usage {
            steps: 3,
            stack_depth: 2,
            stack_syms: 14,
            saved_syms: 0,
        }
    );
}

#[test]
fn measure_counts_each_segment_and_checks_the_proofs() {
    let db = db();
    let all = closure(&db, &ids(&db, &["use", "th"]));
    let report = measure(&db, &all, &ids(&db, &["a1i.1"]));
    assert!(report.failures.is_empty());
    let [first, second] = &report.segments[..] else {
        panic!("expected two segments");
    };
    assert_eq!(first.name, "start");
    assert_eq!(
        (first.axioms, first.theorems, first.floats, first.essentials),
        (3, 0, 3, 2)
    );
    assert_eq!(second.name, "a1i.1");
    assert_eq!(
        (
            second.axioms,
            second.theorems,
            second.floats,
            second.essentials
        ),
        (0, 3, 0, 2)
    );
    assert_eq!((second.proof_refs, second.proof_len), (6, 21));
    assert_eq!(report.total().theorems, 3);
    assert_eq!((report.constants, report.variables), (5, 3));
    assert_eq!(report.longest_statement.label, "ax-1");
}

#[test]
fn measure_reports_a_proof_that_fails() {
    let db = db();
    let report = measure(&db, &closure(&db, &ids(&db, &["bad"])), &[]);
    assert_eq!(
        report.failures,
        [("bad".to_string(), ProofError::WrongConclusion)]
    );
}

fn mcpack(args: &[&str]) -> (Option<i32>, String) {
    let corpus = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/demo0.mm");
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("closure")
        .arg(corpus)
        .args(args)
        .output()
        .expect("mcpack runs");
    (
        out.status.code(),
        String::from_utf8(out.stdout).expect("output is text"),
    )
}

#[test]
fn closure_command_prints_a_row_per_segment() {
    let (code, out) = mcpack(&["th1", "--split", "mp"]);
    assert_eq!(code, Some(0));
    let rows: Vec<&str> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(rows[..4], ["from", "start", "mp", "total"]);
    assert!(out.contains("symbols: "), "{out}");
}

#[test]
fn closure_command_refuses_an_unknown_label() {
    let (code, out) = mcpack(&["nope"]);
    assert_eq!(code, Some(2));
    assert!(out.is_empty());
}
