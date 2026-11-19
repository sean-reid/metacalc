#[path = "cases/raw.rs"]
mod raw;
#[path = "cases/records.rs"]
mod records;
#[path = "cases/references.rs"]
mod references;
#[path = "cases/symbols.rs"]
mod symbols;

use mcpack::library::{self, Kernel, KernelError, Limits, PackError, Reference, Step, crc32, pack};
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

struct Fields {
    hyps: usize,
    dv: usize,
    extra_dv: usize,
    refs: usize,
}

/// Offsets inside a record, read from the documented layout; the last two only exist for a theorem.
fn fields(bytes: &[u8], id: usize) -> Fields {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = u32_at(u32_at(16) + 4 * id);
    at += 2 + bytes[at + 1] as usize;
    let hyps = at + 1;
    let count = bytes[at];
    at += 1;
    for _ in 0..count {
        at += if bytes[at] == 0 {
            2
        } else {
            2 + bytes[at + 1] as usize
        };
    }
    let dv = at + 2;
    at += 2 + 2 * u16_at(at);
    at += 1 + bytes[at] as usize;
    let extra_dv = at + 2;
    let refs = extra_dv + 2 * u16_at(at) + 2;
    Fields {
        hyps,
        dv,
        extra_dv,
        refs,
    }
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
fn kernel_names_each_way_a_proof_can_fail() {
    for (theorem, expected) in [
        (
            "$d x y $. t $p |- R x x $= vx vy vy vx ax-d ax-m $.",
            KernelError::HypothesisMismatch,
        ),
        (
            "$d x y $. t $p |- R x y $= vx ax-d $.",
            KernelError::StackUnderflow,
        ),
        (
            "$d x y $. t $p |- R x y $= vx vy ax-d vx $.",
            KernelError::StackNotSingleton,
        ),
        (
            "$d x y $. t $p |- R y x $= vx vy ax-d $.",
            KernelError::WrongConclusion,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ABCD $.",
            KernelError::BadBackReference,
        ),
        (
            "$d x y $. t $p |- R x y $= ( ax-d ) ZABC $.",
            KernelError::StackUnderflow,
        ),
    ] {
        let db = db(&format!("{DB} ${{ {theorem} $}}"));
        let id = db.find("t").unwrap();
        assert!(Verifier::new(&db).verify(id).is_err(), "{theorem}");
        let bytes = pack(&db, &everything(&db)).unwrap();
        assert_eq!(check(&bytes, number(&db, "t")), Err(expected), "{theorem}");
    }
}

#[test]
fn a_reference_to_a_dummy_that_is_not_a_variable_is_refused() {
    let db = db(DB);
    let dummy = number(&db, "dummy");
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let refs = fields(&bytes, dummy).refs;
    assert_eq!(
        bytes[refs + 1],
        0x80,
        "the first reference is the dummy variable"
    );
    for not_a_variable in [0, 0xFE] {
        bytes[refs] = not_a_variable;
        reseal(&mut bytes);
        assert_eq!(check(&bytes, dummy), Err(KernelError::Malformed));
    }
}

#[test]
fn a_header_that_breaks_the_layout_does_not_load() {
    let db = db(DB);
    let original = pack(&db, &everything(&db)).unwrap();
    let variable_typecode = {
        let mut at = library::HEADER_LEN;
        for _ in 0..original[12] {
            at += 2 + original[at + 1] as usize;
        }
        at
    };
    for (at, value) in [(0, b'X'), (16, original[16] + 1), (variable_typecode, 0xFF)] {
        let mut bytes = original.clone();
        bytes[at] = value;
        reseal(&mut bytes);
        let loaded = Kernel::new(&bytes, Limits::UNBOUNDED);
        assert!(matches!(loaded, Err(KernelError::Malformed)), "byte {at}");
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
        let refs = fields(&bytes, good).refs;
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
    let extra = fields(&bytes, dummy).extra_dv;
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
    let extra = fields(&bytes, both).extra_dv;
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
fn a_mandatory_pair_needs_hypotheses_for_both_variables() {
    let db = db(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let dv = fields(&bytes, axiom).dv;
    bytes[dv + 1] += 1;
    reseal(&mut bytes);
    assert_eq!(check(&bytes, axiom), Err(KernelError::BadRecord));
    assert_eq!(check(&bytes, good), Err(KernelError::BadRecord));
}

#[test]
fn an_essential_hypothesis_listed_before_its_variables_is_refused_wherever_it_is_used() {
    let db = db(DB);
    let (axiom, dummy) = (number(&db, "ax-m"), number(&db, "dummy"));
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let hyps = fields(&bytes, axiom).hyps;
    let floats = bytes[hyps..hyps + 4].to_vec();
    let essential = bytes[hyps + 4..hyps + 10].to_vec();
    assert_eq!(
        (floats[0], floats[2], essential[0], essential[1]),
        (0, 0, 1, 4)
    );
    bytes[hyps..hyps + 6].copy_from_slice(&essential);
    bytes[hyps + 6..hyps + 10].copy_from_slice(&floats);
    reseal(&mut bytes);
    assert_eq!(check(&bytes, number(&db, "good")), Ok(()));
    assert_eq!(check(&bytes, axiom), Err(KernelError::BadRecord));
    assert_eq!(check(&bytes, dummy), Err(KernelError::BadRecord));
}

#[test]
fn a_float_for_a_variable_the_assertion_never_uses_is_refused_wherever_it_is_used() {
    let db = db(DB);
    let (axiom, good) = (number(&db, "ax-d"), number(&db, "good"));
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let next = u32_at(u32_at(16) + 4 * (axiom + 1));
    assert_eq!(
        bytes[next - 1],
        bytes[next - 2] + 1,
        "the conclusion ends in y"
    );
    bytes[next - 1] = bytes[next - 2];
    reseal(&mut bytes);
    assert_eq!(check(&bytes, axiom), Err(KernelError::BadRecord));
    assert_eq!(check(&bytes, good), Err(KernelError::BadRecord));
}

#[test]
fn a_failed_check_leaves_nothing_behind_for_the_next_one() {
    let db = db(DB);
    let bytes = pack(&db, &everything(&db)).unwrap();
    let limits = Limits {
        stack_syms: 8,
        ..Limits::UNBOUNDED
    };
    let mut kernel = Kernel::new(&bytes, limits).unwrap();
    assert_eq!(
        kernel.verify(number(&db, "nodummy")),
        Err(KernelError::DisjointViolation)
    );
    assert_eq!(kernel.verify(number(&db, "good")), Ok(()));
}

#[derive(Clone, Copy)]
enum Tok {
    Var(usize),
    Const,
}

/// A library whose theorem `t` applies an axiom of `terms.len()` variables, all pairwise disjoint,
/// to one term per variable; `nv` variables w0.. are free to use in the terms and the theorem
/// declares all of them disjoint. Without `checked` neither declares a pair, and the proof does
/// the same steps with no disjoint check to make.
fn disjoint_source(terms: &[Vec<Tok>], nv: usize, checked: bool) -> String {
    let zs: Vec<String> = (0..terms.len()).map(|i| format!("z{i}")).collect();
    let ws: Vec<String> = (0..nv).map(|i| format!("w{i}")).collect();
    let mut src = format!(
        "$c wff |- R k $. $v p q {} {} $.\n\
         fp $f wff p $. fq $f wff q $. wk $a wff k $. wc $a wff p q $.\n",
        zs.join(" "),
        ws.join(" ")
    );
    for var in zs.iter().chain(&ws) {
        src += &format!("f{var} $f wff {var} $.\n");
    }
    let dv = |vars: &[String]| {
        if checked && vars.len() > 1 {
            format!("$d {} $.", vars.join(" "))
        } else {
            String::new()
        }
    };
    src += &format!("${{ {} ax-d $a |- R {} $. $}}\n", dv(&zs), zs.join(" "));
    let (mut expr, mut proof) = (Vec::new(), Vec::new());
    for term in terms {
        for (i, tok) in term.iter().enumerate() {
            let (sym, label) = match tok {
                Tok::Var(v) => (format!("w{v}"), format!("fw{v}")),
                Tok::Const => ("k".to_string(), "wk".to_string()),
            };
            expr.push(sym);
            proof.push(label);
            if i > 0 {
                proof.push("wc".into());
            }
        }
    }
    proof.push("ax-d".into());
    src += &format!(
        "${{ {} t $p |- R {} $= {} $. $}}\n",
        dv(&ws),
        expr.join(" "),
        proof.join(" ")
    );
    src
}

/// The units of work `t` costs when `terms` is `[x vars, y vars]`, every variable distinct, with
/// `xk` and `yk` constants added to the two terms.
fn two_term_work(x: usize, y: usize, xk: usize, yk: usize) -> u32 {
    let n = (x + y) * (x + y - 1) / 2;
    (1 + (x + xk) + x * (y + yk) + x * y * (1 + n)) as u32
}

/// The units the proof of `t` charges for substitution and copying, from the tokens of each term
/// and how many of them are constants: 2 for a variable pushed, 4 for `wk` applied, for each `wc`
/// that joins a token to the `i` before it 3 symbols read and `i + 2` written, and for the
/// conclusion `|- R z0 ..` its symbols read and written.
fn term_work(tokens: usize, consts: usize) -> u32 {
    (2 * (tokens - consts) + 4 * consts + (1..tokens).map(|i| i + 5).sum::<usize>()) as u32
}

fn copy_work(terms: &[(usize, usize)]) -> u32 {
    let tokens: usize = terms.iter().map(|t| t.0).sum();
    terms.iter().map(|&(n, k)| term_work(n, k)).sum::<u32>() + (4 + terms.len() + tokens) as u32
}

/// `x` variables then `xk` constants, and `y` other variables then `yk` constants.
fn two_terms(x: usize, y: usize, xk: usize, yk: usize) -> Vec<Vec<Tok>> {
    let mut left: Vec<Tok> = (0..x).map(Tok::Var).collect();
    left.extend(vec![Tok::Const; xk]);
    let mut right: Vec<Tok> = (x..x + y).map(Tok::Var).collect();
    right.extend(vec![Tok::Const; yk]);
    vec![left, right]
}

/// The units the checks on a reference list of `refs` entries charge: one per entry and one per pair.
fn duplicate_work(refs: usize) -> u32 {
    (refs * (refs + 1) / 2) as u32
}

/// The entries in the reference list of the theorem `t`.
fn reference_count(bytes: &[u8], db: &Database) -> usize {
    let library = library::decode(bytes).unwrap();
    library.assertions[number(db, "t")].references.len()
}

/// The outcome of `t`, the work it charged if it was accepted, and the share of it the reference
/// list checks charged.
fn work_of(terms: &[Vec<Tok>], nv: usize) -> (Result<(), KernelError>, Option<u32>, u32) {
    let db = db(&disjoint_source(terms, nv, true));
    let bytes = pack(&db, &everything(&db)).unwrap();
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
    let result = kernel.verify(number(&db, "t"));
    let duplicates = duplicate_work(reference_count(&bytes, &db));
    (result, result.is_ok().then(|| kernel.work()), duplicates)
}

#[test]
fn the_work_of_a_disjoint_check_is_charged_as_documented() {
    for (x, y, xk, yk) in [
        (1, 1, 0, 0),
        (2, 3, 0, 0),
        (2, 3, 5, 0),
        (2, 3, 0, 5),
        (4, 4, 7, 9),
    ] {
        let (result, work, duplicates) = work_of(&two_terms(x, y, xk, yk), x + y);
        assert_eq!(result, Ok(()));
        let expected =
            duplicates + two_term_work(x, y, xk, yk) + copy_work(&[(x + xk, xk), (y + yk, yk)]);
        assert_eq!(work, Some(expected), "{x} {y} {xk} {yk}");
    }
}

#[test]
fn a_disjoint_check_stops_at_the_work_limit() {
    assert_eq!(library::WORK_LIMIT, 225000);
    let at = (31, 9, 36, 51);
    let (result, work, duplicates) = work_of(&two_terms(at.0, at.1, at.2, at.3), 40);
    assert_eq!(
        duplicates
            + two_term_work(at.0, at.1, at.2, at.3)
            + copy_work(&[(at.0 + at.2, at.2), (at.1 + at.3, at.3)]),
        library::WORK_LIMIT
    );
    assert_eq!((result, work), (Ok(()), Some(library::WORK_LIMIT)));
    let (result, ..) = work_of(&two_terms(at.0, at.1, at.2 + 1, at.3), 40);
    assert_eq!(result, Err(KernelError::WorkLimit));
    let (result, ..) = work_of(&two_terms(4, 60, 0, 0), 64);
    assert_eq!(result, Err(KernelError::WorkLimit));
}

#[test]
fn the_work_count_starts_again_with_each_call() {
    let db = db(&disjoint_source(&two_terms(3, 3, 0, 0), 6, true));
    let bytes = pack(&db, &everything(&db)).unwrap();
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
    let id = number(&db, "t");
    let duplicates = duplicate_work(reference_count(&bytes, &db));
    for _ in 0..3 {
        assert_eq!(kernel.verify(id), Ok(()));
        assert_eq!(
            kernel.work(),
            duplicates + two_term_work(3, 3, 0, 0) + copy_work(&[(3, 0), (3, 0)])
        );
    }
    assert_eq!(kernel.verify(number(&db, "ax-d")), Ok(()));
    assert_eq!(kernel.work(), 0);
}

/// A theorem `t` that applies an axiom with the pair x y disjoint once for each `(x, y)` term
/// pair and joins the results; w0.. `listed` of them declared pairwise disjoint, `nv` available.
fn joined_source(applies: &[(Vec<Tok>, Vec<Tok>)], nv: usize, listed: usize) -> String {
    let ws: Vec<String> = (0..nv).map(|i| format!("w{i}")).collect();
    let mut src = format!(
        "$c wff |- R k $. $v p q x y {} $.\n\
         fp $f wff p $. fq $f wff q $. fx $f wff x $. fy $f wff y $.\n\
         wk $a wff k $. wc $a wff p q $. ax-gen $a |- R x y $.\n\
         ${{ $d x y $. ax-e.1 $e |- R x y $. ax-e $a |- R k $. $}}\n\
         ${{ j.1 $e |- R k $. j.2 $e |- R k $. ax-j $a |- R k $. $}}\n",
        ws.join(" ")
    );
    for var in &ws {
        src += &format!("f{var} $f wff {var} $.\n");
    }
    let term = |tokens: &[Tok]| {
        let mut proof = Vec::new();
        for (i, tok) in tokens.iter().enumerate() {
            proof.push(match tok {
                Tok::Var(v) => format!("fw{v}"),
                Tok::Const => "wk".to_string(),
            });
            if i > 0 {
                proof.push("wc".into());
            }
        }
        proof.join(" ")
    };
    let mut proof: Vec<String> = Vec::new();
    for (i, (x, y)) in applies.iter().enumerate() {
        let (x, y) = (term(x), term(y));
        proof.push(format!("{x} {y} {x} {y} ax-gen ax-e"));
        if i > 0 {
            proof.push("ax-j".into());
        }
    }
    let dv = if listed > 1 {
        format!("$d {} $.", ws[..listed].join(" "))
    } else {
        String::new()
    };
    src += &format!("${{ {dv} t $p |- R k $= {} $. $}}\n", proof.join(" "));
    src
}

type Applies = Vec<(Vec<Tok>, Vec<Tok>)>;

fn pair(terms: Vec<Vec<Tok>>) -> (Vec<Tok>, Vec<Tok>) {
    let mut terms = terms.into_iter();
    (terms.next().unwrap(), terms.next().unwrap())
}

/// Three proofs of `|- R k` that apply the disjoint axiom twice. The second apply is refused for a
/// variable outside the 35 listed; its comparison charge is the last the limit allows in one case
/// and one unit too many in the other.
fn joined_cases() -> Vec<(&'static str, Applies, usize, usize, KernelError)> {
    let var = |i| (vec![Tok::Var(0)], vec![Tok::Var(i)]);
    vec![
        (
            "each apply alone is under the limit",
            vec![pair(two_terms(27, 8, 0, 0)), pair(two_terms(27, 8, 0, 0))],
            35,
            35,
            KernelError::WorkLimit,
        ),
        (
            "a violation whose comparison charge is the last the limit allows",
            vec![pair(two_terms(20, 15, 61, 158)), var(35)],
            36,
            35,
            KernelError::DisjointViolation,
        ),
        (
            "a violation whose comparison charge crosses the limit",
            vec![pair(two_terms(12, 23, 114, 164)), var(35)],
            36,
            35,
            KernelError::WorkLimit,
        ),
    ]
}

#[test]
fn the_work_of_all_the_applies_of_a_call_is_added_up() {
    let alone = db(&joined_source(&[pair(two_terms(27, 8, 0, 0))], 35, 35));
    let bytes = pack(&alone, &everything(&alone)).unwrap();
    let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
    assert_eq!(kernel.verify(number(&alone, "t")), Ok(()));
    assert!(kernel.work() < library::WORK_LIMIT && 2 * kernel.work() > library::WORK_LIMIT);
    for (name, applies, nv, listed, expected) in joined_cases() {
        let db = db(&joined_source(&applies, nv, listed));
        let bytes = pack(&db, &everything(&db)).unwrap();
        let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
        assert_eq!(kernel.verify(number(&db, "t")), Err(expected), "{name}");
    }
}

#[test]
fn a_record_that_crosses_a_page_is_refused() {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..700 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let db = db(&src);
    let mut bytes = pack(&db, &everything(&db)).unwrap();
    let u32_at =
        |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let directory = u32_at(&bytes, 16) as usize;
    let offset = |bytes: &[u8], id: usize| u32_at(bytes, directory + 4 * id) as usize;
    let first = (0..700)
        .find(|&id| offset(&bytes, id) >= library::CHUNK)
        .unwrap();
    let (start, end) = (offset(&bytes, first), offset(&bytes, first + 1));
    assert_eq!(start, library::CHUNK, "the packer pads to the page");
    bytes.copy_within(start..end, start - 3);
    let entry = directory + 4 * first;
    bytes[entry..entry + 4].copy_from_slice(&(start as u32 - 3).to_le_bytes());
    reseal(&mut bytes);
    assert_eq!(check(&bytes, first - 1), Ok(()));
    assert_eq!(check(&bytes, first), Err(KernelError::Malformed));
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
fn compressed_step_numbers_past_one_byte_survive_packing() {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let (mut labels, mut proof) = (String::new(), letters(0));
    for i in 0..250 {
        src += &format!("n{i} $a wff T $.\n");
        labels += &format!("n{i} ");
        if i > 0 {
            proof += &letters(i);
            proof += &letters(250);
        }
    }
    src += &format!("big $p wff T $= ( {labels}drop ) {proof} $.\n");
    let db = db(&src);
    Verifier::new(&db)
        .verify(db.find("big").unwrap())
        .expect("the proof is valid");
    let bytes = pack(&db, &everything(&db)).unwrap();
    assert_eq!(check(&bytes, number(&db, "big")), Ok(()));
}

#[test]
fn a_compressed_proof_the_reference_calls_malformed_does_not_pack() {
    for proof in ["AABUZCB", "AABZCBU"] {
        let src = format!(
            "$c ( ) -> wff $. $v p q $. wp $f wff p $. wq $f wff q $.
             wi $a wff ( p -> q ) $.
             twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) {proof} $."
        );
        let db = db(&src);
        assert!(
            Verifier::new(&db)
                .verify(db.find("twice").unwrap())
                .is_err()
        );
        let packed = pack(&db, &everything(&db));
        assert_eq!(
            packed,
            Err(PackError::BrokenProof("twice".into())),
            "{proof}"
        );
    }
}

#[test]
fn a_variable_with_two_typecodes_does_not_pack_and_the_refusal_names_it() {
    let src = "$c wff class |- $. $v x y $.
        ${ wy $f wff y $. ay $a |- y $. $}
        ${ cy $f wff y $. by $a |- y $. $}
        ${ wx $f wff x $. ax $a |- x $. $}
        ${ cx $f class x $. bx $a |- x $. $}";
    let db = db(src);
    assert_eq!(
        pack(&db, &everything(&db)),
        Err(PackError::VariableTypecodes("x".into()))
    );
    let before_clash: Vec<StmtId> = (0..db.find("wx").unwrap()).collect();
    assert!(pack(&db, &before_clash).is_ok());
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
    let _ = std::fs::remove_file(&output);
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
    let _ = std::fs::remove_file(&output);
    assert_eq!(pack_command("demo0-bad1.mm", "th1", &output), Some(1));
    assert!(!output.exists());
}

#[test]
fn a_reference_list_that_is_not_canonical_is_refused() {
    for case in references::cases() {
        assert_eq!(check(&case.bytes, case.id), case.expected, "{}", case.name);
    }
}

#[test]
fn the_reference_list_checks_and_the_proof_symbols_are_charged() {
    for entries in [3, 4, 100, 511] {
        let bytes = references::pack_source(&references::long_source(entries));
        let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).unwrap();
        assert_eq!(kernel.verify(entries), Ok(()), "{entries}");
        assert_eq!(
            kernel.work() as usize,
            references::long_work(entries),
            "{entries}"
        );
    }
}

const SHUFFLED: &str = "
$c wff |- R $.
ax-pad $a |- R $.
${ s.1 $e |- R $. ax-t $a |- R $. $}
${ j.1 $e |- R $. j.2 $e |- R $. ax-j $a |- R $. $}
ax-s $a |- R $.
th $p |- R $= ( ax-pad ax-t ax-j ax-s ) DZBEC $.
";

#[test]
fn the_packer_lists_references_in_first_use_order_and_drops_unused_ones() {
    let db = db(SHUFFLED);
    let bytes = pack(&db, &everything(&db)).unwrap();
    let th = number(&db, "th");
    assert_eq!(check(&bytes, th), Ok(()));
    let library = library::decode(&bytes).unwrap();
    let theorem = &library.assertions[th];
    let named = |label| Reference::Assertion(number(&db, label));
    assert_eq!(
        theorem.references,
        [named("ax-s"), named("ax-t"), named("ax-j")]
    );
    let steps = [
        Step::Number(0),
        Step::Save,
        Step::Number(1),
        Step::Number(3),
        Step::Number(2),
    ];
    assert_eq!(theorem.proof, steps);
    let text = library::export(&bytes).unwrap();
    let exported = mm::parse(text.into_bytes()).unwrap();
    assert_eq!(
        Verifier::new(&exported).verify(exported.find("th").unwrap()),
        Ok(())
    );
}

#[test]
fn step_numbers_of_620_and_more_survive_packing_and_the_reference_verifier() {
    let db = db(&references::saved_source());
    Verifier::new(&db)
        .verify(db.find("big").unwrap())
        .expect("the proof is valid");
    let bytes = pack(&db, &everything(&db)).unwrap();
    assert_eq!(check(&bytes, number(&db, "big")), Ok(()));
    let library = library::decode(&bytes).unwrap();
    let steps = &library.assertions[number(&db, "big")].proof;
    let largest = steps.iter().filter_map(|s| match s {
        Step::Number(n) => Some(*n),
        Step::Save => None,
    });
    assert!(largest.max().unwrap() >= 620);
}

#[test]
fn structured_edits_of_reference_lists_are_judged_by_the_rule() {
    let base = references::base();
    let mut changed_and_accepted = 0;
    for mutant in references::mutants() {
        let got = check(&mutant.bytes, mutant.id);
        if let Some(expected) = mutant.expected {
            assert_eq!(got, expected, "{}", mutant.name);
        }
        if got.is_ok() && mutant.bytes != base {
            changed_and_accepted += 1;
        }
    }
    assert!(changed_and_accepted > 0);
}
