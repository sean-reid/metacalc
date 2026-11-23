//! The checks the app's RAM map makes at assembly: the variables of APP_BUF follow one another
//! with no overlap and no gap and stay inside the buffer, and the Libraries screen's rows fit
//! APP_TEXT. Each check is shown to refuse by assembling a source that breaks it, and to let the
//! limit itself through.

#[path = "../build_support/spasm.rs"]
mod spasm;

use std::fs;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("app_ram_rules");
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The assembler's refusal, or `Ok` when the source assembles, for `source` with the app's include
/// path and the named defines.
fn assemble(name: &str, source: &str, defines: &[&str]) -> Result<(), String> {
    let (src, out) = (
        scratch(&format!("{name}.z80")),
        scratch(&format!("{name}.bin")),
    );
    fs::write(&src, source).unwrap();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (app, ui) = (format!("{root}/app/src"), format!("{root}/ui"));
    spasm::assemble(&src, &out, defines, &["-I", &app, "-I", root, "-I", &ui])
}

const CHAIN: &str = "the APP_BUF variables overlap, leave a gap, or run past APP_BUF_SIZE";

/// Source that adds one variable, `size` bytes at `offset`, when the chain of the app's variables
/// has so far ended at `next`.
fn after_the_chain(next: &str, offset: &str, size: u32) -> String {
    format!(
        "#include \"ram_map.inc\"\n#define APP_NEXT {next}\n APP_VAR(EXTRA, {offset}, {size})\n"
    )
}

#[test]
fn a_variable_that_starts_where_the_one_before_it_ends_and_ends_at_the_buffer_is_accepted() {
    assemble("chain_exact", &after_the_chain("$80", "$80", 128), &[]).unwrap();
}

#[test]
fn a_variable_that_runs_one_byte_past_the_buffer_is_refused() {
    let why = assemble("chain_past", &after_the_chain("$80", "$80", 129), &[]).unwrap_err();
    assert!(why.contains(CHAIN), "{why}");
}

#[test]
fn a_variable_that_overlaps_the_one_before_it_is_refused() {
    let why = assemble("chain_overlap", &after_the_chain("$80", "$7F", 4), &[]).unwrap_err();
    assert!(why.contains(CHAIN), "{why}");
}

#[test]
fn a_variable_that_leaves_a_gap_after_the_one_before_it_is_refused() {
    let why = assemble("chain_gap", &after_the_chain("$80", "$81", 4), &[]).unwrap_err();
    assert!(why.contains(CHAIN), "{why}");
}

const SLOTS: &str = "the Libraries screen's variables are longer than APP_TEXT";

/// The Libraries screen's file with `stages` stages, after the RAM map. Every other label the file
/// uses is undefined, so the assembler also reports those; the test looks for its own message.
fn libraries_with(stages: u32) -> String {
    format!(
        "#include \"ram_map.inc\"\nSTAGE_COUNT .equ {stages}\n#include \"screen_libraries.inc\"\n"
    )
}

#[test]
fn six_stages_fit_the_rows_the_libraries_screen_keeps_in_app_text() {
    let why = assemble("slots_six", &libraries_with(6), &["LIBRARY"]).unwrap_err();
    assert!(!why.contains(SLOTS), "{why}");
}

#[test]
fn seven_stages_do_not_fit_and_the_build_says_so() {
    let why = assemble("slots_seven", &libraries_with(7), &["LIBRARY"]).unwrap_err();
    assert!(why.contains(SLOTS), "{why}");
}
