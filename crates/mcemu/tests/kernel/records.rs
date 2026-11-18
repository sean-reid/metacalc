use crate::raw;
use crate::support::*;

#[test]
fn the_kernel_accepts_and_refuses_the_same_records_as_the_model() {
    for case in raw::record_cases() {
        for (layout, image) in LAYOUTS {
            let what = format!("{}, {layout} layout", case.name);
            agree_in(image, &case.bytes, 0, ASKED_UNBOUNDED, case.expected, &what);
        }
    }
}
