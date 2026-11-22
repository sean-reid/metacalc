//! The shipped libraries through the whole job, and what the job costs over the kernel alone.

use crate::rig::*;

const NAMES: [&str; 3] = ["prop", "pred", "ring"];
const COUNTS: [u16; 3] = [779, 1171, 3326];

// T-states of a whole job (begin, CRC run, every step) and of its work beyond the kernel's own
// run, a little above what the stub measures.
const JOB_CEILINGS: [u32; 3] = [97_500_000, 176_500_000, 945_000_000];
const OVERHEAD_CEILINGS: [u32; 3] = [14_400_000, 22_200_000, 71_700_000];

fn library(name: &str) -> Vec<u8> {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads")
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn a_whole_job_over_each_shipped_library_leaves_every_assertion_checked() {
    for (k, name) in NAMES.iter().enumerate() {
        let bytes = library(name);
        let count = count_of(&bytes);
        assert_eq!(count, COUNTS[k], "{name}: the shipped library's assertions");
        let mut bare = Job::new(&bytes);
        bare.arg(LIB_PAGE, count, 0);
        let bare = bare.call("t_bare").tstates;
        let mut job = Job::new(&bytes);
        let out = job.job(count, 65535);
        assert_eq!(out.a, DONE, "{name}");
        let table = job.table().unwrap();
        assert_eq!(table.values, vec![1; usize::from(count)], "{name}");
        assert_eq!(job.counts(), [0, count, 0, 0], "{name}");
        let calls = job.calls();
        let overhead = out.tstates.saturating_sub(bare);
        println!(
            "{name}: job {} T-states over {calls} step calls, kernel alone {bare}, overhead {overhead} ({:.2} percent)",
            out.tstates,
            100.0 * f64::from(overhead) / f64::from(bare)
        );
        assert!(
            out.tstates <= JOB_CEILINGS[k],
            "{name}: job {}",
            out.tstates
        );
        assert!(
            overhead <= OVERHEAD_CEILINGS[k],
            "{name}: overhead {overhead}"
        );
    }
}

fn median(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn a_steps_own_work_beyond_the_verify_is_measured_on_ring() {
    let bytes = library("ring");
    let count = count_of(&bytes);
    assert_eq!(count, COUNTS[2], "ring's assertions");
    let refs = references(&bytes);
    let mut job = Job::new(&bytes);
    assert_eq!(job.begin(count).a, NEED_CRC);
    assert_eq!(job.run_crc(1000).a, READY);
    let mut verifier = Job::new(&bytes);
    verifier.arg(LIB_PAGE, 0, 0);
    assert_eq!(verifier.call("t_init").a, 0);
    let mut own = Vec::new();
    for i in 0..count {
        let step = job.step();
        assert_eq!(step.a, if i + 1 == count { DONE } else { MORE }, "step {i}");
        verifier.poke("T_IN_DE", &i.to_le_bytes());
        let verify = verifier.call("t_verify");
        assert_eq!(verify.a, 0);
        own.push((
            step.tstates.saturating_sub(verify.tstates),
            refs[usize::from(i)].len(),
        ));
    }
    let mut all: Vec<u32> = own.iter().map(|o| o.0).collect();
    let worst = *own.iter().max().unwrap();
    let med = median(&mut all);
    println!(
        "ring: a step's own work, median {med} T-states, worst {} T-states with {} references",
        worst.0, worst.1
    );
    assert!(med <= 19_800, "median {med}");
    assert!(worst.0 <= 70_200, "worst {}", worst.0);
}
