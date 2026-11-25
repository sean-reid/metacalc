//! The idle step of the app's four screen kinds: each answers "nothing pending" and touches nothing.
//! The call is made with A holding SCR_PUSH and the carry set, so that a routine which only returns
//! hands those back and is told from one that answers nothing.

use crate::app::*;
use crate::builds::*;
use crate::idlerig::IDLE;
use crate::realos::{BASE, RealOs};
use crate::shellrig::*;

const SCR_PUSH: u8 = 2;
const CARRY: u8 = 1;
const BANK_C: u16 = 0xC000;

/// Every declared RAM region's bytes, from the build's labels, with the bytes the call itself
/// writes set to 0: scr_vec, which each dispatch writes with the routine it calls, and the driver's
/// own ranges. The 0x8000 filter skips labels whose base is in flash. Bank B is read as mapped.
/// Bank C is read from the kernel's RAM page by its number, KERNEL_RAM_PAGE, whatever page the call
/// has mapped there.
fn snapshot(app: &mut App) -> Vec<(u16, Vec<u8>)> {
    let labels = app.built.labels.clone();
    let vec = app.label("scr_vec");
    let page = app.label("KERNEL_RAM_PAGE") as u8;
    let mut own = app.scratch.clone();
    own.push((vec, vec + 2));
    let regions: Vec<(u16, u16)> = crate::regions::declared(&labels)
        .into_iter()
        .filter(|&(from, _)| from >= 0x8000)
        .collect();
    assert!(
        regions.iter().any(|&(from, _)| from >= BANK_C),
        "the build declares regions in bank C"
    );
    regions
        .into_iter()
        .map(|(from, to)| {
            let mut bytes = if from >= BANK_C {
                let (lo, hi) = (usize::from(from - BANK_C), usize::from(to - BANK_C));
                app.m.ram_page(page)[lo..hi].to_vec()
            } else {
                app.m.read_ram(from, usize::from(to - from))
            };
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
pub fn idles_to_nothing(app: &mut App, on_real_os: bool, what: &str) {
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
    idle_after(staged, &[F1, F2], KIND_NOTICE, "a notice");
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
    idle_on_the_reference_os(three_stages(), &[F1, F2], KIND_NOTICE, "a notice");
}

/// An idle routine that writes one byte of the kernel's RAM page, at `at`, and answers SCR_STAY
/// with carry clear: it maps the page by its number, increments the byte and maps back.
fn writes_in_the_kernels_page(app: &mut App, kind: u8, at: u16) {
    let page = app.label("KERNEL_RAM_PAGE") as u8;
    let [lo, hi] = at.to_le_bytes();
    let code = [
        0xDB, 0x05, 0xF5, 0x3E, page, 0xD3, 0x05, 0x21, lo, hi, 0x34, 0xF1, 0xD3, 0x05, 0xAF, 0xC9,
    ];
    app.m.load_ram(IDLE, &code);
    patch_idle(&mut app.m, app.built, kind, IDLE);
}

/// The first byte of the first region the build declares in bank C.
fn first_byte_of_bank_c(app: &App) -> u16 {
    let regions = crate::regions::declared(&app.built.labels);
    let from = regions
        .iter()
        .map(|&(from, _)| from)
        .filter(|&f| f >= BANK_C)
        .min();
    from.expect("the build declares regions in bank C")
}

fn a_write_in_the_kernels_page_is_seen(real: bool) {
    let built = three_stages();
    let mut app = if real {
        let mut os = RealOs {
            m: started_on_real_os(built),
        };
        os.tap(F3, 150, 400);
        App::on_real_os_in(os.m, built)
    } else {
        let mut shell = Shell::boot(built, Enter::Real);
        shell.tap(F3);
        shell.app
    };
    let at = first_byte_of_bank_c(&app);
    let page = app.label("KERNEL_RAM_PAGE") as u8;
    let offset = usize::from(at - BANK_C);
    assert_ne!(
        app.m.in_port(5),
        page,
        "the call finds another page in bank C"
    );
    writes_in_the_kernels_page(&mut app, KIND_STAGE, at);
    let was = app.m.ram_page(page)[offset];
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        idles_to_nothing(&mut app, real, "a write in the kernel's page");
    }));
    assert_eq!(
        app.m.ram_page(page)[offset],
        was.wrapping_add(1),
        "the routine wrote the byte"
    );
    let payload = caught.expect_err("the snapshot saw the write");
    let message = payload
        .downcast_ref::<String>()
        .expect("a formatted panic message");
    let want = format!("a write in the kernel's page: the region at {at:04X} changed at offset 0");
    assert_eq!(message, &want);
}

#[test]
fn an_idle_routine_that_writes_the_kernels_page_fails_the_snapshot() {
    a_write_in_the_kernels_page_is_seen(false);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_an_idle_routine_that_writes_the_kernels_page_fails_the_snapshot() {
    a_write_in_the_kernels_page_is_seen(true);
}
