use super::*;
use mcedit::Catalog;
use mcpack::mm::{self, Grammar};

const SRC: &str = "
$c ( ) -> wff |- setvar Q $.
$v ph ps ch x $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
wi $a wff ( ph -> ps ) $.
${ $d x ph $. wq $a wff Q x ph $. $}
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  t.1 $e |- ph $.
  t.2 $e |- ( ph -> ps ) $.
  t $p |- ps $= ? $.
$}
";

fn editor() -> Editor {
    let db: &'static mm::Database =
        Box::leak(Box::new(mm::parse(SRC.as_bytes().to_vec()).unwrap()));
    let grammar: &'static Grammar<'static> =
        Box::leak(Box::new(Grammar::new(db, &[("|-", "wff")]).unwrap()));
    let catalog = Catalog::new(db, grammar, 1);
    let session = Session::start_in(catalog, db.find("t").unwrap()).unwrap();
    Editor {
        session,
        lead: vec![PROVABLE.to_string()],
        groups: Vec::new(),
        replayed: 0,
        barred: Vec::new(),
    }
}

#[test]
fn a_proof_that_cannot_be_checked_is_not_finished() {
    let mut editor = editor();
    let db = editor.session.db();
    let (ax_mp, min, maj) = (
        db.find("ax-mp").unwrap(),
        db.find("t.1").unwrap(),
        db.find("t.2").unwrap(),
    );
    editor.session.apply(0, ax_mp).unwrap();
    editor.session.close(1, min).unwrap();
    editor.session.close(2, maj).unwrap();
    assert!(editor.snap(None, None).finished);
    editor.session.corrupt_metas();
    assert!(!editor.snap(None, None).finished);
}
