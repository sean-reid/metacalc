use crate::damaged::{offset_of, reseal};
use crate::isolation::{Allowed, pattern, prime, stray};
use crate::secrig::ST_NONE;
use crate::sections::*;
use crate::support::*;
use crate::treelibs::{axioms, pack_with_sections, strings};
use mcpack::library::{TREES, decode};

/// The syntax table's entries as (assertion number, arity), and where the first one sits.
fn entries(bytes: &[u8]) -> (Vec<(u16, u8)>, usize) {
    let at = offset_of(bytes, TREES) as usize + 2;
    let count = decoded(bytes).sections.unwrap().trees.unwrap().syntax.len();
    let list = (0..count)
        .map(|i| {
            let e = at + 3 * i;
            (u16::from_le_bytes([bytes[e], bytes[e + 1]]), bytes[e + 2])
        })
        .collect();
    (list, at)
}

fn put(bytes: &mut [u8], at: usize, i: usize, entry: (u16, u8)) {
    let e = at + 3 * i;
    bytes[e..e + 2].copy_from_slice(&entry.0.to_le_bytes());
    bytes[e + 2] = entry.1;
}

#[test]
fn the_reader_returns_a_syntax_entry_as_stored_when_the_decoder_refuses_the_table() {
    let good = headed(60, 2, 0);
    let (list, at) = entries(&good);
    assert!(list.len() >= 3, "{list:?}");
    let mut swapped = good.clone();
    put(&mut swapped, at, 0, list[1]);
    put(&mut swapped, at, 1, list[0]);
    let mut arity = good.clone();
    put(&mut arity, at, 0, (list[0].0, list[0].1 + 1));
    put(&mut arity, at, 2, (list[2].0, 0));
    for (what, mut bytes, stored) in [
        ("out of order", swapped, [list[1], list[0], list[2]]),
        (
            "a wrong arity",
            arity,
            [(list[0].0, list[0].1 + 1), list[1], (list[2].0, 0)],
        ),
    ] {
        reseal(&mut bytes);
        assert!(decode(&bytes).is_err(), "{what}: the decoder refuses");
        let mut rig = Rig::with_library(&bytes);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.open_ok();
        assert_eq!(rig.syntax_count(), (ST_OK, list.len() as u16), "{what}");
        for (i, (assertion, arity)) in stored.iter().enumerate() {
            assert_eq!(
                rig.syntax(i as u16),
                (ST_OK, *assertion, *arity),
                "{what}: entry {i} as stored"
            );
        }
        let found = stray(&mut rig, &Allowed::of(DEFAULT));
        assert!(found.is_empty(), "{what}: {found:x?}");
        let page = rig.m.ram_page(OS_PAGE).to_vec();
        let lowest = (0..page.len())
            .find(|&at| page[at] != pattern(OS_PAGE, at))
            .unwrap();
        let used = page.len() - lowest;
        assert!(used <= 2 + 16, "{what}: {used} bytes of stack");
    }
}

const FILL: u8 = 0xA5;
const SIZE: u16 = 0x200;

/// The arities the host decoder reads from the trees section, in syntax order.
fn host_arities(bytes: &[u8]) -> Vec<u8> {
    let trees = decoded(bytes).sections.unwrap().trees.unwrap();
    trees.syntax.iter().map(|s| s.arity as u8).collect()
}

fn has_trees(bytes: &[u8]) -> bool {
    decoded(bytes).sections.is_some_and(|s| s.trees.is_some())
}

/// `lib_arities` over every library of `libraries` against the decoder and against `lib_syntax`.
fn check_arities(libraries: &[(String, Vec<u8>)]) {
    for (what, bytes) in libraries {
        let want = host_arities(bytes);
        let mut rig = Rig::with_library(bytes);
        rig.m.out_port(5, 0);
        prime(&mut rig);
        rig.open_ok();
        for size in [want.len() as u16, SIZE] {
            rig.poke(BUF, &[FILL; SIZE as usize]);
            let (status, written, buffer) = rig.arities(BUF, size);
            assert_eq!(
                (status, usize::from(written)),
                (ST_OK, want.len()),
                "{what}"
            );
            assert_eq!(buffer[..want.len()], want[..], "{what}: size {size}");
            assert!(
                buffer[want.len()..].iter().all(|&b| b == FILL),
                "{what}: nothing past the table is written, size {size}"
            );
        }
        for (i, arity) in want.iter().enumerate() {
            assert_eq!(rig.syntax(i as u16).2, *arity, "{what}: lib_syntax {i}");
        }
        let found = stray(&mut rig, &Allowed::of(DEFAULT));
        assert!(found.is_empty(), "{what}: {found:x?}");
    }
}

#[test]
fn the_arities_of_the_syntax_table_come_in_one_call_as_the_decoder_reads_them() {
    let mut libraries: Vec<(String, Vec<u8>)> = CORPUS
        .iter()
        .map(|name| (name.to_string(), pack_all(&corpus(name))))
        .filter(|(_, bytes)| has_trees(bytes))
        .collect();
    assert!(libraries.len() > 3);
    libraries.push(("headed".into(), headed(400, 3, 0)));
    libraries.push(("383 axioms".into(), axioms(383)));
    libraries.push(("127 axioms".into(), axioms(127)));
    let most = libraries.iter().map(|(_, b)| host_arities(b).len()).max();
    assert_eq!(most, Some(383));
    check_arities(&libraries);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_arities_of_the_shipped_libraries_come_in_one_call_as_the_decoder_reads_them() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let libraries: Vec<(String, Vec<u8>)> = ["prop", "pred", "ring"]
        .iter()
        .map(|name| {
            let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
            (name.to_string(), bytes)
        })
        .collect();
    check_arities(&libraries);
}

/// One call of lib_arities with `size` bytes at `at` and the memory from 16 bytes before it to 16
/// after the table primed, the state block excepted. Returns (status, written) and the bytes of
/// that stretch before and after the call, the state's two scratch words masked.
fn around(rig: &mut Rig, at: u16, size: u16, whole: bool) -> ((u8, u16), Vec<u8>, Vec<u8>) {
    rig.sp = 0x9000;
    let state = usize::from(rig.label("LIB_STATE"));
    let length = usize::from(rig.label("LIB_STATE_SIZE"));
    let lo = usize::from(at).saturating_sub(16);
    let hi = (usize::from(at) + usize::from(size.max(1)) + 16).min(0x1_0000);
    for addr in (lo..hi).filter(|a| !(state..state + length).contains(a)) {
        rig.poke(addr as u16, &[FILL]);
    }
    let masked: Vec<usize> = if whole {
        (state..state + length).collect()
    } else {
        [0x40, 0x41, 0xA4, 0xA5].iter().map(|o| state + o).collect()
    };
    let seen = |rig: &mut Rig| {
        let mut bytes = rig.peek(lo as u16, hi - lo);
        for addr in masked.iter().filter(|a| (lo..hi).contains(a)) {
            bytes[addr - lo] = 0;
        }
        bytes
    };
    let before = seen(rig);
    let (status, written, _) = rig.arities(at, size);
    ((status, written), before, seen(rig))
}

fn refused(rig: &mut Rig, at: u16, size: u16, what: &str) {
    let (said, before, after) = around(rig, at, size, false);
    assert_eq!(said, (ST_ARG, 0), "{what}");
    assert_eq!(
        before, after,
        "{what}: nothing is written to the buffer or beside it"
    );
}

fn accepted(rig: &mut Rig, at: u16, want: &[u8], what: &str) {
    let (said, before, after) = around(rig, at, want.len() as u16, true);
    assert_eq!(said, (ST_OK, want.len() as u16), "{what}");
    let lo = usize::from(at).saturating_sub(16);
    let start = usize::from(at) - lo;
    assert_eq!(after[start..start + want.len()], *want, "{what}");
    assert_eq!(
        before[..start],
        after[..start],
        "{what}: nothing before the buffer"
    );
    assert_eq!(
        before[start + want.len()..],
        after[start + want.len()..],
        "{what}: nothing after the buffer"
    );
}

#[test]
fn a_buffer_smaller_than_the_table_is_refused_with_nothing_written() {
    let bytes = axioms(383);
    let mut rig = opened(&bytes);
    for size in [0, 1, 200, 382] {
        refused(&mut rig, BUF, size, &format!("size {size}"));
    }
    assert_eq!(rig.arities(BUF, 383).0, ST_OK);
}

#[test]
fn a_buffer_that_touches_the_state_block_is_refused_and_one_beside_it_is_filled() {
    let bytes = axioms(383);
    let want = host_arities(&bytes);
    let mut rig = opened(&bytes);
    let state = rig.label("LIB_STATE");
    let length = rig.label("LIB_STATE_SIZE");
    let n = 383;
    for (what, at) in [
        ("inside the state", state + 0x40),
        ("last byte on the first byte of the state", state - (n - 1)),
        (
            "last byte on the last byte of the state",
            state + length - n,
        ),
        (
            "first byte on the last byte of the state",
            state + length - 1,
        ),
        ("first byte on the first byte of the state", state),
    ] {
        refused(&mut rig, at, n, what);
    }
    accepted(
        &mut rig,
        state - n,
        &want,
        "last byte right before the state",
    );
    accepted(
        &mut rig,
        state + length,
        &want,
        "first byte right after the state",
    );
}

#[test]
fn a_buffer_in_bank_c_is_filled_and_one_on_the_bounce_buffer_is_refused() {
    let bytes = axioms(383);
    let want = host_arities(&bytes);
    let mut rig = opened(&bytes);
    let bounce = rig.label("LIB_BOUNCE");
    let n = 383;
    accepted(&mut rig, 0xC100, &want, "bank C");
    accepted(
        &mut rig,
        bounce - n,
        &want,
        "last byte right before the bounce buffer",
    );
    rig.m.out_port(5, SPARE_PAGE);
    refused(
        &mut rig,
        bounce,
        n,
        "on the bounce buffer with the spare page mapped",
    );
    refused(
        &mut rig,
        bounce - n + 1,
        n,
        "last byte on the first byte of the bounce buffer",
    );
    rig.m.out_port(5, 0);
    for (what, at) in [("past the top", (0xFFFF - n + 1)), ("from $FF00", 0xFF00)] {
        refused(&mut rig, at, n, what);
    }
}

#[test]
fn a_buffer_ends_at_the_top_of_memory_short_of_the_last_byte_and_not_on_it() {
    let bytes = axioms(383);
    let want = host_arities(&bytes);
    let mut rig = Rig::with_image(MOVED);
    rig.load(&bytes);
    rig.m.out_port(5, 0);
    rig.open_ok();
    let bounce = rig.label("LIB_BOUNCE");
    assert!(
        bounce + 256 + 383 < 0xFFFE,
        "the moved bounce buffer is well below the top"
    );
    let n = 383;
    refused(&mut rig, 0xFFFF - n + 1, n, "last byte $FFFF");
    accepted(&mut rig, 0xFFFF - n, &want, "last byte $FFFE");
    refused(
        &mut rig,
        bounce + 255,
        n,
        "first byte on the last byte of the bounce buffer",
    );
    accepted(
        &mut rig,
        bounce + 256,
        &want,
        "first byte right after the bounce buffer",
    );
    accepted(
        &mut rig,
        bounce - n,
        &want,
        "last byte right before the bounce buffer",
    );
    refused(
        &mut rig,
        bounce - n + 1,
        n,
        "last byte on the first byte of the bounce buffer",
    );
}

/// A library of 40 syntax axioms whose syntax table has a boundary of the flash inside it, found
/// by padding the records before it with statements the grammar does not read.
fn table_across(boundary: usize) -> Vec<u8> {
    let build = |pad: usize| {
        let mut src = String::from("$c |- wff a b $.\n");
        for (i, body) in strings(40).iter().enumerate() {
            src += &format!("w{i} $a wff {body} $.\nax-{i} $a |- {body} $.\n");
        }
        for i in 0..pad {
            src += &format!("pad-{i:06} $a |- a $.\n");
        }
        pack_with_sections(&src)
    };
    let start = |bytes: &[u8]| offset_of(bytes, TREES) as usize + 2;
    let (low, high) = (start(&build(0)), start(&build(200)));
    let per = ((high - low) / 200).max(1);
    let (mut pad, mut bytes) = (boundary.saturating_sub(60 + low) / per, Vec::new());
    for _ in 0..20 {
        bytes = build(pad);
        let at = start(&bytes);
        if at < boundary && boundary < at + 3 * 40 {
            return bytes;
        }
        let aim = boundary - 60;
        pad = if at < aim {
            pad + (aim - at).div_ceil(per)
        } else {
            pad - (at - aim).div_ceil(per)
        };
    }
    panic!(
        "no padding puts the table across {boundary}, last at {}",
        start(&bytes)
    );
}

#[test]
fn a_syntax_table_across_a_chunk_or_past_64_kb_is_read_as_the_decoder_reads_it() {
    for boundary in [CHUNK, 2 * CHUNK, 1 << 16, 3 << 16] {
        let bytes = table_across(boundary);
        let want = host_arities(&bytes);
        assert_eq!(want.len(), 40);
        let mut rig = opened(&bytes);
        accepted(&mut rig, BUF, &want, &format!("table across {boundary:#x}"));
        for (i, arity) in want.iter().enumerate() {
            assert_eq!(
                rig.syntax(i as u16).2,
                *arity,
                "{boundary:#x}: lib_syntax {i}"
            );
        }
    }
}

#[test]
fn a_closed_reader_and_a_library_without_a_syntax_table_refuse_the_call() {
    let bytes = axioms(10);
    let mut closed = Rig::with_library(&bytes);
    closed.poke(BUF, &[FILL; SIZE as usize]);
    let (status, written, buffer) = closed.arities(BUF, SIZE);
    assert_eq!((status, written), (ST_CLOSED, 0));
    assert!(buffer.iter().all(|&b| b == FILL));
    let db = parse("$c wff |- $.\n$v p $.\nwp $f wff p $.\nax $a |- p $.\n");
    let kernel = mcpack::library::pack_kernel(&db, &everything(&db)).unwrap();
    let mut bare = opened(&kernel);
    bare.poke(BUF, &[FILL; SIZE as usize]);
    let (status, written, buffer) = bare.arities(BUF, SIZE);
    assert_eq!((status, written), (ST_NONE, 0));
    assert!(buffer.iter().all(|&b| b == FILL));
    let (status, written, _) = bare.arities(BUF, 0);
    assert_eq!(
        (status, written),
        (ST_NONE, 0),
        "the section is checked before the buffer"
    );
}

#[test]
fn an_entry_naming_no_assertion_stops_the_call_where_lib_syntax_refuses_it() {
    let good = headed(60, 2, 0);
    let (list, at) = entries(&good);
    let assertions = decoded(&good).assertions.len() as u16;
    let want = host_arities(&good);
    for (index, number) in [
        (0, assertions),
        (1, 0xFFFF),
        (list.len() / 2, assertions),
        (list.len() - 1, assertions),
    ] {
        let mut bytes = good.clone();
        put(&mut bytes, at, index, (number, list[index].1));
        reseal(&mut bytes);
        assert!(decode(&bytes).is_err());
        let mut rig = opened(&bytes);
        rig.poke(BUF, &[FILL; SIZE as usize]);
        let (status, written, buffer) = rig.arities(BUF, SIZE);
        assert_eq!(
            (status, usize::from(written)),
            (ST_BAD, index),
            "entry {index}"
        );
        assert_eq!(buffer[..index], want[..index], "entry {index}");
        assert!(
            buffer[index..].iter().all(|&b| b == FILL),
            "entry {index}: nothing from the refused entry on is written"
        );
        assert_eq!(rig.syntax(index as u16).0, ST_BAD, "entry {index}");
        if index > 0 {
            assert_eq!(rig.syntax(index as u16 - 1).0, ST_OK, "entry {index}");
        }
    }
}

#[test]
fn a_table_the_library_ends_inside_stops_the_call_at_the_last_whole_entry() {
    let bytes = axioms(60);
    let (list, at) = entries(&bytes);
    let want = host_arities(&bytes);
    assert!(list.len() > 25);
    for whole in [1usize, 10, 11, 20, 24] {
        for extra in [0usize, 1, 2] {
            let mut rig = opened(&bytes);
            let length = (at + 3 * whole + extra) as u32;
            rig.poke(rig.label("LR_LENGTH"), &length.to_le_bytes()[..3]);
            rig.poke(BUF, &[FILL; SIZE as usize]);
            let (status, written, buffer) = rig.arities(BUF, SIZE);
            let what = format!("{whole} entries and {extra} bytes");
            assert_eq!((status, usize::from(written)), (ST_BAD, whole), "{what}");
            assert_eq!(buffer[..whole], want[..whole], "{what}");
            assert!(buffer[whole..].iter().all(|&b| b == FILL), "{what}");
        }
    }
}

#[test]
fn a_table_longer_than_its_section_is_refused_at_open_and_the_call_stays_closed() {
    let good = headed(60, 2, 0);
    let section = offset_of(&good, TREES) as usize;
    let mut bytes = good.clone();
    bytes[section..section + 2].copy_from_slice(&383u16.to_le_bytes());
    reseal(&mut bytes);
    assert!(decode(&bytes).is_err());
    let mut rig = Rig::with_library(&bytes);
    assert_eq!(rig.open(), 8);
    rig.poke(BUF, &[FILL; SIZE as usize]);
    let (status, written, buffer) = rig.arities(BUF, SIZE);
    assert_eq!((status, written), (ST_CLOSED, 0));
    assert!(buffer.iter().all(|&b| b == FILL));
}
