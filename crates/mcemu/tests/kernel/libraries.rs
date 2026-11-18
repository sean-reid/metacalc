use crate::support::*;

// T-state ceilings per library, 5 percent above the measured cost; they fail a kernel change that slows verification.
const CEILINGS: [(&str, u32); 3] = [
    ("prop", 74_400_000),
    ("pred", 140_800_000),
    ("ring", 827_600_000),
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
