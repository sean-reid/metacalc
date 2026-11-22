#[path = "../../../mcpack/tests/cases/references.rs"]
pub mod cases;

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
    let mut seen = [0usize; 17];
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
    let sizes = (3..=40).chain([100, 255, 256, 257, 361, 362, 363, 364, 510, 511]);
    for entries in sizes {
        let bytes = cases::pack_source(&cases::long_source(entries));
        let mut kernel = Kernel::new(&bytes, clamped(ASKED_UNBOUNDED)).unwrap();
        assert_eq!(kernel.verify(entries), Ok(()));
        let charged = cases::long_work(entries);
        assert_eq!(kernel.work() as usize, charged);
        agree(&bytes, entries, Ok(()), "long list");
        let mut device = Device::with_library(&bytes);
        assert_eq!(device.verify(entries, ASKED_UNBOUNDED).verify, 0);
        let left = device.kram_label("kv_work", 3);
        let left = u32::from_le_bytes([left[0], left[1], left[2], 0]);
        assert_eq!(
            left as usize,
            constant("WORK_LIMIT") as usize - charged,
            "{entries}"
        );
    }
    let bytes = cases::pack_source(&cases::long_source(cases::LONGEST));
    agree(
        &bytes,
        cases::LONGEST,
        Err(KernelError::WorkLimit),
        "one entry too many",
    );
}

#[test]
fn the_on_key_interrupts_the_check_for_equal_entries_and_the_call_can_be_run_again() {
    let bytes = cases::pack_source(&cases::long_source(511));
    let mut device = Device::with_library(&bytes);
    let (pc, after) = crate::work::press_on_during(&mut device, 511, 2_000_000);
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

const BUDGET_DUMMY_PASS: u32 = 1_500_000;

#[test]
fn a_list_of_thousands_of_dummy_entries_is_charged_and_stays_within_budget() {
    for (entries, mandatory) in [(2500, true), (2500, false), (511, true)] {
        let (bytes, id) = cases::many_dummies(entries, mandatory);
        agree(&bytes, id, Err(KernelError::BadRecord), "dummy entries");
        let got = Device::with_library(&bytes).verify(id, ASKED_UNBOUNDED);
        println!("{entries} dummy entries: {} T-states", got.tstates);
        assert!(got.tstates < BUDGET_DUMMY_PASS, "{} T-states", got.tstates);
    }
}

#[test]
fn a_list_of_thousands_of_entries_stops_at_the_work_limit_within_budget() {
    let bytes = cases::pack_source(&cases::long_source(2500));
    agree(&bytes, 2500, Err(KernelError::WorkLimit), "2500 entries");
    let got = Device::with_library(&bytes).verify(2500, ASKED_UNBOUNDED);
    println!("2500 entries: {} T-states", got.tstates);
    assert!(got.tstates < 68_000_000, "{} T-states", got.tstates);
}

#[test]
fn the_on_key_interrupts_the_dummy_check_and_the_call_can_be_run_again() {
    let (bytes, id) = cases::many_dummies(2500, false);
    let whole = Device::with_library(&bytes)
        .verify(id, ASKED_UNBOUNDED)
        .tstates;
    let init = Device::with_library(&bytes)
        .verify_all(0, ASKED_UNBOUNDED)
        .2;
    let mut device = Device::with_library(&bytes);
    let press_at = init + (whole - init) / 2;
    let (pc, after) = crate::work::press_on_during(&mut device, id, press_at);
    let ranges = [
        ("CHECK_REFERENCES", "CF_PAIRS"),
        ("WORK_ONE", "PAIR_ALLOWED"),
    ];
    assert!(crate::work::in_code(pc, &ranges), "pc {pc:#06x}");
    assert!(after < 25_000, "{after} T-states");
    assert_eq!(
        device.byte("TEST_VERIFY_STATUS"),
        crate::work::status_code("ST_INTERRUPTED")
    );
    assert_eq!(
        crate::work::verify_again(&mut device),
        status(Err(KernelError::BadRecord))
    );
}

#[test]
fn the_kernel_judges_structured_edits_of_a_reference_list_as_the_model_does() {
    let base = cases::base();
    let mut changed_and_accepted = 0;
    for mutant in cases::mutants() {
        let want = model_one(&mutant.bytes, mutant.id, ASKED_UNBOUNDED);
        if let Some(expected) = mutant.expected {
            assert_eq!(want, status(expected), "{}: the model", mutant.name);
        }
        let got = Device::with_library(&mutant.bytes).verify(mutant.id, ASKED_UNBOUNDED);
        assert_eq!(got.verify, want, "{}: the device", mutant.name);
        changed_and_accepted += usize::from(want == 0 && mutant.bytes != base);
    }
    assert!(changed_and_accepted > 0);
}
