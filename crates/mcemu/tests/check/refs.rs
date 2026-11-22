//! Reference lists: longer than the buffer, with dummy entries, and none at all.

use crate::cases::{TH2, base, long_source};
use crate::rig::*;

fn run_with_prior(bytes: &[u8], set: &[(usize, u8)]) -> Vec<u8> {
    let count = usize::from(count_of(bytes));
    let mut values = vec![0; count];
    for &(i, v) in set {
        values[i] = v;
    }
    let mut job = Job::new(bytes);
    job.put_table(&table_for(bytes, values), 0);
    let out = job.job(count as u16, 65535);
    assert_eq!(out.a, DONE, "{out:?}");
    job.table().unwrap().values
}

#[test]
fn a_list_longer_than_the_buffer_is_read_in_pieces_and_every_entry_counts() {
    for entries in [121, 130, 240, 241, 300] {
        let bytes = pack_source(&long_source(entries));
        let count = usize::from(count_of(&bytes));
        let big = count - 1;
        let list = &references(&bytes)[big];
        assert!(list.len() >= entries - 1, "{} entries", list.len());
        let all = run_with_prior(&bytes, &[]);
        assert_eq!(all[big], 1, "{entries}: every entry is 1");
        let ones = vec![1; count - 1];
        assert_eq!(&all[..big], &ones[..]);
        for &at in &[0, 1, 119, 120, 121, list.len() / 2, list.len() - 1] {
            if at >= list.len() {
                continue;
            }
            for bad in [2, 3] {
                let got = run_with_prior(&bytes, &[(list[at], bad)]);
                assert_eq!(got[big], 3, "{entries} entries, entry {at} is {bad}");
            }
        }
    }
}

#[test]
fn a_dummy_entry_is_skipped_and_the_assertions_beside_it_decide_the_mark() {
    let bytes = base();
    let lists = references(&bytes);
    let stored = crate::cases::spot(&bytes, TH2);
    assert_eq!(stored.count, 3, "two assertions and a dummy");
    assert_eq!(lists[TH2].len(), 2);
    let all = run_with_prior(&bytes, &[]);
    assert_eq!(all[TH2], 1);
    for &r in &lists[TH2] {
        assert_eq!(
            run_with_prior(&bytes, &[(r, 2)])[TH2],
            3,
            "reference {r} failed"
        );
        assert_eq!(
            run_with_prior(&bytes, &[(r, 3)])[TH2],
            3,
            "reference {r} rests"
        );
    }
}

#[test]
fn an_axiom_has_no_list_and_is_marked_checked() {
    let bytes = base();
    let all = run_with_prior(&bytes, &[]);
    let lists = references(&bytes);
    let axioms: Vec<usize> = (0..lists.len()).filter(|&i| lists[i].is_empty()).collect();
    assert!(axioms.len() >= 5);
    for i in axioms {
        assert_eq!(all[i], 1, "assertion {i}");
    }
}

#[test]
fn the_list_is_fetched_once_per_passing_assertion_and_never_for_a_failure() {
    let mut job = Job::new(&corpus("anatomy-bad1.mm"));
    let bytes = corpus("anatomy-bad1.mm");
    let out = job.job(count_of(&bytes), 65535);
    assert_eq!(out.a, DONE);
    let k = job.kernel_calls();
    let passes = verdicts(&bytes).unwrap().iter().filter(|&&p| p).count();
    assert_eq!(usize::from(k[5]), passes);
    assert!(usize::from(k[4]) > passes);
}
