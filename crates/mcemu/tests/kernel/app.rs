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
const PORT_BANK_A: u8 = 6;

fn app_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app")
}

struct App {
    pages: Vec<Vec<u8>>,
    labels: String,
}

/// Runs `make -C app` on a library and the checker on what it produced, then decodes the file.
fn build(name: &str, library: &[u8]) -> App {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("app-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let library_path = dir.join("library.mclib");
    std::fs::write(&library_path, library).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let make = Command::new("make")
        .arg("-s")
        .arg("-C")
        .arg(app_dir())
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", dir.display()))
        .arg(format!("LIBRARY={}", library_path.display()))
        .output()
        .expect("make runs");
    assert!(
        make.status.success(),
        "make: {}",
        String::from_utf8_lossy(&make.stdout)
    );
    let app_path = dir.join("metacalc.8xk");
    let check = Command::new("sh")
        .arg(app_dir().join("scripts/check-8xk.sh"))
        .arg("-l")
        .arg(&library_path)
        .arg(&app_path)
        .output()
        .expect("the checker runs");
    assert!(
        check.status.success(),
        "check-8xk: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    App {
        pages: decode(&std::fs::read(app_path).unwrap()),
        labels: std::fs::read_to_string(dir.join("metacalc.lab")).unwrap(),
    }
}

/// The pages of an .8xk: a type 02 Intel hex record starts each page, type 00 records fill it.
fn decode(file: &[u8]) -> Vec<Vec<u8>> {
    let text = String::from_utf8_lossy(file);
    let mut pages: Vec<Vec<u8>> = Vec::new();
    for line in text.lines() {
        let Some(at) = line.rfind(':') else { continue };
        let record = &line[at + 1..];
        let byte = |i: usize| u8::from_str_radix(&record[2 * i..2 * i + 2], 16).unwrap();
        let len = usize::from(byte(0));
        match byte(3) {
            2 => {
                assert_eq!(usize::from(byte(5)), pages.len(), "page records in order");
                pages.push(Vec::new());
            }
            0 => {
                let page = pages.last_mut().expect("data after a page record");
                let address = usize::from(byte(1)) << 8 | usize::from(byte(2));
                assert_eq!(address, 0x4000 + page.len(), "records are contiguous");
                page.extend((0..len).map(|i| byte(4 + i)));
            }
            _ => {}
        }
    }
    pages
}

/// Page 0 of the app runs in bank A, as it does when TI-OS enters the app, and its accessor
/// returns the kernel's library page for the app's base page.
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
        ("free96", &small, 3 * PAGE - 96, 4),
        ("free95", &anatomy, 3 * PAGE - 95, 5),
        ("short", &unifier, 2 * PAGE + 400, 4),
    ] {
        assert!(
            base.len() <= len,
            "{name}: the library is {} bytes",
            base.len()
        );
        let bytes = padded(base, len);
        let app = build(name, &bytes);
        assert_eq!(app.pages.len(), want_pages, "{name}: pages");
        for (c, chunk) in bytes.chunks(PAGE).enumerate() {
            assert_eq!(&app.pages[1 + c][..chunk.len()], chunk, "{name}: chunk {c}");
        }

        let mut device = Device::new();
        for (k, page) in app.pages.iter().enumerate() {
            device.m.load_flash(BASE - k as u8, 0, page);
        }
        device.lib_page = lib_page_from_app(&mut device, &app);
        assert_eq!(device.lib_page, BASE - 1, "{name}: the accessor");

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
