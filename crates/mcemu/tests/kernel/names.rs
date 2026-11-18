use crate::raw::{self, Case};
use crate::support::*;

fn agree_on(cases: Vec<Case>) {
    for case in cases {
        let (want_init, want) = model_statuses(&case.bytes, ASKED_UNBOUNDED);
        assert_eq!(want_init, status(case.init), "{}: the model", case.name);
        for (layout, image) in LAYOUTS {
            let mut device = Device::with_image_library(image, &case.bytes);
            let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
            assert_eq!(
                (init, &got),
                (want_init, &want),
                "{}: the device, {layout} layout",
                case.name
            );
        }
    }
}

#[test]
fn a_symbol_table_must_end_inside_the_first_page() {
    agree_on(raw::crossing_cases());
}

#[test]
fn the_kernel_verifies_libraries_with_bad_labels_as_the_model_does() {
    for (what, bytes, _) in raw::bad_label_libraries() {
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        assert!(want_init == 0 && want.iter().all(|&s| s == 0), "{what}");
        let (init, got, _) = Device::with_library(&bytes).verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, got), (0, want), "{what}");
    }
}

#[test]
fn symbol_names_are_checked_at_init_as_the_model_checks_them() {
    agree_on(raw::name_cases());
}
