//! The room each code page has, printed by the build and checked here, and the build's refusal
//! of a page that runs over.

use crate::app::*;
use std::path::Path;
use std::process::Command;

const PAGE: u32 = 16384;

#[test]
fn every_code_page_fits_and_the_build_says_how_much_it_uses() {
    for (name, built) in [("plain", plain()), ("with a library", with_library())] {
        assert_eq!(built.used.len(), 3, "{name}: one line for each code page");
        for (page, used) in built.used.iter().enumerate() {
            println!("{name}: code page {page} uses {used} of {PAGE} bytes");
            assert!(*used > 0 && *used <= PAGE, "{name}: page {page}: {used}");
        }
    }
}

/// Assembles pages.inc with `lines` after it, as the app's own include path does.
fn assemble(name: &str, lines: &str) -> Result<(), String> {
    assemble_with(name, "pages.inc", lines, &[])
}

/// Assembles the include `first` with `lines` after it and the `-D` defines, as the app's own
/// include path does.
fn assemble_with(name: &str, first: &str, lines: &str, defines: &[&str]) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pages-room-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("room.z80");
    std::fs::write(&source, format!("#include \"{first}\"\n{lines}\n")).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let run = Command::new("sh")
        .arg(root.join("scripts/spasm.sh"))
        .args(defines.iter().map(|d| format!("-D{d}")))
        .arg("-I")
        .arg(root.join("app/src"))
        .arg(&source)
        .arg(dir.join("room.bin"))
        .env("SPASM", spasm)
        .output()
        .expect("the assembler runs");
    let text =
        String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr);
    if run.status.success() {
        Ok(())
    } else {
        Err(text)
    }
}

#[test]
fn a_page_of_exactly_16384_bytes_builds_and_one_byte_more_does_not() {
    assemble("full", ".org 4000h\n.fill 4000h, 0\nAPP_PAGE_END(1)").expect("a full page builds");
    let over = assemble("over", ".org 4000h\n.fill 4001h, 0\nAPP_PAGE_END(1)")
        .expect_err("an overfull page");
    assert!(over.contains("a code page is over 16384 bytes"), "{over}");
    let filled =
        assemble("fill", ".org 4000h\n.fill 4001h, 0\nAPP_PAGE_FILL(1)").expect_err("filling too");
    assert!(
        filled.contains("a code page is over 16384 bytes"),
        "{filled}"
    );
}

#[test]
fn the_pages_are_refused_out_of_order() {
    assemble("order", "APP_PAGE(1)\nAPP_PAGE(2)").expect("in order");
    let late = assemble("late", "APP_PAGE(2)").expect_err("page 2 first");
    assert!(late.contains("not assembled in order"), "{late}");
    let twice = assemble("twice", "APP_PAGE(1)\nAPP_PAGE(1)").expect_err("page 1 twice");
    assert!(twice.contains("not assembled in order"), "{twice}");
}

#[test]
fn the_last_page_must_leave_96_bytes_for_the_signature() {
    assemble("last-ok", ".org 4000h\n.fill 3FA0h, 0\nAPP_PAGE_LAST(2)").expect("16288 bytes fit");
    let over = assemble("last-over", ".org 4000h\n.fill 3FA1h, 0\nAPP_PAGE_LAST(2)")
        .expect_err("16289 bytes spill the signature to a fourth page");
    assert!(over.contains("leaves no room for the signature"), "{over}");
    // With a library after it the last page may run to its end.
    assemble("end-full", ".org 4000h\n.fill 4000h, 0\nAPP_PAGE_END(2)").expect("full page");
}

#[test]
fn app_buf_refuses_every_region_it_could_overlap_and_the_end_of_bank_b() {
    let go =
        |name: &str, at: &str| assemble_with(name, "ram_map.inc", "", &[&format!("APP_BUF={at}")]);
    go("buf-default", "$8806").expect("the default place");
    for (at, want) in [
        ("$85F0", "APP_BUF and UI_VARS overlap"),
        ("$8800", "APP_BUF and FM_SCRATCH overlap"),
        ("$9250", "APP_BUF and UI_FRAME overlap"),
        ("$9780", "APP_BUF and KD_SCRATCH overlap"),
        ("$BF80", "APP_BUF runs past bank B"),
    ] {
        let why = go(&format!("buf-{}", &at[1..]), at).expect_err(want);
        assert!(why.contains(want), "{at}: {why}");
    }
}

#[test]
fn the_far_entries_are_the_same_bytes_on_the_entry_page_and_the_draw_page() {
    for built in [plain(), with_library()] {
        let pages = mcemu::decode_app(&built.file).unwrap();
        let (start, end) = (built.label("far_table"), built.label("far_table_end"));
        let at = usize::from(start - 0x4000)..usize::from(end - 0x4000);
        assert!(end > start, "a table of stubs");
        assert_eq!(pages[0][at.clone()], pages[1][at], "the reader's stubs");
    }
}

#[test]
fn a_font_directory_without_font_inc_stops_the_build() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pages-no-font");
    std::fs::create_dir_all(&dir).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app");
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let run = Command::new("make")
        .arg("-C")
        .arg(&source)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}/build", dir.display()))
        .arg(format!("FONT={}", dir.display()))
        .arg("app")
        .output()
        .expect("make runs");
    assert!(!run.status.success(), "the build went on");
    let text = String::from_utf8_lossy(&run.stderr);
    assert!(text.contains("font.inc is missing"), "{text}");
}

/// The reader's public routines are the ones its header lists, and each is in the far entries
/// three times: a stub, an alias for fm's calls and its removal.
#[test]
fn the_far_entries_of_the_reader_are_the_routines_its_header_lists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let header = std::fs::read_to_string(root.join("lib/lib.z80")).unwrap();
    let mut listed: Vec<&str> = header
        .lines()
        .filter_map(|l| l.strip_prefix(";   lib_"))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    listed.sort_unstable();
    let far = std::fs::read_to_string(root.join("app/src/far_reader.inc")).unwrap();
    for name in &listed {
        let name = format!("lib_{name}");
        for line in [
            format!("FAR_DATA_ENTRY({name}, far_{name})"),
            format!("#define {name} far_{name}"),
            format!("#undef {name}"),
        ] {
            assert!(
                far.lines().any(|l| l == line),
                "far_reader.inc lacks `{line}`"
            );
        }
    }
    let entries = far
        .lines()
        .filter(|l| l.starts_with("FAR_DATA_ENTRY("))
        .count();
    assert_eq!(
        entries,
        listed.len(),
        "an entry for a routine the header does not list"
    );
}
