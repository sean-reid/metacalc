pub use crate::guard::{Allowed, offset, pattern, prime, stray};
use crate::support::*;

/// A long run of every kind of call, including refused ones and reads across chunks.
fn workout(rig: &mut Rig, bytes: &[u8]) {
    rig.open_ok();
    let count = u16::from_le_bytes([bytes[14], bytes[15]]);
    for split in [0usize, 1, 100, 255, 256] {
        let offset = CHUNK - split;
        assert_eq!(rig.read(offset as u32, 256).0, ST_OK);
    }
    for len in [1u16, 17, 256] {
        assert_eq!(rig.read(3, len).0, ST_OK);
    }
    assert_eq!(rig.read(bytes.len() as u32, 1).0, ST_RANGE);
    assert_eq!(rig.read_call(0, 0, BUF).status(), ST_ARG);
    for id in 0..(bytes[12] + bytes[13]).min(12) {
        assert_eq!(rig.symbol_name(id).0, ST_OK);
    }
    for n in (0..count).step_by(97).chain([count - 1]) {
        assert_eq!(rig.label_of(n).0, ST_OK);
        assert_eq!(rig.record(n), ST_OK);
        assert_eq!(rig.conclusion(n).0, ST_OK);
        rig.hypothesis(n, 0);
    }
    assert_eq!(rig.label_of(count).0, ST_INDEX);
    assert_eq!(rig.counts().0, ST_OK);
}

#[test]
fn a_long_run_of_reader_calls_writes_only_the_bounce_buffer_the_state_and_the_callers_own_bytes() {
    let bytes = big_library();
    for (image, name) in [(DEFAULT, "default"), (MOVED, "moved")] {
        let mut rig = Rig::with_image(image);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.load(&bytes);
        workout(&mut rig, &bytes);
        let mut allowed = Allowed::of(image);
        if image.code == MOVED.code {
            allowed.spare = 5;
        }
        let found = stray(&mut rig, &allowed);
        assert!(found.is_empty(), "{name}: {found:x?}");
        let bounce = offset(allowed.bounce, 0xC000);
        let written = rig.m.ram_page(allowed.spare)[bounce..bounce + 256].to_vec();
        let untouched: Vec<u8> = (0..256)
            .map(|i| pattern(allowed.spare, bounce + i))
            .collect();
        assert_ne!(
            written, untouched,
            "{name}: the reader used its bounce buffer"
        );
    }
}

#[test]
fn the_reader_uses_at_most_16_bytes_of_the_callers_stack() {
    let bytes = big_library();
    let mut rig = Rig::with_library(&bytes);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    let page = rig.m.ram_page(OS_PAGE).to_vec();
    let lowest = (0..page.len())
        .find(|&at| page[at] != pattern(OS_PAGE, at))
        .unwrap();
    let used = page.len() - lowest;
    println!("stack used, return address of the driver included: {used} bytes");
    assert!(used > 2 && used <= 2 + 16, "{used} bytes");
}

#[test]
fn the_isolation_check_fails_for_a_bounce_buffer_that_runs_past_the_end_of_the_region() {
    let bytes = big_library();
    let mut rig = Rig::with_image(STRADDLE);
    prime(&mut rig);
    rig.load(&bytes);
    workout(&mut rig, &bytes);
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert!(
        found
            .iter()
            .any(|&(page, addr)| page == SPARE_PAGE && (0xFF00..0xFF80).contains(&addr)),
        "{found:x?}"
    );
    assert!(
        found.iter().all(|&(page, _)| page == SPARE_PAGE),
        "{found:x?}"
    );
}

#[test]
fn a_stray_write_to_the_caller_stack_page_is_caught() {
    let bytes = big_library();
    let mut rig = Rig::with_library(&bytes);
    prime(&mut rig);
    workout(&mut rig, &bytes);
    rig.m.ram_page(OS_PAGE)[0x1234] ^= 1;
    rig.m.ram_page(APP_PAGE)[0x0100] ^= 1;
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert_eq!(found, [(OS_PAGE, 0xD234), (APP_PAGE, 0x8100)]);
}

#[test]
fn a_stray_write_to_the_spare_page_is_caught_through_any_of_its_page_numbers() {
    let bytes = big_library();
    for via in [2u8, 3, 5, 7] {
        let mut rig = Rig::with_library(&bytes);
        prime(&mut rig);
        workout(&mut rig, &bytes);
        rig.m.ram_page(via)[0x0200] ^= 1;
        let found = stray(&mut rig, &Allowed::of(DEFAULT));
        assert_eq!(found, [(SPARE_PAGE, 0xC200)], "written through page {via}");
    }
}

/// Pages 82h to 87h are one page, so only the value written to port 5 shows which one the reader
/// selects: the page its image documents, and nothing else but the caller's own page.
#[test]
fn the_reader_selects_the_page_it_documents_and_no_other_through_the_ports() {
    let bytes = big_library();
    for (image, spare) in [(DEFAULT, 3u8), (MOVED, 5)] {
        let mut rig = Rig::with_image(image);
        rig.load(&bytes);
        rig.open_ok();
        let written = rig.m.banks_written();
        assert_eq!(rig.read(0, 64).0, ST_OK);
        let read = rig.m.banks_written();
        for (what, written) in [("open", written), ("read", read)] {
            let values = |port: u8| -> Vec<u8> {
                written
                    .iter()
                    .filter(|&&(p, _)| p == port)
                    .map(|&(_, v)| v)
                    .collect()
            };
            let port5 = values(5);
            assert!(port5.contains(&spare), "{what}: {port5:x?}");
            assert!(
                port5.iter().all(|&v| v == spare || v == 0),
                "{what}: port 5 got {port5:x?}"
            );
            assert!(
                values(7).iter().all(|&v| v == 0x81 || v < 0x80),
                "{what}: port 7 got {:x?}",
                values(7)
            );
        }
    }
}
