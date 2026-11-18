use mcpack::mm::{self, Database, StmtId, Sym};

const SRC: &str = "
$c wff |- $.
$v a $.
fa $f wff a $.
${
  $v b $.
  fb $f wff b $.
  s1 $a |- a $.
  ${
    $v c $.
    fc $f wff c $.
    s2 $a |- c $.
  $}
  s3 $a |- b $.
$}
s4 $a |- a $.
${
  $v b $.
  fb2 $f wff b $.
  s5 $a |- b $.
$}
s6 $a |- a $.
";

fn db() -> Database {
    mm::parse(SRC.as_bytes().to_vec()).expect("parses")
}

fn at(db: &Database, label: &str) -> StmtId {
    db.find(label).unwrap_or_else(|| panic!("no {label}"))
}

fn var(db: &Database, name: &str) -> Sym {
    db.syms.iter().position(|s| s.name == name).unwrap() as Sym
}

fn active(db: &Database, name: &str, labels: &[&str]) -> Vec<bool> {
    let v = var(db, name);
    labels.iter().map(|l| db.var_active(v, at(db, l))).collect()
}

const ALL: [&str; 9] = ["fa", "fb", "s1", "fc", "s2", "s3", "s4", "fb2", "s5"];

#[test]
fn a_variable_declared_at_the_outermost_level_is_active_from_its_declaration_on() {
    let db = db();
    let mut all = ALL.to_vec();
    all.push("s6");
    assert_eq!(active(&db, "a", &all), [true; 10]);
}

#[test]
fn a_block_variable_is_active_from_its_first_statement_to_its_last() {
    let db = db();
    assert_eq!(
        active(&db, "b", &ALL),
        [false, true, true, true, true, true, false, true, true]
    );
    assert!(!db.var_active(var(&db, "b"), at(&db, "s6")));
}

#[test]
fn a_nested_block_variable_ends_with_the_inner_block() {
    let db = db();
    assert_eq!(
        active(&db, "c", &ALL),
        [false, false, false, true, true, false, false, false, false]
    );
}

#[test]
fn a_variable_redeclared_after_its_block_closed_has_two_ranges() {
    let db = db();
    let b = var(&db, "b");
    let ranges: Vec<(StmtId, StmtId)> = db
        .var_scopes
        .iter()
        .filter(|e| e.0 == b)
        .map(|e| (e.1, e.2))
        .collect();
    assert_eq!(
        ranges,
        [
            (at(&db, "fb"), at(&db, "s4")),
            (at(&db, "fb2"), at(&db, "s6"))
        ]
    );
    assert!(!db.var_active(b, at(&db, "s4")));
    assert!(db.var_active(b, at(&db, "fb2")));
    assert!(db.var_active(b, at(&db, "s5")));
}
