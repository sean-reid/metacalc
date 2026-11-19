use super::refusals::{packed, patched, record};
use super::*;
use mcpack::library::{Export, KernelError, Limits, export_accepted, export_within};
use mcpack::mm::ProofError;

const SRC: &str = "$c |- wff R S $. $v x $. vx $f wff x $.
    ax-r $a |- R x $.
    ax-s $a |- S x $.
    ${ m.1 $e |- R x $. ax-rs $a |- S x $. $}
    t1 $p |- R x $= vx ax-r $.
    t2 $p |- S x $= vx vx t1 ax-rs $.
    t3 $p |- S x $= vx vx ax-r ax-rs $.
    t4 $p |- R x $= vx ax-r $.
    t5 $p |- S x $= vx vx ax-r ax-rs $.";

const LABELS: [&str; 8] = ["ax-r", "ax-s", "ax-rs", "t1", "t2", "t3", "t4", "t5"];

/// The library with the first reference of each named theorem pointed at `ax-s`, which no proof
/// of it can use.
fn damaged(original: &[u8], labels: &[&str]) -> Vec<u8> {
    let ax_s = 1u16.to_le_bytes();
    patched(original.to_vec(), |bytes| {
        for label in labels {
            let id = LABELS.iter().position(|l| l == label).unwrap();
            let refs = record(original, id).refs;
            bytes[refs..refs + 2].copy_from_slice(&ax_s);
        }
    })
}

fn rejected_labels(export: &Export) -> Vec<&str> {
    export.rejected.iter().map(|(l, _)| l.as_str()).collect()
}

/// The clean export with each rejected theorem's proof replaced by `?`.
fn with_unknown_proofs(clean: &str, labels: &[&str]) -> String {
    let mut blocks: Vec<String> = clean.split_inclusive("$}\n").map(String::from).collect();
    for block in &mut blocks {
        let proved = labels
            .iter()
            .any(|l| block.contains(&format!("\n  {l} $p ")));
        if proved {
            let end = block.find("$=").unwrap() + 2;
            block.truncate(end);
            block.push_str(" ? $.\n$}\n");
        }
    }
    blocks.concat()
}

#[test]
fn a_rejected_theorem_keeps_its_statement_and_gives_up_its_proof() {
    let original = packed(SRC);
    let clean = export(&original).unwrap();
    for rejected in [
        vec!["t1"],
        vec!["t3"],
        vec!["t5"],
        vec!["t3", "t5"],
        vec!["t1", "t5"],
    ] {
        let bytes = damaged(&original, &rejected);
        let out = export_accepted(&bytes).unwrap();
        assert_eq!(rejected_labels(&out), rejected);
        assert_eq!(
            out.text,
            with_unknown_proofs(&clean, &rejected),
            "{rejected:?}"
        );
        let Err(CheckError::Assertions(failed)) = check(&bytes) else {
            panic!("the kernel accepts {rejected:?}");
        };
        assert_eq!(out.rejected, failed);
        assert_eq!(
            export(&bytes),
            Err(ExportError::Check(CheckError::Assertions(failed)))
        );
    }
}

#[test]
fn the_text_verifies_except_for_the_rejected_proofs() {
    let original = packed(SRC);
    for rejected in [vec!["t1"], vec!["t3"], vec!["t5"], vec!["t3", "t5"]] {
        let out = export_accepted(&damaged(&original, &rejected)).unwrap();
        let db = mm::parse(out.text.into_bytes()).expect("the export parses");
        let mut verifier = Verifier::new(&db);
        let incomplete: Vec<String> = db
            .theorems()
            .filter(|&id| match verifier.verify(id) {
                Ok(()) => false,
                Err(ProofError::Incomplete) => true,
                Err(e) => panic!("{}: {e:?}", db.stmt(id).label),
            })
            .map(|id| db.stmt(id).label.clone())
            .collect();
        assert_eq!(incomplete, rejected);
    }
}

#[test]
fn a_theorem_that_cites_a_rejected_one_is_accepted_and_exported_as_stored() {
    let original = packed(SRC);
    let clean = export(&original).unwrap();
    let out = export_accepted(&damaged(&original, &["t1"])).unwrap();
    assert_eq!(rejected_labels(&out), ["t1"]);
    let block = |text: &str| {
        text.split_inclusive("$}\n")
            .find(|b| b.contains("\n  t2 $p "))
            .unwrap()
            .to_string()
    };
    assert_eq!(block(&out.text), block(&clean));
}

#[test]
fn a_clean_library_exports_the_same_bytes_either_way() {
    let original = packed(SRC);
    let out = export_accepted(&original).unwrap();
    assert!(out.rejected.is_empty());
    assert_eq!(Ok(out.text), export(&original));
    let mut packed_corpus = 0;
    for name in [
        "anatomy.mm",
        "demo0.mm",
        "miu.mm",
        "emptyline.mm",
        "big-unifier.mm",
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/corpus")
            .join(name);
        let db = mm::parse(std::fs::read(path).unwrap()).unwrap();
        let Ok(bytes) = pack(&db, &all(&db)) else {
            continue;
        };
        packed_corpus += 1;
        let out = export_accepted(&bytes).unwrap();
        assert!(out.rejected.is_empty(), "{name}");
        assert_eq!(Ok(out.text), export(&bytes), "{name}");
    }
    assert!(packed_corpus >= 2, "{packed_corpus} corpus files pack");
}

#[test]
fn a_library_that_does_not_decode_or_export_fails_either_way() {
    let original = damaged(&packed(SRC), &["t3"]);
    let mut short = original.clone();
    short.truncate(40);
    let mut symbol = original.clone();
    let at = super::refusals::symbol(&original, 1) + 1;
    symbol[at] = b'$';
    for bytes in [&short[..], &symbol[..], &[1, 2, 3][..]] {
        let strict = export(bytes).expect_err("export refuses");
        assert_eq!(export_accepted(bytes), Err(strict));
    }
}

#[test]
fn every_way_a_proof_can_be_wrong_is_tolerated_and_written_as_unknown() {
    let original = packed(SRC);
    let clean = export(&original).unwrap();
    let t3 = LABELS.iter().position(|l| *l == "t3").unwrap();
    let at = record(&original, t3);
    let refs = |bytes: &mut [u8], list: [u16; 2]| {
        for (slot, value) in list.into_iter().enumerate() {
            bytes[at.refs + 2 * slot..][..2].copy_from_slice(&value.to_le_bytes());
        }
    };
    type Damage<'a> = Box<dyn Fn(&mut [u8]) + 'a>;
    let cases: Vec<(KernelError, Damage)> = vec![
        (KernelError::ForwardReference, Box::new(|b| refs(b, [6, 2]))),
        (
            KernelError::LooseReference,
            Box::new(|b| b[at.proof..at.proof + 4].copy_from_slice(&[0, 0, 2, 1])),
        ),
        (
            KernelError::StackUnderflow,
            Box::new(|b| b[at.proof..at.proof + 4].copy_from_slice(&[1, 0, 1, 2])),
        ),
        (
            KernelError::BadBackReference,
            Box::new(|b| b[at.proof..at.proof + 4].copy_from_slice(&[0, 9, 1, 2])),
        ),
    ];
    for (error, damage) in cases {
        let bytes = patched(original.clone(), |b| damage(b));
        let out = export_accepted(&bytes).unwrap_or_else(|e| panic!("{error:?}: {e}"));
        assert_eq!(out.rejected, [("t3".to_string(), error)], "{error:?}");
        assert_eq!(out.text, with_unknown_proofs(&clean, &["t3"]), "{error:?}");
        let db = mm::parse(out.text.into_bytes()).unwrap();
        let mut verifier = Verifier::new(&db);
        let unproved: Vec<String> = db
            .theorems()
            .filter(|&id| verifier.verify(id) == Err(ProofError::Incomplete))
            .map(|id| db.stmt(id).label.clone())
            .collect();
        assert_eq!(unproved, ["t3"], "{error:?}");
        let others = db.theorems().filter(|&id| verifier.verify(id).is_ok());
        assert_eq!(others.count(), 4, "{error:?}");
    }
}

const SOUND: &str = "$c |- wff R $. $v x y $. vx $f wff x $. vy $f wff y $.
    ${ $d x y $. ax-d $a |- R x y $. $}
    ${ m.1 $e |- R x y $. ax-m $a |- R x x $. $}
    ${ $d x y $. good $p |- R x y $= vx vy ax-d $. $}";

/// Both entry points refuse the library with the same error, which names only refusals that are
/// not about a proof, and the tolerant one never gets as far as a rejected list.
fn refused_whole(name: &str, bytes: &[u8], want: KernelError) {
    let strict = export(bytes).expect_err("export refuses");
    assert_eq!(export_accepted(bytes), Err(strict), "{name}");
    let Err(ExportError::Check(CheckError::Assertions(failed))) = export(bytes) else {
        panic!("{name}: not a kernel refusal");
    };
    assert!(failed.iter().any(|(_, e)| *e == want), "{name}: {failed:?}");
    assert!(
        failed.iter().any(|(_, e)| !e.is_proof_rejection()),
        "{name}"
    );
}

#[test]
fn a_statement_the_kernel_cannot_trust_fails_the_whole_export() {
    let original = packed(SOUND);
    let variable = 3;
    let conclusion = |id| record(&original, id).conclusion;
    let hypothesis = record(&original, 1).hyps + 6;
    assert!(check(&original).is_ok());
    let cases = [
        ("a damaged conclusion", conclusion(2), variable),
        ("a damaged hypothesis", hypothesis, variable),
        (
            "a damaged $d",
            conclusion(0) - 2,
            original[conclusion(0) - 3],
        ),
    ];
    for (name, at, value) in cases {
        let bytes = patched(original.clone(), |b| b[at] = value);
        refused_whole(name, &bytes, KernelError::BadRecord);
    }
}

#[test]
fn a_theorem_citing_an_unsound_record_fails_the_whole_export() {
    let original = packed(SOUND);
    let at = record(&original, 0).conclusion;
    let bytes = patched(original, |b| b[at] = 3);
    let Err(ExportError::Check(CheckError::Assertions(failed))) = export_accepted(&bytes) else {
        panic!("the export went through");
    };
    let labels: Vec<&str> = failed.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, ["ax-d", "good"]);
    assert!(failed.iter().all(|(_, e)| *e == KernelError::BadRecord));
}

#[test]
fn a_proof_the_host_cannot_decide_fails_the_whole_export() {
    let tiny = Limits {
        stack_syms: 1,
        stack_entries: 1,
        saved_syms: 1,
        saved_entries: 1,
    };
    let bytes = packed(SRC);
    assert!(export_accepted(&bytes).is_ok());
    let Err(ExportError::Check(CheckError::Assertions(failed))) = export_within(&bytes, tiny)
    else {
        panic!("the export went through under a stack of one");
    };
    assert!(!failed.is_empty());
    assert!(failed.iter().all(|(_, e)| *e == KernelError::OutOfMemory));
    assert_eq!(
        export_within(&bytes, tiny),
        Err(ExportError::Check(CheckError::Assertions(failed)))
    );
}

#[test]
fn a_proof_past_the_host_limits_fails_the_whole_export() {
    let src = format!(
        "$c wff $. $v x $. vx $f wff x $. ax $a wff {} $. t $p wff x $= vx {} $.",
        "x ".repeat(250),
        "ax ".repeat(3)
    );
    let bytes = references::pack_source(&src);
    refused_whole(
        "a proof of millions of symbols",
        &bytes,
        KernelError::OutOfMemory,
    );
}

#[test]
fn a_proof_past_the_work_limit_fails_the_whole_export() {
    let bytes = references::pack_source(&references::long_source(512));
    refused_whole("512 reference entries", &bytes, KernelError::WorkLimit);
}

/// The shipped libraries, as scripts/build-libraries.sh writes them into `MC_LIBRARIES`, give the
/// same bytes from both entry points.
#[test]
#[ignore = "needs MC_LIBRARIES pointing at the built libraries"]
fn the_shipped_libraries_export_the_same_bytes_either_way() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES is set");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(Path::new(&dir).join(format!("{name}.mclib"))).unwrap();
        let out = export_accepted(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(out.rejected.is_empty(), "{name}");
        assert_eq!(Ok(out.text), export(&bytes), "{name}");
    }
}
