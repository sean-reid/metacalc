use super::*;
use mcpack::library::{Reference, read};
use mcpack::manifest::{Manifest, Pin};
use mcpack::page;

const LABELS: &str = "$c |- wff R $. $v x $. vx $f wff x $.
    one $a |- R $. two $a |- R $. six $a |- R $.";

const UNUSED: &str = "$c |- wff R $. $v x y $. vx $f wff x $. vy $f wff y $.
    ${ $d x y $. ax-d $a |- R x y $. $}";

const ESSENTIAL: &str = "$c |- wff R $. $v x y $. vx $f wff x $. vy $f wff y $.
    ${ m.1 $e |- R x y $. ax-m $a |- R x x $. $}";

const REFS: &str = "$c R wff |- $. $v x y z $.
    vx $f wff x $. vy $f wff y $. vz $f wff z $.
    ${ $d x y $. ax-d $a |- R x y $. $}
    ${ m.1 $e |- R x y $. ax-m $a |- R x y $. $}
    ${ n.1 $e |- R x y $. ax-n $a |- R x y $. $}
    ${ $d x y $. th $p |- R x y $= ( ax-d ax-m ax-n ) ABABABCDE $. $}";

fn packed(src: &str) -> Vec<u8> {
    let db = parse(src);
    pack(&db, &all(&db)).expect("it packs")
}

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

/// Positions inside a record, read from the documented layout.
struct Record {
    start: usize,
    hyps: usize,
    conclusion: usize,
    refs: usize,
    proof: usize,
}

fn record(bytes: &[u8], id: usize) -> Record {
    let start = u32_at(bytes, u32_at(bytes, 16) + 4 * id);
    let mut at = start + 2 + bytes[start + 1] as usize;
    let hyps = at + 1;
    let count = bytes[at];
    at += 1;
    for _ in 0..count {
        at += 2 + if bytes[at] == 0 {
            0
        } else {
            bytes[at + 1] as usize
        };
    }
    at += 2 + 2 * u16_at(bytes, at);
    let conclusion = at + 1;
    at += 1 + bytes[at] as usize;
    if bytes[start] == 0 {
        return Record {
            start,
            hyps,
            conclusion,
            refs: 0,
            proof: 0,
        };
    }
    let count_at = at + 2 + 2 * u16_at(bytes, at);
    let refs = count_at + 2;
    let proof = refs + 2 * u16_at(bytes, count_at) + 2;
    Record {
        start,
        hyps,
        conclusion,
        refs,
        proof,
    }
}

/// Where symbol `id`'s name length and name sit in the symbol table.
fn symbol(bytes: &[u8], id: usize) -> usize {
    let mut at = 20;
    for _ in 0..id {
        at += 2 + bytes[at + 1] as usize;
    }
    at + 1
}

fn patched(mut bytes: Vec<u8>, edit: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    edit(&mut bytes);
    reseal(&mut bytes);
    bytes
}

fn refused(bytes: &[u8]) -> ExportError {
    check(bytes).expect("the kernel accepts the library");
    export(bytes).expect_err("the export refuses it")
}

fn assertion_error(label: &str, why: &'static str) -> ExportError {
    ExportError::Assertion(label.into(), why)
}

#[test]
fn a_symbol_that_cannot_be_a_math_symbol_is_refused() {
    let original = packed(LABELS);
    let name = |id| symbol(&original, id) + 1;
    let (wff, r, x) = (1, 2, 3);
    let dollar = patched(original.clone(), |b| b[name(wff)] = b'$');
    assert_eq!(refused(&dollar), ExportError::Symbol("$ff".into()));
    let control = patched(original.clone(), |b| b[name(0)] = 1);
    assert_eq!(refused(&control), ExportError::Symbol("\u{1}-".into()));
    let repeat = patched(original.clone(), |b| b[name(x)] = b'R');
    assert_eq!(refused(&repeat), ExportError::Symbol("R".into()));
    let empty = patched(original.clone(), |b| {
        b[name(r) - 1] = 0;
        b.remove(name(r));
        b.insert(u32_at(&original, 16) - 1, 0);
    });
    assert_eq!(refused(&empty), ExportError::Symbol(String::new()));
}

#[test]
fn a_label_that_cannot_be_a_unique_label_is_refused() {
    let original = packed(LABELS);
    let label = |id| record(&original, id).start + 2;
    let invalid = patched(original.clone(), |b| b[label(0) + 1] = b'$');
    assert_eq!(refused(&invalid), ExportError::Label("o$e".into()));
    let repeat = patched(original.clone(), |b| {
        b.copy_within(label(0)..label(0) + 3, label(1))
    });
    assert_eq!(refused(&repeat), ExportError::Label("one".into()));
    let symbol = patched(original.clone(), |b| {
        b[label(2)..label(2) + 3].copy_from_slice(b"wff")
    });
    assert_eq!(refused(&symbol), ExportError::Label("wff".into()));
    let empty = patched(original.clone(), |b| {
        let at = record(&original, 2).start;
        b[at + 1] = 0;
        b.drain(at + 2..at + 5);
        let length = b.len() as u32;
        b[8..12].copy_from_slice(&length.to_le_bytes());
    });
    assert_eq!(refused(&empty), ExportError::Label(String::new()));
}

#[test]
fn a_float_for_a_variable_the_assertion_does_not_use_is_refused() {
    let original = packed(UNUSED);
    let conclusion = record(&original, 0).conclusion;
    let bytes = patched(original, |b| b[conclusion + 3] = b[conclusion + 2]);
    let why = "a floating hypothesis names a variable the assertion does not use";
    assert_eq!(refused(&bytes), assertion_error("ax-d", why));
}

#[test]
fn an_essential_hypothesis_that_uses_a_variable_before_its_float_is_refused() {
    let original = packed(ESSENTIAL);
    let bytes = swapped_essential(original);
    let why = "an essential hypothesis uses a variable before its float";
    assert_eq!(refused(&bytes), assertion_error("ax-m", why));
}

fn swapped_essential(original: Vec<u8>) -> Vec<u8> {
    let hyps = record(&original, 0).hyps;
    patched(original, |b| {
        let floats = b[hyps..hyps + 4].to_vec();
        let essential = b[hyps + 4..hyps + 10].to_vec();
        b[hyps..hyps + 6].copy_from_slice(&essential);
        b[hyps + 6..hyps + 10].copy_from_slice(&floats);
    })
}

#[test]
fn a_reference_list_a_block_cannot_write_is_refused_by_the_kernel() {
    let original = packed(REFS);
    let refs = record(&original, 3).refs;
    let (x, z) = (0x8000u16 + 3, 0x8000u16 + 5);
    let set = |slots: [(usize, u16); 2]| {
        patched(original.clone(), |b| {
            for (slot, value) in slots {
                b[refs + 2 * slot..][..2].copy_from_slice(&value.to_le_bytes());
            }
        })
    };
    for (slots, error) in [
        ([(1, x), (2, z)], KernelError::BadRecord),
        ([(1, z), (2, z)], KernelError::BadRecord),
        ([(1, 0), (2, z)], KernelError::BadRecord),
        ([(1, 3), (2, z)], KernelError::ForwardReference),
        ([(1, 4), (2, z)], KernelError::ForwardReference),
    ] {
        let bytes = set(slots);
        let failed = CheckError::Assertions(vec![("th".into(), error)]);
        assert_eq!(export(&bytes), Err(ExportError::Check(failed)), "{slots:?}");
    }
}

#[test]
fn a_library_the_kernel_rejects_is_not_exported() {
    let bad = "$c R wff |- $. $v x y $. vx $f wff x $. vy $f wff y $.
        ${ $d x y $. ax-d $a |- R x y $. $}
        bad $p |- R x y $= vx vy ax-d $.";
    let failed = CheckError::Assertions(vec![("bad".into(), KernelError::DisjointViolation)]);
    assert_eq!(export(&packed(bad)), Err(ExportError::Check(failed)));
    let mut damaged = packed(bad);
    *damaged.last_mut().unwrap() ^= 1;
    let load = CheckError::Load(KernelError::Malformed);
    assert_eq!(export(&damaged), Err(ExportError::Check(load)));
}

#[test]
fn a_label_that_is_not_utf8_cannot_be_read() {
    let original = packed(LABELS);
    let at = record(&original, 0).start + 2;
    let bytes = patched(original, |b| b[at] = 0xFF);
    assert_eq!(refused(&bytes), ExportError::Read(ReadError));
}

#[test]
fn a_proof_that_ends_inside_a_step_number_cannot_be_read() {
    let original = packed(REFS);
    let record = record(&original, 3);
    let last = record.proof + 9 - 1;
    let mut bytes = original;
    assert_eq!(bytes[last], 4, "the proof is nine one-byte steps");
    bytes[last] = 0xF0;
    assert_eq!(decode(&bytes), Err(ReadError));
    reseal(&mut bytes);
    assert!(matches!(
        export(&bytes),
        Err(ExportError::Check(CheckError::Assertions(_)))
    ));
}

#[test]
fn the_command_names_the_assertion_the_exporter_refuses() {
    let dir = temp("export-refused");
    let (library, output) = (dir.join("ess.mclib"), dir.join("ess.mm"));
    std::fs::write(&library, swapped_essential(packed(ESSENTIAL))).unwrap();
    let (code, stderr) = run_export(&library, &output);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        format!(
            "{}: ax-m: an essential hypothesis uses a variable before its float\n",
            library.display()
        )
    );
    assert!(!output.exists());
}

fn page_renders(bytes: &[u8]) -> bool {
    let manifest = "name = \"t\"\ntitle = \"T\"\ndescription = \"d\"\norder = 1\nroots = [\"a\"]\n";
    let pin = Pin {
        repository: "https://example.org/r".into(),
        commit: "abc".into(),
        path: "x.mm".into(),
        sha256: "0".repeat(64),
    };
    page::render(&Manifest::parse(manifest).unwrap(), &pin, bytes).is_ok()
}

#[test]
fn a_dummy_entry_that_names_no_variable_is_read_but_not_exported() {
    let original = packed(REFS);
    let refs = record(&original, 3).refs;
    for entry in [0x8000u16, 0x8000 + 300] {
        let bytes = patched(original.clone(), |b| {
            b[refs + 2..][..2].copy_from_slice(&entry.to_le_bytes());
        });
        assert_eq!(
            check(&bytes),
            Err(CheckError::Assertions(vec![(
                "th".into(),
                KernelError::Malformed
            )]))
        );
        assert!(read(&bytes).is_ok(), "{entry:x}");
        assert!(page_renders(&bytes), "{entry:x}");
        let library = decode(&bytes).unwrap();
        assert_eq!(
            library.assertions[3].references[1],
            Reference::Invalid(entry)
        );
        assert!(matches!(export(&bytes), Err(ExportError::Check(_))));
    }
}

const SPARE_PAIR: &str = "$c R wff |- $. $v x y z w $.
    vx $f wff x $. vy $f wff y $. vz $f wff z $. vw $f wff w $.
    ${ $d x y $. ax-d $a |- R x y $. $}
    ${ ez.1 $e |- R x z $. ax-ez $a |- R x y $. $}
    ax-w $a |- R w $.
    ${ $d x y $. $d x z $. $d z w $. t2 $p |- R x y $=
        ( ax-d ax-ez vz ) ABEAECD $. $}";

#[test]
fn a_pair_with_a_variable_the_block_has_no_float_for_is_written_and_verifies() {
    let original = packed(SPARE_PAIR);
    let t2 = record(&original, 3);
    assert_eq!(
        decode(&original).unwrap().assertions[3]
            .extra_disjoint
            .len(),
        2
    );
    let (y, z) = (4u8, 5u8);
    let pair = t2.refs - 4;
    assert_eq!(original[pair], z);
    let bytes = patched(original, |b| b[pair] = y);
    check(&bytes).expect("the kernel accepts a pair with a variable the block has no float for");
    let text = export(&bytes).expect("a $d needs only an active variable");
    let block = text.rsplit("${").next().unwrap();
    assert!(block.contains("  $d y w $.\n"), "{text}");
    assert!(!block.contains(" $f wff w $."), "{text}");
    let db = mm::parse(text.clone().into_bytes()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    let id = db.find("t2").unwrap();
    assert_eq!(Verifier::new(&db).verify(id), Ok(()));
}

const MIXED: &str = "
$c R wff |- ( ) -> $.
$v x y z p q $.
vx $f wff x $.
vy $f wff y $.
vz $f wff z $.
wp $f wff p $.
wq $f wff q $.
wi $a wff ( p -> q ) $.
${ $d x y $. ax-d $a |- R x y $. $}
${ m.1 $e |- R x y $. ax-m $a |- R x x $. $}
${ $d x z $. dummy $p |- R x x $= vx vz vx vz ax-d ax-m $. $}
${ t.1 $e |- R x z $. ax-t $a |- R x y $. $}
${ $d x y z $. both $p |- R x y $= vx vy vz vx vz ax-d ax-t $. $}
twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.
";

fn big() -> Vec<u8> {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let (mut labels, mut proof) = (String::new(), letters(0));
    for i in 0..250 {
        src += &format!("n{i} $a wff T $.\n");
        labels += &format!("n{i} ");
        if i > 0 {
            proof += &(letters(i) + &letters(250));
        }
    }
    src += &format!("big $p wff T $= ( {labels}drop ) {proof} $.\n");
    packed(&src)
}

#[test]
fn no_byte_change_makes_the_decoder_or_exporter_panic() {
    let mixed = packed(MIXED);
    check(&mixed).expect("the kernel accepts the library");
    let big = big();
    check(&big).expect("the kernel accepts the library");
    let from = record(&big, 250 + 1).proof - 4;
    for (original, first) in [(mixed, 8), (big, from)] {
        for at in first..original.len() {
            for value in [0x00, 0x80, 0xF0, 0xFF] {
                let mut bytes = original.clone();
                bytes[at] = value;
                let _ = decode(&bytes);
                let _ = read(&bytes);
                reseal(&mut bytes);
                let _ = decode(&bytes);
                let _ = read(&bytes);
                let _ = export(&bytes);
            }
        }
        for len in 0..original.len() {
            assert!(decode(&original[..len]).is_err());
            assert!(export(&original[..len]).is_err());
        }
    }
}
