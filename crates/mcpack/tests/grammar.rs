use mcpack::mm::{self, Database, Grammar, GrammarError, StmtId, SyntaxError};
use std::path::{Path, PathBuf};
use std::process::Command;

const DB: &str = "
$c ( ) -> <- = T wff class setvar |- $.
$v p q x y A B $.
wp $f wff p $.
wq $f wff q $.
vx $f setvar x $.
vy $f setvar y $.
cA $f class A $.
cB $f class B $.
wi $a wff ( p -> q ) $.
wrev $a wff ( q <- p ) $.
cv $a class x $.
wceq $a wff A = B $.
wtru $a wff T $.
ax-rev $a |- ( ( p -> q ) <- x = y ) $.
${
  inf.1 $e |- ( p -> T ) $.
  inf.2 $e |- x = A $.
  inf $a |- q $.
$}
wth $p wff ( T -> p ) $= wtru wp wi $.
";

fn db(tail: &str) -> Database {
    mm::parse(format!("{DB}{tail}").into_bytes()).expect("database parses")
}

fn grammar(db: &Database) -> Grammar<'_> {
    Grammar::new(db, &[("|-", "wff")]).expect("grammar builds")
}

fn rpn(db: &Database, tree: &mm::Tree) -> String {
    let mut steps: Vec<StmtId> = Vec::new();
    tree.rpn(&mut steps);
    let labels: Vec<&str> = steps.iter().map(|&s| db.stmt(s).label.as_str()).collect();
    labels.join(" ")
}

#[test]
fn children_follow_hypothesis_order_not_reading_order() {
    let db = db("");
    let grammar = grammar(&db);
    let id = db.find("ax-rev").unwrap();
    let parsed = grammar.parse(id).expect("statement parses");
    assert!(parsed.hyps.is_empty());
    assert_eq!(
        rpn(&db, &parsed.conclusion),
        "vx cv vy cv wceq wp wq wi wrev"
    );
    assert_eq!(grammar.expr(&parsed.conclusion)[1..], db.stmt(id).expr[1..]);
}

#[test]
fn essential_hypotheses_parse_with_the_assertion() {
    let db = db("");
    let parsed = grammar(&db)
        .parse(db.find("inf").unwrap())
        .expect("statement parses");
    let hyps: Vec<String> = parsed.hyps.iter().map(|tree| rpn(&db, tree)).collect();
    assert_eq!(hyps, ["wp wtru wi", "vx cv cA wceq"]);
    assert_eq!(rpn(&db, &parsed.conclusion), "wq");
}

#[test]
fn a_syntax_theorem_parses_as_its_own_typecode() {
    let db = db("");
    let parsed = grammar(&db)
        .parse(db.find("wth").unwrap())
        .expect("statement parses");
    assert_eq!(rpn(&db, &parsed.conclusion), "wtru wp wi");
}

fn parse_error(tail: &str, label: &str) -> SyntaxError {
    let db = db(tail);
    grammar(&db)
        .parse(db.find(label).unwrap())
        .expect_err("statement is refused")
}

#[test]
fn a_statement_outside_the_grammar_is_refused() {
    for body in ["( p -> )", "p q", "x", "( p -> q ) T"] {
        let tail = format!("odd $a |- {body} $.");
        assert_eq!(parse_error(&tail, "odd"), SyntaxError::NoParse, "{body}");
    }
}

#[test]
fn a_statement_with_two_readings_is_refused() {
    let tail = "wi2 $a wff ( q -> p ) $. two $a |- ( p -> T ) $.";
    assert_eq!(parse_error(tail, "two"), SyntaxError::Ambiguous);
}

#[test]
fn two_readings_of_a_subexpression_reach_the_whole_statement() {
    let tail = "wtru2 $a wff T $. inner $a |- ( ( p -> T ) <- q ) $.";
    assert_eq!(parse_error(tail, "inner"), SyntaxError::Ambiguous);
}

fn grammar_error(tail: &str, provable: &[(&str, &str)]) -> GrammarError {
    let db = db(tail);
    match Grammar::new(&db, provable) {
        Ok(_) => panic!("grammar built"),
        Err(e) => e,
    }
}

#[test]
fn unusable_syntax_axioms_are_named() {
    let wff = [("|-", "wff")];
    for (tail, error) in [
        (
            "bare $a wff p -> q $.",
            GrammarError::LeftRecursion("bare".into()),
        ),
        (
            "empty $a wff $.",
            GrammarError::UnsupportedRule("empty".into()),
        ),
        (
            "dup $a wff ( p = p ) $.",
            GrammarError::UnsupportedRule("dup".into()),
        ),
        (
            "${ cond.1 $e |- p $. cond $a wff ( p ) $. $}",
            GrammarError::UnsupportedRule("cond".into()),
        ),
    ] {
        assert_eq!(grammar_error(tail, &wff), error, "{tail}");
    }
    assert_eq!(
        grammar_error("", &[("|-", "formula")]),
        GrammarError::UnknownSymbol("formula".into())
    );
}

#[test]
fn left_recursion_through_another_typecode_is_found() {
    let tail = "
        $c term $.
        $v t $.
        tt $f term t $.
        tw $a term p = $.
        wt $a wff t <- $.
    ";
    assert!(matches!(
        grammar_error(tail, &[("|-", "wff")]),
        GrammarError::LeftRecursion(_)
    ));
}

fn mcpack(command: &str, database: &Path, roots: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg(command)
        .arg(database)
        .args(roots)
        .output()
        .expect("mcpack runs");
    let text = |bytes| String::from_utf8(bytes).expect("output is text");
    (out.status.code(), text(out.stdout), text(out.stderr))
}

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name)
}

fn unreadable_database(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let src = format!("{DB} odd $a |- ( p -> ) $. uses $p |- ( p -> ) $= wp odd $.");
    std::fs::write(&path, src).expect("database is written");
    path
}

#[test]
fn parse_command_counts_the_assertions_it_read() {
    let (code, out, _) = mcpack("parse", &corpus("demo0.mm"), &[]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "8 assertions parsed, 0 failed\n")
    );
}

#[test]
fn parse_command_names_the_axiom_that_breaks_the_grammar() {
    let (code, out, err) = mcpack("parse", &corpus("miu.mm"), &[]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.contains("UnsupportedRule(\"we\")"), "{err}");
}

#[test]
fn parse_command_names_each_statement_it_cannot_read() {
    let (code, out, err) = mcpack("parse", &unreadable_database("parse.mm"), &[]);
    assert_eq!(
        (code, out.as_str()),
        (Some(1), "10 assertions parsed, 2 failed\n")
    );
    assert_eq!(err, "odd: NoParse\nuses: NoParse\n");
}

#[test]
fn closure_command_names_the_statement_it_cannot_read() {
    let (code, out, err) = mcpack("closure", &unreadable_database("closure.mm"), &["uses"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.ends_with(": NoParse\n"), "{err}");
}
