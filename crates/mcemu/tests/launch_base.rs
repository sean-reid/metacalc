//! Which bases `install_app` takes and how it marks them. The reference-ROM tests are ignored
//! without the image.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{AppError, Machine, STUB_ROM, Stop};

const PAGES: usize = 0x80;
const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/page_ports1.8xk"));
const RECORD: u16 = 0x9872;

fn reference() -> Machine {
    let rom = realos::rom().expect("the reference ROM is not at $HOME/ti84-rom/84PSE_v243.rom");
    Machine::new_os(&rom).expect("the reference ROM boots")
}

fn flash(m: &mut Machine) -> Vec<Vec<u8>> {
    (0..PAGES as u8)
        .map(|page| m.read_flash(page, 0, 0x4000))
        .collect()
}

fn slot(m: &mut Machine, base: u8) -> Vec<u8> {
    m.read_flash(0x7E, 0x1E50 + 2 * usize::from(0x69 - base), 2)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_slots_the_tests_use_read_ffff_on_the_reference_image() {
    let mut m = reference();
    for base in 0x09..=0x69 {
        assert_eq!(slot(&mut m, base), [0xFF, 0xFF], "slot of base {base:02X}");
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_base_above_69h_is_refused_and_writes_nothing_on_the_reference_image() {
    let mut m = reference();
    let before = flash(&mut m);
    for base in [0x6A, 0x6B, 0x6F, 0x7E] {
        assert_eq!(
            m.install_app(APP, base),
            Err(AppError(
                "the OS's launch table has no slot for the base page"
            )),
            "base {base:02X}"
        );
    }
    assert!(flash(&mut m) == before, "flash is as it was");
    assert_eq!(m.install_app(APP, 0x69), Ok(()), "the top base installs");
    assert_eq!(slot(&mut m, 0x69), [0x80, 0x00]);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_slot_holding_other_bytes_is_refused_and_left_as_it_was() {
    for held in [[0x12, 0x34], [0xFF, 0x7F], [0x80, 0x01], [0x00, 0x00]] {
        let mut m = reference();
        m.load_flash(0x7E, 0x1E52, &held);
        let before = flash(&mut m);
        assert_eq!(
            m.install_app(APP, 0x68),
            Err(AppError(
                "the launch table is not where this image keeps it"
            )),
            "slot {held:02X?}"
        );
        assert!(
            flash(&mut m) == before,
            "flash is as it was for {held:02X?}"
        );
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_mark_lands_in_the_slot_of_its_base_and_nowhere_else() {
    for (base, offset) in [(0x69u8, 0x1E50usize), (0x68, 0x1E52), (0x50, 0x1E82)] {
        let mut m = reference();
        let before = flash(&mut m);
        assert_eq!(
            slot(&mut m, base),
            [0xFF, 0xFF],
            "slot of {base:02X} is free"
        );
        m.install_app(APP, base).expect("the app installs");
        assert_eq!(
            m.read_flash(0x7E, offset, 2),
            [0x80, 0x00],
            "base {base:02X}"
        );
        let after = flash(&mut m);
        let changed: Vec<(usize, usize)> = (0..PAGES)
            .flat_map(|p| (0..0x4000).map(move |i| (p, i)))
            .filter(|&(p, i)| before[p][i] != after[p][i])
            .collect();
        assert_eq!(
            changed.iter().filter(|&&(p, _)| p == 0x7E).count(),
            2,
            "two bytes of page 7Eh change for {base:02X}"
        );
        assert!(
            changed.iter().all(|&(p, i)| p == usize::from(base)
                || (p == 0x7E && (offset..offset + 2).contains(&i))),
            "only the app's page and its slot change"
        );
        assert!(
            changed.iter().any(|&(p, _)| p == usize::from(base)),
            "the app's page was written"
        );
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn installing_twice_leaves_the_mark() {
    let mut m = reference();
    m.install_app(APP, 0x69).expect("the first install");
    m.install_app(APP, 0x69).expect("the second install");
    assert_eq!(slot(&mut m, 0x69), [0x80, 0x00]);
}

#[test]
fn a_base_above_69h_still_installs_and_launches_on_the_stub() {
    for base in [0x6A, 0x6F, 0x7E - 1] {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(APP, base).expect("the app installs");
        m.launch(base).expect("the app launches");
        assert_eq!(m.run_app(100_000).stop, Stop::Limit, "base {base:02X}");
        assert_eq!(
            m.read_ram(RECORD, 4),
            [base, 0x08, base - 1, 0xA5],
            "base {base:02X}"
        );
    }
}
