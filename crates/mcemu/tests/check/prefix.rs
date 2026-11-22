//! Jobs below an end N, over tables a job left half done, and the "next" number in the header.

use crate::damage::{big, libraries};
use crate::layout::Table;
use crate::rig::*;

/// A table a real job could have left: the final marks of a random set of assertions closed under
/// the assertions they name, every other value 0.
pub fn prior(bytes: &[u8], want: &[u8], seed: &mut u32, per_cent: u32) -> Vec<u8> {
    let refs = references(bytes);
    let mut kept = vec![false; want.len()];
    for i in 0..want.len() {
        kept[i] = xorshift(seed) % 100 < per_cent && refs[i].iter().all(|&r| kept[r]);
    }
    (0..want.len())
        .map(|i| if kept[i] { want[i] } else { 0 })
        .collect()
}

#[test]
fn a_job_over_a_half_done_table_finishes_with_the_host_models_marks() {
    let mut seed = 0xFACE_0001;
    for (name, bytes, passes) in libraries() {
        let want = marks(&bytes, &passes);
        for per_cent in [0, 30, 70, 95] {
            let table = table_for(&bytes, prior(&bytes, &want, &mut seed, per_cent));
            let mut job = Job::new(&bytes);
            job.put_table(&table, 0xFF);
            let out = job.job(count_of(&bytes), 65535);
            assert_eq!(out.a, DONE, "{name} {per_cent}%");
            assert_eq!(job.table().unwrap().values, want, "{name} {per_cent}%");
            assert_eq!(job.kernel_calls()[3], 0, "the table's key skipped the run");
            let kept = table.values.iter().filter(|&&v| v != 0).count();
            assert_eq!(
                usize::from(job.kernel_calls()[4]) + kept,
                want.len(),
                "{name} {per_cent}%: only the unchecked were verified"
            );
        }
    }
}

#[test]
fn a_job_below_n_leaves_everything_at_and_above_n_alone() {
    let mut seed = 0xFACE_0002;
    for (name, bytes, passes) in libraries() {
        let want = marks(&bytes, &passes);
        let count = count_of(&bytes);
        for n in [0, 1, count / 3, count / 2, count - 1, count] {
            let mut values = prior(&bytes, &want, &mut seed, 20);
            let junk = xorshift(&mut seed) as usize;
            for v in &mut values[usize::from(n)..] {
                *v = (junk % 4) as u8;
            }
            let table = table_for(&bytes, values.clone());
            let mut job = Job::new(&bytes);
            job.put_table(&table, 0);
            let out = job.job(n, 65535);
            assert_eq!(out.a, DONE, "{name} N {n}");
            let got = job.table().unwrap().values;
            assert_eq!(
                &got[usize::from(n)..],
                &values[usize::from(n)..],
                "{name} N {n}"
            );
            for i in 0..usize::from(n) {
                if values[i] == 0 {
                    assert_eq!(got[i], want[i], "{name} N {n} assertion {i}");
                } else {
                    assert_eq!(got[i], values[i], "{name} N {n} assertion {i}");
                }
            }
            let counts = job.counts();
            let zeros = got[..usize::from(n)].iter().filter(|&&v| v == 0).count();
            assert_eq!((counts[0] as usize, zeros), (0, 0), "{name} N {n}");
            let total: u16 = counts.iter().sum();
            assert_eq!(total, n, "{name} N {n}: the counters cover the prefix");
        }
    }
}

#[test]
fn an_end_above_the_count_is_refused() {
    let (_, bytes, _) = libraries().remove(0);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    assert_eq!(job.begin(count + 1).a, RANGE);
    assert!(job.raw().is_none());
    assert_eq!(job.step().a, BAD_STATE);
}

#[test]
fn a_stale_or_wrong_next_never_makes_the_job_skip_an_unchecked_assertion() {
    let mut seed = 0xFACE_0003;
    for (name, bytes, passes) in libraries() {
        let want = marks(&bytes, &passes);
        let count = count_of(&bytes);
        for trial in 0..8 {
            let values = prior(&bytes, &want, &mut seed, 60);
            let first_zero = values.iter().position(|&v| v == 0).unwrap_or(values.len());
            let mut table = table_for(&bytes, values);
            table.next = match trial {
                0 => count,
                1 => count - 1,
                2 => first_zero as u16 + 1,
                3 => first_zero as u16,
                4 => 0,
                5 => (xorshift(&mut seed) % u32::from(count + 1)) as u16,
                6 => count.saturating_sub(first_zero as u16 / 2),
                _ => (first_zero as u16 + count) / 2,
            };
            let mut job = Job::new(&bytes);
            job.put_table(&table, 0);
            let n = if trial % 2 == 0 { count } else { count - 1 };
            let out = job.job(n, 65535);
            assert_eq!(out.a, DONE, "{name} trial {trial}");
            let got = job.table().unwrap().values;
            for (i, &v) in got.iter().take(usize::from(n)).enumerate() {
                assert_ne!(v, 0, "{name} trial {trial}: assertion {i} stayed unchecked");
            }
        }
    }
}

#[test]
fn a_next_past_the_count_in_the_header_is_no_hint() {
    let (name, bytes, passes) = big();
    let want = marks(&bytes, &passes);
    let mut table = table_for(&bytes, vec![0; want.len()]);
    table.next = 0xFFFF;
    let mut job = Job::new(&bytes);
    job.put_table(&table, 0);
    assert_eq!(job.job(count_of(&bytes), 65535).a, DONE, "{name}");
    assert_eq!(job.table().unwrap().values, want);
}

#[test]
fn next_is_written_when_the_job_pauses_or_ends_and_is_what_a_resume_starts_at() {
    let (_, bytes, passes) = big();
    let want = marks(&bytes, &passes);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    assert_eq!(job.job(count, 7).a, MORE);
    let t = job.table().unwrap();
    assert_eq!(t.next, 0, "nothing writes next while the job runs");
    job.end();
    assert_eq!(job.table().unwrap().next, 7);
    job.clear_kernel_calls();
    assert_eq!(job.job(count, 65535).a, DONE);
    assert_eq!(
        usize::from(job.kernel_calls()[4]),
        want.len() - 7,
        "the resume starts at the first unchecked"
    );
    job.end();
    assert_eq!(job.table().unwrap().next, count);
}

#[test]
fn a_table_of_all_ones_makes_the_job_done_at_once() {
    let (_, bytes, _) = libraries().remove(0);
    let table = table_for(&bytes, vec![1; usize::from(count_of(&bytes))]);
    let mut job = Job::new(&bytes);
    job.put_table(&table, 0);
    let out = job.begin(count_of(&bytes));
    assert_eq!(out.a, READY);
    assert_eq!(job.step().a, DONE);
    assert_eq!(job.kernel_calls()[4], 0);
    let _ = Table::new(0, 0);
}
