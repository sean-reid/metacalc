use crate::crc::{padded, run_crc};
use crate::support::*;
use mcemu::FLASH_TEST_PAGE;
use mcpack::library::crc32;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The page the app is entered on; page k of the app is on flash page BASE - k.
const BASE: u8 = 0x30;
const GLUE: u16 = 0x9000;
const GLUE_RESULT: u16 = 0x9100;
const APP_ENTRY: u16 = 0x4000;
/// The app's code pages: entry, draw and data. The library follows them.
const CODE_PAGES: u8 = 3;
const PORT_BANK_A: u8 = 6;

fn app_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app")
}

pub struct App {
    pub file: Vec<u8>,
    pub pages: Vec<Vec<u8>>,
    pub labels: String,
}

/// Runs `make -C app` and the checker on what it produced, with a library or without, then
/// decodes the file.
pub fn build(name: &str, library: Option<&[u8]>) -> App {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("app-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let library_path = dir.join("library.mclib");
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let mut make = Command::new("make");
    make.arg("-s")
        .arg("-C")
        .arg(app_dir())
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", dir.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")));
    let mut check = Command::new("sh");
    check
        .arg(app_dir().join("scripts/check-8xk.sh"))
        .arg("-p")
        .arg(CODE_PAGES.to_string());
    if let Some(library) = library {
        std::fs::write(&library_path, library).unwrap();
        make.arg(format!("LIBRARY={}", library_path.display()));
        check.arg("-l").arg(&library_path);
    }
    let made = make.output().expect("make runs");
    assert!(
        made.status.success(),
        "make: {}",
        String::from_utf8_lossy(&made.stdout)
    );
    let app_path = dir.join("metacalc.8xk");
    let checked = check.arg(&app_path).output().expect("the checker runs");
    assert!(
        checked.status.success(),
        "check-8xk: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let file = std::fs::read(app_path).unwrap();
    App {
        pages: mcemu::decode_app(&file).expect("the app decodes"),
        file,
        labels: std::fs::read_to_string(dir.join("metacalc.lab")).unwrap(),
    }
}

/// Page 0 of the app runs in bank A, as it does when TI-OS enters the app, and its accessor
/// returns the kernel's library page for the app's base page: the page below its last code page.
fn lib_page_from_app(device: &mut Device, app: &App) -> u8 {
    let accessor = label_of(&app.labels, "APP_LIB_PAGE");
    assert!((APP_ENTRY..APP_ENTRY + 0x4000).contains(&accessor));
    let [lo, hi] = accessor.to_le_bytes();
    let [rlo, rhi] = GLUE_RESULT.to_le_bytes();
    let glue = [
        0x3E,
        BASE, // ld a, BASE
        0xD3,
        PORT_BANK_A, // out (6), a
        0xCD,
        lo,
        hi, // call app_lib_page, A is BASE as read from port 6 at entry
        0x32,
        rlo,
        rhi, // ld (result), a
        0x3E,
        FLASH_TEST_PAGE, // ld a, FLASH_TEST_PAGE
        0xD3,
        PORT_BANK_A, // out (6), a
        0x76,        // halt
    ];
    device.m.load_ram(GLUE, &glue);
    let run = device.m.run(GLUE, RUN_LIMIT);
    assert_eq!(run.stop, mcemu::Stop::Halted);
    device.m.read_ram(GLUE_RESULT, 1)[0]
}

#[test]
fn the_kernel_checks_a_library_stored_in_the_pages_of_a_built_app() {
    let (_, small) = small_library();
    let anatomy = pack_all(&corpus("anatomy.mm"));
    let unifier = pack_all(&corpus("big-unifier.mm"));
    // Library, its length after padding, and the app's page count. The last chunk leaves 96 free
    // bytes, then 95, then all but 400.
    for (name, base, len, want_pages) in [
        ("free96", &small, 3 * PAGE - 96, 6),
        ("free95", &anatomy, 3 * PAGE - 95, 7),
        ("short", &unifier, 2 * PAGE + 400, 6),
    ] {
        assert!(
            base.len() <= len,
            "{name}: the library is {} bytes",
            base.len()
        );
        let bytes = padded(base, len);
        let app = build(name, Some(&bytes));
        assert_eq!(app.pages.len(), want_pages, "{name}: pages");
        for (c, chunk) in bytes.chunks(PAGE).enumerate() {
            assert_eq!(
                &app.pages[usize::from(CODE_PAGES) + c][..chunk.len()],
                chunk,
                "{name}: chunk {c}"
            );
        }

        let mut device = Device::new();
        device
            .m
            .install_app(&app.file, BASE)
            .expect("the app installs");
        device.lib_page = lib_page_from_app(&mut device, &app);
        assert_eq!(device.lib_page, BASE - CODE_PAGES, "{name}: the accessor");

        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        assert_eq!(want_init, 0, "{name}: the model accepts the library");
        let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, &got), (want_init, &want), "{name}: statuses");
        assert!(got.contains(&0), "{name}: some assertion verifies");

        let crc = run_crc(&mut device);
        assert_eq!(
            (crc.init, crc.begin, crc.overall()),
            (0, 0, 0),
            "{name}: crc run"
        );
        assert_eq!(crc.value, crc32(&bytes[8..]), "{name}: crc");
    }
}

type Decoded = Result<Vec<Vec<u8>>, mcemu::AppError>;

/// The broken files test-check-8xk.sh feeds the checker, for the app without a library and for the app
/// with a library, as (file name, decoded).
fn fixtures(name: &str, library: Option<&[u8]>) -> Vec<(String, Decoded)> {
    build(name, library);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("app-{name}"));
    let fixtures = dir.join("fixtures");
    let _ = std::fs::remove_dir_all(&fixtures);
    let mut script = Command::new("sh");
    script
        .arg(app_dir().join("scripts/test-check-8xk.sh"))
        .arg("-p")
        .arg(CODE_PAGES.to_string())
        .env("KEEP_FIXTURES", &fixtures);
    if library.is_some() {
        script.arg("-l").arg(dir.join("library.mclib"));
    }
    let ran = script
        .arg(dir.join("metacalc.8xk"))
        .output()
        .expect("the script runs");
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let mut found: Vec<_> = std::fs::read_dir(&fixtures)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "8xk"))
        .map(|e| {
            let file = std::fs::read(e.path()).unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                mcemu::decode_app(&file),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// The checker refuses some broken files for what is in the app (its name, signature, library
/// bytes, page count), and those are valid hex the decoder takes. The ones it refuses for the
/// hex itself, the decoder must refuse too.
#[test]
fn the_decoder_and_the_checker_agree_on_the_broken_files_the_checker_is_tested_with() {
    let (_, small) = small_library();
    let library = padded(&small, 3 * PAGE - 96);
    let hex_level = ["empty", "checksum", "gap", "long-page", "no-end"];
    for (label, library) in [("fixtures-one", None), ("fixtures-lib", Some(&library[..]))] {
        let found = fixtures(label, library);
        assert!(found.len() >= 5, "{label}: {} fixtures", found.len());
        let mut seen = Vec::new();
        for (file, decoded) in &found {
            let stem = file.trim_end_matches(".8xk");
            if hex_level.contains(&stem) {
                assert!(decoded.is_err(), "{label}: the decoder took {file}");
                seen.push(stem.to_string());
            } else {
                assert!(
                    decoded.is_ok(),
                    "{label}: the decoder refused {file}: {decoded:?}"
                );
            }
        }
        assert!(seen.contains(&"checksum".to_string()), "{label}: {seen:?}");
        if library.is_some() {
            for stem in ["gap", "long-page"] {
                assert!(seen.contains(&stem.to_string()), "{label}: {seen:?}");
            }
        }
    }
}
