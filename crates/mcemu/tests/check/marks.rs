//! The table after a whole job against the host model's marks.

use crate::damage::libraries;
use crate::layout::Table;
use crate::rig::*;

#[test]
fn a_whole_job_leaves_the_marks_the_host_model_gives() {
    let libs = libraries();
    let mut resting = 0;
    let mut failed = 0;
    for (name, bytes, passes) in &libs {
        let want = marks(bytes, passes);
        resting += want.iter().filter(|&&v| v == 3).count();
        failed += want.iter().filter(|&&v| v == 2).count();
        let mut job = Job::new(bytes);
        let count = count_of(bytes);
        let out = job.job(count, 65535);
        assert_eq!(out.a, DONE, "{name}");
        let table = job.table().expect("the job made a table");
        assert_eq!(table.values, want, "{name}");
        assert_eq!(table.crc, header_crc(bytes), "{name}");
        assert_eq!(table.count(), count, "{name}");
        let counts = job.counts();
        let tally = |v: u8| want.iter().filter(|&&w| w == v).count() as u16;
        assert_eq!(counts, [0, tally(1), tally(2), tally(3)], "{name}");
        let first = want.iter().position(|&v| v == 2);
        assert_eq!(
            job.first_failure().map(|(n, _)| usize::from(n)),
            passes.iter().position(|&p| !p),
            "{name}"
        );
        let _ = first;
    }
    assert!(failed > 0, "some assertion fails");
    assert!(resting > 0, "some assertion rests on a failed one");
    let _ = Table::new(0, 0);
}
