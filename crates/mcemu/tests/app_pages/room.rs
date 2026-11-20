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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pages-room-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("room.z80");
    std::fs::write(&source, format!("#include \"pages.inc\"\n{lines}\n")).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let run = Command::new("sh")
        .arg(root.join("scripts/spasm.sh"))
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
