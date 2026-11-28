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
        ("$9897", "APP_BUF and FAR_VARS overlap"),
        ("$9AC0", "APP_BUF and LIB_STATE overlap"),
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

/// The public routines of the reader and of the proof walk are the ones their headers list, and
/// each is in the far entries three times: a stub, an alias for fm's calls and its removal.
#[test]
fn the_far_entries_of_the_reader_are_the_routines_its_header_lists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let far = std::fs::read_to_string(root.join("app/src/far_reader.inc")).unwrap();
    for (file, prefix) in [("lib/lib.z80", "lib_"), ("pw/pw.z80", "pw_")] {
        let header = std::fs::read_to_string(root.join(file)).unwrap();
        let mut listed: Vec<&str> = header
            .lines()
            .filter_map(|l| l.strip_prefix(";   "))
            .filter(|l| l.starts_with(prefix))
            .filter_map(|l| l.split_whitespace().next())
            .collect();
        listed.sort_unstable();
        assert!(!listed.is_empty(), "{file} lists no routine");
        for name in &listed {
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
            .filter(|l| l.starts_with(&format!("FAR_DATA_ENTRY({prefix}")))
            .count();
        assert_eq!(
            entries,
            listed.len(),
            "an entry for a routine {file} does not list"
        );
    }
}

#[test]
fn libpage_and_appstatus_must_lie_inside_kd_scratch() {
    let go = |name: &str, size: &str| {
        assemble_with(
            name,
            "equates.inc",
            "",
            &[&format!("KD_SCRATCH_SIZE={size}")],
        )
    };
    go("kd-2", "2").expect("libPage and appStatus fit in two bytes");
    let why = go("kd-1", "1").expect_err("appStatus is the second byte");
    assert!(why.contains("must lie inside KD_SCRATCH"), "{why}");
}

/// The far call's code is 63 bytes from FAR_CALL, the length crates/mcsend/tests/tifile.rs compares
/// on the pages of its fixture.
#[test]
fn the_far_call_is_63_bytes_long() {
    let built = plain();
    assert_eq!(built.label("far_end") - built.label("far_jump"), 63);
}

#[test]
fn the_sentinel_must_lie_inside_save_screen_and_face_the_same_offset_of_the_frame() {
    let go = |name: &str, at: &str| {
        assemble_with(name, "equates.inc", "", &[&format!("APP_SENTINEL={at}")])
    };
    go("sentinel-first", "$86EC").expect_err("UI_VARS is there");
    go("sentinel-default", "$89EB").expect("the last byte of saveSScreen");
    for at in ["$86EB", "$89EC", "$9000"] {
        let why = go(&format!("sentinel-{}", &at[1..]), at).expect_err(at);
        assert!(
            why.contains("APP_SENTINEL in ram_map.inc must lie inside saveSScreen"),
            "{at}: {why}"
        );
    }
    let built = plain();
    let offset = built.label("APP_SENTINEL") - built.label("saveSScreen");
    assert_eq!(
        offset,
        built.label("APP_SENTINEL_FRAME") - built.label("UI_FRAME")
    );
    assert!(offset < built.label("SAVE_SCREEN_SIZE"));
}

/// `make -C app` for `target` with a workspace file holding `toml`, on the font the tests share.
fn make_with_toml(name: &str, toml: &str, target: &str) -> std::process::Output {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pages-version-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Cargo.toml");
    std::fs::write(&file, toml).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    Command::new("make")
        .arg("-C")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app"))
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}/build", dir.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")))
        .arg(format!("WORKSPACE_TOML={}", file.display()))
        .arg(target)
        .output()
        .expect("make runs")
}

#[test]
fn a_version_that_is_not_a_plain_line_stops_the_app_build_but_not_make_clean() {
    for (name, toml) in [
        ("beta", "[workspace.package]\nversion = \"1.0.0-beta\"\n"),
        (
            "comment",
            "[workspace.package]\nversion = \"1.0.0\" # next\n",
        ),
        ("single", "[workspace.package]\nversion = '1.0.0'\n"),
        ("short", "[workspace.package]\nversion = \"1.0\"\n"),
        ("missing", "[workspace.package]\nedition = \"2024\"\n"),
        (
            "elsewhere",
            "[package]\nversion = \"1.0.0\"\n[workspace.package]\n",
        ),
    ] {
        let app = make_with_toml(name, toml, "app");
        let text = String::from_utf8_lossy(&app.stderr).into_owned();
        assert!(!app.status.success(), "{name}: the build went on");
        assert!(
            text.contains("is not a line version = \"major.minor.patch\""),
            "{name}: {text}"
        );
        let clean = make_with_toml(name, toml, "clean");
        assert!(clean.status.success(), "{name}: make clean failed");
    }
}

#[test]
fn a_plain_version_line_builds_the_app() {
    let toml = "[workspace.package]\nedition = \"2024\"\nversion = \"12.0.345\"\n";
    let app = make_with_toml("plain", toml, "app");
    assert!(
        app.status.success(),
        "{}",
        String::from_utf8_lossy(&app.stderr)
    );
}
