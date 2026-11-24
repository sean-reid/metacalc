//! The idle step of the app's four screen kinds: each answers "nothing pending" and touches nothing.
//! The call is made with A holding SCR_PUSH and the carry set, so that a routine which only returns
//! hands those back and is told from one that answers nothing.

use crate::app::*;
use crate::builds::*;
use crate::realos::{BASE, RealOs};
use crate::shellrig::*;

const SCR_PUSH: u8 = 2;
const CARRY: u8 = 1;

/// Every declared RAM region's bytes, from the build's labels, with the bytes the call itself
/// writes set to 0: scr_vec, which each dispatch writes with the routine it calls, and the driver's
/// own ranges. The 0x8000 filter skips labels whose base is in flash. Bank C is read as mapped at
/// the call, which is the OS's page, so the kernel's RAM page is not compared.
fn snapshot(app: &mut App) -> Vec<(u16, Vec<u8>)> {
    let labels = app.built.labels.clone();
    let vec = app.label("scr_vec");
    let mut own = app.scratch.clone();
    own.push((vec, vec + 2));
    crate::regions::declared(&labels)
        .into_iter()
        .filter(|&(from, _)| from >= 0x8000)
        .map(|(from, to)| {
            let mut bytes = app.m.read_ram(from, usize::from(to - from));
            for (i, byte) in bytes.iter_mut().enumerate() {
                let at = from + i as u16;
                if own.iter().any(|&(lo, hi)| (lo..hi).contains(&at)) {
                    *byte = 0;
                }
            }
            (from, bytes)
        })
        .collect()
}

/// One scr_idle, which must answer SCR_STAY with carry clear and leave every region as it was.
fn idles_to_nothing(app: &mut App, on_real_os: bool, what: &str) {
    let before = snapshot(app);
    let at = app.label("scr_idle");
    let input = In {
        a: SCR_PUSH,
        f: CARRY,
        ..In::default()
    };
    let done = if on_real_os {
        app.timed(BASE, at, input)
    } else {
        app.call(BASE, at, input)
    };
    assert_eq!(done.status(), 0, "{what}: the action");
    assert_eq!(done.regs.af & 1, 0, "{what}: the carry");
    let after = snapshot(app);
    for ((from, was), (_, now)) in before.iter().zip(&after) {
        if let Some(i) = was.iter().zip(now).position(|(a, b)| a != b) {
            panic!("{what}: the region at {from:04X} changed at offset {i}");
        }
    }
}

/// Opens the screen the keys lead to on a fresh app, then idles it. The call leaves the machine
/// at the driver's halt, so each screen gets an app of its own. Returns the frame the app shows.
fn idle_after(built: &'static Built, keys: &[u8], kind: u8, what: &str) -> Vec<u8> {
    let mut shell = Shell::boot(built, Enter::Real);
    for &key in keys {
        shell.tap(key);
    }
    assert_eq!(shell.top_kind(), kind, "{what}");
    let frame = shell.app.frame();
    idles_to_nothing(&mut shell.app, false, what);
    assert_eq!(shell.top_kind(), kind, "{what}");
    frame
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

/// The screen `keys` open on the reference OS is the one the stub shows for the same keys, and it
/// idles to nothing there.
fn idle_on_the_reference_os(built: &'static Built, keys: &[u8], kind: u8, what: &str) {
    let want = idle_after(built, keys, kind, what);
    let mut os = RealOs {
        m: started_on_real_os(built),
    };
    for &key in keys {
        os.tap(key, 150, 400);
    }
    let mut app = App::on_real_os_in(os.m, built);
    assert!(
        app.frame() == want,
        "{what}: the reference OS shows the stub's screen"
    );
    idles_to_nothing(&mut app, true, what);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_every_screen_of_the_app_idles_to_nothing() {
    idle_on_the_reference_os(with_library(), &[], KIND_LIBRARIES, "Libraries");
    idle_on_the_reference_os(with_library(), &[F3], KIND_ABOUT, "About");
    idle_on_the_reference_os(three_stages(), &[F3], KIND_STAGE, "a stage");
    idle_on_the_reference_os(three_stages(), &[F1], KIND_NOTICE, "a notice");
}
