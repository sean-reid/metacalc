//! The paths a failure takes through the job: ON in the kernel's first calls, the reader
//! failing, the kernel refusing after a pass, the retry of kernel_refs, and the first failure kept.

use crate::damage::{libraries, named};
use crate::rig::*;

/// A job on `bytes` that has taken its CRC run and `steps` steps.
fn stepped(bytes: &[u8], steps: u16) -> Job {
    let mut job = Job::new(bytes);
    let mut out = job.begin(count_of(bytes));
    if out.a == NEED_CRC {
        out = job.run_crc(1000);
    }
    assert_eq!(out.a, READY);
    if steps > 0 {
        assert_eq!(job.run_steps(steps).a, MORE);
    }
    job
}

/// The first assertion of the library with a reference list.
fn first_with_refs(bytes: &[u8]) -> u16 {
    references(bytes)
        .iter()
        .position(|r| !r.is_empty())
        .expect("an assertion with references") as u16
}

#[test]
fn on_during_kernel_init_pauses_begin_and_leaves_nothing_defined_but_zero() {
    let bytes = long_name_axioms(40, 60);
    let mut job = Job::new(&bytes);
    job.arg(LIB_PAGE, count_of(&bytes), 0);
    let (out, pressed) = job.call_with_on("t_begin", 0);
    assert!(pressed);
    assert_eq!(out.a, PAUSED);
    assert!(job.raw().is_none());
    job.release_on();
    assert_eq!(job.counts(), [0; 4]);
    assert_eq!(job.first_failure(), None);
    assert_eq!(job.step().a, BAD_STATE);
    assert_eq!(job.begin(count_of(&bytes)).a, NEED_CRC, "begun again");
}

#[test]
fn on_during_the_first_crc_call_pauses_it_and_the_next_run_starts_with_a_begin() {
    let bytes = axioms(700);
    let mut job = Job::new(&bytes);
    assert_eq!(job.begin(count_of(&bytes)).a, NEED_CRC);
    let (out, pressed) = job.call_with_on("t_crc_step", 0);
    assert!(pressed);
    assert_eq!(out.a, PAUSED);
    assert!(job.raw().is_none());
    assert_eq!(job.kernel_calls()[2], 1, "the call was kernel_crc_begin");
    assert_eq!(job.kernel_calls()[3], 0);
    job.release_on();
    job.clear_kernel_calls();
    assert_eq!(job.run_crc(1000).a, READY);
    let k = job.kernel_calls();
    assert_eq!((k[2], k[3]), (1, 3), "begun again, then every chunk");
}

#[test]
fn a_reader_that_fails_in_begin_is_reported_with_its_status_and_nothing_is_created() {
    let bytes = named("demo0.mm").1;
    let mut job = Job::new(&bytes);
    job.poke("lr_length", &[0, 0, 0]);
    let out = job.begin(count_of(&bytes));
    assert_eq!((out.a, out.b()), (READER, job.label_byte("LIB_ERR_RANGE")));
    assert!(job.raw().is_none());
    assert_eq!(job.counts(), [0; 4]);
    assert_eq!(job.step().a, BAD_STATE);
}

#[test]
fn a_reader_that_fails_in_a_step_stops_the_job_and_writes_nothing_for_that_assertion() {
    let bytes = named("anatomy.mm").1;
    let at = first_with_refs(&bytes);
    let mut job = stepped(&bytes, at);
    let before = job.raw();
    job.poke("lr_length", &[0x10, 0, 0]);
    let out = job.step();
    assert_eq!((out.a, out.b()), (READER, job.label_byte("LIB_ERR_RANGE")));
    assert_eq!(job.raw(), before);
    assert_eq!(job.step().a, BAD_STATE);
    assert_eq!(job.end().a, 0);
    assert_eq!(job.raw(), before, "ending a stopped job writes nothing");
}

#[test]
fn a_kernel_refusal_after_a_pass_stops_the_job_and_writes_nothing_for_that_assertion() {
    let bytes = named("anatomy.mm").1;
    let at = first_with_refs(&bytes);
    let mut job = stepped(&bytes, at);
    let before = job.raw();
    let page = job.label_byte("KERNEL_RAM_PAGE");
    let listed = usize::from(label("kv_listed")) - 0xC000;
    let out = job.call_hooked("t_step", BASE, "far_kernel_refs", |m| {
        assert_eq!(m.ram_page(page)[listed], 1, "the verify left a list");
        m.ram_page(page)[listed] = 0;
    });
    assert_eq!(
        (out.a, out.b()),
        (REFUSED, 17),
        "the kernel's NoReferences for a verify that passed"
    );
    assert_eq!(job.raw(), before);
    assert_eq!(job.step().a, BAD_STATE);
}

#[test]
fn a_tap_inside_kernel_refs_is_asked_again_when_the_os_takes_it_and_pauses_when_it_stays() {
    let bytes = named("anatomy.mm").1;
    let mut with_os = stepped(&bytes, 0);
    with_os.enter_as(0, true);
    let before = with_os.raw();
    let out = with_os.tap_in_kernel("t_step", "kernel_refs", 30);
    assert_eq!(out.a, MORE, "interrupts on: the retry finds no stop");
    assert_ne!(with_os.raw(), before, "the mark was written");
    assert_eq!(with_os.kernel_calls()[5], 2, "kernel_refs was asked twice");

    let mut without = stepped(&bytes, 0);
    let before = without.raw();
    let out = without.tap_in_kernel("t_step", "kernel_refs", 30);
    assert_eq!(
        out.a, PAUSED,
        "interrupts off: the latched tap is still there"
    );
    assert_eq!(without.raw(), before, "nothing was written");
    assert_eq!(without.kernel_calls()[5], 2, "asked twice, then paused");
}

#[test]
fn the_first_failure_is_the_first_assertion_refused_with_the_kernels_status_and_is_kept() {
    let (mut with_failures, mut with_two) = (0, 0);
    for (name, bytes, _) in libraries() {
        let want = failures(&bytes);
        let mut job = Job::new(&bytes);
        assert_eq!(job.job(count_of(&bytes), 65535).a, DONE, "{name}");
        assert_eq!(job.first_failure(), want.first().copied(), "{name}");
        with_failures += usize::from(!want.is_empty());
        with_two += usize::from(want.len() >= 2);
    }
    assert!(
        with_failures >= 4,
        "{with_failures} libraries fail somewhere"
    );
    assert!(with_two >= 2, "{with_two} libraries fail twice");
}

/// Fills the job's whole block with a byte, as a run that stopped somewhere leaves it.
fn dirty(job: &mut Job, byte: u8) {
    let used = usize::from(label("CK_USED"));
    job.poke("ck_params", &vec![byte; used]);
}

/// What a begin is given and what it must answer: the library, N, the status, whether ON is held,
/// and the library page.
type Case<'a> = (&'a str, &'a [u8], u16, u8, bool, u8);

#[test]
fn begin_leaves_the_counters_the_first_failure_and_the_chunk_count_zero_on_every_refusal() {
    let small = named("demo0.mm").1;
    let count = count_of(&small);
    let long = long_name_axioms(40, 60);
    let cases: [Case; 5] = [
        (
            "paused in kernel_init",
            &long,
            count_of(&long),
            PAUSED,
            true,
            LIB_PAGE,
        ),
        (
            "refused by kernel_init",
            &small,
            count,
            REFUSED,
            false,
            0x90,
        ),
        (
            "end above the count",
            &small,
            count + 1,
            RANGE,
            false,
            LIB_PAGE,
        ),
        ("no table yet", &small, count, NEED_CRC, false, LIB_PAGE),
        ("reader failing", &small, count, READER, false, LIB_PAGE),
    ];
    for (what, bytes, n, status, on, page) in cases {
        let mut job = Job::new(bytes);
        if status == READER {
            job.poke("lr_length", &[0, 0, 0]);
        }
        dirty(&mut job, 0xAA);
        job.arg(page, n, 0);
        let out = if on {
            job.call_with_on("t_begin", 0).0
        } else {
            job.call("t_begin")
        };
        assert_eq!(out.a, status, "{what}");
        assert_eq!(job.counts(), [0; 4], "{what}: the counters");
        assert_eq!(job.first_failure(), None, "{what}: the first failure");
        assert_eq!(job.byte("ck_chunks"), 0, "{what}: the chunk count");
        assert_eq!(job.byte("ck_have_first"), 0, "{what}");
        if on {
            job.release_on();
        }
    }
}
