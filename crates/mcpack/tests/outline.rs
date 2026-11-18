use std::path::Path;
use std::process::{Command, Output};

const LOGIC: &str = "
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
  a1i $p |- ( q -> p ) $= wp wq wp wi a1i.1 wp wq ax-1 ax-mp $.
$}
${
  a2i.1 $e |- ( p -> ( q -> r ) ) $.
  a2i $p |- ( ( p -> q ) -> ( p -> r ) ) $=
    wp wq wr wi wi wp wq wi wp wr wi wi a2i.1 wp wq wr ax-2 ax-mp $.
$}
${
  mpd.1 $e |- ( p -> q ) $.
  mpd.2 $e |- ( p -> ( q -> r ) ) $.
  mpd $p |- ( p -> r ) $=
    wp wq wi wp wr wi mpd.1 wp wq wr mpd.2 a2i ax-mp $.
$}
${
  twice.1 $e |- p $.
  twice $p |- ( p -> p ) $= ( wi a1i ax-mp ) AACAACAABDZAACAACFDE $.
  twice-n $p |- ( p -> p ) $=
    wp wp wi wp wp wi wp wp twice.1 a1i wp wp wi wp wp wi wp wp twice.1 a1i a1i ax-mp $.
$}
plain $p |- ( p -> ( q -> p ) ) $= wp wq ax-1 $.
${
  foo.1 $e |- p $.
  foo $p |- p $= foo.1 $.
$}
bad $p |- ( p -> p ) $= wp wq ax-1 $.
syntax-reuse $p |- ( ( p -> p ) -> ( ( p -> p ) -> ( p -> p ) ) ) $= ( wi ax-1 ) AABZDC $.
syntax $p wff ( p -> p ) $= wp wp wi $.
";

fn outline(dir: &str, label: &str) -> Output {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(dir);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let database = dir.join("logic.mm");
    std::fs::write(&database, LOGIC).unwrap();
    Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("outline")
        .arg(&database)
        .arg(label)
        .output()
        .expect("mcpack runs")
}

fn stdout(label: &str) -> String {
    let out = outline(&format!("outline-ok-{label}"), label);
    assert_eq!(out.status.code(), Some(0), "{label}");
    assert_eq!(out.stderr, b"");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn nested_essential_hypotheses_indent_under_their_parent() {
    assert_eq!(
        stdout("mpd"),
        "\
|- ( p -> r )  [ax-mp]
  |- ( p -> q )  [mpd.1]
  |- ( ( p -> q ) -> ( p -> r ) )  [a2i]
    |- ( p -> ( q -> r ) )  [mpd.2]
"
    );
}

const TWICE: &str = "\
|- ( p -> p )  [ax-mp]
  |- ( p -> p )  [a1i]
    |- p  [twice.1]
  |- ( ( p -> p ) -> ( p -> p ) )  [a1i]
    |- ( p -> p )  [a1i]
      |- p  [twice.1]
";

#[test]
fn a_saved_step_appears_at_every_place_it_is_used() {
    assert_eq!(stdout("twice"), TWICE);
}

#[test]
fn compressed_and_normal_proofs_give_the_same_tree() {
    assert_eq!(stdout("twice-n"), TWICE);
}

#[test]
fn a_theorem_applying_one_axiom_has_a_single_node() {
    assert_eq!(stdout("plain"), "|- ( p -> ( q -> p ) )  [ax-1]\n");
}

#[test]
fn an_unknown_label_exits_2() {
    let out = outline("outline-unknown", "nope");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"no statement is labelled nope\n");
}

#[test]
fn an_axiom_exits_1() {
    let out = outline("outline-axiom", "ax-1");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"ax-1: it is not a theorem\n");
}

#[test]
fn a_syntax_theorem_exits_1() {
    let out = outline("outline-syntax", "syntax");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        out.stderr,
        b"syntax: it does not conclude a provable statement\n"
    );
}

#[test]
fn a_proof_ending_in_a_hypothesis_prints_the_hypothesis() {
    assert_eq!(stdout("foo"), "|- p  [foo.1]\n");
}

#[test]
fn a_proof_that_does_not_verify_exits_1_with_its_error() {
    let out = outline("outline-bad", "bad");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        out.stderr,
        b"bad: its proof does not verify: WrongConclusion\n"
    );
}

#[test]
fn a_reused_syntax_step_leaves_no_trace_in_the_tree() {
    assert_eq!(
        stdout("syntax-reuse"),
        "|- ( ( p -> p ) -> ( ( p -> p ) -> ( p -> p ) ) )  [ax-1]\n"
    );
}
