#![allow(dead_code)]

use mcpack::library::{self, pack, pack_kernel};
use mcpack::mm::{self, Database, Grammar, StmtId};
use mcui::{Installed, Key, Status, Statuses, Ui};
use std::path::PathBuf;

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

/// The logic plus a theorem proved by a hypothesis alone, a syntax theorem, a theorem whose proof
/// applies an assertion with eight hypotheses, and an axiom with forty-five distinct variable pairs.
pub const EXTRA: &str = "
$v y z w v u t s f g $.
vy $f setvar y $.
vz $f setvar z $.
vw $f setvar w $.
vv $f setvar v $.
vu $f setvar u $.
vt $f setvar t $.
vs $f setvar s $.
vf $f setvar f $.
vg $f setvar g $.
${
  foo.1 $e |- ph $.
  foo $p |- ph $= foo.1 $.
$}
wtwice $p wff ( ph -> ph ) $= wph wph wi $.
${
  many2.1 $e |- ph $.
  many2.2 $e |- ps $.
  many2.3 $e |- ch $.
  many2.4 $e |- ( ph -> ps ) $.
  many2.5 $e |- ( ps -> ch ) $.
  many2.6 $e |- ( ph -> ch ) $.
  many2.7 $e |- ( ch -> ph ) $.
  many2.8 $e |- ( ps -> ph ) $.
  many2 $p |- ( ph -> ( ps -> ch ) ) $=
    wph wps wch many2.1 many2.2 many2.3 many2.4 many2.5 many2.6 many2.7 many2.8 ax-many $.
$}
${
  $d x y z w v u t s f g $.
  ax-dvs $a |- A. x A. y A. z A. w A. v A. u A. t A. s A. f A. g ph $.
$}
";

/// The logic library with two more axioms whose labels are eight of the widest characters.
pub fn wide_ui() -> Ui {
    let src = format!("{LOGIC}\nmmmmmmmm $a |- ph $.\nvwvwvwvw $a |- ph $.\n");
    ui_over(pack_all(&db(&src)).expect("the wide labels pack"), &[], &[])
}

pub fn extra_bytes() -> Vec<u8> {
    let src = format!("{LOGIC}{EXTRA}");
    let bytes = pack_all(&db(&src)).expect("the extended logic packs");
    library::check(&bytes).expect("the kernel accepts the extended logic");
    bytes
}

/// A user interface over one library named `logic`, every assertion checked except the ones named.
pub fn ui_over(bytes: Vec<u8>, unchecked: &[&str], failed: &[&str]) -> Ui {
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

/// A library of 1200 axioms, enough for four-digit counts.
pub fn big_bytes() -> Vec<u8> {
    pack_all(&db(&big_source())).expect("the big library packs")
}

pub fn big_kernel_bytes() -> Vec<u8> {
    pack_all_kernel(&db(&big_source())).expect("the big library packs")
}

fn big_source() -> String {
    let mut src = String::from("$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n");
    for n in 1..=1200 {
        src.push_str(&format!("ax{n} $a |- ph $.\n"));
    }
    src
}

pub fn extra_ui() -> Ui {
    ui_over(extra_bytes(), &[], &[])
}

/// The bytes of `bytes` with the `which`th reference of assertion `label` set to `to`.
pub fn patch_reference(bytes: &[u8], label: &str, which: usize, to: u16) -> Vec<u8> {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let count = u16_at(14);
    let directory = u32_at(16);
    for i in 0..count {
        let mut at = u32_at(directory + 4 * i);
        let theorem = bytes[at] == 1;
        let len = bytes[at + 1] as usize;
        if &bytes[at + 2..at + 2 + len] != label.as_bytes() {
            continue;
        }
        assert!(theorem);
        at += 2 + len;
        let hyps = bytes[at] as usize;
        at += 1;
        for _ in 0..hyps {
            at += if bytes[at] == 0 {
                2
            } else {
                2 + bytes[at + 1] as usize
            };
        }
        at += 2 + 2 * u16_at(at);
        at += 1 + bytes[at] as usize;
        at += 2 + 2 * u16_at(at);
        let refs = u16_at(at);
        assert!(which < refs);
        let mut out = bytes.to_vec();
        out[at + 2 + 2 * which..at + 4 + 2 * which].copy_from_slice(&to.to_le_bytes());
        return out;
    }
    panic!("no theorem {label}");
}

/// Whether the title row of `frame` is `left` at the left margin and `right` at the right margin
/// in white on black, and nothing else is drawn in it.
pub fn title_is(frame: &mcglyph::Bitmap, left: &str, right: &str) -> bool {
    let font = mcglyph::Font::builtin();
    let (l, r) = (font.text(left), font.text(right));
    let right_x = 95usize.saturating_sub(r.width());
    (0..8).all(|y| {
        (0..96).all(|x| {
            let ink = |t: &mcglyph::Bitmap, at: usize| x >= at && y >= 1 && t.get(x - at, y - 1);
            frame.get(x, y) == !(ink(&l, 1) || ink(&r, right_x))
        })
    })
}

pub fn db(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

/// Every assertion of `db` packed into a library, or `None` when it cannot be packed.
pub fn pack_all(db: &Database) -> Option<Vec<u8>> {
    pack_with(db, pack)
}

/// [`pack_all`] with no sections, the way a library from before them is.
pub fn pack_all_kernel(db: &Database) -> Option<Vec<u8>> {
    pack_with(db, pack_kernel)
}

type Packer = fn(&Database, &[StmtId]) -> Result<Vec<u8>, library::PackError>;

fn pack_with(db: &Database, packer: Packer) -> Option<Vec<u8>> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).ok()?;
    let roots: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .collect();
    let kept = mm::closure(db, &grammar, &roots).ok()?;
    packer(db, &kept).ok()
}

pub fn kernel_only_bytes() -> Vec<u8> {
    pack_all_kernel(&db(LOGIC)).expect("the logic packs")
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

/// The logic with the proof of `a1i` damaged so the kernel rejects it, and the table that records it.
pub fn rejected_a1i() -> (Vec<u8>, Vec<Status>) {
    let sound = logic_bytes();
    let alid = index_of(&sound, "alid") as u16;
    let bytes = refreshed(patch_reference(&sound, "a1i", 0, alid));
    let table = statuses(&bytes, &[], &["a1i"]);
    (bytes, table)
}

/// A user interface over `rejected_a1i`.
pub fn rejected_ui() -> Ui {
    let (bytes, table) = rejected_a1i();
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

/// The proving library with the proof of `a1i` damaged so the kernel rejects it.
pub fn rejected_proving_bytes() -> Vec<u8> {
    let sound = proving_bytes();
    let alid = index_of(&sound, "alid") as u16;
    let bytes = refreshed(patch_reference(&sound, "a1i", 0, alid));
    assert!(library::check(&bytes).is_err());
    bytes
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
    keys.iter().for_each(|&k| ui.press_settled(k));
}

pub fn typed(ui: &mut Ui, text: &str) {
    text.chars().for_each(|c| ui.press_settled(Key::Letter(c)));
}

/// Opens the logic library and the theorem `label` in it.
pub fn open(ui: &mut Ui, label: &str) {
    press(ui, &[Key::Enter, Key::F2]);
    typed(ui, label);
    while !matches!(ui.state(), mcui::State::List { label: Some(ref l), .. } if l == label) {
        ui.press_settled(Key::Down);
    }
    ui.press_settled(Key::Enter);
}

/// Compares `frame` with the checked-in text art. With MCUI_BLESS set the art is rewritten first
/// and compared as usual, so a blessing run cannot pass without a comparison; a CI run cannot bless.
/// With MCUI_PNG naming a directory the frame is also written there as a PNG at six times its size.
pub fn check_frame(frame: &mcglyph::Bitmap, name: &str) {
    assert_eq!((frame.width(), frame.height()), (96, 64), "{name}");
    if let Some(dir) = std::env::var_os("MCUI_PNG") {
        std::fs::create_dir_all(&dir).expect("create png directory");
        let path = PathBuf::from(dir).join(format!("{name}.png"));
        std::fs::write(path, mcglyph::png::encode(frame, 6)).expect("write png");
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/frames")
        .join(format!("{name}.txt"));
    let got = format!("{frame}\n");
    if std::env::var("MCUI_BLESS").is_ok_and(|v| v == "1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "frames cannot be blessed in CI"
        );
        std::fs::write(&path, &got).expect("write frame");
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        got == want,
        "{name} differs\n--- got\n{got}--- want\n{want}"
    );
}

/// The bytes with their checksum recomputed after a patch.
pub fn refreshed(mut bytes: Vec<u8>) -> Vec<u8> {
    let crc = library::crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
    bytes
}

/// A user interface over one library named `logic` whose assertions are all still unchecked.
pub fn unchecked_ui(bytes: Vec<u8>) -> Ui {
    let table = vec![Status::Unchecked; library::read(&bytes).expect("library reads").len()];
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

/// Theorems and axioms for the proving screen: many candidates for one goal, a long statement,
/// and a theorem that applying `ax-5` to would break a distinct variable rule.
pub const PROVING: &str = "
axa1 $a |- ( ph -> ph ) $.
axa2 $a |- ( ph -> ph ) $.
axa3 $a |- ( ph -> ph ) $.
axa4 $a |- ( ph -> ph ) $.
axa5 $a |- ( ph -> ph ) $.
axa6 $a |- ( ph -> ph ) $.
tmany $p |- ( ph -> ph ) $= wph id $.
tlong $p |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $= wph wps wch ax-2 $.
axq $a |- ( ph -> A. y ph ) $.
tq $p |- ( ph -> A. y ph ) $= wph vy axq $.
${
  ta1.1 $e |- ch $.
  ta1 $p |- ( ph -> ch ) $= wch wph ta1.1 a1i $.
$}
${
  $d x ps $.
  axu.1 $e |- ( ph -> A. x ps ) $.
  axu $a |- ph $.
$}
${
  $d z ph $.
  tdv2 $p |- ph $= wph wph vz wph vz ax-5 axu $.
$}
";

pub fn proving_bytes() -> Vec<u8> {
    let src = format!("{LOGIC}{EXTRA}{PROVING}");
    let bytes = pack_all(&db(&src)).expect("the proving library packs");
    library::check(&bytes).expect("the kernel accepts the proving library");
    bytes
}

/// A logic with nothing that fits a bare variable, and a theorem that is its own hypothesis.
pub const BARE: &str = "
$c ( ) -> wff |- $.
$v ph ps $.
wph $f wff ph $.
wps $f wff ps $.
wi $a wff ( ph -> ps ) $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
${
  th.1 $e |- ph $.
  th $p |- ph $= th.1 $.
$}
";

pub fn bare_bytes() -> Vec<u8> {
    pack_all(&db(BARE)).expect("the bare logic packs")
}

/// The number of `label` in the library `bytes`.
pub fn index_of(bytes: &[u8], label: &str) -> usize {
    library::read(bytes)
        .expect("library reads")
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no {label}"))
}

/// A logic with one rule that turns `ps` into `( ph -> ps )`, and a theorem that nests `ph`
/// under `depth` implications, so proving it takes `depth` applications and a hypothesis.
pub fn chain_bytes(depth: usize) -> Vec<u8> {
    let (mut statement, mut syntax, mut proof) =
        ("ph".to_string(), "wph".to_string(), "chain.1".to_string());
    for _ in 0..depth {
        proof = format!("wph {syntax} {proof} axs");
        statement = format!("( ph -> {statement} )");
        syntax = format!("wph {syntax} wi");
    }
    let src = format!(
        "$c ( ) -> wff |- $.
$v ph ps $.
wph $f wff ph $.
wps $f wff ps $.
wi $a wff ( ph -> ps ) $.
${{
  axs.1 $e |- ps $.
  axs $a |- ( ph -> ps ) $.
$}}
${{
  chain.1 $e |- ph $.
  chain $p |- {statement} $= {proof} $.
$}}
"
    );
    let bytes = pack_all(&db(&src)).expect("the chain library packs");
    library::check(&bytes).expect("the kernel accepts the chain library");
    bytes
}

/// `( ph -> ( ps -> ( ch -> ... ph ) ) )` with `depth` implications.
pub fn nested(depth: usize) -> String {
    let names = ["ph", "ps", "ch"];
    let mut statement = "ph".to_string();
    for i in 0..depth {
        statement = format!("( {} -> {statement} )", names[i % 3]);
    }
    statement
}

/// The logic plus a rule with seven hypotheses and a conclusion that each take several rows.
pub fn long_bytes() -> Vec<u8> {
    let long = nested(9);
    let hyps: String = (1..=7)
        .map(|n| format!("  axlong.{n} $e |- {long} $.\n"))
        .collect();
    let src = format!("{LOGIC}\n${{\n{hyps}  axlong $a |- {long} $.\n$}}\n");
    let bytes = pack_all(&db(&src)).expect("the long library packs");
    library::check(&bytes).expect("the kernel accepts the long library");
    bytes
}

/// The logic, without trees and with a bad checksum, plus an axiom of `depth` implications: its
/// statements are flat runs of tokens.
pub fn flat_bytes(depth: usize) -> Vec<u8> {
    let src = format!("{LOGIC}\naxflat $a |- {} $.\n", nested(depth));
    let mut bytes = pack_all_kernel(&db(&src)).expect("the flat library packs");
    bytes[4] ^= 0xFF;
    bytes
}

/// The logic plus two theorems whose goal takes eight rows once unfolded: `tl`, proved by
/// `ax-mp` from two hypotheses, and `lh`, proved by its own hypothesis.
pub fn tall_goal_bytes() -> Vec<u8> {
    let depth = 30;
    let names = ["ph", "ps", "ch"];
    let mut syntax = "wph".to_string();
    for i in 0..depth {
        syntax = format!("w{} {syntax} wi", names[i % 3]);
    }
    let long = nested(depth);
    let src = format!(
        "{LOGIC}
${{
  tl.1 $e |- ph $.
  tl.2 $e |- ( ph -> {long} ) $.
  tl $p |- {long} $= wph {syntax} tl.1 tl.2 ax-mp $.
$}}
${{
  lh.1 $e |- {long} $.
  lh $p |- {long} $= lh.1 $.
$}}
"
    );
    let bytes = pack_all(&db(&src)).expect("the tall goal library packs");
    library::check(&bytes).expect("the kernel accepts the tall goal library");
    bytes
}

/// The logic plus `t1` to `t12`, each proved by the one before it.
pub fn hops_bytes() -> Vec<u8> {
    let mut src = format!("{LOGIC}\nt1 $p |- ( ph -> ph ) $= wph id $.\n");
    for n in 2..=12 {
        src.push_str(&format!("t{n} $p |- ( ph -> ph ) $= wph t{} $.\n", n - 1));
    }
    let bytes = pack_all(&db(&src)).expect("the hops library packs");
    library::check(&bytes).expect("the kernel accepts the hops library");
    bytes
}

/// Whether `frame` shows `text` in the main font with its first column at `x` on text row `row`
/// (the title bar being row 0), white on black when `inverted`.
pub fn text_at(frame: &mcglyph::Bitmap, x: usize, row: usize, text: &str, inverted: bool) -> bool {
    let ink = mcglyph::Font::builtin().text(text);
    ink_matches(frame, &ink, x, 8 * row + 1, inverted)
}

/// Whether `frame` shows `tokens`, laid out as a statement, with its first column at `x` on text
/// row `row`.
pub fn tokens_at(frame: &mcglyph::Bitmap, x: usize, row: usize, tokens: &[&str]) -> bool {
    let font = mcglyph::Font::builtin();
    let ink = font.render(tokens, 94).expect("the tokens render");
    let width = font.layout(tokens, 94).expect("the tokens lay out")[0].width;
    let mut cut = mcglyph::Bitmap::new(width, ink.height());
    cut.blit(&ink, 0, 0);
    ink_matches(frame, &cut, x, 8 * row + 1, false)
}

fn ink_matches(
    frame: &mcglyph::Bitmap,
    ink: &mcglyph::Bitmap,
    x: usize,
    y: usize,
    inverted: bool,
) -> bool {
    (0..ink.height()).all(|dy| {
        (0..ink.width()).all(|dx| frame.get(x + dx, y + dy) == (ink.get(dx, dy) != inverted))
    })
}

/// Whether the label of soft key `key` (from 0) reads `label`, in the key font on its black box.
pub fn key_label_is(frame: &mcglyph::Bitmap, key: usize, label: &str) -> bool {
    let ink = mcglyph::Font::keys().text(label);
    let (from, to) = ((96 * key + 2) / 5, (96 * (key + 1) + 2) / 5 - 1);
    let x = from + (to - from - ink.width()).div_ceil(2);
    ink_matches(frame, &ink, x, 57, true)
}

/// Whether the last ink on text row `row` of `frame` is an ellipsis.
pub fn ends_in_ellipsis(frame: &mcglyph::Bitmap, row: usize) -> bool {
    let ellipsis = mcglyph::Font::builtin().text(&mcglyph::ELLIPSIS.to_string());
    let last = (0..96)
        .rev()
        .find(|&x| (8 * row..8 * row + 8).any(|y| frame.get(x, y)));
    last.is_some_and(|last| {
        (last + 1)
            .checked_sub(ellipsis.width())
            .is_some_and(|x| ink_matches(frame, &ellipsis, x, 8 * row + 1, false))
    })
}

/// The tokens of a statement laid out in `width` pixels by the font alone, one list per line.
pub fn font_lines(tokens: &[&str], width: usize) -> Vec<Vec<String>> {
    mcglyph::Font::builtin()
        .layout(tokens, width)
        .expect("the tokens lay out")
        .iter()
        .map(|line| {
            line.tokens
                .iter()
                .map(|p| tokens[p.index].to_string())
                .collect()
        })
        .collect()
}

pub fn rule(pattern: &str) -> String {
    pattern.repeat(80)[..79].to_string()
}

pub fn title(depth: u8, text: &str) -> String {
    let pattern = ["##", "#*", "=-", "-."][depth as usize - 1];
    format!("$(\n{}\n  {text}\n{}\n$)\n", rule(pattern), rule(pattern))
}

pub const LONG: &str = "Equivalence of a very long section title that cannot fit";

/// The logic with titles: two on the first assertion, a title at depth 3 and one at depth 4, a
/// title over axioms only in the middle and one at the very end, and a title over theorems only.
pub fn sectioned() -> String {
    source("")
}

/// [`sectioned`] with `lead` before the first title.
pub fn source(lead: &str) -> String {
    let mut src = String::from(
        "$c ( ) -> wff |- setvar A. $.\n$v ph ps ch x $.\nwph $f wff ph $.\nwps $f wff ps $.\nwch $f wff ch $.\nvx $f setvar x $.\n",
    );
    src.push_str(lead);
    src.push_str(&title(1, "Propositional calculus"));
    src.push_str(&title(2, "Syntax"));
    src.push_str("wi $a wff ( ph -> ps ) $.\nwal $a wff A. x ph $.\n");
    src.push_str(&title(2, "Axioms"));
    src.push_str("ax-1 $a |- ( ph -> ( ps -> ph ) ) $.\n");
    src.push_str("ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.\n");
    src.push_str("${\n  min $e |- ph $.\n  maj $e |- ( ph -> ps ) $.\n  ax-mp $a |- ps $.\n$}\n");
    src.push_str(&title(1, "Derived rules"));
    src.push_str(&title(2, "Inference"));
    src.push_str(
        "${\n  a1i.1 $e |- ph $.\n  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.\n$}\n",
    );
    src.push_str(&title(3, "Deduction"));
    src.push_str("${\n  a2i.1 $e |- ( ph -> ( ps -> ch ) ) $.\n  a2i $p |- ( ( ph -> ps ) -> ( ph -> ch ) ) $=\n    wph wps wch wi wi wph wps wi wph wch wi wi a2i.1 wph wps wch ax-2 ax-mp $.\n$}\n");
    src.push_str(&title(4, LONG));
    src.push_str("${\n  mpd.1 $e |- ( ph -> ps ) $.\n  mpd.2 $e |- ( ph -> ( ps -> ch ) ) $.\n  mpd $p |- ( ph -> ch ) $=\n    wph wps wi wph wch wi mpd.1 wph wps wch mpd.2 a2i ax-mp $.\n$}\n");
    src.push_str(&title(1, "Predicate calculus"));
    src.push_str("${\n  $d x ph $.\n  ax-5 $a |- ( ph -> A. x ph ) $.\n$}\n");
    src.push_str(&title(2, "Quantifier theorems"));
    src.push_str("${\n  $d x ph $.\n  alid $p |- ( ph -> A. x ph ) $= wph vx ax-5 $.\n$}\n");
    src.push_str(&title(1, "Appendix"));
    src.push_str("ax-last $a |- ( ph -> ph ) $.\n");
    src
}
