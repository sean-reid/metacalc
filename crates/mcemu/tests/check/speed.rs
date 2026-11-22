//! Every routine of the job gives the caller's speed and interrupt state back as it found them,
//! the kernel runs at 15 MHz and the store at the caller's speed.

use crate::damage::named;
use crate::rig::*;

/// The caller's speed port, 0 for 6 MHz and 1 for 15 MHz, and its interrupt state.
const STATES: [(u8, bool); 4] = [(0, false), (0, true), (1, false), (1, true)];

fn small() -> Vec<u8> {
    named("demo0.mm").1
}

/// Calls `call` on a job from `make` in each caller state, and checks the status and that the
/// speed and the interrupt state are the caller's again.
fn comes_back(what: &str, make: &dyn Fn() -> Job, call: &dyn Fn(&mut Job) -> Out, status: u8) {
    for (speed, interrupts) in STATES {
        let mut job = make();
        job.enter_as(speed, interrupts);
        let out = call(&mut job);
        let state = format!("{what} at speed {speed}, interrupts {interrupts}: {out:?}");
        assert_eq!(out.a, status, "{state}");
        assert_eq!(out.speed, speed, "{state}");
        assert_eq!(out.iff, u8::from(interrupts), "{state}");
    }
}

fn begun(bytes: &[u8]) -> Job {
    let mut job = Job::new(bytes);
    assert_eq!(job.begin(count_of(bytes)).a, NEED_CRC);
    job
}

fn ready(bytes: &[u8]) -> Job {
    let mut job = begun(bytes);
    assert_eq!(job.run_crc(1000).a, READY);
    job
}

#[test]
fn begin_returns_the_callers_speed_and_interrupts_whatever_it_answers() {
    let bytes = small();
    let count = count_of(&bytes);
    let fresh = || Job::new(&bytes);
    comes_back(
        "begin needing the run",
        &fresh,
        &|j| j.begin(count),
        NEED_CRC,
    );
    comes_back("begin out of range", &fresh, &|j| j.begin(count + 1), RANGE);
    comes_back(
        "begin refused",
        &fresh,
        &|j| {
            j.arg(0x90, count, 0);
            j.call("t_begin")
        },
        REFUSED,
    );
    let made = || {
        let mut job = ready(&bytes);
        job.end();
        job
    };
    comes_back("begin on a table", &made, &|j| j.begin(count), READY);
    let long = long_name_axioms(40, 60);
    comes_back(
        "begin paused by ON",
        &|| Job::new(&long),
        &|j| j.call_with_on("t_begin", 0).0,
        PAUSED,
    );
}

#[test]
fn a_crc_chunk_returns_the_callers_speed_and_interrupts() {
    let bytes = axioms(700);
    let more = || begun(&bytes);
    comes_back("the first crc call", &more, &|j| j.crc_step(), MORE);
    let second = || {
        let mut job = begun(&bytes);
        assert_eq!(job.crc_step().a, MORE);
        job
    };
    comes_back("a crc chunk", &second, &|j| j.crc_step(), MORE);
    let last = || {
        let mut job = begun(&bytes);
        for _ in 0..bytes.len().div_ceil(1 << 14) {
            assert_eq!(job.crc_step().a, MORE);
        }
        job
    };
    comes_back(
        "the chunk that ends the run",
        &last,
        &|j| j.crc_step(),
        READY,
    );
    let mut bad = bytes.clone();
    bad[20_000] ^= 1;
    let refused = || {
        let mut job = Job::new(&bad);
        assert_eq!(job.begin(count_of(&bad)).a, NEED_CRC);
        for _ in 0..bytes.len().div_ceil(1 << 14) {
            assert_eq!(job.crc_step().a, MORE);
        }
        job
    };
    comes_back("a refused crc run", &refused, &|j| j.crc_step(), REFUSED);
    comes_back(
        "a crc chunk paused by ON",
        &second,
        &|j| j.call_with_on("t_crc_step", 0).0,
        PAUSED,
    );
}

#[test]
fn a_step_returns_the_callers_speed_and_interrupts() {
    let bytes = small();
    let count = count_of(&bytes);
    let started = || ready(&bytes);
    comes_back("a step that passes", &started, &|j| j.step(), MORE);
    comes_back(
        "a step paused by ON",
        &started,
        &|j| j.call_with_on("t_step", 0).0,
        PAUSED,
    );
    let last = || {
        let mut job = ready(&bytes);
        assert_eq!(job.run_steps(count - 1).a, MORE);
        job
    };
    comes_back("the step that finishes", &last, &|j| j.step(), DONE);
    let gone = || {
        let mut job = ready(&bytes);
        job.m.delete_var(VAR);
        job
    };
    comes_back("a step stopped by the store", &gone, &|j| j.step(), STORE);
    let (_, damaged, _) = crate::damage::libraries()
        .into_iter()
        .find(|l| {
            failures(&l.1)
                .first()
                .is_some_and(|f| usize::from(f.0) + 2 < usize::from(count_of(&l.1)))
        })
        .expect("a library that fails before its last assertion");
    let first = failures(&damaged)[0].0;
    let failing = || {
        let mut job = ready(&damaged);
        if first > 0 {
            assert_eq!(job.run_steps(first).a, MORE);
        }
        job
    };
    comes_back(
        "a step that fails its assertion",
        &failing,
        &|j| j.step(),
        MORE,
    );
    assert!(usize::from(failures(&damaged)[0].0) + 2 < usize::from(count_of(&damaged)));
    let finished = || {
        let mut job = ready(&bytes);
        assert_eq!(job.run_steps(1000).a, DONE);
        job
    };
    comes_back("a step with nothing left", &finished, &|j| j.step(), DONE);
}

#[test]
fn end_returns_the_callers_speed_and_interrupts() {
    let bytes = small();
    let paused = || {
        let mut job = ready(&bytes);
        assert_eq!(job.step().a, MORE);
        job
    };
    comes_back("end", &paused, &|j| j.end(), 0);
    let gone = || {
        let mut job = paused();
        job.m.delete_var(VAR);
        assert_eq!(job.step().a, STORE);
        job
    };
    comes_back("end of a stopped job", &gone, &|j| j.end(), 0);
}

#[test]
fn the_kernel_runs_at_15_mhz_whatever_speed_the_caller_has() {
    let bytes = axioms(700);
    for (speed, interrupts) in STATES {
        let mut job = Job::new(&bytes);
        job.enter_as(speed, interrupts);
        let out = job.job(count_of(&bytes), 1);
        assert_eq!(out.a, MORE);
        let seen = job.kernel_speeds();
        let state = format!("speed {speed}, interrupts {interrupts}");
        assert_eq!(
            seen,
            [Some(1); 6],
            "{state}: every kernel call is at 15 MHz"
        );
        assert_eq!(out.speed, speed, "{state}");
    }
}

/// Runs `entry` with the caller in each state and says the speed port read at every entry of the
/// store routine `routine`: each must be the caller's.
fn store_runs_at_the_callers_speed(entry: &str, routine: &str, make: &dyn Fn() -> Job) {
    for (speed, interrupts) in STATES {
        let mut job = make();
        job.enter_as(speed, interrupts);
        let seen = job.speeds_at(entry, routine);
        assert!(!seen.is_empty(), "{entry} never called {routine}");
        assert!(
            seen.iter().all(|&s| s == speed),
            "{entry}: {routine} entered at {seen:?}, caller's speed {speed}"
        );
    }
}

#[test]
fn the_store_runs_at_the_callers_speed() {
    let bytes = crate::damage::big().1;
    let with_refs = references(&bytes)
        .iter()
        .position(|r| !r.is_empty())
        .unwrap() as u16;
    let started = || {
        let mut job = ready(&bytes);
        assert_eq!(job.run_steps(with_refs).a, MORE);
        job
    };
    for routine in ["store_next_zero", "store_all_ones", "store_set"] {
        store_runs_at_the_callers_speed("t_step", routine, &started);
    }
    let made = || {
        let mut job = ready(&bytes);
        assert_eq!(job.run_steps(3).a, MORE);
        job.end();
        job
    };
    for routine in ["store_has", "store_count_ends", "store_get_next"] {
        store_runs_at_the_callers_speed("t_begin", routine, &made);
    }
    let counted = || {
        let mut job = ready(&bytes);
        assert_eq!(job.step().a, MORE);
        job
    };
    store_runs_at_the_callers_speed("t_end", "store_set_next", &counted);
    let long = axioms(700);
    let last_chunk = || {
        let mut job = begun(&long);
        for _ in 0..long.len().div_ceil(1 << 14) {
            assert_eq!(job.crc_step().a, MORE);
        }
        job
    };
    store_runs_at_the_callers_speed("t_crc_step", "store_open", &last_chunk);
}
