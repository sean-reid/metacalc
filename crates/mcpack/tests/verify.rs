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
fn compressed_proof_matches_normal_proof() {
    let tail = "
        th $p |- ( p -> ( q -> p ) ) $= ( ax-1 ) ABC $.
        th2 $p wff ( p -> ( p -> p ) ) $= ( wi ) AAABZBC $.
    ";
    assert_eq!(check(tail, "th"), Ok(()));
    assert_eq!(check(tail, "th2"), Err(ProofError::StackNotSingleton));
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
}

#[test]
fn variable_can_be_redeclared_after_its_scope_closes() {
    let src = "$c a $. ${ $v x $. f $f a x $. s $a a x $. $} ${ $v x $. g $f a x $. t $a a x $. $}";
    assert!(mm::parse(src.as_bytes().to_vec()).is_ok());
}
