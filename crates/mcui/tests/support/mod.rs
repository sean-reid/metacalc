#![allow(dead_code)]

use mcpack::library::{self, pack};
use mcpack::mm::{self, Database, Grammar, StmtId};
use mcui::{Installed, Key, Status, Statuses, Ui};

/// A small logic with one binder, a distinct variable condition and a theorem whose
/// hypotheses alone fill more rows than the screen has.
pub const LOGIC: &str = "
$c ( ) -> wff |- setvar A. $.
$v ph ps ch x $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
wi $a wff ( ph -> ps ) $.
wal $a wff A. x ph $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  $d x ph $.
  ax-5 $a |- ( ph -> A. x ph ) $.
$}
${
  a1i.1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.
$}
${
  a2i.1 $e |- ( ph -> ( ps -> ch ) ) $.
  a2i $p |- ( ( ph -> ps ) -> ( ph -> ch ) ) $=
    wph wps wch wi wi wph wps wi wph wch wi wi a2i.1 wph wps wch ax-2 ax-mp $.
$}
${
  mpd.1 $e |- ( ph -> ps ) $.
  mpd.2 $e |- ( ph -> ( ps -> ch ) ) $.
  mpd $p |- ( ph -> ch ) $=
    wph wps wi wph wch wi mpd.1 wph wps wch mpd.2 a2i ax-mp $.
$}
${
  syl.1 $e |- ( ph -> ps ) $.
  syl.2 $e |- ( ps -> ch ) $.
  syl $p |- ( ph -> ch ) $= wph wps wch syl.1 wps wch wi wph syl.2 a1i mpd $.
$}
id $p |- ( ph -> ph ) $= wph wph wph wi wph wph wph ax-1 wph wph wph wi ax-1 mpd $.
${
  $d x ph $.
  alid $p |- ( ph -> A. x ph ) $= wph vx ax-5 $.
$}
${
  many.1 $e |- ph $.
  many.2 $e |- ps $.
  many.3 $e |- ch $.
  many.4 $e |- ( ph -> ps ) $.
  many.5 $e |- ( ps -> ch ) $.
  many.6 $e |- ( ph -> ch ) $.
  many.7 $e |- ( ch -> ph ) $.
  many.8 $e |- ( ps -> ph ) $.
  ax-many $a |- ( ph -> ( ps -> ch ) ) $.
$}
";

pub fn db(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

/// Every assertion of `db` packed into a library, or `None` when it cannot be packed.
pub fn pack_all(db: &Database) -> Option<Vec<u8>> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).ok()?;
    let roots: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .collect();
    let kept = mm::closure(db, &grammar, &roots).ok()?;
    pack(db, &kept).ok()
}

pub fn logic_bytes() -> Vec<u8> {
    let bytes = pack_all(&db(LOGIC)).expect("the logic packs");
    library::check(&bytes).expect("the kernel accepts the logic");
    bytes
}

/// Axioms checked, theorems checked except the ones named, one marked failed.
pub fn statuses(bytes: &[u8], unchecked: &[&str], failed: &[&str]) -> Vec<Status> {
    library::read(bytes)
        .expect("library reads")
        .iter()
        .map(|a| {
            if failed.contains(&a.label.as_str()) {
                Status::Failed
            } else if unchecked.contains(&a.label.as_str()) {
                Status::Unchecked
            } else {
                Status::Checked
            }
        })
        .collect()
}

pub fn ui_with(unchecked: &[&str], failed: &[&str]) -> Ui {
    let bytes = logic_bytes();
    let table = statuses(&bytes, unchecked, failed);
    Ui::new(
        vec![Installed {
            name: "logic".into(),
            bytes,
        }],
        Statuses {
            libraries: vec![table],
            my_proofs: 2,
        },
    )
    .expect("the library loads")
}

pub fn ui() -> Ui {
    ui_with(&["a2i"], &["id"])
}

pub fn press(ui: &mut Ui, keys: &[Key]) {
    keys.iter().for_each(|&k| ui.press(k));
}

pub fn typed(ui: &mut Ui, text: &str) {
    text.chars().for_each(|c| ui.press(Key::Letter(c)));
}

/// Opens the logic library and the theorem `label` in it.
pub fn open(ui: &mut Ui, label: &str) {
    press(ui, &[Key::Enter, Key::F2]);
    typed(ui, label);
    while !matches!(ui.state(), mcui::State::List { label: Some(ref l), .. } if l == label) {
        ui.press(Key::Down);
    }
    ui.press(Key::Enter);
}
