//! ON stops a job at every assertion boundary and at a spread of points inside a verify, and a
//! job that goes on from there ends with the table an uninterrupted job leaves.

use crate::damage::{big, named};
use crate::rig::*;

/// A job past begin and the CRC run, ready to step.
fn ready(bytes: &[u8], n: u16) -> Job {
    let mut job = Job::new(bytes);
    let mut out = job.begin(n);
    if out.a == NEED_CRC {
        out = job.run_crc(1000);
    }
    assert_eq!(out.a, READY, "{out:?}");
    job
}

/// The table a job that is never interrupted leaves.
fn uninterrupted(bytes: &[u8]) -> Vec<u8> {
    let mut job = Job::new(bytes);
    assert_eq!(job.job(count_of(bytes), 65535).a, DONE);
    job.table().unwrap().values
}

/// The counters must be the tally of the first `n` values of the table.
fn assert_counts_are_the_tally(job: &mut Job, n: u16, what: &str) {
    let values = job.table().unwrap().values;
    let tally = |v: u8| values[..usize::from(n)].iter().filter(|&&w| w == v).count() as u16;
    assert_eq!(
        job.counts(),
        [tally(0), tally(1), tally(2), tally(3)],
        "{what}"
    );
}

/// Presses ON before the step, so that the first kernel poll sees it, and checks that the pause
/// wrote nothing and left the counters.
fn pause_at_the_boundary(job: &mut Job, name: &str, at: u16) {
    let before = job.raw();
    let counts = job.counts();
    let calls = job.kernel_calls();
    let (out, pressed) = job.call_with_on("t_step", 0);
    assert!(pressed);
    assert_eq!(out.a, PAUSED, "{name} at {at}");
    assert_eq!(job.raw(), before, "{name} at {at}: nothing is written");
    assert_eq!(job.counts(), counts, "{name} at {at}: the counters stand");
    assert_eq!(
        job.kernel_calls()[5],
        calls[5],
        "{name} at {at}: no list is read for a paused verify"
    );
    assert!(job.kernel_calls()[4] > calls[4], "{name}: the verify ran");
    job.release_on();
}

#[test]
fn on_at_every_assertion_boundary_writes_nothing_and_the_resumed_job_ends_with_the_same_table() {
    for (name, bytes, passes) in [big(), named("anatomy-bad1.mm")] {
        let want = marks(&bytes, &passes);
        let count = count_of(&bytes);
        let mut job = ready(&bytes, count);
        let mut boundaries = 0;
        while job.counts()[0] > 0 {
            let at = boundaries as u16;
            pause_at_the_boundary(&mut job, &name, at);
            assert_eq!(job.end().a, 0);
            assert_eq!(job.begin(count).a, READY, "{name} resumes at {at}");
            assert_counts_are_the_tally(&mut job, count, &format!("{name} resumed at {at}"));
            let out = job.step();
            assert!(out.a == MORE || out.a == DONE, "{name} at {at}: {out:?}");
            boundaries += 1;
        }
        assert_eq!(boundaries, want.len(), "{name}: one boundary per assertion");
        job.end();
        let got = job.table().unwrap().values;
        assert_eq!(
            got,
            uninterrupted(&bytes),
            "{name}: an uninterrupted job's table"
        );
        assert_eq!(got, want, "{name}: the host model's marks");
        assert_counts_are_the_tally(&mut job, count, &name);
    }
}

#[test]
fn on_inside_a_verify_writes_nothing_for_that_assertion_and_the_job_goes_on() {
    const AFTER: [u32; 7] = [50, 400, 5_000, 40_000, 150_000, 400_000, 1_200_000];
    let (name, bytes, passes) = named("demo0.mm");
    let want = marks(&bytes, &passes);
    let count = count_of(&bytes);
    let mut job = ready(&bytes, count);
    let (mut inside, mut late, mut at) = (0, 0, 0);
    while job.counts()[0] > 0 {
        let mut advanced = false;
        for after in AFTER {
            let before = job.raw();
            let counts = job.counts();
            let (out, pressed) = job.call_with_on("t_step", after);
            if !pressed {
                assert!(out.a == MORE || out.a == DONE, "{out:?}");
                advanced = true;
                break;
            }
            if out.a != PAUSED {
                late += 1;
                assert_ne!(job.raw(), before, "{name} at {at}: the step wrote its mark");
                job.release_on();
                advanced = true;
                break;
            }
            inside += 1;
            assert_eq!(job.raw(), before, "{name} at {at} after {after}: written");
            assert_eq!(job.counts(), counts, "{name} at {at} after {after}");
            job.release_on();
        }
        if !advanced {
            let out = job.step();
            assert!(out.a == MORE || out.a == DONE, "{out:?}");
        }
        at += 1;
    }
    assert!(
        inside >= usize::from(count),
        "{inside} presses landed inside a verify, {late} too late"
    );
    job.end();
    let got = job.table().unwrap().values;
    assert_eq!(got, uninterrupted(&bytes), "{name}");
    assert_eq!(got, want, "{name}");
    assert_counts_are_the_tally(&mut job, count, &name);
}

#[test]
fn two_pauses_in_one_job_still_finish_the_same_table() {
    let (name, bytes, passes) = big();
    let want = marks(&bytes, &passes);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    assert_eq!(job.job(count, 3).a, MORE);
    job.call_with_on("t_step", 0);
    job.release_on();
    job.end();
    job.arg(LIB_PAGE, count, 5);
    let out = job.call("t_job");
    assert_eq!(out.a, MORE, "{name}");
    job.call_with_on("t_step", 0);
    job.release_on();
    job.end();
    let out = job.job(count, 65535);
    assert_eq!(out.a, DONE);
    assert_eq!(job.table().unwrap().values, want);
}
