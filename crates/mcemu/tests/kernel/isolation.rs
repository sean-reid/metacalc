use crate::crc::padded;
use crate::edges::failing_libraries;
use crate::support::*;
use mcemu::Machine;
use mcpack::library::CHUNK;
use std::ops::Range;

const OS_PAGE: u8 = 0;
const APP_PAGE: u8 = 1;
const PARAMS: u16 = 0x8100;
const STACK_BYTES: usize = 64;

pub fn pattern(page: u8, at: usize) -> u8 {
    ((at.wrapping_mul(31) ^ (at >> 8).wrapping_mul(7)) as u8) ^ page.wrapping_mul(0x55) ^ 0xA7
}

pub fn offset(addr: u16) -> usize {
    usize::from(addr - 0xC000)
}

/// A device with the library loaded and RAM pages 80h, 81h and 83h filled with known patterns,
/// the parameter block for init in bank B where an app keeps its data.
pub fn primed(image: Image, bytes: &[u8]) -> Device {
    let mut device = Device::with_image(image);
    device.load(bytes);
    for page in [OS_PAGE, APP_PAGE, SPARE_PAGE] {
        for (at, byte) in device.m.ram_page(page).iter_mut().enumerate() {
            *byte = pattern(page, at);
        }
    }
    device.params_in_bank_b(ASKED_UNBOUNDED, PARAMS);
    device
}

/// What a run may legitimately change on page 80h: the test's own variables and status array, and
/// the bytes the test's call pushed on its stack.
fn test_own(device: &Device, statuses: usize) -> Vec<Range<usize>> {
    let block = offset(device.label("TEST_BLOCK"));
    let first = offset(device.label("TEST_STATUSES"));
    vec![
        block..block + 0x30,
        first..first + statuses,
        0x4000 - STACK_BYTES..0x4000,
    ]
}

/// Every byte the run changed that it had no right to, as (page, address).
pub fn stray_writes(
    device: &mut Device,
    statuses: usize,
    expected_regions: [(u16, usize); 2],
) -> Vec<(u8, u16)> {
    let own = test_own(device, statuses);
    let mut found = Vec::new();
    let page_80 = device.m.ram_page(OS_PAGE).to_vec();
    for (at, &byte) in page_80.iter().enumerate() {
        if byte != pattern(OS_PAGE, at) && !own.iter().any(|r| r.contains(&at)) {
            found.push((0x80, 0xC000 + at as u16));
        }
    }
    let block = usize::from(PARAMS - 0x8000);
    let page_81 = device.m.ram_page(APP_PAGE).to_vec();
    for (at, &byte) in page_81.iter().enumerate() {
        let in_block = (block..block + 9).contains(&at);
        if byte != pattern(APP_PAGE, at) && !in_block {
            found.push((0x81, 0x8000 + at as u16));
        }
    }
    let page_83 = device.m.ram_page(SPARE_PAGE).to_vec();
    for (at, &byte) in page_83.iter().enumerate() {
        let addr = 0xC000 + at;
        let inside = expected_regions
            .iter()
            .any(|&(start, len)| (usize::from(start)..usize::from(start) + len).contains(&addr));
        if byte != pattern(SPARE_PAGE, at) && !inside {
            found.push((0x83, addr as u16));
        }
    }
    found
}

/// The two gaps TI-OS leaves on the spare page, written out here and not read from the kernel.
pub fn expected() -> [(u16, usize); 2] {
    [(0xC500, 0x1200), (0xDF00, 0x2000)]
}

pub fn run_all(device: &mut Device, count: usize) -> (u8, Vec<u8>) {
    device
        .m
        .load_ram(device.label("TEST_COUNT"), &(count as u16).to_le_bytes());
    device.run("ENTRY_ALL");
    let statuses = device.m.read_ram(device.label("TEST_STATUSES"), count);
    (device.byte("TEST_INIT_STATUS"), statuses)
}

pub fn run_one(device: &mut Device, id: usize) -> (u8, u8) {
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.run("ENTRY_ONE");
    (
        device.byte("TEST_INIT_STATUS"),
        device.byte("TEST_VERIFY_STATUS"),
    )
}

const CORPUS: [&str; 12] = [
    "anatomy.mm",
    "anatomy-bad1.mm",
    "anatomy-bad2.mm",
    "anatomy-bad3.mm",
    "big-unifier.mm",
    "big-unifier-bad1.mm",
    "big-unifier-bad2.mm",
    "big-unifier-bad3.mm",
    "demo0.mm",
    "demo0-bad1.mm",
    "emptyline.mm",
    "miu.mm",
];

#[test]
fn verifying_a_whole_corpus_library_touches_only_the_two_regions() {
    for name in CORPUS {
        let bytes = pack_all(&corpus(name));
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        let mut device = primed(DEFAULT, &bytes);
        let (init, got) = run_all(&mut device, want.len());
        assert_eq!((init, &got), (want_init, &want), "{name}");
        let stray = stray_writes(&mut device, want.len(), expected());
        assert!(stray.is_empty(), "{name}: {stray:x?}");
        let written = device.kram_label("KERNEL_RAM_LOW", 0x1200);
        assert_ne!(
            written,
            (0..0x1200)
                .map(|at| pattern(SPARE_PAGE, offset(device.label("KERNEL_RAM_LOW")) + at))
                .collect::<Vec<u8>>(),
            "{name}: the kernel used its low region"
        );
    }
}

#[test]
fn every_failure_status_leaves_the_other_pages_alone() {
    let mut seen = std::collections::BTreeSet::new();
    for (n, (bytes, id)) in failing_libraries().into_iter().enumerate() {
        let want = model_one(&bytes, id, ASKED_UNBOUNDED);
        seen.insert(want);
        let mut device = primed(DEFAULT, &bytes);
        let (_, got) = run_one(&mut device, id);
        assert_eq!(got, want, "case {n}");
        let stray = stray_writes(&mut device, 0, expected());
        assert!(stray.is_empty(), "case {n}, status {want}: {stray:x?}");
    }
    for code in [0, 1, 2, 4, 6, 7, 8, 9, 10, 11, 12, 13] {
        assert!(seen.contains(&code), "no case ended in status {code}");
    }
}

#[test]
fn an_interrupted_call_leaves_the_other_pages_alone() {
    let (db, bytes) = small_library();
    let mut device = primed(DEFAULT, &bytes);
    device.m.press_key(Machine::KEY_ON);
    let (init, got) = run_one(&mut device, number(&db, "both"));
    assert_eq!((init, got), (0, label("ST_INTERRUPTED") as u8));
    let stray = stray_writes(&mut device, 0, expected());
    assert!(stray.is_empty(), "{stray:x?}");
}

#[test]
fn a_checksum_run_touches_only_the_two_regions() {
    let (_, small) = small_library();
    let good = padded(&small, 3 * CHUNK + 100);
    let mut wrong = good.clone();
    wrong[CHUNK + 9] ^= 0x40;
    for (bytes, entry, last) in [
        (&good, "ENTRY_CRC", 0u8),
        (&wrong, "ENTRY_CRC", 1),
        (&good, "ENTRY_CRC_RAW", 1),
    ] {
        let mut device = primed(DEFAULT, bytes);
        device.m.load_ram(device.label("TEST_COUNT"), &[255, 0]);
        device.run(entry);
        let calls = usize::from(device.byte("TEST_CRC_CALLS"));
        let statuses = device.m.read_ram(device.label("TEST_STATUSES"), calls);
        assert_eq!(statuses.last(), Some(&last), "{entry}");
        let stray = stray_writes(&mut device, calls, expected());
        assert!(stray.is_empty(), "{entry}: {stray:x?}");
    }
}

#[test]
fn a_run_with_a_stray_write_is_caught() {
    let bytes = pack_all(&corpus("demo0.mm"));
    let (_, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    let mut device = primed(MOVED, &bytes);
    let (init, got) = run_all(&mut device, want.len());
    assert_eq!((init, &got), (0, &want));
    let stray = stray_writes(&mut device, want.len(), expected());
    assert!(
        stray
            .iter()
            .any(|&(page, addr)| page == 0x83 && (0xC100..0xC500).contains(&addr)),
        "the low region at C100 runs into the first 500h bytes TI-OS owns, which the check must see: {stray:x?}"
    );
    assert!(stray.iter().all(|&(page, _)| page == 0x83));
}

#[test]
fn the_default_regions_are_the_gaps_ti_os_leaves() {
    assert_eq!(regions(DEFAULT), expected());
}

/// Runs init, one verify, a checksum run and one chunk call too many, from `initial` in port 5
/// and with interrupts on or off, and returns the device afterwards.
fn p5_run(image: Image, bytes: &[u8], id: usize, initial: u8, interrupts: u8, on: bool) -> Device {
    let mut device = Device::with_image(image);
    device.load(bytes);
    device.m.out_port(5, initial);
    device.params(ASKED_UNBOUNDED);
    let load = |device: &mut Device, name: &str, value: &[u8]| {
        device.m.load_ram(device.label(name), value);
    };
    load(&mut device, "TEST_ASSERTION", &(id as u16).to_le_bytes());
    load(&mut device, "TEST_IFF_IN", &[interrupts]);
    load(&mut device, "TEST_P5_WANT", &[initial]);
    if on {
        device.m.press_key(Machine::KEY_ON);
    }
    device.run("ENTRY_P5");
    device
}

fn assert_port5_kept(device: &mut Device, initial: u8, interrupts: u8, what: &str) {
    assert_eq!(
        device.byte("TEST_P5_BAD"),
        0,
        "{what}: port 5 differed after a call"
    );
    assert!(device.byte("TEST_P5_N") >= 5, "{what}: calls probed");
    assert_eq!(
        device.byte("TEST_PORT5"),
        initial,
        "{what}: port 5 at the end"
    );
    assert_eq!(device.byte("TEST_IFF"), interrupts, "{what}: interrupts");
}

#[test]
fn port_5_is_restored_on_every_exit_path() {
    let (db, small) = small_library();
    let mut wrong_sum = small.clone();
    wrong_sum[4] ^= 1;
    let mut cases: Vec<(Vec<u8>, usize, Option<u8>)> = failing_libraries()
        .into_iter()
        .map(|(bytes, id)| (bytes, id, None))
        .collect();
    cases.push((small.clone(), number(&db, "both"), None));
    cases.push((wrong_sum, number(&db, "both"), Some(0)));
    cases.push((padded(&small, 2 * CHUNK + 5), 0, None));
    let mut statuses = std::collections::BTreeSet::new();
    for initial in [0u8, 1, 3] {
        for interrupts in [0u8, 1] {
            for (n, (bytes, id, device_says)) in cases.iter().enumerate() {
                let mut device = p5_run(DEFAULT, bytes, *id, initial, interrupts, false);
                let what = format!("case {n}, port 5 {initial}, interrupts {interrupts}");
                assert_port5_kept(&mut device, initial, interrupts, &what);
                let want = device_says.unwrap_or_else(|| model_one(bytes, *id, ASKED_UNBOUNDED));
                assert_eq!(device.byte("TEST_VERIFY_STATUS"), want, "{what}");
                statuses.insert(want);
            }
            let id = number(&db, "both");
            let mut device = p5_run(DEFAULT, &small, id, initial, interrupts, true);
            let what = format!("ON held, port 5 {initial}, interrupts {interrupts}");
            assert_port5_kept(&mut device, initial, interrupts, &what);
            assert_eq!(
                device.byte("TEST_VERIFY_STATUS"),
                label("ST_INTERRUPTED") as u8
            );
        }
    }
    for code in [0, 1, 2, 4, 6, 7, 8, 9, 10, 11, 12, 13] {
        assert!(statuses.contains(&code), "no case ended in status {code}");
    }
}

#[test]
fn port_5_is_restored_by_the_image_with_moved_regions_too() {
    let (db, small) = small_library();
    let id = number(&db, "both");
    for initial in [0u8, 1, 3] {
        let mut device = p5_run(MOVED, &small, id, initial, 1, false);
        assert_port5_kept(&mut device, initial, 1, "moved");
        assert_eq!(device.byte("TEST_VERIFY_STATUS"), 0);
    }
}

#[test]
fn a_caller_already_on_the_spare_page_keeps_everything_outside_the_regions() {
    let (db, small) = small_library();
    let id = number(&db, "both");
    let mut device = Device::new();
    device.load(&small);
    device.m.out_port(5, SPARE_PAGE);
    fill_outside(device.m.ram_page(SPARE_PAGE), DEFAULT, 0x6B);
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device
        .m
        .load_ram(device.label("TEST_P5_WANT"), &[SPARE_PAGE, 0, 0]);
    device.run("ENTRY_P5");
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), 0);
    assert_eq!(device.byte("TEST_P5_BAD"), 0);
    let block = offset(device.label("TEST_BLOCK"));
    let mut page = device.m.ram_page(SPARE_PAGE).to_vec();
    page[block..block + 0x30].fill(0x6B);
    page[0x4000 - STACK_BYTES..].fill(0x6B);
    assert_eq!(first_outside_not(&page, DEFAULT, 0x6B), None);
}
