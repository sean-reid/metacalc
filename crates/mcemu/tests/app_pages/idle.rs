//! The idle step of the app's four screen kinds: each answers "nothing pending" and touches nothing.

use crate::app::*;
use crate::builds::*;
use crate::realos::{BASE, RealOs};
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

fn snapshot(app: &mut App) -> Vec<Vec<u8>> {
    REGIONS
        .iter()
        .map(|(base, size)| {
            let (at, len) = (app.label(base), usize::from(app.label(size)));
            app.m.read_ram(at, len)
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

#[test]
fn the_libraries_screen_and_about_idle_to_nothing_and_change_no_byte() {
    for built in [plain(), with_library()] {
        let mut shell = Shell::boot(built, Enter::Real);
        idles_to_nothing(&mut shell.app, false, "Libraries");
        assert_eq!(shell.top_kind(), KIND_LIBRARIES);
        shell.tap(F3);
        assert_eq!(shell.top_kind(), KIND_ABOUT);
        idles_to_nothing(&mut shell.app, false, "About");
        assert_eq!(shell.top_kind(), KIND_ABOUT);
    }
}

#[test]
fn a_stage_and_a_notice_idle_to_nothing_and_change_no_byte() {
    let mut shell = Shell::boot(three_stages(), Enter::Real);
    shell.tap(F3);
    assert_eq!(shell.top_kind(), KIND_STAGE);
    idles_to_nothing(&mut shell.app, false, "a stage");
    shell.tap(CLEAR);
    shell.tap(F1);
    assert_eq!(shell.top_kind(), KIND_NOTICE);
    idles_to_nothing(&mut shell.app, false, "a notice");
    assert_eq!(shell.top_kind(), KIND_NOTICE);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_the_libraries_screen_and_about_idle_to_nothing() {
    let mut app = App::on_real_os(with_library());
    idles_to_nothing(&mut app, true, "Libraries");
    let mut os = RealOs { m: app.m };
    os.tap(F3, 150, 400);
    let mut app = App { m: os.m, ..app };
    let at = app.label("scr_count");
    assert_eq!(app.m.read_ram(at, 1), [2], "About is open");
    idles_to_nothing(&mut app, true, "About");
}
