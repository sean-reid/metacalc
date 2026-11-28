//! No table exists until the CRC run over the library has passed, and an existing table with this
//! library's key skips the run.

use crate::layout::Table;
use crate::rig::*;

fn chunks(bytes: &[u8]) -> u16 {
    bytes.len().div_ceil(1 << 14) as u16
}

#[test]
fn begin_asks_for_the_crc_and_creates_nothing_until_it_has_passed() {
    let bytes = axioms(700);
    assert!(chunks(&bytes) >= 3);
    let mut job = Job::new(&bytes);
    let out = job.begin(count_of(&bytes));
    assert_eq!(out.a, NEED_CRC);
    assert!(job.raw().is_none());
    let mut calls = 0;
    loop {
        let out = job.crc_step();
        calls += 1;
        assert!(
            job.raw().is_none() == (out.a == MORE),
            "step {calls}: {out:?}"
        );
        if out.a != MORE {
            assert_eq!(out.a, READY);
            break;
        }
    }
    assert_eq!(
        calls,
        1 + usize::from(chunks(&bytes)),
        "a begin and a chunk each"
    );
    let k = job.kernel_calls();
    assert_eq!((k[2], k[3]), (1, chunks(&bytes)));
    let t = job.table().expect("the table is made once the run passed");
    assert_eq!(t.crc, header_crc(&bytes));
    assert_eq!(t.values, vec![0; 700]);
}

#[test]
fn a_damaged_library_byte_is_refused_and_creates_nothing() {
    let good = axioms(700);
    let count = count_of(&good);
    for at in [17_000, 20_000, good.len() - 1] {
        let mut bytes = good.clone();
        bytes[at] ^= 0x40;
        let mut job = Job::new(&bytes);
        let out = job.begin(count);
        if out.a == REFUSED {
            assert!(job.raw().is_none());
            continue;
        }
        assert_eq!(out.a, NEED_CRC, "byte {at}");
        let out = job.run_crc(1000);
        assert_eq!(
            (out.a, out.b()),
            (REFUSED, 1),
            "byte {at}: the kernel's Malformed"
        );
        assert!(job.raw().is_none(), "byte {at}: nothing is created");
        assert_eq!(job.step().a, BAD_STATE, "byte {at}");
    }
}

#[test]
fn a_header_checksum_that_the_bytes_do_not_have_is_refused() {
    let mut bytes = axioms(700);
    bytes[4] ^= 1;
    let mut job = Job::new(&bytes);
    assert_eq!(job.begin(count_of(&bytes)).a, NEED_CRC);
    let out = job.run_crc(1000);
    assert_eq!((out.a, out.b()), (REFUSED, 1));
    assert!(job.raw().is_none());
}

#[test]
fn a_table_with_this_librarys_key_skips_the_run() {
    let bytes = axioms(700);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    let out = job.job(count, 65535);
    assert_eq!(out.a, DONE);
    job.end();
    job.clear_kernel_calls();
    let out = job.begin(count);
    assert_eq!(out.a, READY);
    let k = job.kernel_calls();
    assert_eq!((k[2], k[3]), (0, 0), "no kernel_crc_begin or chunk call");
    assert_eq!(k[1], 1, "kernel_init ran");
    assert_eq!(job.step().a, DONE, "everything is checked already");
    assert_eq!(job.kernel_calls()[4], 0);
}

#[test]
fn a_table_with_another_key_is_replaced_only_after_the_run_passed() {
    let bytes = axioms(700);
    let count = count_of(&bytes);
    let crc = header_crc(&bytes);
    for (other_crc, other_count) in [(crc ^ 1, count), (crc, count + 1), (crc, count - 1)] {
        let mut other = Table::new(other_crc, other_count);
        other.values.fill(3);
        let mut job = Job::new(&bytes);
        job.put_table(&other, 0);
        let before = job.raw();
        assert_eq!(job.begin(count).a, NEED_CRC);
        assert_eq!(job.crc_step().a, MORE);
        assert_eq!(job.crc_step().a, MORE);
        assert_eq!(job.raw(), before, "the other table stands mid-run");
        let out = job.run_crc(100);
        assert_eq!(out.a, READY);
        let t = job.table().unwrap();
        assert_eq!((t.crc, t.count()), (crc, count));
        assert_eq!(t.values, vec![0; 700], "replaced by a zeroed table");
    }
}

#[test]
fn a_refused_run_leaves_another_librarys_table_alone() {
    let mut bytes = axioms(700);
    let count = count_of(&bytes);
    bytes[20_000] ^= 1;
    let mut other = Table::new(1, count);
    other.values.fill(2);
    let mut job = Job::new(&bytes);
    job.put_table(&other, 0);
    let before = job.raw();
    assert_eq!(job.begin(count).a, NEED_CRC);
    assert_eq!(job.run_crc(100).a, REFUSED);
    assert_eq!(job.raw(), before);
}

#[test]
fn on_during_the_run_leaves_no_table_and_the_next_run_starts_again() {
    let bytes = axioms(700);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    assert_eq!(job.begin(count).a, NEED_CRC);
    assert_eq!(job.crc_step().a, MORE);
    assert_eq!(job.crc_step().a, MORE);
    let (out, pressed) = job.call_with_on("t_crc_step", 0);
    assert!(pressed);
    assert_eq!(out.a, PAUSED);
    assert!(job.raw().is_none());
    job.release_on();
    assert_eq!(job.byte("ck_chunks"), 0, "the run starts over");
    job.clear_kernel_calls();
    let out = job.run_crc(100);
    assert_eq!(out.a, READY);
    let k = job.kernel_calls();
    assert_eq!(
        (k[2], k[3]),
        (1, chunks(&bytes)),
        "begin once and every chunk again"
    );
    assert!(job.raw().is_some());
}

#[test]
fn a_table_archived_by_the_user_is_unarchived_when_the_run_has_passed() {
    let bytes = axioms(700);
    let count = count_of(&bytes);
    let mut job = Job::new(&bytes);
    job.job(count, 65535);
    job.end();
    job.m.archive_var(VAR);
    assert_eq!(
        job.begin(count).a,
        NEED_CRC,
        "an archived table is not found in RAM"
    );
    assert_eq!(job.run_crc(100).a, READY);
    assert!(!job.m.read_var(VAR).unwrap().archived);
}
