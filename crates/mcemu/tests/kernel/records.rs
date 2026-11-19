use crate::raw;
use crate::support::*;
use mcpack::library::{KernelError, decode};

#[test]
fn the_kernel_accepts_and_refuses_the_same_records_as_the_model() {
    for case in raw::record_cases() {
        for (layout, image) in LAYOUTS {
            let what = format!("{}, {layout} layout", case.name);
            agree_in(image, &case.bytes, 0, ASKED_UNBOUNDED, case.expected, &what);
        }
    }
}

const ORDERED_BY_TWO_RECORDS: &str = "
$c |- T $.
$v r q p $.
${
  a.q $f T q $.
  a.r $f T r $.
  ax-1 $a |- q r $.
$}
${
  b.p $f T p $.
  b.q $f T q $.
  ax-2 $a |- p q $.
$}
";

const ESSENTIAL_FIRST: &str = "
$c |- T $.
$v x y $.
vx $f T x $.
${
  early.1 $e |- x $.
  early.y $f T y $.
  ax-early $a |- y $.
$}
${
  late.y $f T y $.
  late.1 $e |- x $.
  ax-late $a |- y $.
$}
";

fn expect(src: &str, cases: &[(&str, Result<(), KernelError>)]) {
    let db = parse(src);
    let bytes = pack_all(&db);
    for (label, expected) in cases {
        agree(&bytes, number(&db, label), *expected, label);
    }
}

#[test]
fn records_whose_floats_only_one_numbering_orders_are_accepted_on_both_sides() {
    expect(
        ORDERED_BY_TWO_RECORDS,
        &[("ax-1", Ok(())), ("ax-2", Ok(()))],
    );
}

#[test]
fn an_essential_hypothesis_before_a_float_is_refused_on_both_sides() {
    expect(
        ESSENTIAL_FIRST,
        &[
            ("ax-late", Ok(())),
            ("ax-early", Err(KernelError::BadRecord)),
        ],
    );
}

#[test]
fn variables_are_numbered_to_put_every_records_floats_in_rising_order() {
    let bytes = pack_all(&parse(ORDERED_BY_TWO_RECORDS));
    let names: Vec<String> = decode(&bytes)
        .unwrap()
        .variables
        .into_iter()
        .map(|v| v.name)
        .collect();
    assert_eq!(names, ["p", "q", "r"]);
}

#[test]
fn a_record_with_falling_ids_is_refused_and_the_same_record_with_rising_ids_is_accepted() {
    use raw::Hyp::Float as F;
    let t = 0;
    let record = |floats: &[raw::Hyp]| raw::record(b"ax", floats, &[t, 1, 2]);
    let declared = raw::assemble(&[b"T"], &[b"y", b"x"], &[record(&[F(2), F(1)])]);
    let by_float = raw::assemble(&[b"T"], &[b"x", b"y"], &[record(&[F(1), F(2)])]);
    agree(
        &declared,
        0,
        Err(KernelError::BadRecord),
        "declaration order",
    );
    agree(&by_float, 0, Ok(()), "float order");
}

#[test]
fn a_float_outside_any_block_numbers_a_variable_before_an_earlier_float_inside_one() {
    let src = "
$c |- T $.
$v y x $.
${
  l.y $f T y $.
  ax-y $a |- y $.
$}
g.x $f T x $.
g.y $f T y $.
ax-x $a |- x $.
";
    let bytes = pack_all(&parse(src));
    let names: Vec<String> = decode(&bytes)
        .unwrap()
        .variables
        .into_iter()
        .map(|v| v.name)
        .collect();
    assert_eq!(names, ["x", "y"]);
}
