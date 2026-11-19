use crate::support::*;

fn run_reset(device: &mut Device, id: usize) {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.run("ENTRY_RESET");
}

#[test]
fn a_verify_after_a_reset_is_refused_and_a_second_init_agrees_with_the_model() {
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
fn init_reset_verify_init_verify_writes_nothing_outside_the_kernels_regions() {
    let (_, bytes) = small_library();
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, &bytes);
        fill_outside(device.m.ram_page(SPARE_PAGE), image, 0x5A);
        run_reset(&mut device, 0);
        let page = device.m.ram_page(SPARE_PAGE);
        assert_eq!(first_outside_not(page, image, 0x5A), None, "{layout}");
    }
}

fn run_reset_alone(device: &mut Device, interrupts: u8, bank_b: u8) {
    device.m.out_port(PORT_BANK_B, bank_b);
    device
        .m
        .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
    device.run("ENTRY_RESET_ALONE");
}

#[test]
fn a_reset_alone_writes_nothing_outside_the_kernels_regions() {
    let (_, bytes) = small_library();
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, &bytes);
        fill_outside(device.m.ram_page(SPARE_PAGE), image, 0x5A);
        run_reset_alone(&mut device, 0, RAM_PAGE);
        let page = device.m.ram_page(SPARE_PAGE);
        assert_eq!(first_outside_not(page, image, 0x5A), None, "{layout}");
    }
}

#[test]
fn a_reset_on_a_page_of_0xff_with_no_init_returns_0_and_restores_the_caller_state() {
    let (_, bytes) = small_library();
    for (layout, image) in LAYOUTS {
        for interrupts in [0, 1] {
            for bank_b in [RAM_PAGE, 0x50] {
                let mut device = Device::with_image_library(image, &bytes);
                let [(low, len), _] = regions(image);
                device.kram_write(low, &vec![0xFF; len]);
                let port5 = device.m.in_port(5);
                run_reset_alone(&mut device, interrupts, bank_b);
                let what = format!("{layout}, interrupts {interrupts}, bank B {bank_b:#x}");
                assert_eq!(device.byte("TEST_RESET_STATUS"), 0, "{what}: status");
                let sp = device.m.read_ram(device.label("TEST_SP"), 4);
                assert_eq!(sp[..2], sp[2..], "{what}: SP");
                assert_eq!(device.byte("TEST_PORT7"), bank_b, "{what}: port 7");
                assert_eq!(device.byte("TEST_PORT5"), port5, "{what}: port 5");
                assert_eq!(device.byte("TEST_IFF"), interrupts, "{what}: interrupts");
            }
        }
    }
}
