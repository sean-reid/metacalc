use crate::support::*;

fn run_reset(device: &mut Device, id: usize) {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.run("ENTRY_RESET");
}

#[test]
fn a_verify_after_a_reset_is_refused_and_an_init_after_it_agrees_with_the_model() {
    let (_, bytes) = small_library();
    let (init, all) = model_statuses(&bytes, ASKED_UNBOUNDED);
    let ok = status(Ok(()));
    for (id, &verdict) in all.iter().enumerate() {
        let mut device = Device::with_library(&bytes);
        run_reset(&mut device, id);
        assert_eq!(device.byte("TEST_INIT_STATUS"), init, "first init");
        assert_eq!(device.byte("TEST_RESET_STATUS"), ok, "reset");
        assert_eq!(
            device.byte("TEST_VERIFY_STATUS"),
            status(Err(mcpack::library::KernelError::Malformed)),
            "verify {id} after reset"
        );
        assert_eq!(device.byte("TEST_STATUSES"), init, "init after reset");
        assert_eq!(device.byte("TEST_VERIFY2_STATUS"), verdict, "verify {id}");
    }
}

#[test]
fn a_reset_writes_nothing_outside_the_kernels_regions() {
    let (_, bytes) = small_library();
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, &bytes);
        fill_outside(device.m.ram_page(SPARE_PAGE), image, 0x5A);
        run_reset(&mut device, 0);
        let page = device.m.ram_page(SPARE_PAGE);
        assert_eq!(first_outside_not(page, image, 0x5A), None, "{layout}");
    }
}
