use crate::support::constant_of;

const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/ramplan.lab"));

fn at(name: &str) -> usize {
    constant_of(LABELS, name) as usize
}

/// Every region of app/src/ram_map.inc at its default base, as the assembler resolved it.
fn regions() -> Vec<(&'static str, usize, usize)> {
    [
        ("UI_VARS", "UI_VARS_SIZE"),
        ("UI_FRAME", "UI_FRAME_SIZE"),
        ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
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
}
