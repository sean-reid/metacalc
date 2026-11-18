use crate::support::constant_of;

const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/ramplan.lab"));
const DRIVER: &str = include_str!("../../../../probe/kerndriver.inc");

fn at(name: &str) -> usize {
    constant_of(LABELS, name) as usize
}

/// The probe's kernel driver keeps its scratch from `libPage` on; its size is the offset of `K_END`.
fn driver_scratch() -> (usize, usize) {
    let line = DRIVER
        .lines()
        .find(|l| l.trim_start().starts_with("K_END"))
        .expect("the driver names the end of its scratch");
    let size: usize = line.rsplit('+').next().unwrap().trim().parse().unwrap();
    (at("LIBPAGE"), size)
}

fn overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.0 + b.1 && b.0 < a.0 + a.1
}

#[test]
fn the_readers_state_block_overlaps_nothing_else_the_app_keeps_in_ram() {
    let reader = (at("LIB_STATE"), at("LIB_STATE_SIZE"));
    let others = [
        ("libPage", (at("LIBPAGE"), 1)),
        ("the UI frame", (at("UI_FRAME"), at("UI_FRAME_SIZE"))),
        ("the UI variables", (at("UI_VARS"), at("UI_VARS_SIZE"))),
        ("the probe driver's scratch", driver_scratch()),
    ];
    for (name, region) in others {
        assert!(!overlap(reader, region), "the state block overlaps {name}");
    }
    assert!(
        reader.0 + reader.1 <= at("APP_BACKUP_END"),
        "the state block ends at {:#x}, past appBackUpScreen",
        reader.0 + reader.1
    );
}

/// Overlaps that involve neither the reader nor this change are printed, not asserted.
#[test]
fn other_overlaps_in_the_apps_ram_are_reported() {
    let regions = [
        ("libPage", (at("LIBPAGE"), 1)),
        ("the UI frame", (at("UI_FRAME"), at("UI_FRAME_SIZE"))),
        ("the UI variables", (at("UI_VARS"), at("UI_VARS_SIZE"))),
        ("the probe driver's scratch", driver_scratch()),
    ];
    for (i, (a, ra)) in regions.iter().enumerate() {
        for (b, rb) in &regions[i + 1..] {
            if overlap(*ra, *rb) {
                println!("known overlap: {a} {ra:x?} and {b} {rb:x?}");
            }
        }
    }
}
