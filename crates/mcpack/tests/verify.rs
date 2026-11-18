use mcpack::mm::{self, ProofError, Verifier};

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
";

fn check(tail: &str, label: &str) -> Result<(), ProofError> {
    let db = mm::parse(format!("{LOGIC}{tail}").into_bytes()).expect("database parses");
    Verifier::new(&db).verify(db.find(label).expect("label exists"))
}

fn parse_error(src: &str) -> String {
    match mm::parse(src.as_bytes().to_vec()) {
        Ok(_) => panic!("database parsed"),
        Err(e) => e.message,
    }
}

fn identity_proof() -> String {
    let pp = "wp wp wi";
    let a = format!("wp {pp} wi");
    let b = format!("wp {pp} wp wi wi");
    let c = format!("{a} {pp} wi");
    let s1 = format!("wp {pp} ax-1");
    let s2 = format!("wp {pp} wp ax-2");
    let s3 = format!("{b} {c} {s1} {s2} ax-mp");
    format!("{a} {pp} wp wp ax-1 {s3} ax-mp")
}

#[test]
fn normal_proof_is_accepted() {
    let tail = format!("id $p |- ( p -> p ) $= {} $.", identity_proof());
    assert_eq!(check(&tail, "id"), Ok(()));
}

#[test]
fn compressed_and_normal_proofs_agree() {
    let tail = "
        th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABC $.
        th-n $p |- ( p -> ( q -> p ) ) $= wp wq ax-1 $.
        w $p wff ( p -> ( p -> p ) ) $= ( wi ) AAABB $.
        w-n $p wff ( p -> ( p -> p ) ) $= wp wp wp wi wi $.
    ";
    for label in ["th", "th-n", "w", "w-n"] {
        assert_eq!(check(tail, label), Ok(()), "{label}");
    }
}

#[test]
fn comment_inside_a_compressed_proof_is_skipped() {
    let tail = "th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) A $( first $) B\n $( second $) C $.";
    assert_eq!(check(tail, "th"), Ok(()));
}

#[test]
fn each_application_rebinds_its_variables() {
    let p = "wp wq wp wi wi";
    let tail = format!(
        "th $p |- ( q -> ( p -> ( q -> p ) ) ) $= {p} wq {p} wi wp wq ax-1 {p} wq ax-1 ax-mp $."
    );
    assert_eq!(check(&tail, "th"), Ok(()));
}

#[test]
fn only_theorems_can_be_verified() {
    assert_eq!(check("", "ax-1"), Err(ProofError::NotATheorem));
    assert_eq!(check("", "wp"), Err(ProofError::NotATheorem));
}

#[test]
fn saved_step_is_reused() {
    let tail = "th $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.";
    assert_eq!(check(tail, "th"), Ok(()));
}

#[test]
fn incomplete_proofs_are_never_accepted() {
    assert_eq!(
        check("th $p |- ( p -> p ) $= ? $.", "th"),
        Err(ProofError::Incomplete)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= wp ? ax-1 $.", "th"),
        Err(ProofError::Incomplete)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) A?C $.", "th"),
        Err(ProofError::Incomplete)
    );
}

#[test]
fn wrong_conclusion_is_rejected() {
    assert_eq!(
        check("th $p |- ( q -> ( p -> q ) ) $= wp wq ax-1 $.", "th"),
        Err(ProofError::WrongConclusion)
    );
}

#[test]
fn leftover_stack_is_rejected() {
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= wp wp wq ax-1 $.", "th"),
        Err(ProofError::StackNotSingleton)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= $.", "th"),
        Err(ProofError::StackNotSingleton)
    );
}

#[test]
fn stack_underflow_is_rejected() {
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= wp ax-1 $.", "th"),
        Err(ProofError::StackUnderflow)
    );
}

#[test]
fn typecode_mismatch_is_rejected() {
    let tail = "th $p |- ( p -> ( q -> p ) ) $= wp wp wq ax-1 ax-1 $.";
    assert_eq!(check(tail, "th"), Err(ProofError::TypecodeMismatch));
}

#[test]
fn essential_hypothesis_mismatch_is_rejected() {
    let tail = "
        ${ h $e |- p $.
           th $p |- q $= wp wq h wq wp ax-1 ax-mp $. $}
    ";
    assert_eq!(check(tail, "th"), Err(ProofError::HypothesisMismatch));
}

#[test]
fn hypothesis_from_a_closed_scope_is_rejected() {
    let tail = "
        ${ h $e |- p $. dummy $a |- p $. $}
        th $p |- p $= h $.
    ";
    assert_eq!(check(tail, "th"), Err(ProofError::InactiveHypothesis));
}

#[test]
fn later_and_unknown_labels_are_rejected() {
    let tail = "
        th $p |- ( p -> ( q -> p ) ) $= wp wq later $.
        later $a |- ( p -> ( q -> p ) ) $.
        th2 $p |- ( p -> ( q -> p ) ) $= wp wq th2 $.
    ";
    assert_eq!(check(tail, "th"), Err(ProofError::UnknownLabel));
    assert_eq!(check(tail, "th2"), Err(ProofError::UnknownLabel));
}

#[test]
fn mandatory_hypothesis_in_compressed_label_list_is_rejected() {
    let tail = "th $p |- ( p -> ( q -> p ) ) $= ( wp ax-1 ) ABD $.";
    assert_eq!(
        check(tail, "th"),
        Err(ProofError::MandatoryHypothesisInLabelList)
    );
}

#[test]
fn bad_back_reference_and_bad_letters_are_rejected() {
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABD $.", "th"),
        Err(ProofError::BadBackReference)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABCU $.", "th"),
        Err(ProofError::MalformedCompressed)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ZABC $.", "th"),
        Err(ProofError::MalformedCompressed)
    );
    assert_eq!(
        check("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABc $.", "th"),
        Err(ProofError::MalformedCompressed)
    );
}

const DISJOINT: &str = "
$c A. = $.
$v x y z $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
${ $d x y $. ax-dv $a |- ( x -> y ) $. $}
";

#[test]
fn disjoint_condition_needs_a_matching_condition_in_the_proof_scope() {
    let ok = "${ $d p q $. th $p |- ( p -> q ) $= wp wq ax-dv $. $}";
    let missing = "th $p |- ( p -> q ) $= wp wq ax-dv $.";
    let other_pair = "${ $d p r $. th $p |- ( p -> q ) $= wp wq ax-dv $. $}";
    assert_eq!(check(&format!("{DISJOINT}{ok}"), "th"), Ok(()));
    assert_eq!(
        check(&format!("{DISJOINT}{missing}"), "th"),
        Err(ProofError::DisjointViolation)
    );
    assert_eq!(
        check(&format!("{DISJOINT}{other_pair}"), "th"),
        Err(ProofError::DisjointViolation)
    );
}

#[test]
fn disjoint_condition_rejects_a_shared_variable() {
    let same = "th $p |- ( p -> p ) $= wp wp ax-dv $.";
    let inside = "${ $d p q $. th $p |- ( ( p -> q ) -> q ) $= wp wq wi wq ax-dv $. $}";
    assert_eq!(
        check(&format!("{DISJOINT}{same}"), "th"),
        Err(ProofError::DisjointViolation)
    );
    assert_eq!(
        check(&format!("{DISJOINT}{inside}"), "th"),
        Err(ProofError::DisjointViolation)
    );
}

#[test]
fn disjoint_condition_from_a_closed_scope_does_not_count() {
    let tail = "
        ${ $d p q $. unrelated $a |- ( p -> q ) $. $}
        th $p |- ( p -> q ) $= wp wq ax-dv $.
    ";
    assert_eq!(
        check(&format!("{DISJOINT}{tail}"), "th"),
        Err(ProofError::DisjointViolation)
    );
}

#[test]
fn optional_disjoint_condition_covers_a_dummy_variable() {
    let axioms = "
        ax-yy $a |- ( y -> y ) $.
        ${ $d x y $. g $e |- ( y -> y ) $. ax-g $a |- x $. $}
    ";
    let with = "${ $d p r $. th $p |- p $= wp wr wr ax-yy ax-g $. $}";
    let without = "th $p |- p $= wp wr wr ax-yy ax-g $.";
    assert_eq!(check(&format!("{DISJOINT}{axioms}{with}"), "th"), Ok(()));
    assert_eq!(
        check(&format!("{DISJOINT}{axioms}{without}"), "th"),
        Err(ProofError::DisjointViolation)
    );
}

#[test]
fn optional_floating_hypothesis_is_allowed_in_the_compressed_label_list() {
    let axioms = "
        ax-yy $a |- ( y -> y ) $.
        ${ $d x y $. g $e |- ( y -> y ) $. ax-g $a |- x $. $}
    ";
    let tail = "${ $d p r $. th $p |- p $= ( wr ax-yy ax-g ) ABBCD $. $}";
    assert_eq!(check(&format!("{DISJOINT}{axioms}{tail}"), "th"), Ok(()));
}

#[test]
fn disjoint_statement_with_three_variables_covers_every_pair() {
    for (a, b, wa, wb) in [
        ("p", "q", "wp", "wq"),
        ("p", "r", "wp", "wr"),
        ("q", "r", "wq", "wr"),
    ] {
        let tail = format!("${{ $d p q r $. th $p |- ( {a} -> {b} ) $= {wa} {wb} ax-dv $. $}}");
        assert_eq!(check(&format!("{DISJOINT}{tail}"), "th"), Ok(()), "{a} {b}");
    }
}

#[test]
fn floating_hypothesis_from_a_closed_scope_is_rejected() {
    let src = "$c a $. ${ $v x $. f $f a x $. $} ax $a a $. th $p a $= f $.";
    let db = mm::parse(src.as_bytes().to_vec()).unwrap();
    assert_eq!(
        Verifier::new(&db).verify(db.find("th").unwrap()),
        Err(ProofError::InactiveHypothesis)
    );
}

fn compressed(letters: &str) -> Result<(), ProofError> {
    check(
        &format!("th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) {letters} $."),
        "th",
    )
}

fn prefix_for(mut n: usize) -> String {
    let mut digits = Vec::new();
    while n > 0 {
        let d = (n - 1) % 5;
        digits.push(b'U' + d as u8);
        n = (n - 1) / 5;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

#[test]
fn save_after_a_pending_prefix_is_rejected() {
    assert_eq!(compressed("ABUZC"), Err(ProofError::MalformedCompressed));
}

#[test]
fn compressed_numbers_that_overflow_are_rejected() {
    assert_eq!(
        compressed(&format!("AB{}C", "Y".repeat(64))),
        Err(ProofError::MalformedCompressed)
    );
    let wraps_on_add = format!("AB{}T", prefix_for(usize::MAX / 20));
    assert_eq!(
        compressed(&wraps_on_add),
        Err(ProofError::MalformedCompressed)
    );
    let wraps_on_prefix = format!("AB{}YA", prefix_for(usize::MAX / 5));
    assert_eq!(
        compressed(&wraps_on_prefix),
        Err(ProofError::MalformedCompressed)
    );
    let largest = format!("AB{}A", prefix_for(usize::MAX / 20));
    assert_eq!(compressed(&largest), Err(ProofError::BadBackReference));
}

#[test]
fn repeated_label_in_the_compressed_label_list_is_rejected() {
    let tail = "th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ax-1 ) ABC $.";
    assert_eq!(check(tail, "th"), Err(ProofError::MalformedCompressed));
}

#[test]
fn mandatory_hypotheses_follow_database_order() {
    let tail = "
        ${ e1 $e |- p $. e2 $e |- q $. both $a |- ( p -> q ) $. $}
        ${ a $e |- q $. b $e |- p $.
           right $p |- ( p -> q ) $= wp wq b a both $.
           wrong $p |- ( p -> q ) $= wp wq a b both $. $}
    ";
    assert_eq!(check(tail, "right"), Ok(()));
    assert_eq!(check(tail, "wrong"), Err(ProofError::HypothesisMismatch));
}

#[test]
fn structural_errors_fail_the_parse() {
    assert!(parse_error("$c a $. ${ $c b $. $}").contains("$c inside"));
    assert!(parse_error("$c a $. $c a $.").contains("already declared"));
    assert!(parse_error("$c a $. $}").contains("without"));
    assert!(parse_error("$c a $. ${").contains("unclosed"));
    assert!(parse_error("$c a $. $( open").contains("unterminated comment"));
    assert!(parse_error("$c a $. $v x $. l $a a x $.").contains("no active $f"));
    assert!(parse_error("$c a $. $v x $. ${ $v y $. $} l $f a y $.").contains("not active"));
    assert!(
        parse_error("$c a $. $v x $. l $f a x $. m $f a x $.").contains("already has an active $f")
    );
    assert!(parse_error("$c a $. $v x $. l $f a x $. l $a a x $.").contains("already used"));
    assert!(parse_error("$c a $. $v x $. a $f a x $.").contains("also a math symbol"));
    assert!(parse_error("$c a $. $v x $. l $f a x $. $c l $.").contains("also a label"));
    assert!(
        parse_error("$c a $. $v x $. l $f a x $. m $a x a $.").contains("start with a constant")
    );
    assert!(parse_error("$c a $. $v x $. $d x x $.").contains("repeated"));
    assert!(parse_error("$c a $. $v x $. $v x $.").contains("already active"));
    assert!(parse_error("$c a $. l $a a b $.").contains("undeclared"));
    assert!(parse_error("$c a $. $[ other.mm $]").contains("inclusion"));
    assert!(parse_error("$c a $. $( outer $( inner $) $)").contains("inside a comment"));
    assert!(
        parse_error("$c a $. $v x y $. ${ $v z $. $} $d x z $.").contains("not an active variable")
    );
}

#[test]
fn variable_can_be_redeclared_after_its_scope_closes() {
    let src = "$c a $. ${ $v x $. f $f a x $. s $a a x $. $} ${ $v x $. g $f a x $. t $a a x $. $}";
    assert!(mm::parse(src.as_bytes().to_vec()).is_ok());
}
