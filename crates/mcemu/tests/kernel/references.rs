#[path = "../../../mcpack/tests/cases/references.rs"]
mod cases;

use crate::support::*;
use mcpack::library::{Kernel, KernelError};

#[test]
fn the_kernel_judges_a_reference_list_as_the_model_does() {
    assert_eq!(label("ST_LOOSE_REF"), 15);
    for case in cases::cases() {
        agree(&case.bytes, case.id, case.expected, case.name);
    }
}

#[test]
fn no_byte_of_a_reference_list_or_proof_makes_the_kernel_disagree() {
    let base = cases::base();
    let replacements = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 0x7F, 0x80, 0xFF];
    let mut device = Device::new();
    let mut seen = [0usize; 16];
    for id in [cases::TH, cases::TH2, cases::TH3] {
        let spot = cases::spot(&base, id);
        let len = u16::from_le_bytes([base[spot.proof - 2], base[spot.proof - 1]]) as usize;
        let entries = spot.refs..spot.refs + 2 * spot.count;
        for at in entries.chain(spot.proof..spot.proof + len) {
            for value in replacements {
                let mut bytes = base.clone();
                bytes[at] = value;
                reseal(&mut bytes);
                device.load(&bytes);
                let want = model_one(&bytes, id, ASKED_UNBOUNDED);
                seen[want as usize] += 1;
                let got = device.verify(id, ASKED_UNBOUNDED);
                assert_eq!(
                    got.verify, want,
                    "assertion {id} byte {at} set to {value:#x}"
                );
            }
        }
    }
    for code in [0, 2, 4, 15] {
        assert!(seen[code] > 0, "no mutation gave status {code}: {seen:?}");
    }
}

#[test]
fn the_device_charges_the_duplicate_check_as_the_model_does() {
    for entries in [3, 4, 100, 512] {
        let bytes = cases::pack_source(&cases::long_source(entries));
        let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
        assert_eq!(kernel.verify(entries), Ok(()));
        assert_eq!(kernel.work() as usize, entries * (entries - 1) / 2);
        agree(&bytes, entries, Ok(()), "long list");
    }
    let bytes = cases::pack_source(&cases::long_source(513));
    agree(
        &bytes,
        513,
        Err(KernelError::WorkLimit),
        "one entry too many",
    );
}

#[test]
fn the_on_key_interrupts_the_check_for_equal_entries_and_the_call_can_be_run_again() {
    let bytes = cases::pack_source(&cases::long_source(512));
    let mut device = Device::with_library(&bytes);
    let (pc, after) = crate::work::press_on_during(&mut device, 512, 2_000_000);
    assert!(
        crate::work::in_code(pc, &[("CHECK_REFERENCES", "VERIFY_BODY")]),
        "pc {pc:#06x}"
    );
    assert!(after < 25_000, "{after} T-states");
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        crate::work::status_code("ST_INTERRUPTED")
    );
    assert_eq!(crate::work::verify_again(&mut device), status(Ok(())));
}
