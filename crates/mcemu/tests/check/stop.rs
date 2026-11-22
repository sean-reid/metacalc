//! ON stops a job at an assertion boundary or inside a verify, and a resumed job finishes with
//! the table an uninterrupted job leaves.

use crate::damage::{big, libraries};
use crate::rig::*;

/// Steps `k` assertions, so that the table and the counters are in the state a pause finds them.
fn at_step(bytes: &[u8], k: u16) -> Job {
    let mut job = Job::new(bytes);
    let out = job.job(count_of(bytes), k);
    assert!(out.a == MORE || out.a == DONE, "{out:?}");
    job
}

fn finish(job: &mut Job, bytes: &[u8]) -> Vec<u8> {
    job.end();
    let out = job.job(count_of(bytes), 65535);
    assert_eq!(out.a, DONE);
    job.table().unwrap().values
}

#[test]
fn on_at_every_assertion_boundary_writes_nothing_and_a_resumed_job_finishes_the_table() {
    let mut taken = 0;
    for (name, bytes, passes) in libraries() {
        let want = marks(&bytes, &passes);
        let count = count_of(&bytes);
        let stride = (count / 12).max(1);
        for k in (1..count).step_by(usize::from(stride)) {
            let mut job = at_step(&bytes, k);
            let before = job.raw();
            let counts = job.counts();
            let calls = job.kernel_calls();
            let (out, pressed) = job.call_with_on("t_step", 0);
            assert!(pressed);
            assert_eq!(out.a, PAUSED, "{name} at {k}");
            assert_eq!(job.raw(), before, "{name} at {k}: nothing is written");
            assert_eq!(job.counts(), counts, "{name} at {k}: the counters stand");
            assert!(job.kernel_calls()[4] > calls[4], "{name}: the verify ran");
            job.release_on();
            assert_eq!(finish(&mut job, &bytes), want, "{name} resumed at {k}");
            taken += 1;
        }
    }
    assert!(taken > 20);
}

#[test]
fn on_inside_a_verify_writes_nothing_for_that_assertion_and_a_resumed_job_finishes() {
    let mut inside = 0;
    for (name, bytes, passes) in libraries() {
        let want = marks(&bytes, &passes);
        let count = count_of(&bytes);
        for k in (1..count).step_by(usize::from((count / 6).max(1))) {
            for after in [300, 3_000, 30_000] {
                let mut job = at_step(&bytes, k);
                let before = job.raw();
                let counts = job.counts();
                let (out, pressed) = job.call_with_on("t_step", after);
                if !pressed {
                    continue;
                }
                if out.a == PAUSED {
                    assert_eq!(job.raw(), before, "{name} at {k}: nothing is written");
                    assert_eq!(job.counts(), counts);
                    inside += 1;
                } else {
                    assert!(out.a == MORE || out.a == DONE, "{name} at {k}: {out:?}");
                }
                job.release_on();
                assert_eq!(
                    finish(&mut job, &bytes),
                    want,
                    "{name} at {k} after {after}"
                );
            }
        }
    }
    assert!(inside > 20, "{inside} presses landed inside a step");
}

#[test]
fn two_pauses_in_one_job_still_finish_the_same_table() {
    let (name, bytes, passes) = big();
    let want = marks(&bytes, &passes);
    let count = count_of(&bytes);
    let mut job = at_step(&bytes, 3);
    job.call_with_on("t_step", 0);
    job.release_on();
    job.end();
    job.arg(LIB_PAGE, count, 5);
    let out = job.call("t_job");
    assert_eq!(out.a, MORE, "{name}");
    job.call_with_on("t_step", 0);
    job.release_on();
    assert_eq!(finish(&mut job, &bytes), want);
}
