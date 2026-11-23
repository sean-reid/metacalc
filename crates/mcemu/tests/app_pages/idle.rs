//! The idle step of the app's four screen kinds: each answers "nothing pending" and touches nothing.

use crate::app::*;
use crate::builds::*;
use crate::realos::{BASE, RealOs, libraries_pixels};
use crate::shellrig::*;

/// The RAM the screens and the canvas keep: the screen stack and its variables, the canvas, the
/// UI's variables and every region of the app's RAM map.
const REGIONS: [(&str, &str); 10] = [
    ("SCR_STACK", "SCR_STACK_SIZE"),
    ("SCR_VARS", "SCR_VARS_SIZE"),
    ("UI_VARS", "UI_VARS_SIZE"),
    ("UI_FRAME", "UI_FRAME_SIZE"),
    ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
    ("FAR_VARS", "FAR_VARS_SIZE"),
    ("LIB_STATE", "LIB_STATE_SIZE"),
    ("APP_BUF", "APP_BUF_SIZE"),
    ("APP_STAGE", "APP_STAGE_SIZE"),
    ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
];

/// Every region's bytes, except scr_vec, which each dispatch writes with the routine it calls.
fn snapshot(app: &mut App) -> Vec<Vec<u8>> {
    let vec = usize::from(app.label("scr_vec") - app.label("SCR_VARS"));
    REGIONS
        .iter()
        .map(|(base, size)| {
            let (at, len) = (app.label(base), usize::from(app.label(size)));
            let mut bytes = app.m.read_ram(at, len);
            if *base == "SCR_VARS" {
                bytes.drain(vec..vec + 2);
            }
            bytes
        })
        .collect()
}

/// One scr_idle, which must answer SCR_STAY with carry clear and leave every region as it was.
fn idles_to_nothing(app: &mut App, on_real_os: bool, what: &str) {
    let before = snapshot(app);
    let at = app.label("scr_idle");
    let done = if on_real_os {
        app.timed(BASE, at, In::default())
    } else {
        app.call(BASE, at, In::default())
    };
    assert_eq!(done.status(), 0, "{what}: the action");
    assert_eq!(done.regs.af & 1, 0, "{what}: the carry");
    for (i, (base, _)) in REGIONS.iter().enumerate() {
        assert!(snapshot(app)[i] == before[i], "{what}: {base} changed");
    }
}

/// Opens the screen the keys lead to on a fresh app, then idles it. The call leaves the machine
/// at the driver's halt, so each screen gets an app of its own.
fn idle_after(built: &'static Built, keys: &[u8], kind: u8, what: &str) {
    let mut shell = Shell::boot(built, Enter::Real);
    for &key in keys {
        shell.tap(key);
    }
    assert_eq!(shell.top_kind(), kind, "{what}");
    idles_to_nothing(&mut shell.app, false, what);
    assert_eq!(shell.top_kind(), kind, "{what}");
}

#[test]
fn every_screen_of_the_app_idles_to_nothing_and_changes_no_byte() {
    for built in [plain(), with_library()] {
        idle_after(built, &[], KIND_LIBRARIES, "Libraries");
        idle_after(built, &[F3], KIND_ABOUT, "About");
    }
    let staged = three_stages();
    idle_after(staged, &[F3], KIND_STAGE, "a stage");
    idle_after(staged, &[F1], KIND_NOTICE, "a notice");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_libraries_screen_idles_to_nothing() {
    let mut app = App::on_real_os(with_library());
    idles_to_nothing(&mut app, true, "Libraries");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_about_idles_to_nothing() {
    let built = with_library();
    let libraries = libraries_pixels(built.shown().as_ref());
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    os.tap(F3, 150, 400);
    assert!(os.screen() != libraries, "About is open");
    let mut app = App {
        m: os.m,
        built,
        driver: 0xA800,
        stack: 0xAA00,
        scratch: vec![(0xA800, 0xA810), (0xA9C0, 0xAA00)],
    };
    idles_to_nothing(&mut app, true, "About");
}
