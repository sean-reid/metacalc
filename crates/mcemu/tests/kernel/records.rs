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

const CYCLE: &str = "
$c |- T $.
$v y x $.
${
  ax-a.x $f T x $.
  ax-a.y $f T y $.
  ax-a $a |- x y $.
  ax-c $a |- y x $.
$}
${
  ax-b.y $f T y $.
  ax-b.x $f T x $.
  ax-b $a |- x y $.
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
fn a_cycle_of_float_orders_across_two_records_is_refused_by_the_second_on_both_sides() {
    expect(
        CYCLE,
        &[
            ("ax-a", Ok(())),
            ("ax-c", Ok(())),
            ("ax-b", Err(KernelError::BadRecord)),
        ],
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
fn variables_are_numbered_by_their_first_float_and_not_by_their_declaration() {
    let bytes = pack_all(&parse(CYCLE));
    let names: Vec<String> = decode(&bytes)
        .unwrap()
        .variables
        .into_iter()
        .map(|v| v.name)
        .collect();
    assert_eq!(names, ["x", "y"]);
}

#[test]
fn the_declaration_numbering_gives_a_record_the_kernel_refuses_and_the_float_numbering_one_it_accepts()
 {
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
$v x y $.
${
  l.y $f T y $.
  l.x $f T x $.
  ax-local $a |- y x $.
$}
g.x $f T x $.
g.y $f T y $.
ax-global $a |- x y $.
";
    let db = parse(src);
    let bytes = pack_all(&db);
    agree(&bytes, number(&db, "ax-global"), Ok(()), "ax-global");
    agree(
        &bytes,
        number(&db, "ax-local"),
        Err(KernelError::BadRecord),
        "ax-local",
    );
}
