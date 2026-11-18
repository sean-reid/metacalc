use mcpack::library::{self, Kernel, KernelError, Limits, crc32, pack};
use mcpack::mm::{self, Database, Kind, StmtId, Verifier};
use std::path::Path;
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
bad $p |- R x y $= vx vy ax-d $.
same $p |- R x x $= vx vx ax-d $.
${
  $d x z $.
  dummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
$}
nodummy $p |- R x x $= vx vz vx vz ax-d ax-m $.
${
  t.1 $e |- R x z $.
  ax-t $a |- R x y $.
$}
${
  $d x y z $.
  both $p |- R x y $= vx vy vz vx vz ax-d ax-t $.
$}
";

fn db(src: &str) -> Database {
    mm::parse(src.as_bytes().to_vec()).expect("database parses")
}

fn everything(db: &Database) -> Vec<StmtId> {
    (0..db.stmts.len() as StmtId).collect()
}

fn assertions(db: &Database) -> Vec<StmtId> {
    let is_assertion = |&id: &StmtId| matches!(db.stmt(id).kind, Kind::Axiom | Kind::Theorem);
    everything(db).into_iter().filter(is_assertion).collect()
}

fn number(db: &Database, label: &str) -> usize {
    let id = db.find(label).expect("label exists");
    assertions(db).iter().position(|&a| a == id).unwrap()
}

fn check(bytes: &[u8], id: usize) -> Result<(), KernelError> {
    Kernel::new(bytes, Limits::UNBOUNDED)?.verify(id)
}

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

/// Offsets of a theorem's extra disjoint pairs and of its references, read from the documented layout.
fn theorem_fields(bytes: &[u8], id: usize) -> (usize, usize) {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = u32_at(u32_at(16) + 4 * id);
    assert_eq!(bytes[at], 1, "a theorem");
    at += 2 + bytes[at + 1] as usize;
    let hyps = bytes[at];
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
    let extra = at + 2;
    let refs = extra + 2 * u16_at(at) + 2;
    (extra, refs)
}

#[test]
fn kernel_agrees_with_the_reference_verifier_on_the_corpus() {
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/corpus")
            .join(name);
        let db = mm::parse(std::fs::read(path).expect("corpus file reads")).expect("it parses");
        let bytes = pack(&db, &everything(&db)).expect("it packs");
        let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).expect("library loads");
        let mut reference = Verifier::new(&db);
        let ids = assertions(&db);
        assert_eq!(kernel.assertions(), ids.len(), "{name}");
        for (local, &id) in ids.iter().enumerate() {
            let stmt = db.stmt(id);
            assert_eq!(kernel.label(local).unwrap(), stmt.label.as_bytes());
            let expected = stmt.kind == Kind::Axiom || reference.verify(id).is_ok();
            assert_eq!(
                kernel.verify(local).is_ok(),
                expected,
                "{name} {}",
                stmt.label
            );
        }
    }
}

#[test]
fn kernel_reports_what_is_wrong_with_a_proof() {
    let db = db(DB);
    let bytes = pack(&db, &everything(&db)).unwrap();
    for (label, expected) in [
        ("ax-d", Ok(())),
        ("good", Ok(())),
        ("dummy", Ok(())),
        ("both", Ok(())),
        ("bad", Err(KernelError::DisjointViolation)),
        ("same", Err(KernelError::DisjointViolation)),
        ("nodummy", Err(KernelError::DisjointViolation)),
    ] {
        assert_eq!(check(&bytes, number(&db, label)), expected, "{label}");
    }
}

#[test]
fn kernel_needs_exactly_the_memory_the_reference_verifier_measures() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/big-unifier.mm");
    let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
    let bytes = pack(&db, &everything(&db)).unwrap();
    let mut reference = Verifier::new(&db);
    let mut theorems = 0;
    for (local, &id) in assertions(&db).iter().enumerate() {
        if db.stmt(id).kind != Kind::Theorem {
            continue;
        }
        theorems += 1;
        reference.verify(id).expect("the proof is valid");
        let usage = reference.usage();
        let exact = Limits {
            stack_syms: usage.stack_syms,
            stack_entries: usage.stack_depth,
            ..Limits::UNBOUNDED
        };
        assert_eq!(Kernel::new(&bytes, exact).unwrap().verify(local), Ok(()));
        for short in [
            Limits {
                stack_syms: usage.stack_syms - 1,
                ..exact
            },
            Limits {
                stack_entries: usage.stack_depth - 1,
                ..exact
            },
        ] {
            let got = Kernel::new(&bytes, short).unwrap().verify(local);
            assert_eq!(got, Err(KernelError::OutOfMemory));
        }
    }
    assert!(theorems > 0);
}

#[test]
fn saved_steps_count_against_their_own_limit() {
    let src = "
        $c ( ) -> wff $.
        $v p q $.
        wp $f wff p $.
        wq $f wff q $.
        wi $a wff ( p -> q ) $.
        twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
    ";
    let db = db(src);
    let bytes = pack(&db, &everything(&db)).unwrap();
    let limits = |saved_syms, saved_entries| Limits {
        saved_syms,
        saved_entries,
        ..Limits::UNBOUNDED
    };
    let verify = |limits| Kernel::new(&bytes, limits).unwrap().verify(1);
    assert_eq!(verify(limits(6, 1)), Ok(()));
    assert_eq!(verify(limits(5, 1)), Err(KernelError::OutOfMemory));
    assert_eq!(verify(limits(6, 0)), Err(KernelError::OutOfMemory));
}

#[test]
fn step_numbers_past_one_byte_survive_packing() {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let mut proof = String::from("n0");
    for i in 0..250 {
        src += &format!("n{i} $a wff T $.\n");
        if i > 0 {
            proof += &format!(" n{i} drop");
        }
    }
    src += &format!("big $p wff T $= {proof} $.\n");
    let db = db(&src);
    Verifier::new(&db)
        .verify(db.find("big").unwrap())
        .expect("the proof is valid");
    let bytes = pack(&db, &everything(&db)).unwrap();
    assert_eq!(check(&bytes, number(&db, "big")), Ok(()));
}

#[test]
fn a_library_that_fails_its_checksum_does_not_load() {
    let db = db(DB);
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    assert_eq!(&bytes[..4], library::MAGIC);
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert!(matches!(
        Kernel::new(&bytes, Limits::UNBOUNDED),
        Err(KernelError::Malformed)
    ));
    assert!(matches!(
        Kernel::new(&bytes[..last], Limits::UNBOUNDED),
        Err(KernelError::Malformed)
    ));
}

#[test]
fn a_proof_cannot_use_itself_or_a_later_assertion() {
    let db = db(DB);
    let good = number(&db, "good");
    for target in [good, good + 1] {
        let mut bytes = pack(&db, &everything(&db)).unwrap();
        let (_, refs) = theorem_fields(&bytes, good);
        bytes[refs..refs + 2].copy_from_slice(&(target as u16).to_le_bytes());
        reseal(&mut bytes);
        assert_eq!(check(&bytes, good), Err(KernelError::ForwardReference));
    }
}

#[test]
fn a_dummy_variable_needs_its_own_disjoint_pair() {
    let db = db(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let (extra, _) = theorem_fields(&bytes, dummy);
    let (x, y, z) = (bytes[extra], bytes[extra] + 1, bytes[extra + 1]);
    assert_eq!(z, x + 2, "the pair is x z");
    bytes[extra] = y;
    reseal(&mut bytes);
    assert_eq!(check(&bytes, dummy), Err(KernelError::DisjointViolation));
}

#[test]
fn a_pair_of_mandatory_variables_cannot_hide_among_the_dummy_pairs() {
    let db = db(DB);
    let both = number(&db, "both");
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let (extra, _) = theorem_fields(&bytes, both);
    let x = bytes[extra];
    bytes[extra + 1] = x + 1;
    reseal(&mut bytes);
    assert_eq!(check(&bytes, both), Err(KernelError::BadRecord));
}

#[test]
fn an_assertion_with_a_variable_it_has_no_hypothesis_for_is_refused_wherever_it_is_used() {
    let db = db(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let next = u32_at(u32_at(16) + 4 * (axiom + 1));
    let z = bytes[next - 1] + 1;
    bytes[next - 1] = z;
    reseal(&mut bytes);
    assert_eq!(check(&bytes, axiom), Err(KernelError::BadRecord));
    assert_eq!(check(&bytes, good), Err(KernelError::BadRecord));
}

#[test]
fn no_single_byte_change_makes_the_kernel_panic() {
    let db = db(DB);
    let original = pack(&db, &everything(&db)).unwrap();
    for at in 8..original.len() {
        for value in [0x00, 0x01, 0x7F, 0x80, 0xEF, 0xF0, 0xFE, 0xFF] {
            let mut bytes = original.clone();
            bytes[at] = value;
            reseal(&mut bytes);
            if let Ok(mut kernel) = Kernel::new(&bytes, Limits::UNBOUNDED) {
                for id in 0..kernel.assertions() {
                    let _ = kernel.verify(id);
                }
            }
        }
    }
}

#[test]
fn records_stay_inside_one_page() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..2500 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = db(&src);
    let bytes = pack(&db, &everything(&db)).unwrap();
    assert!(bytes.len() > 4 * library::CHUNK);
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let directory = u32_at(16);
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
    for id in 0..2500 {
        let start = u32_at(directory + 4 * id);
        let end = start + 2 + bytes[start + 1] as usize + 5;
        assert_eq!(start / library::CHUNK, (end - 1) / library::CHUNK, "{id}");
        assert_eq!(kernel.verify(id), Ok(()));
    }
}

fn pack_command(database: &str, root: &str, output: &Path) -> Option<i32> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let status = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("pack")
        .arg(corpus.join(database))
        .arg(root)
        .arg("-o")
        .arg(output)
        .status()
        .expect("mcpack runs");
    status.code()
}

#[test]
fn pack_command_writes_a_library_the_kernel_accepts() {
    let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("demo0.mclib");
    assert_eq!(pack_command("demo0.mm", "th1", &output), Some(0));
    let bytes = std::fs::read(&output).expect("the library was written");
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).expect("library loads");
    let last = kernel.assertions() - 1;
    assert_eq!(kernel.label(last).unwrap(), b"th1");
    assert_eq!(kernel.verify(last), Ok(()));
}

#[test]
fn pack_command_writes_nothing_when_a_proof_fails() {
    let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join("demo0-bad1.mclib");
    assert_eq!(pack_command("demo0-bad1.mm", "th1", &output), Some(1));
    assert!(!output.exists());
}
