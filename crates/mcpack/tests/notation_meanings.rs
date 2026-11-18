//! Checks that the notation table of every shipped library explains every symbol.
//!
//! Run where set.mm is present: `SETMM=.setmm/set.mm cargo test -p mcpack --test notation_meanings -- --ignored`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn mcpack(args: &[&Path]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .args(args)
        .output()
        .expect("mcpack runs")
}

#[test]
#[ignore = "needs set.mm; run through the libraries CI job"]
fn every_symbol_in_a_shipped_notation_table_has_a_meaning() {
    let setmm = PathBuf::from(std::env::var_os("SETMM").expect("SETMM names set.mm"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("notation-meanings");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let mut shipped = 0;
    for entry in std::fs::read_dir(&root).unwrap() {
        let manifest = entry.unwrap().path();
        if manifest.extension().is_none_or(|e| e != "toml") || manifest.ends_with("source.toml") {
            continue;
        }
        shipped += 1;
        let name = manifest.file_stem().unwrap();
        let library = out.join(name).with_extension("mclib");
        let page = out.join(name).with_extension("md");
        let built = mcpack(&[
            Path::new("build"),
            &manifest,
            Path::new("--database"),
            &setmm,
            Path::new("-o"),
            &library,
        ]);
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let doc = mcpack(&[
            Path::new("doc"),
            &manifest,
            Path::new("--library"),
            &library,
            Path::new("-o"),
            &page,
            Path::new("--require-meaning"),
        ]);
        assert!(
            doc.status.success(),
            "{}",
            String::from_utf8_lossy(&doc.stderr)
        );
    }
    assert_eq!(shipped, 3);
}
