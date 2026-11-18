#[path = "../build_support/spasm.rs"]
mod spasm;

use std::fs;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("spasm_rule");
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn assemble(name: &str, source: &str) -> (Result<(), String>, bool) {
    let src = scratch(&format!("{name}.z80"));
    let out = scratch(&format!("{name}.bin"));
    fs::write(&src, source).unwrap();
    let _ = fs::remove_file(&out);
    let result = spasm::assemble(&src, &out, &[], &[]);
    (result, out.exists())
}

#[test]
fn a_clean_routine_assembles() {
    let (result, wrote) = assemble("clean", "\tld b,3\n\tret\n");
    result.unwrap();
    assert!(wrote);
}

#[test]
fn an_undefined_label_is_an_error_and_leaves_no_output() {
    let (result, wrote) = assemble("undefined_label", "\tld b,nolabel\n\tjp missing\n");
    let why = result.expect_err("an undefined label must fail");
    assert!(why.contains("SE106"), "{why}");
    assert!(!wrote);
}

#[test]
fn a_truncated_value_is_an_error() {
    let (result, wrote) = assemble("truncation", "\tld a,$1FF\n");
    let why = result.expect_err("an 8-bit truncation must fail");
    assert!(why.contains("SW800"), "{why}");
    assert!(!wrote);
}

#[test]
fn a_negative_check_sees_the_text_of_the_refusal() {
    let (result, _) = assemble("refusal", " .error \"the specific refusal\"\n");
    assert!(result.unwrap_err().contains("the specific refusal"));
}
