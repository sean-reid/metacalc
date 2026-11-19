use crate::support::*;
use mcpack::library::{Kernel, WORK_LIMIT};

// T-state ceilings per library, 5 percent above the measured cost; they fail a kernel change that slows verification.
const CEILINGS: [(&str, u32); 3] = [
    ("prop", 75_300_000),
    ("pred", 142_600_000),
    ("ring", 839_300_000),
];

// T-state ceilings of kernel_init alone, 5 percent above the measured cost.
const INIT_CEILINGS: [(&str, u32); 3] = [("prop", 155_300), ("pred", 246_900), ("ring", 2_485_000)];

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_verify_on_the_device_as_the_model_does() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for (name, ceiling) in CEILINGS {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        let mut device = Device::with_library(&bytes);
        let (init, got, tstates) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, &got), (want_init, &want), "{name}");
        let mut init_only = Device::with_library(&bytes);
        init_only.params(ASKED_UNBOUNDED);
        let init_ceiling = INIT_CEILINGS.iter().find(|c| c.0 == name).unwrap().1;
        let init_tstates = init_only.run("ENTRY_IFF");
        println!("{name}: kernel_init alone {init_tstates} T-states");
        assert!(
            init_tstates <= init_ceiling,
            "{name}: init took {init_tstates} T-states, over the ceiling of {init_ceiling}"
        );
        let accepted = got.iter().filter(|&&s| s == 0).count();
        println!(
            "{name}: {accepted} of {} accepted, {tstates} T-states, {:.1} s at 15 MHz, {:.1} s at 6 MHz",
            got.len(),
            f64::from(tstates) / 15e6,
            f64::from(tstates) / 6e6,
        );
        assert_eq!(accepted, got.len(), "{name}");
        assert!(
            tstates <= ceiling,
            "{name} took {tstates} T-states, over the ceiling of {ceiling}"
        );
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn no_honest_proof_in_a_shipped_library_charges_over_an_eighth_of_the_work_limit() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let mut largest = (0, String::new());
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).expect("library loads");
        let mut worst = (0, 0);
        for id in 0..kernel.assertions() {
            assert_eq!(kernel.verify(id), Ok(()), "{name} {id}");
            if kernel.work() > worst.0 {
                worst = (kernel.work(), id);
            }
        }
        let (work, id) = worst;
        let label = String::from_utf8_lossy(kernel.label(id).unwrap()).into_owned();
        let mut device = Device::with_library(&bytes);
        assert_eq!(
            device.verify(id, ASKED_UNBOUNDED).verify,
            0,
            "{name} {label}"
        );
        assert_eq!(
            device.work_charged(),
            work,
            "{name} {label}: the device's count"
        );
        println!("{name}: largest count {work}, in {label}");
        if work > largest.0 {
            largest = (work, format!("{name} {label}"));
        }
    }
    println!(
        "largest count {} in {}, limit {WORK_LIMIT}",
        largest.0, largest.1
    );
    assert!(
        u64::from(largest.0) * 8 <= u64::from(WORK_LIMIT),
        "{} is over an eighth of {WORK_LIMIT}",
        largest.0
    );
}
