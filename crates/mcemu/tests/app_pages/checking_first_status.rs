//! A check that cannot start because the reader fails: the error notice shows the job's status,
//! not the status Libraries' own enter fails with when the stack re-enters it after the push.

use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcemu::Stop;

const CK_READER: u16 = 8;
/// The reader's ST_MALFORMED, which a library of length 0 gets from every read.
const ST_MALFORMED: u16 = 1;
/// The notice's number for CK_READER with that detail: (CK_READER - CK_REFUSED) * 32 + 1.
const JOBS_STATUS: u8 = 0x61;

/// A booted run on the one-stage proofs library with its first Libraries frame drawn, whose reader
/// is then made to fail every read, as a library whose length reads 0 does.
fn truncated() -> (Run, Vec<u8>) {
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    let libraries = run.frame();
    assert_eq!(run.depth(), 1);
    let length = run.shell.app.label("lr_length");
    assert_ne!(
        run.m().read_ram(length, 3),
        [0, 0, 0],
        "a library with a length"
    );
    run.m().load_ram(length, &[0, 0, 0]);
    (run, libraries)
}

/// check_begin fails with CK_READER, the reader's status 1 in B, and Libraries fails on the same
/// library when the stack enters it again, so the notice must carry the job's.
#[test]
fn a_read_that_fails_at_begin_shows_the_jobs_status_and_not_the_libraries_screens() {
    let (mut run, libraries) = truncated();
    let before = run.frames();
    run.tap(F2);
    let begin = run.shell.app.label("check_begin");
    run.run_to_addr("check_begin", begin, WAIT);
    let sp = run.m().regs().sp;
    let back = run.m().read_ram(sp, 2);
    run.run_to_addr("its return", u16::from_le_bytes([back[0], back[1]]), WAIT);
    let regs = run.m().regs();
    assert_eq!(
        (regs.af >> 8, regs.bc >> 8),
        (CK_READER, ST_MALFORMED),
        "check_begin answers CK_READER with the reader's status"
    );
    let libs_enter = run.shell.app.label("libs_enter");
    run.run_to_addr("the Libraries enter after the push", libs_enter, WAIT);
    let sp = run.m().regs().sp;
    let back = run.m().read_ram(sp, 2);
    run.run_to_addr("its return", u16::from_le_bytes([back[0], back[1]]), WAIT);
    let regs = run.m().regs();
    println!(
        "Libraries' enter answers {:02X}, the job's status is {JOBS_STATUS:02X}",
        regs.af >> 8
    );
    assert_eq!(
        regs.af >> 8,
        ST_MALFORMED,
        "Libraries fails on the same library, with the reader's own status"
    );
    let frame = run.next_blit();
    let shown = run.frames();
    println!("the notice's frame is {:02X?}", &shown[1][..3]);
    assert_eq!(run.depth(), 2);
    assert_eq!(shown[0], before[0], "Libraries' frame is as it was");
    assert_eq!(shown[1][..3], [KIND_NOTICE, NOTICE_ERROR, JOBS_STATUS]);
    same_frame("the job's status", &frame, &model::error(JOBS_STATUS));
    assert_ne!(frame, model::error(ST_MALFORMED as u8));
    assert_ne!(frame, libraries);
}

/// With the notice dismissed the stack re-enters Libraries, which fails again, and a failure while
/// the error notice is on top leaves the app: the user lands on the home screen with no second
/// notice.
#[test]
fn dismissing_that_notice_leaves_the_app_because_libraries_fails_again() {
    let (mut run, _) = truncated();
    run.tap(F2);
    run.next_blit();
    assert_eq!((run.depth(), run.top_kind()), (2, KIND_NOTICE));
    let enters = run.shell.enters(KIND_LIBRARIES);
    run.m().press_key(CLEAR);
    let stop = run.m().run_app(8 * 120_000).stop;
    assert_eq!(stop, Stop::Exited, "the app leaves");
    assert!(
        run.shell.enters(KIND_LIBRARIES) > enters,
        "Libraries was entered again and failed"
    );
    run.shell.assert_regions_put_away();
}

/// The same on the reference OS: the notice over Libraries carries the job's status, 61, and the
/// frame is the model's.
#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_read_that_fails_at_begin_shows_the_jobs_status() {
    use crate::checking_real::{SETTLE, launched};
    let built = proofs();
    let mut run = launched(built, 0, &[]);
    let length = run.shell.app.label("lr_length");
    assert_ne!(
        run.m().read_ram(length, 3),
        [0, 0, 0],
        "a library with a length"
    );
    run.m().load_ram(length, &[0, 0, 0]);
    let blits = run.blits();
    run.press_us(F2, 150_000);
    run.step_us(SETTLE);
    assert!(run.blits() > blits, "a frame was shown");
    let shown = run.frames();
    println!("the notice's frame is {:02X?}", &shown[1][..3]);
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[1][..3], [KIND_NOTICE, NOTICE_ERROR, JOBS_STATUS]);
    same_frame("the job's status", &run.frame(), &model::error(JOBS_STATUS));
}
