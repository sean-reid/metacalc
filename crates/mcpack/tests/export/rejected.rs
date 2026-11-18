use super::refusals::{packed, patched, record};
use super::*;
use mcpack::library::{Export, export_accepted};
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
        assert!(export_accepted(bytes).is_err());
        assert!(matches!(
            strict,
            ExportError::Check(_) | ExportError::Read(_)
        ));
    }
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
