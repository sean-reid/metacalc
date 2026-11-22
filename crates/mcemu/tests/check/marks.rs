//! The table after a whole job against the host model's marks.

use crate::damage::libraries;
use crate::rig::*;

#[test]
fn a_whole_job_leaves_the_marks_the_host_model_gives() {
    let libs = libraries();
    let damaged = libs.iter().filter(|l| l.0.contains("damaged")).count();
    assert!(
        damaged >= 1,
        "the generator made {damaged} damaged libraries"
    );
    let (mut on_ones_and_threes, mut on_a_two, mut failed, mut resting) = (0, 0, 0, 0);
    for (name, bytes, passes) in &libs {
        let want = marks(bytes, passes);
        let refs = references(bytes);
        for (i, &v) in want.iter().enumerate() {
            failed += usize::from(v == 2);
            if v != 3 {
                continue;
            }
            resting += 1;
            let marks_of = |r: &[usize]| r.iter().map(|&j| want[j]).collect::<Vec<_>>();
            let listed = marks_of(&refs[i]);
            on_a_two += usize::from(listed.contains(&2));
            on_ones_and_threes +=
                usize::from(listed.iter().all(|&m| m == 1 || m == 3) && listed.contains(&3));
        }
        let mut job = Job::new(bytes);
        let count = count_of(bytes);
        let out = job.job(count, 65535);
        assert_eq!(out.a, DONE, "{name}");
        let table = job.table().expect("the job made a table");
        assert_eq!(table.values, want, "{name}");
        assert_eq!(table.crc, header_crc(bytes), "{name}");
        assert_eq!(table.count(), count, "{name}");
        let tally = |v: u8| want.iter().filter(|&&w| w == v).count() as u16;
        assert_eq!(job.counts(), [0, tally(1), tally(2), tally(3)], "{name}");
    }
    assert!(failed > 0, "some assertion fails");
    assert!(resting > 0, "some assertion rests on one that fails");
    assert!(on_a_two > 0, "a 3 rests directly on a 2");
    assert!(
        on_ones_and_threes > 0,
        "a 3 rests on 1s and 3s only, a chain of resting"
    );
}
