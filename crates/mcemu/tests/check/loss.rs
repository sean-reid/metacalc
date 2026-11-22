//! The table deleted, archived or replaced between two steps.

use crate::damage::big;
use crate::layout::Table;
use crate::rig::*;

fn running() -> (Job, Vec<u8>) {
    let (_, bytes, _) = big();
    let mut job = Job::new(&bytes);
    assert_eq!(job.job(count_of(&bytes), 4).a, MORE);
    (job, bytes)
}

#[test]
fn a_table_deleted_between_steps_stops_the_job_and_none_is_made() {
    let (mut job, _) = running();
    job.m.delete_var(VAR);
    let out = job.step();
    assert_eq!((out.a, out.b()), (STORE, ST_ABSENT));
    assert!(
        job.raw().is_none(),
        "nothing is created in the middle of a job"
    );
    assert_eq!(job.step().a, BAD_STATE);
    let out = job.end();
    assert_eq!(out.a, 0, "ending a stopped job writes nothing");
    assert!(job.raw().is_none());
}

#[test]
fn a_table_archived_between_steps_stops_the_job_and_stays_archived() {
    let (mut job, _) = running();
    job.m.archive_var(VAR);
    let out = job.step();
    assert_eq!((out.a, out.b()), (STORE, ST_ARCHIVED));
    assert!(job.m.read_var(VAR).unwrap().archived);
    job.end();
    assert!(job.m.read_var(VAR).unwrap().archived);
}

#[test]
fn a_table_with_nothing_to_check_under_the_name_is_not_written_to() {
    let (mut job, bytes) = running();
    let mut other = Table::new(header_crc(&bytes), count_of(&bytes) + 5);
    other.values.fill(2);
    job.put_table(&other, 0);
    let before = job.raw();
    let out = job.step();
    assert_eq!(out.a, DONE);
    assert_eq!(job.raw(), before);
}

#[test]
fn a_whole_table_of_another_library_put_under_the_name_between_steps_is_written_to() {
    let (mut job, bytes) = running();
    let count = count_of(&bytes);
    let other = Table::new(header_crc(&bytes) ^ 1, count);
    job.put_table(&other, 0);
    let out = job.step();
    assert_eq!(out.a, MORE, "the job does not notice: {out:?}");
    let table = job.table().unwrap();
    assert_eq!(
        table.crc, other.crc,
        "it is still the other library's table"
    );
    let written: Vec<usize> = (0..table.values.len())
        .filter(|&i| table.values[i] != 0)
        .collect();
    assert_eq!(written, [4], "the step wrote its mark into it");
}

#[test]
fn a_stopped_job_begins_again_with_a_crc_run_and_a_new_table() {
    let (mut job, bytes) = running();
    job.m.delete_var(VAR);
    assert_eq!(job.step().a, STORE);
    job.end();
    let count = count_of(&bytes);
    assert_eq!(job.begin(count).a, NEED_CRC);
    assert_eq!(job.run_crc(100).a, READY);
    assert_eq!(job.table().unwrap().values, vec![0; usize::from(count)]);
}
