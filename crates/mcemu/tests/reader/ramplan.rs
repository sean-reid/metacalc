use crate::support::constant_of;

const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/ramplan.lab"));

fn at(name: &str) -> usize {
    constant_of(LABELS, name) as usize
}

/// Every region of app/src/ram_map.inc at its default base, as the assembler resolved it.
fn regions() -> Vec<(&'static str, usize, usize)> {
    [
        ("SCR_STATE", "SCR_STATE_SIZE"),
        ("UI_VARS", "UI_VARS_SIZE"),
        ("UI_FRAME", "UI_FRAME_SIZE"),
        ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
        ("FAR_VARS", "FAR_VARS_SIZE"),
        ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
        ("LIB_STATE", "LIB_STATE_SIZE"),
        ("KERNEL_RAM_LOW", "LOW_SIZE"),
        ("KERNEL_RAM_HIGH", "HIGH_SIZE"),
    ]
    .into_iter()
    .map(|(base, size)| (base, at(base), at(size)))
    .collect()
}

#[test]
fn no_two_regions_of_the_ram_map_overlap() {
    let regions = regions();
    for (i, (a, base, size)) in regions.iter().enumerate() {
        for (b, other, other_size) in &regions[i + 1..] {
            assert!(
                base + size <= *other || other + other_size <= *base,
                "{a} {base:#x}+{size:#x} overlaps {b} {other:#x}+{other_size:#x}"
            );
        }
    }
}

#[test]
fn the_drivers_scratch_starts_at_libpage_and_the_reader_ends_inside_appbackupscreen() {
    assert_eq!(at("LIBPAGE"), at("KD_SCRATCH"));
    let end = at("LIB_STATE") + at("LIB_STATE_SIZE");
    assert!(
        end <= at("APP_BACKUP_END"),
        "the state block ends at {end:#x}, past appBackUpScreen"
    );
    let far = at("FAR_VARS");
    assert!(
        far >= at("KD_SCRATCH") + at("KD_SCRATCH_SIZE")
            && far + at("FAR_VARS_SIZE") <= at("LIB_STATE"),
        "the far call's scratch is between the driver's and the reader's"
    );
    assert!(
        far + at("FAR_VARS_SIZE") <= at("APP_BACKUP_END"),
        "the far call's scratch ends past appBackUpScreen"
    );
}

#[test]
fn fms_scratch_follows_the_ui_variables_and_its_memory_stays_off_the_bounce_buffer() {
    assert_eq!(at("FM_SCRATCH"), at("UI_VARS") + at("UI_VARS_SIZE"));
    assert_eq!(at("FM_SCRATCH_SIZE"), 256);
    let (base, size) = (at("FM_RAM"), at("FM_RAM_SIZE"));
    let high = (at("KERNEL_RAM_HIGH"), at("HIGH_SIZE"));
    assert!(base >= high.0 && base + size <= high.0 + high.1);
    let bounce = (at("LIB_BOUNCE"), at("LIB_BOUNCE_SIZE"));
    assert!(base + size <= bounce.0 || bounce.0 + bounce.1 <= base);
}

#[test]
fn fm_low_overlays_the_kernels_low_region_above_the_page_of_its_parameters() {
    let (base, size) = (at("FM_LOW"), at("FM_LOW_SIZE"));
    let low = (at("KERNEL_RAM_LOW"), at("LOW_SIZE"));
    assert_eq!((base, size), (0xC600, 0x1100));
    assert!(base >= low.0 + 0x100, "the kernel's first page stays alone");
    assert!(base + size <= low.0 + low.1);
    assert_eq!(
        base + size,
        low.0 + low.1,
        "it runs to the end of the region"
    );
}

#[test]
fn the_screen_state_is_160_bytes_inside_appdata() {
    let (base, size) = (at("SCR_STATE"), at("SCR_STATE_SIZE"));
    assert_eq!(size, 160);
    assert!(
        base >= 0x8000 && base + size <= 0x8100,
        "{base:#x}+{size:#x} is not inside appData, 8000h to 80FFh"
    );
}
