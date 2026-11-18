use crate::support::*;

fn state(rig: &Rig) -> (u16, usize) {
    (
        rig.label("LIB_STATE"),
        usize::from(rig.label("LIB_STATE_SIZE")),
    )
}

/// The state block without the scratch every call writes: what a refused call must leave alone.
fn persistent(rig: &mut Rig) -> Vec<u8> {
    let (start, size) = state(rig);
    let block = rig.peek(start, size);
    [0x10..0x40, 0x5B..0xA4, 0xA8..size]
        .into_iter()
        .flat_map(|r| block[r].to_vec())
        .collect()
}

fn opened(bytes: &[u8]) -> Rig {
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    rig
}

fn demo() -> Vec<u8> {
    pack_all(&corpus("demo0.mm"))
}

#[test]
fn a_destination_that_touches_the_state_block_is_refused_and_nothing_is_copied() {
    let bytes = demo();
    let mut rig = opened(&bytes);
    let (start, size) = state(&rig);
    let end = start + size as u16;
    let block = persistent(&mut rig);
    for (dest, count) in [
        (start + 5, 1u16),
        (start + 5, 20),
        (start + 6, 1),
        (start - 1, 2),
        (start - 16, 17),
        (start - 255, 256),
        (end - 1, 1),
        (end - 1, 16),
        (start, 1),
        (start, size as u16),
        (start - 8, size as u16 + 16),
    ] {
        let status = rig.read_call(0, count, dest).status();
        assert_eq!(status, ST_ARG, "dest {dest:#x} count {count}");
        assert_eq!(persistent(&mut rig), block, "dest {dest:#x} count {count}");
    }
    for (dest, count) in [
        (start - 8, 8u16),
        (start - 1, 1),
        (end, 1),
        (end, 16),
        (start - 256, 256),
    ] {
        let status = rig.read_call(0, count, dest).status();
        assert_eq!(status, ST_OK, "dest {dest:#x} count {count}");
        assert_eq!(
            rig.peek(dest, usize::from(count)),
            bytes[..usize::from(count)]
        );
    }
    assert_eq!(rig.label_of(0).0, ST_OK, "the reader still works");
}

#[test]
fn every_accessor_refuses_a_buffer_that_touches_the_state_block() {
    let bytes = demo();
    let library = decoded(&bytes);
    let n = library
        .assertions
        .iter()
        .position(|a| !a.hypotheses.is_empty())
        .unwrap() as u16;
    let mut rig = opened(&bytes);
    let (start, size) = state(&rig);
    let block = persistent(&mut rig);
    for buf in [start + 5, start - 100, start, start + size as u16 - 1] {
        let name = Regs {
            a: 0,
            de: buf,
            ..Regs::default()
        };
        let at = |hl: u16| Regs {
            hl,
            de: buf,
            ..Regs::default()
        };
        let hyp = Regs {
            hl: n,
            bc: 0,
            de: buf,
            ..Regs::default()
        };
        for (call, regs) in [
            ("LIB_SYMBOL_NAME", name),
            ("LIB_LABEL", at(n)),
            ("LIB_CONCLUSION", at(n)),
            ("LIB_HYPOTHESIS", hyp),
        ] {
            assert_eq!(
                rig.call(call, regs).status(),
                ST_ARG,
                "{call} into {buf:#x}"
            );
            assert_eq!(persistent(&mut rig), block, "{call} into {buf:#x}");
        }
        let open = Regs {
            a: LIB_PAGE,
            de: buf,
            ..Regs::default()
        };
        assert_eq!(
            rig.call("LIB_OPEN", open).status(),
            ST_ARG,
            "open with scratch {buf:#x}"
        );
        assert_eq!(
            rig.counts().0,
            ST_CLOSED,
            "a refused open leaves the reader closed"
        );
        rig.open_ok();
    }
    for buf in [0x7F00u16, 0x0100, 0xBF02, 0xC000, 0xFF00] {
        let name = Regs {
            a: 0,
            de: buf,
            ..Regs::default()
        };
        let at = |hl: u16| Regs {
            hl,
            de: buf,
            ..Regs::default()
        };
        let hyp = Regs {
            hl: n,
            bc: 0,
            de: buf,
            ..Regs::default()
        };
        for (call, regs) in [
            ("LIB_SYMBOL_NAME", name),
            ("LIB_LABEL", at(n)),
            ("LIB_CONCLUSION", at(n)),
            ("LIB_HYPOTHESIS", hyp),
        ] {
            assert_eq!(
                rig.call(call, regs).status(),
                ST_ARG,
                "{call} into {buf:#x}"
            );
        }
    }
}

#[test]
fn a_bad_buffer_is_status_2_even_when_the_string_is_empty() {
    let bytes = demo();
    let library = decoded(&bytes);
    let (n, k) = library
        .assertions
        .iter()
        .enumerate()
        .find_map(|(n, a)| {
            let k = a
                .hypotheses
                .iter()
                .position(|h| matches!(h, mcpack::library::Hypothesis::Float { .. }))?;
            Some((n as u16, k as u8))
        })
        .expect("a float hypothesis, whose string is empty");
    let mut rig = opened(&bytes);
    let good = rig.hypothesis(n, k);
    assert_eq!((good.0, good.1.len()), (ST_OK, 0));
    let regs = Regs {
        hl: n,
        bc: u16::from(k) << 8,
        de: 0x7000,
        ..Regs::default()
    };
    assert_eq!(rig.call("LIB_HYPOTHESIS", regs).status(), ST_ARG);
    let regs = Regs {
        hl: n,
        de: 0xC100,
        ..Regs::default()
    };
    assert_eq!(rig.call("LIB_LABEL", regs).status(), ST_ARG);
    assert_eq!(rig.call("LIB_CONCLUSION", regs).status(), ST_ARG);
    let regs = Regs {
        a: 0,
        de: 0xC100,
        ..Regs::default()
    };
    assert_eq!(rig.call("LIB_SYMBOL_NAME", regs).status(), ST_ARG);
}

#[test]
fn lib_read_refuses_until_a_library_is_open() {
    let bytes = demo();
    let mut rig = Rig::with_library(&bytes);
    let canary = vec![0x77; 256];
    rig.poke(BUF, &canary);
    for (offset, len) in [
        (0u32, 1u16),
        (0, 256),
        (0x0010_0000, 16),
        (0x00FF_FF00, 256),
    ] {
        let status = rig.read_call(offset, len, BUF).status();
        assert_eq!(status, ST_CLOSED, "before any open: {offset:#x}");
        assert_eq!(rig.peek(BUF, 256), canary);
    }
    let mut bad = bytes.clone();
    bad[0] = b'X';
    rig.load(&bad);
    assert_eq!(rig.open(), ST_BAD);
    assert_eq!(
        rig.read_call(0, 1, BUF).status(),
        ST_CLOSED,
        "after a failed open"
    );
    assert_eq!(rig.read_call(0x0010_0000, 16, BUF).status(), ST_CLOSED);
    rig.load(&bytes);
    rig.open_ok();
    assert_eq!(rig.read(0, 4), (ST_OK, bytes[..4].to_vec()));
    rig.load(&bad);
    assert_eq!(rig.open(), ST_BAD);
    assert_eq!(
        rig.read_call(0, 1, BUF).status(),
        ST_CLOSED,
        "after a good open and a failed one"
    );
    assert_eq!(rig.peek(BUF, 256)[4..], canary[4..]);
}

#[test]
fn a_record_number_past_the_end_is_refused_and_leaves_no_descriptor() {
    let bytes = demo();
    let library = decoded(&bytes);
    let count = library.assertions.len() as u16;
    let mut rig = opened(&bytes);
    let none = |rig: &mut Rig| rig.named("LR_DN", 2) == [0xFF, 0xFF];
    assert!(none(&mut rig), "nothing described after an open");
    for n in [0xFFFFu16, count, count + 1, 0x8000] {
        assert_eq!(rig.record(n), ST_INDEX, "record {n:#x}");
        assert_eq!(rig.conclusion(n).0, ST_INDEX, "conclusion {n:#x}");
        assert_eq!(rig.hypothesis(n, 0).0, ST_INDEX, "hypothesis {n:#x}");
        assert_eq!(rig.label_of(n).0, ST_INDEX, "label {n:#x}");
        assert!(none(&mut rig));
    }
    let last = count - 1;
    assert_eq!(rig.record(last), ST_OK);
    assert_eq!(rig.named("LR_DN", 2), last.to_le_bytes());
    assert_eq!(rig.record(0xFFFF), ST_INDEX);
    assert!(none(&mut rig), "a failed record leaves no descriptor");
    assert_eq!(rig.conclusion(0xFFFF).0, ST_INDEX);
    assert_eq!(rig.record(last), ST_OK);
    assert_eq!(rig.record(count), ST_INDEX);
    assert!(none(&mut rig));
    let (status, got) = rig.conclusion(last);
    let want: Vec<u8> = {
        let names = names(&library);
        let id = |s: &String| names.iter().position(|n| n == s).unwrap() as u8;
        library.assertions[last as usize]
            .conclusion
            .iter()
            .map(id)
            .collect()
    };
    assert_eq!((status, got), (ST_OK, want));
}

#[test]
fn a_reopened_reader_forgets_the_descriptor_of_the_library_before() {
    let mut libraries: Vec<(u16, Vec<u8>)> = CORPUS
        .iter()
        .map(|name| {
            let bytes = pack_all(&corpus(name));
            (decoded(&bytes).assertions.len() as u16, bytes)
        })
        .collect();
    libraries.retain(|(count, _)| *count > 0);
    libraries.sort();
    let (small_count, small) = libraries.first().unwrap().clone();
    let (big_count, big) = libraries.last().unwrap().clone();
    assert!(big_count > small_count + 2);
    let mut rig = opened(&big);
    let n = big_count - 1;
    assert_eq!(rig.record(n), ST_OK);
    assert_eq!(rig.conclusion(n).0, ST_OK);
    rig.load(&small);
    rig.open_ok();
    assert_eq!(rig.named("LR_DN", 2), [0xFF, 0xFF]);
    assert_eq!(rig.conclusion(n).0, ST_INDEX);
    assert_eq!(rig.record(n), ST_INDEX);
    assert_eq!(rig.hypothesis(n, 0).0, ST_INDEX);
    assert_eq!(rig.conclusion(0xFFFF).0, ST_INDEX);
    assert_eq!(rig.label_of(n).0, ST_INDEX);
    assert_eq!(rig.conclusion(small_count - 1).0, ST_OK);
}
