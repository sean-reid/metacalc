use mcpack::library;
use mcpack::measure::measure;
use mcpack::mm::{self, Database, Grammar, Kind, ProofError, StmtId, SyntaxError, Usage, Verifier};
use std::path::Path;
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
twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
broken $p |- p $= nope $.
plain $p |- ( p -> ( q -> p ) ) $= wp wq ax-1 $.
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

fn closure(db: &Database, roots: &[&str]) -> Vec<StmtId> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).expect("grammar builds");
    mm::closure(db, &grammar, &ids(db, roots)).expect("statements parse")
}

fn labels(db: &Database, ids: &[StmtId]) -> Vec<String> {
    ids.iter().map(|&id| db.stmt(id).label.clone()).collect()
}

#[test]
fn closure_follows_proofs_and_hypotheses_in_database_order() {
    let db = db();
    let got = closure(&db, &["use"]);
    assert_eq!(
        labels(&db, &got),
        [
            "wp", "wq", "wr", "wi", "ax-1", "min", "maj", "ax-mp", "a1i.1", "a1i", "use.1", "use"
        ]
    );
}

#[test]
fn closure_keeps_syntax_axioms_no_proof_step_names() {
    let src = "
        $c wff |- T $.
        wtru $a wff T $.
        tru $a |- T $.
        tru2 $p |- T $= tru $.
    ";
    let db = mm::parse(src.as_bytes().to_vec()).expect("database parses");
    let got = closure(&db, &["tru2"]);
    assert_eq!(labels(&db, &got), ["wtru", "tru", "tru2"]);
}

#[test]
fn closure_stops_at_a_statement_the_grammar_cannot_read() {
    let src = "
        $c wff |- T F $.
        wtru $a wff T $.
        fal $a |- F $.
        fal2 $p |- T $= fal $.
    ";
    let db = mm::parse(src.as_bytes().to_vec()).expect("database parses");
    let grammar = Grammar::new(&db, &[("|-", "wff")]).expect("grammar builds");
    let got = mm::closure(&db, &grammar, &ids(&db, &["fal2"]));
    assert_eq!(got, Err((db.find("fal").unwrap(), SyntaxError::NoParse)));
}

#[test]
fn closure_keeps_syntax_axioms_only_an_essential_hypothesis_needs() {
    let src = "
        $c wff |- T ~ $.
        $v p $.
        wp $f wff p $.
        wtru $a wff T $.
        wnot $a wff ~ p $.
        ax $a |- T $.
        ${
          th.1 $e |- ~ T $.
          th $p |- T $= ax $.
        $}
    ";
    let db = mm::parse(src.as_bytes().to_vec()).expect("database parses");
    let got = closure(&db, &["th"]);
    assert_eq!(
        labels(&db, &got),
        ["wp", "wtru", "wnot", "ax", "th.1", "th"]
    );
    let bytes = library::pack(&db, &got).expect("the closure packs");
    let text = library::export(&bytes).expect("the library exports");
    let exported = mm::parse(text.into_bytes()).expect("the export parses");
    let th = exported.find("th").expect("th is exported");
    assert_eq!(Verifier::new(&exported).verify(th), Ok(()));
    let grammar = Grammar::new(&exported, &[("|-", "wff")]).expect("the export has a grammar");
    for id in 0..exported.stmts.len() as StmtId {
        if matches!(exported.stmt(id).kind, Kind::Axiom | Kind::Theorem) {
            let label = &exported.stmt(id).label;
            grammar
                .parse(id)
                .unwrap_or_else(|e| panic!("{label}: {e:?}"));
        }
    }
}

#[test]
fn closure_leaves_out_what_the_roots_do_not_use() {
    let db = db();
    let got = closure(&db, &["th", "th"]);
    assert_eq!(labels(&db, &got), ["wp", "wq", "wi", "ax-1", "th"]);
}

#[test]
fn usage_reports_the_high_water_marks_of_a_proof() {
    let usage = Usage {
        steps: 3,
        stack_depth: 2,
        stack_syms: 14,
        saved_syms: 0,
    };
    assert_eq!(usage_of("th"), (Ok(()), usage));
}

fn usage_of(label: &str) -> (Result<(), ProofError>, Usage) {
    let db = db();
    let mut verifier = Verifier::new(&db);
    let result = verifier.verify(db.find(label).unwrap());
    (result, verifier.usage())
}

#[test]
fn usage_counts_the_scratch_of_an_essential_hypothesis() {
    let usage = Usage {
        steps: 9,
        stack_depth: 5,
        stack_syms: 30,
        saved_syms: 0,
    };
    assert_eq!(usage_of("a1i"), (Ok(()), usage));
}

#[test]
fn usage_counts_saved_steps() {
    let usage = Usage {
        steps: 5,
        stack_depth: 2,
        stack_syms: 26,
        saved_syms: 6,
    };
    assert_eq!(usage_of("twice"), (Ok(()), usage));
}

#[test]
fn usage_is_empty_for_a_proof_that_never_ran() {
    let db = db();
    let mut verifier = Verifier::new(&db);
    assert_eq!(verifier.verify(db.find("th").unwrap()), Ok(()));
    assert_eq!(
        verifier.verify(db.find("broken").unwrap()),
        Err(ProofError::UnknownLabel)
    );
    assert_eq!(verifier.usage(), Usage::default());
}

#[test]
fn measure_counts_each_segment_and_checks_the_proofs() {
    let db = db();
    let all = closure(&db, &["use", "th"]);
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
fn measure_orders_splits_and_ignores_repeats() {
    let db = db();
    let all = closure(&db, &["use", "th"]);
    let report = measure(&db, &all, &ids(&db, &["a1i.1", "ax-1", "a1i.1"]));
    let rows: Vec<(&str, usize)> = report
        .segments
        .iter()
        .map(|s| (s.name.as_str(), s.axioms))
        .collect();
    assert_eq!(rows, [("start", 1), ("ax-1", 2), ("a1i.1", 0)]);
}

#[test]
fn measure_counts_an_uncompressed_proof_as_length_only() {
    let db = db();
    let report = measure(&db, &closure(&db, &["plain"]), &[]);
    let total = report.total();
    assert_eq!((total.proof_refs, total.proof_len, total.steps), (0, 3, 3));
}

#[test]
fn measure_reports_a_proof_that_fails() {
    let db = db();
    let report = measure(&db, &closure(&db, &["bad"]), &[]);
    assert_eq!(
        report.failures,
        [("bad".to_string(), ProofError::WrongConclusion)]
    );
}

fn mcpack(database: &str, args: &[&str]) -> (Option<i32>, String) {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("closure")
        .arg(corpus.join(database))
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
    let (code, out) = mcpack("demo0.mm", &["th1", "--split", "mp"]);
    assert_eq!(code, Some(0));
    let rows: Vec<&str> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(rows[..4], ["from", "start", "mp", "total"]);
    assert!(out.contains("symbols: "), "{out}");
}

#[test]
fn closure_command_fails_when_a_kept_proof_fails() {
    let (code, out) = mcpack("demo0-bad1.mm", &["th1"]);
    assert_eq!(code, Some(1));
    assert!(out.contains("symbols: "), "{out}");
}

#[test]
fn closure_command_refuses_what_it_cannot_find() {
    for (database, args) in [
        ("demo0.mm", &["nope"][..]),
        ("demo0.mm", &["th1", "--split", "nope"]),
        ("missing.mm", &["th1"]),
    ] {
        let (code, out) = mcpack(database, args);
        assert_eq!(code, Some(2), "{database} {args:?}");
        assert!(out.is_empty());
    }
}
