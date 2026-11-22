//! The store module as the app will link it, with no STORE_VAR override.

use std::path::Path;
use std::process::Command;

#[test]
fn the_default_build_names_its_variable_mcstat() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store_name.bin");
    let run = Command::new("sh")
        .arg(root.join("../../scripts/spasm.sh"))
        .args(["-L", "-I"])
        .arg(root.join("../../app/src"))
        .arg(root.join("tests/routines/store_name.z80"))
        .arg(&out)
        .output()
        .expect("the assembler runs");
    assert!(
        run.status.success(),
        "spasm: {}",
        String::from_utf8_lossy(&run.stdout)
    );
    let labels = std::fs::read_to_string(out.with_extension("lab")).unwrap();
    let at = labels
        .lines()
        .find_map(|l| l.strip_prefix("STORE_NAME_TEXT = $"))
        .map(|v| usize::from_str_radix(v, 16).unwrap())
        .expect("a label store_name_text");
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(&bytes[at..at + 9], b"\x15MCSTAT\0\0");
}
