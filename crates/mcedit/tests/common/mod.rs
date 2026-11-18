#![allow(dead_code)]

use mcedit::oracle::{Failure, verify_spliced};
use mcedit::{Catalog, Finished, Session};
use mcpack::mm::{self, Database, Grammar, StmtId, Sym};
use std::rc::Rc;

pub const SRC: &str = "
$c ( ) -> -. wff |- A. setvar ~ $.
$v ph ps ch th x y z w q $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
wth $f wff th $.
vx $f setvar x $.
vy $f setvar y $.
vz $f setvar z $.
${
  vw $f setvar w $.
$}
${
  $v u $.
  vu $f setvar u $.
$}
wi $a wff ( ph -> ps ) $.
wn $a wff -. ph $.
wal $a wff A. x ph $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  a1i.1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.
$}
${
  $d x ph $.
  ax-5 $a |- ( ph -> A. x ph ) $.
$}
${
  ax-gen.1 $e |- ph $.
  ax-gen $a |- A. x ph $.
$}
${
  $d x ph $.
  axfree.1 $e |- A. x ph $.
  axfree $a |- ph $.
$}
${
  $d x ch $.
  axdv.1 $e |- ( ph -> ch ) $.
  axdv $a |- ( ph -> A. x ph ) $.
$}
${
  t1.1 $e |- ph $.
  t1.2 $e |- ( ph -> ps ) $.
  t1 $p |- ps $= wph wps t1.1 t1.2 ax-mp $.
$}
${
  tax5-bad $p |- ( ps -> A. y ps ) $= wps vy ax-5 $.
$}
${
  $d y ps $.
  tax5-ok $p |- ( ps -> A. y ps ) $= wps vy ax-5 $.
$}
${
  tdv.1 $e |- ( ps -> th ) $.
  tdv $p |- ( ps -> A. y ps ) $= wps vy ax-5 $.
$}
${
  $d y th $.
  tdv-ok.1 $e |- ( ps -> th ) $.
  tdv-ok $p |- ( ps -> A. y ps ) $= wps vy ax-5 $.
$}
${
  $d y th $.
  tdv-shared.1 $e |- ( ps -> A. y th ) $.
  tdv-shared $p |- ( ps -> A. y ps ) $= wps vy ax-5 $.
$}
${
  tdum.1 $e |- ps $.
  tdum $p |- ps $= tdum.1 $.
$}
${
  $d ps z $.
  tdum-ok.1 $e |- ps $.
  tdum-ok $p |- ps $= tdum-ok.1 $.
$}
${
  tnew.1 $e |- ps $.
  tnew $p |- ps $= ? $.
$}
${
  tauto.1 $e |- ph $.
  tauto $p |- ( ch -> ( ps -> ph ) ) $= wph wps wph wi wch tauto.1 wph wps ax-1 ax-mp wps wph wi wch ax-1 ax-mp $.
$}
${
  tstuck.1 $e |- ph $.
  tstuck $p |- ( ps -> ch ) $= tstuck.1 $.
$}
${
  tfwd.1 $e |- ph $.
  tfwd $p |- ( ps -> ph ) $= wph wps tfwd.1 a1i $.
$}
${
  tafter.1 $e |- ph $.
  tafter $p |- ( ps -> ph ) $= wph wps tafter.1 a1i $.
$}
wlate $a wff ~ ph $.
${
  axh.1 $e |- ( ph -> ( ps -> ( ph -> ps ) ) ) $.
  axh $a |- ch $.
$}
${
  tocc.1 $e |- ps $.
  tocc $p |- ps $= tocc.1 $.
$}
${
  tbad.1 $e |- ( ph ) $.
  tbad $p |- ps $= tbad.1 $.
$}
${
  $d x ph $.
  axq $a |- ( A. x ph -> ph ) $.
$}
${
  axpair.1 $e |- ph $.
  axpair.2 $e |- ps $.
  axpair $a |- ( ph -> ps ) $.
$}
${
  tdup $p |- ( ( A. y ps -> ps ) -> ( A. y ps -> ps ) ) $= wps vy wal wps wi wps vy wal wps wi wps vy axq wps vy axq axpair $.
$}
${
  $d y ps $.
  tdup-ok $p |- ( ( A. y ps -> ps ) -> ( A. y ps -> ps ) ) $= wps vy wal wps wi wps vy wal wps wi wps vy axq wps vy axq axpair $.
$}
vq $f setvar q $.
";

pub fn db() -> Database {
    mm::parse(SRC.as_bytes().to_vec()).expect("fixture parses")
}

pub fn grammar(db: &Database) -> Grammar<'_> {
    Grammar::new(db, &[("|-", "wff")]).expect("grammar builds")
}

pub fn id(db: &Database, label: &str) -> StmtId {
    db.find(label).unwrap_or_else(|| panic!("no {label}"))
}

pub fn sym(db: &Database, name: &str) -> Sym {
    db.syms
        .iter()
        .position(|s| s.name == name)
        .unwrap_or_else(|| panic!("no {name}")) as Sym
}

pub fn catalog<'a>(db: &'a Database, grammar: &'a Grammar<'a>) -> Rc<Catalog<'a>> {
    Catalog::new(db, grammar)
}

pub fn session<'a>(cat: &Rc<Catalog<'a>>, theorem: &str) -> Session<'a> {
    Session::start_in(cat.clone(), id(cat.db, theorem)).expect("session starts")
}

/// The exported proof written into the fixture in place of the theorem's own and checked by the reference verifier.
pub fn verified(db: &Database, finished: &Finished) -> Result<(), Failure> {
    verify_spliced(SRC, db, finished)
}

pub fn labels(db: &Database, finished: &Finished) -> String {
    let names: Vec<&str> = finished
        .proof
        .iter()
        .map(|l| match *l {
            mcedit::Label::Stmt(s) => db.stmt(s).label.as_str(),
            mcedit::Label::Dummy(i) => finished.dummies[i].label.as_str(),
        })
        .collect();
    names.join(" ")
}
