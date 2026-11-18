use crate::support::*;

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_verify_on_the_device_as_the_model_does() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        let mut device = Device::with_library(&bytes);
        let (init, got, tstates) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, &got), (want_init, &want), "{name}");
        let accepted = got.iter().filter(|&&s| s == 0).count();
        println!(
            "{name}: {accepted} of {} accepted, {tstates} T-states",
            got.len()
        );
        assert_eq!(accepted, got.len(), "{name}");
    }
}
