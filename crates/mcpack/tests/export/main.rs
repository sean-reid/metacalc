#[path = "../cases/references.rs"]
mod references;

use mcpack::library::{
    CheckError, ExportError, KernelError, ReadError, Step, check, crc32, decode, export, pack,
    pack_kernel,
};
use mcpack::mm::{self, Database, Kind, StmtId, Verifier};
use std::path::{Path, PathBuf};
use std::process::Command;

const DB: &str = "
$c R wff |- $.
$v x y z $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
${
  $d x y $.
  ax-d $a |- R x y $.
$}
${
  m.1 $e |- R x y $.
  ax-m $a |- R x x $.
$}
${
  $d x y $.
  good $p |- R x y $= vx vy ax-d $.
$}
${
  $d x z $.
  dummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
$}
${
  t.1 $e |- R x z $.
  ax-t $a |- R x y $.
$}
${
  $d x y z $.
  both $p |- R x y $= vx vy vz vx vz ax-d ax-t $.
$}
";

const TWICE: &str = "
$c ( ) -> wff $.
$v p q $.
wp $f wff p $.
wq $f wff q $.
wi $a wff ( p -> q ) $.
twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
";

const ESSENTIAL_FIRST: &str = "
$c wff |- R $.
$v x y $.
vx $f wff x $.
${
  e.1 $e |- R x $.
  vy $f wff y $.
  ax-mid $a |- R y $.
$}
${
  th.1 $e |- R x $.
  wy $f wff y $.
  th-mid $p |- R y $= vx th.1 wy ax-mid $.
$}
${
  e.2 $e |- R $.
  vy2 $f wff y $.
  ax-first $a |- R y $.
$}
";

const COLLISION: &str = "
$c |- wff R $.
$v x y $.
vx $f wff x $.
vy $f wff y $.
th.e1 $a |- R $.
f.0 $a |- R $.
f.0_ $a |- R $.
${
  h $e |- R $.
  th $p |- R $= h $.
$}
${
  th2.1 $e |- R x $.
  th2.2 $e |- R y $.
  th2 $p |- R y $= th2.2 $.
$}
";

fn parse(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

fn all(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

fn names(db: &Database, expr: &[u32]) -> Vec<String> {
    expr.iter()
        .map(|&s| db.syms[s as usize].name.clone())
        .collect()
}

type Shape = (Kind, Vec<String>);
type Summary = (String, Shape, Vec<Shape>, Vec<(String, String)>);

/// Every assertion with its hypotheses and mandatory disjoint pairs, spelled without labels.
fn shapes(db: &Database) -> Vec<Summary> {
    let spell = |id: StmtId| {
        let stmt = db.stmt(id);
        (stmt.kind, names(db, &stmt.expr))
    };
    db.stmts
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind.is_assertion())
        .map(|(id, s)| {
            let dv = s.frame.dv.iter();
            let dv = dv.map(|&(a, b)| {
                (
                    db.syms[a as usize].name.clone(),
                    db.syms[b as usize].name.clone(),
                )
            });
            let hyps = s.frame.hyps.iter().map(|&h| spell(h)).collect();
            (s.label.clone(), spell(id as StmtId), hyps, dv.collect())
        })
        .collect()
}

struct Exported {
    bytes: Vec<u8>,
    text: String,
    db: Database,
}

/// Exports the library, and holds the text to everything the export promises.
fn round_trip(name: &str, db: &Database) -> Exported {
    let bytes = pack(db, &all(db)).unwrap_or_else(|e| panic!("{name} packs: {e:?}"));
    let text = export(&bytes).unwrap_or_else(|e| panic!("{name} exports: {e}"));
    assert!(
        text.lines().all(|l| l.len() <= 79),
        "{name}: a line is too long"
    );
    let reparsed =
        mm::parse(text.clone().into_bytes()).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
    let again = pack(&reparsed, &all(&reparsed)).expect("the export packs");
    assert!(again == bytes, "{name}: the repacked export differs");
    assert_eq!(shapes(&reparsed), shapes(db), "{name}");
    let mut verifier = Verifier::new(&reparsed);
    for id in reparsed.theorems() {
        let label = &reparsed.stmt(id).label;
        assert_eq!(verifier.verify(id), Ok(()), "{name} {label}");
    }
    Exported {
        bytes,
        text,
        db: reparsed,
    }
}

fn corpus(name: &str) -> Database {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name);
    mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses")
}

#[test]
fn every_corpus_library_the_kernel_accepts_exports_to_the_same_bytes() {
    let mut skipped = Vec::new();
    for name in [
        "anatomy.mm",
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier.mm",
        "big-unifier-bad1.mm",
        "big-unifier-bad2.mm",
        "big-unifier-bad3.mm",
        "demo0.mm",
        "demo0-bad1.mm",
        "emptyline.mm",
        "miu.mm",
    ] {
        let db = corpus(name);
        let bytes = pack(&db, &all(&db)).expect("it packs");
        if check(&bytes).is_err() {
            skipped.push(name);
            continue;
        }
        round_trip(name, &db);
    }
    assert_eq!(
        skipped,
        [
            "anatomy-bad1.mm",
            "anatomy-bad2.mm",
            "anatomy-bad3.mm",
            "big-unifier-bad1.mm",
            "big-unifier-bad2.mm",
            "big-unifier-bad3.mm",
            "demo0-bad1.mm"
        ]
    );
}

#[test]
fn floats_are_declared_once_after_the_variables_and_no_block_holds_one() {
    let db = parse(DB);
    let out = round_trip("DB", &db);
    assert!(check(&out.bytes).is_ok());
    assert!(
        out.text
            .contains("$v x y z $.\nf.0 $f wff x $.\nf.1 $f wff y $.\nf.2 $f wff z $.\n${\n"),
        "{}",
        out.text
    );
    let indented = out.text.lines().filter(|l| l.starts_with("  "));
    assert!(
        indented.clone().all(|l| !l.contains(" $f ")),
        "{}",
        out.text
    );
    assert!(
        out.text
            .contains("  $d x z $.\n  dummy $p |- R x x $= ( f.2 ax-d ax-m )")
    );
    assert!(
        out.text.contains("  both $p |- R x y $= ( f.2 "),
        "{}",
        out.text
    );
    assert!(out.text.contains("  $d x y $.\n"));
}

#[test]
fn saved_steps_survive() {
    round_trip("twice", &parse(TWICE));
}

#[test]
fn step_numbers_past_one_and_two_bytes_survive() {
    for count in [250, 500] {
        let mut src =
            String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
        let (mut normal, mut labels, mut compressed) =
            (String::from("n0"), String::new(), letters(0));
        for i in 0..count {
            src += &format!("n{i} $a wff T $.\n");
            labels += &format!("n{i} ");
            if i > 0 {
                normal += &format!(" n{i} drop");
                compressed += &(letters(i) + &letters(count));
            }
        }
        for (name, proof) in [
            ("normal", normal),
            ("compressed", format!("( {labels}drop ) {compressed}")),
        ] {
            let db = parse(&format!("{src} big $p wff T $= {proof} $.\n"));
            let out = round_trip(&format!("{name} {count}"), &db);
            assert!(check(&out.bytes).is_ok());
            let steps = decode(&out.bytes).unwrap().assertions.pop().unwrap().proof;
            let largest = steps.iter().filter_map(|s| match s {
                Step::Number(n) => Some(*n),
                Step::Save => None,
            });
            let largest = largest.max().unwrap();
            assert!(largest >= 240, "{name} {count}");
            assert_eq!(largest >= 496, count == 500, "{name} {count}");
        }
    }
}

fn letters(mut number: usize) -> String {
    let mut out = vec![b'A' + (number % 20) as u8];
    number /= 20;
    while number > 0 {
        number -= 1;
        out.push(b'U' + (number % 5) as u8);
        number /= 5;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[test]
fn a_record_with_an_essential_hypothesis_before_a_float_is_refused() {
    let bytes = pack(&parse(ESSENTIAL_FIRST), &all(&parse(ESSENTIAL_FIRST))).unwrap();
    let bad = |label: &str| (label.into(), KernelError::BadRecord);
    let failed = CheckError::Assertions(vec![bad("ax-mid"), bad("th-mid"), bad("ax-first")]);
    assert_eq!(check(&bytes), Err(failed));
    assert!(matches!(export(&bytes), Err(ExportError::Check(_))));
}

#[test]
fn generated_labels_step_around_labels_and_symbols_that_exist() {
    let db = parse(COLLISION);
    let out = round_trip("collision", &db);
    check(&out.bytes).expect("the kernel accepts the library");
    assert!(out.text.contains("  th.e1_ $e |- R $.\n"), "{}", out.text);
    assert!(
        out.text.contains("\nf.0__ $f wff x $.\nf.1 $f wff y $.\n"),
        "{}",
        out.text
    );
    assert!(out.text.contains("  f.0 $a |- R $."));
    assert!(out.text.contains("  f.0_ $a |- R $."));
    let again = export(&out.bytes).unwrap();
    assert_eq!(again, out.text, "the export is deterministic");
    let labels: Vec<&str> = out.db.stmts.iter().map(|s| s.label.as_str()).collect();
    let mut unique = labels.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), labels.len());
}

#[test]
fn a_symbol_named_like_a_generated_label_is_not_reused() {
    let src = "$c wff |- f.0 $. $v x $. f.0-pad $a f.0 $. vx $f wff x $.
        ${ th.1 $e |- x $. th $a |- x $. $}";
    let db = parse(src);
    let out = round_trip("symbol", &db);
    assert!(out.text.contains("\nf.0_ $f wff x $.\n"), "{}", out.text);
}

fn temp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_export(library: &Path, output: &Path) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("export")
        .arg(library)
        .arg("-o")
        .arg(output)
        .output()
        .expect("mcpack runs");
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

#[test]
fn the_command_exports_a_library_to_a_database_that_parses() {
    let dir = temp("export-ok");
    let (library, output) = (dir.join("demo0.mclib"), dir.join("demo0.mm"));
    let db = corpus("demo0.mm");
    std::fs::write(&library, pack(&db, &all(&db)).unwrap()).unwrap();
    assert_eq!(run_export(&library, &output), (Some(0), String::new()));
    let exported = mm::parse(std::fs::read(&output).unwrap()).expect("the export parses");
    assert_eq!(shapes(&exported), shapes(&db));
}

#[test]
fn the_command_refuses_a_missing_file() {
    let dir = temp("export-missing");
    let (library, output) = (dir.join("none.mclib"), dir.join("none.mm"));
    let (code, stderr) = run_export(&library, &output);
    assert_eq!(code, Some(2));
    assert!(
        stderr.starts_with(&format!("{}: ", library.display())),
        "{stderr}"
    );
    assert!(!output.exists());
}

#[test]
fn the_command_refuses_a_damaged_library() {
    let dir = temp("export-damaged");
    let (library, output) = (dir.join("damaged.mclib"), dir.join("damaged.mm"));
    let db = corpus("demo0.mm");
    let mut bytes = pack(&db, &all(&db)).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(&library, bytes).unwrap();
    let (code, stderr) = run_export(&library, &output);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        format!(
            "{}: the packed library does not load: Malformed\n",
            library.display()
        )
    );
    assert!(!output.exists());
}

#[test]
fn the_command_refuses_a_library_that_fails_the_kernel() {
    let dir = temp("export-kernel");
    let (library, output) = (dir.join("bad.mclib"), dir.join("bad.mm"));
    let db = corpus("demo0-bad1.mm");
    std::fs::write(&library, pack(&db, &all(&db)).unwrap()).unwrap();
    let (code, stderr) = run_export(&library, &output);
    assert_eq!(code, Some(1));
    assert!(
        stderr.starts_with(&format!("{}: ", library.display())),
        "{stderr}"
    );
    assert!(
        stderr.ends_with("assertions failed the kernel check\n"),
        "{stderr}"
    );
    assert!(!output.exists());
}

const COLLIDING_DUMMY: &str = "
$c R wff |- $.
$v x y z $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
${
  $d x y $.
  ax-d $a |- R x y $.
$}
${
  m.1 $e |- R x y $.
  ax-m $a |- R x x $.
$}
f.2 $a |- R $.
${
  $d x z $.
  dummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
$}
";

#[test]
fn a_float_label_that_collides_is_suffixed_and_the_proof_still_verifies() {
    let out = round_trip("colliding dummy", &parse(COLLIDING_DUMMY));
    assert!(out.text.contains("\nf.2_ $f wff z $.\n"), "{}", out.text);
    assert!(out.text.contains("( f.2_ ax-d ax-m )"), "{}", out.text);
}

/// The shipped libraries, as scripts/build-libraries.sh writes them into `MC_LIBRARIES`, export to
/// a database whose repacked records are the library's own, byte for byte.
#[test]
#[ignore = "needs MC_LIBRARIES pointing at the built libraries"]
fn the_shipped_libraries_repack_from_their_export_to_the_same_records() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES is set");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(Path::new(&dir).join(format!("{name}.mclib"))).unwrap();
        let text = export(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let db = mm::parse(text.into_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let kernel = pack_kernel(&db, &all(&db)).unwrap();
        assert!(kernel.len() <= bytes.len(), "{name}");
        assert!(
            bytes[12..kernel.len()] == kernel[12..],
            "{name}: the records differ"
        );
        println!(
            "{name}: {} bytes of records repack identically",
            kernel.len()
        );
    }
}

mod refusals;
mod rejected;

/// Whether the export of a library the kernel accepts parses and verifies in the reference
/// verifier.
fn exports_and_verifies(bytes: &[u8]) -> Result<(), String> {
    let text = export(bytes).map_err(|e| format!("does not export: {e}"))?;
    let db = mm::parse(text.into_bytes()).map_err(|e| format!("does not parse: {e}"))?;
    let mut verifier = Verifier::new(&db);
    for id in db.theorems() {
        let label = &db.stmt(id).label;
        verifier.verify(id).map_err(|e| format!("{label}: {e:?}"))?;
    }
    Ok(())
}

#[test]
fn a_hand_built_library_the_kernel_accepts_exports_and_verifies() {
    for case in references::cases() {
        let accepted = check(&case.bytes).is_ok();
        assert_eq!(accepted, case.expected.is_ok(), "{}", case.name);
        if accepted {
            assert_eq!(exports_and_verifies(&case.bytes), Ok(()), "{}", case.name);
        } else {
            assert!(
                matches!(export(&case.bytes), Err(ExportError::Check(_))),
                "{}",
                case.name
            );
        }
    }
}

#[test]
fn any_reference_list_or_proof_the_kernel_accepts_exports_and_verifies() {
    let base = references::base();
    let replacements = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 0x7F, 0x80, 0xFF];
    let (mut accepted, mut refused) = (0, 0);
    for id in [references::TH, references::TH2, references::TH3] {
        let spot = references::spot(&base, id);
        let proof_len = u16::from_le_bytes([base[spot.proof - 2], base[spot.proof - 1]]) as usize;
        let entries = spot.refs..spot.refs + 2 * spot.count;
        for at in entries.chain(spot.proof..spot.proof + proof_len) {
            for value in replacements {
                let mut bytes = base.clone();
                bytes[at] = value;
                references::reseal(&mut bytes);
                if check(&bytes).is_ok() {
                    accepted += 1;
                    let result = exports_and_verifies(&bytes);
                    assert_eq!(result, Ok(()), "assertion {id} byte {at} set to {value:#x}");
                } else {
                    refused += 1;
                }
            }
        }
    }
    assert!(
        accepted > 0 && refused > accepted,
        "{accepted} accepted, {refused} refused"
    );
}

#[test]
fn a_structured_edit_the_kernel_accepts_exports_and_verifies() {
    let base = references::base();
    let mut changed_and_accepted = 0;
    for mutant in references::mutants() {
        if check(&mutant.bytes).is_ok() {
            assert_eq!(
                exports_and_verifies(&mutant.bytes),
                Ok(()),
                "{}",
                mutant.name
            );
            changed_and_accepted += usize::from(mutant.bytes != base);
        } else {
            let exported = export(&mutant.bytes);
            assert!(
                matches!(exported, Err(ExportError::Check(_))),
                "{}",
                mutant.name
            );
        }
    }
    assert!(changed_and_accepted > 0);
}
