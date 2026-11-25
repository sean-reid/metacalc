//! What the checking screen does with a status that stops the job: a library whose CRC is wrong, a
//! table deleted or archived between two steps, a reader that fails, ON during kernel_init, and the
//! status the error notice shows for each class of the job's statuses.

use crate::app::*;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::ui_support::{pack, same_frame};
use mcemu::Machine;
use std::sync::OnceLock;

const CK_REFUSED: u8 = 5;
const CK_STORE: u8 = 6;
const CK_RANGE: u8 = 7;
const CK_READER: u8 = 8;
const CK_BAD_STATE: u8 = 9;

/// The model's notice for a library whose CRC is wrong.
fn damaged_frame() -> Vec<u8> {
    pack(&mcui::notice_frame(
        "Verify",
        &["Library is damaged", "Install it again"],
    ))
}

/// `bytes` with a byte of the stored CRC changed after packing.
fn crc_wrong(bytes: &[u8]) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes[4] ^= 0xFF;
    bytes
}

/// A library with a wrong CRC of one chunk and of several, each as one stage.
pub fn damaged(many: bool) -> &'static Built {
    static ONE: OnceLock<&'static Built> = OnceLock::new();
    static MANY: OnceLock<&'static Built> = OnceLock::new();
    if many {
        MANY.get_or_init(|| around("damaged-many", &crc_wrong(&proofs_padded(1200))))
    } else {
        ONE.get_or_init(|| around("damaged-one", &crc_wrong(&crate::fixture::proofs())))
    }
}

/// A CRC that is wrong replaces the screen with the model's notice for a damaged library, after the
/// last chunk, creates no table, and CLEAR on the notice returns to Libraries.
#[test]
fn a_crc_that_is_wrong_replaces_the_screen_with_the_damaged_notice_and_makes_no_table() {
    for many in [false, true] {
        let built = damaged(many);
        let chunks = chunks_of(built);
        assert_eq!(chunks > 1, many);
        let mut run = Run::boot(built, None);
        let libraries = run.frame();
        let vars = run.vars();
        run.m().press_key(F2);
        let mut frames = 0;
        let notice = loop {
            let (frame, values) = run.next_frame();
            frames += 1;
            assert_eq!(values, None, "no table is made");
            if frame == damaged_frame() {
                break frame;
            }
            assert!(frames < 100, "the notice never came");
        };
        run.m().release_key(F2);
        assert_eq!(
            frames,
            chunks + 2,
            "the push, crc_begin and each chunk, then the notice"
        );
        same_frame("the damaged notice", &notice, &damaged_frame());
        assert_eq!((run.depth(), run.top_kind()), (2, KIND_NOTICE));
        assert_eq!(run.frames()[1][..3], [KIND_NOTICE, NOTICE_DAMAGED, 0]);
        assert_eq!(run.vars(), vars, "no variable was created");
        run.tap(CLEAR);
        let back = run.next_blit();
        same_frame("Libraries after the notice", &back, &libraries);
        assert_eq!(run.depth(), 1);
        assert_eq!(run.vars(), vars);
    }
}

/// The padded library with one byte of the content changed after packing, in the first chunk or in
/// the last: the first assertion's label, and a byte of a section the last chunk holds, neither of
/// which the kernel reads, so only the CRC run can tell. Returns the library and the offset changed.
fn content_wrong(first: bool) -> (&'static Built, usize) {
    static FIRST: OnceLock<(&'static Built, usize)> = OnceLock::new();
    static LAST: OnceLock<(&'static Built, usize)> = OnceLock::new();
    let cell = if first { &FIRST } else { &LAST };
    *cell.get_or_init(|| {
        let mut bytes = proofs_padded(1200);
        let chunks = bytes.len().div_ceil(1 << 14);
        let at = if first {
            let label = "a-long-label-to-fill-the-page-0";
            bytes
                .windows(label.len())
                .position(|w| w == label.as_bytes())
                .expect("the label is in the library")
        } else {
            ((chunks - 1) << 14) + 1
        };
        assert!(
            at >= 8 && at < bytes.len() - 64,
            "the byte is under the CRC"
        );
        let chunk = at >> 14;
        assert_eq!(
            chunk,
            if first { 0 } else { chunks - 1 },
            "the chunk it is in"
        );
        bytes[at] ^= 1;
        let name = if first {
            "content-first"
        } else {
            "content-last"
        };
        (around(name, &bytes), at)
    })
}

/// One byte of the content changed after packing, in the first chunk and in the last, is a CRC
/// that is wrong: the run reads every chunk, the damaged notice replaces the screen, and the
/// variable list is as it was.
#[test]
fn a_content_byte_changed_after_packing_gives_the_damaged_notice_and_makes_no_variable() {
    for first in [true, false] {
        let (built, at) = content_wrong(first);
        let name = format!(
            "byte {at}, {}",
            if first { "first chunk" } else { "last chunk" }
        );
        let chunks = chunks_of(built);
        assert!(chunks >= 4, "{name}: {chunks} chunks");
        let mut run = Run::boot(built, None);
        let libraries = run.frame();
        let vars = run.vars();
        run.m().press_key(F2);
        let mut frames = 0;
        loop {
            let (frame, values) = run.next_frame();
            frames += 1;
            assert_eq!(values, None, "{name}: no table is made");
            if frame == damaged_frame() {
                break;
            }
            assert_ne!(run.top_kind(), KIND_NOTICE, "{name}: some other notice");
            assert!(frames < 100, "{name}: the notice never came");
        }
        run.m().release_key(F2);
        assert_eq!(
            frames,
            chunks + 2,
            "{name}: the push, crc_begin and each chunk, then the notice"
        );
        assert_eq!(
            run.crc_calls(),
            (1, u16::try_from(chunks).unwrap()),
            "{name}: every chunk was read"
        );
        assert_eq!(run.vars(), vars, "{name}: no variable was created");
        run.tap(CLEAR);
        let back = run.next_blit();
        same_frame(&name, &back, &libraries);
    }
}

/// A refusal the kernel gives in the CRC run with a status other than 1, which it never gives
/// itself and the rig puts in place of a chunk call's answer, is not a wrong CRC: the error notice
/// shows the kernel's status, nothing was created and CLEAR returns to Libraries.
#[test]
fn a_refusal_in_the_crc_run_other_than_a_wrong_crc_shows_the_error_notice_with_its_status() {
    let built = padded();
    assert!(chunks_of(built) >= 4);
    for (call, status) in [(1u8, 5u8), (3, 17), (2, 2)] {
        let name = format!("chunk call {call}, status {status}");
        let mut run = Run::boot(built, None);
        let libraries = run.frame();
        let vars = run.vars();
        run.fail_chunk(call, status);
        run.m().press_key(F2);
        let mut frames = 0;
        let notice = loop {
            let (frame, values) = run.next_frame();
            frames += 1;
            assert_eq!(values, None, "{name}: no table is made");
            if run.top_kind() == KIND_NOTICE {
                break frame;
            }
            assert!(frames < 100, "{name}: no notice came");
        };
        run.m().release_key(F2);
        assert_ne!(notice, damaged_frame(), "{name}: not the damaged notice");
        same_frame(&name, &notice, &error_frame(status));
        assert_eq!(
            run.frames()[1][..3],
            [KIND_NOTICE, NOTICE_ERROR, status],
            "{name}"
        );
        assert_eq!(
            run.crc_calls().1,
            u16::from(call),
            "{name}: the run stopped at the call"
        );
        assert_eq!(
            frames,
            usize::from(call) + 2,
            "{name}: the push, crc_begin, the chunks read and the notice"
        );
        assert_eq!(run.vars(), vars, "{name}: no variable was created");
        run.tap(CLEAR);
        let back = run.next_blit();
        same_frame(&name, &back, &libraries);
        assert_eq!(run.depth(), 1, "{name}");
    }
}

/// The frame of the error notice for `status`.
fn error_frame(status: u8) -> Vec<u8> {
    model::error(status)
}

/// A table deleted between two steps, and one archived: the step answers CK_STORE with the store's
/// status, the screen is replaced by the error notice at the same depth, nothing is created, and
/// CLEAR returns to Libraries, whose counts are what the table holds, which is none.
#[test]
fn a_table_lost_between_two_steps_replaces_the_screen_with_the_error_notice() {
    // The store's ST_ABSENT and ST_ARCHIVED, under the class of CK_STORE.
    for (what, status) in [("deleted", 0x27), ("archived", 0x22)] {
        let built = proofs();
        let mut run = Run::boot(built, Some(&empty_table(built)));
        run.m().press_key(F2);
        for _ in 0..2 {
            run.next_blit();
        }
        run.m().release_key(F2);
        let before = run.verifies();
        match what {
            "deleted" => {
                run.m().delete_var(crate::table::VAR);
            }
            _ => run.m().archive_var(crate::table::VAR),
        }
        let frame = run.next_blit();
        same_frame(what, &frame, &error_frame(status));
        assert_eq!(
            run.verifies(),
            before,
            "{what}: no step was verified without its mark"
        );
        assert_eq!((run.depth(), run.top_kind()), (2, KIND_NOTICE), "{what}");
        assert_eq!(run.frames()[1][..3], [KIND_NOTICE, NOTICE_ERROR, status]);
        match what {
            "deleted" => assert_eq!(run.raw_table(), None, "nothing is created"),
            _ => assert!(run.m().read_var(crate::table::VAR).unwrap().archived),
        }
        run.tap(CLEAR);
        let back = run.next_blit();
        same_frame(what, &back, &model::libraries(built));
        assert_eq!(run.depth(), 1, "{what}: CLEAR returns to Libraries");
    }
}

/// A library the kernel refuses at the start of the job, which a zero length in the reader's state
/// makes it do, fails the enter, so the error notice is pushed over Libraries with the job's
/// CK_REFUSED class and the kernel's status, and CLEAR on it returns to Libraries.
#[test]
fn a_library_the_kernel_refuses_at_begin_shows_the_error_notice_over_libraries() {
    let built = proofs();
    let mut run = Run::boot(built, Some(&empty_table(built)));
    let libraries = run.frame();
    let length = run.shell.app.label("lr_length");
    let saved = run.m().read_ram(length, 3);
    run.m().load_ram(length, &[0, 0, 0]);
    // CK_REFUSED, class 0, with ST_MALFORMED as its detail.
    let status = 0x01;
    run.tap(F2);
    let frame = run.next_blit();
    assert_eq!((run.depth(), run.top_kind()), (2, KIND_NOTICE));
    assert_eq!(run.frames()[1][..3], [KIND_NOTICE, NOTICE_ERROR, status]);
    same_frame("the kernel's refusal", &frame, &error_frame(status));
    run.m().load_ram(length, &saved);
    run.tap(CLEAR);
    let back = run.next_blit();
    same_frame("Libraries", &back, &libraries);
}

/// ON during kernel_init: check_begin answers CK_PAUSED, the enter succeeds, and the first idle
/// call leaves as an ON does, so the screen is open for one frame and gone before any step.
#[test]
fn on_during_kernel_init_opens_the_screen_and_the_first_idle_call_pops_it() {
    use crate::support::{pack_all, parse};
    let extra: Vec<String> = (0..60)
        .map(|i| format!("{}{i:03}", "n".repeat(30)))
        .collect();
    let mut src = format!("$c T wff |- $.\n$c {} $.\n", extra.join(" "));
    for i in 0..40 {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    let library = pack_all(&parse(&src));
    let built = around("slow-init", &library);
    let mut run = Run::boot(built, None);
    let libraries = run.frame();
    run.m().press_key(F2);
    run.run_to("far_kernel_init", WAIT);
    run.m().press_key(Machine::KEY_ON);
    // The kernel answers the press, and the press is over before the idle point can see it.
    run.run_to("kn_paused", WAIT);
    run.m().release_key(Machine::KEY_ON);
    let flags = run.m().read_ram(0x89F9, 1)[0];
    run.m().load_ram(0x89F9, &[flags & !0x10]);
    let frame = run.next_blit();
    assert_eq!(run.top_kind(), KIND_CHECKING, "the screen is open");
    assert_ne!(frame, libraries);
    let back = run.next_blit();
    same_frame("Libraries after the first idle call", &back, &libraries);
    assert_eq!(run.depth(), 1);
    assert_eq!(run.raw_table(), None, "no table was created");
    assert_eq!(run.verifies(), 0, "no step ran");
    run.m().release_key(F2);
    run.run(3_000_000);
    assert_eq!(run.depth(), 1, "the app did not leave");
}

/// The status the error notice shows is ((class - CK_REFUSED) * 32) + (detail and 1Fh), the detail
/// taken as 0 for CK_RANGE and CK_BAD_STATE, whatever B holds.
#[test]
fn the_error_notice_shows_the_class_and_the_detail_of_the_jobs_status() {
    let built = with_library();
    let mut app = App::boot(built);
    let cases: [(u8, u8, u8); 11] = [
        (CK_REFUSED, 1, 0x01),
        (CK_REFUSED, 17, 0x11),
        (CK_REFUSED, 0x1F, 0x1F),
        (CK_REFUSED, 0xE3, 0x03),
        (CK_STORE, 2, 0x22),
        (CK_STORE, 7, 0x27),
        (CK_RANGE, 0x55, 0x40),
        (CK_READER, 3, 0x63),
        (CK_READER, 0xFF, 0x7F),
        (CK_BAD_STATE, 9, 0x80),
        (CK_BAD_STATE, 0, 0x80),
    ];
    for (class, detail, want) in cases {
        let done = app.call(
            BASE,
            built.label("checking_status"),
            In {
                a: class,
                bc: u16::from(detail) << 8 | 0x77,
                f: 0xFF,
                ..In::default()
            },
        );
        assert_eq!(done.status(), want, "class {class}, detail {detail:02X}");
    }
}
