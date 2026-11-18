use crate::limits::copies_src;
use crate::support::*;
use crate::sweep::disagreement;
use mcpack::library::{self, KernelError};
use mcpack::mm::Database;

fn long_labels(count: usize) -> Database {
    let mut src = String::from("$c T wff |- $.\n");
    for i in 0..count {
        src += &format!("a-long-label-to-fill-the-page-{i} $a |- T $.\n");
    }
    parse(&src)
}

fn chunks(bytes: &[u8]) -> usize {
    bytes.len().div_ceil(PAGE)
}

#[test]
fn a_library_ending_at_page_zero_is_accepted_and_one_chunk_further_is_refused() {
    let bytes = pack_all(&long_labels(700));
    let last = chunks(&bytes) - 1;
    assert!(last >= 1);
    let mut device = Device::with_library_at(last as u8, &bytes);
    let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    let (init, got, _) = device.verify_all(700, ASKED_UNBOUNDED);
    assert_eq!((init, got), (want_init, want));
    assert_eq!(init, 0);

    device.lib_page = last as u8 - 1;
    let got = device.verify(0, ASKED_UNBOUNDED);
    assert_eq!(got.init, status(Err(KernelError::Malformed)));
    assert_eq!(got.verify, status(Err(KernelError::Malformed)));
}

#[test]
fn the_validated_page_and_limits_are_the_ones_used() {
    let (db, bytes) = small_library();
    let both = number(&db, "both");
    let want = status(Ok(()));
    let mut device = Device::with_library(&bytes);
    assert_eq!(device.verify(both, ASKED_UNBOUNDED).verify, want);
    device.m.load_ram(
        device.label("KERNEL_LIB_PAGE"),
        &[0x20, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(both as u16).to_le_bytes());
    device.run("ENTRY_VERIFY");
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), want);

    let many = pack_all(&long_labels(2500));
    let mut device = Device::with_library(&many);
    device.verify(0, ASKED_UNBOUNDED);
    device.m.load_ram(device.label("KERNEL_LIB_PAGE"), &[0x7F]);
    for id in [0, 1200, 2499] {
        device
            .m
            .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
        device.run("ENTRY_VERIFY");
        assert_eq!(device.byte("TEST_VERIFY_STATUS"), want, "{id}");
    }
}

#[test]
fn the_block_can_be_assembled_at_another_address() {
    let base = label_in(MOVED, "KERNEL_RAM");
    assert_eq!(base, 0xC400);
    assert_ne!(base, label("KERNEL_RAM"));
    let (small_db, small) = small_library();
    let demo = pack_all(&corpus("demo0.mm"));
    for bytes in [small, demo] {
        let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
        let mut device = Device::with_image(MOVED);
        device.load(&bytes);
        device.m.load_ram(0xC000, &[0x5A; 0x400]);
        let (init, got, _) = device.verify_all(want.len(), ASKED_UNBOUNDED);
        assert_eq!((init, got), (want_init, want));
        assert_eq!(device.m.read_ram(0xC000, 0x400), vec![0x5A; 0x400]);
        let end = device.label("KERNEL_RAM_END");
        assert_eq!(end, label("KERNEL_RAM_END") + 0x400);
    }
    let id = number(&small_db, "both");
    let (_, bytes) = small_library();
    let mut device = Device::with_image(MOVED);
    device.load(&bytes);
    assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, status(Ok(())));
}

fn many_axioms(count: usize) -> (Database, Vec<u8>) {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    for i in 0..count {
        src += &format!("n{i} $a wff T $.\n");
    }
    src += &format!(
        "t-low $p wff T $= n0 n1 drop $.\n\
         t-edge $p wff T $= n{} n{} drop $.\n\
         t-high $p wff T $= n{} n{} drop $.\n\
         t-last $p wff T $= n{} n{} drop $.\n",
        8189,
        8190,
        count - 1,
        count - 2,
        count - 3,
        count - 4
    );
    let db = parse(&src);
    let bytes = pack_all(&db);
    (db, bytes)
}

#[test]
fn assertions_past_the_cache_and_a_directory_over_pages_agree_with_the_model() {
    let (db, bytes) = many_axioms(8300);
    let count = assertions(&db).len();
    assert!(count > 8300 && chunks(&bytes) > 8);
    let ok = status(Ok(()));
    let mut device = Device::with_library(&bytes);
    let mut ids = vec![8190, 8191, 8192, 8193, count - 1];
    ids.extend(["t-low", "t-edge", "t-high", "t-last"].map(|name| number(&db, name)));
    for id in ids {
        let got = same_as_model(
            &mut device,
            &bytes,
            id,
            ASKED_UNBOUNDED,
            &format!("id {id}"),
        );
        assert_eq!(got, ok, "id {id}");
    }
    let directory = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let boundaries: Vec<usize> = (0..count)
        .filter(|&e| (directory + 4 * e) % PAGE == 0 && e > 0)
        .collect();
    assert!(!boundaries.is_empty(), "the directory crosses a page");
    for &entry in &boundaries {
        for id in [entry - 1, entry] {
            for byte in 0..4 {
                let at = directory + 4 * id + byte;
                for value in [
                    0x00,
                    0x01,
                    0x7F,
                    0x80,
                    0xFE,
                    0xFF,
                    bytes[at] ^ 1,
                    bytes[at] ^ 0x40,
                ] {
                    let mut mutated = bytes.clone();
                    mutated[at] = value;
                    reseal(&mut mutated);
                    device.load(&mutated);
                    let got = device.verify(id, ASKED_UNBOUNDED);
                    assert_eq!(got.init, 0);
                    assert_eq!(
                        got.verify,
                        model_one(&mutated, id, ASKED_UNBOUNDED),
                        "entry {id} byte {byte} = {value:#04x}"
                    );
                }
            }
        }
    }
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

/// The packed library with the record before the second page moved to end `shift` bytes
/// past where it ends now; returns it with that record's number.
fn shifted_before_the_page(shift: usize) -> (Vec<u8>, usize) {
    let mut bytes = pack_all(&long_labels(700));
    let directory = u32_at(&bytes, 16);
    let offset = |bytes: &[u8], id: usize| u32_at(bytes, directory + 4 * id);
    let first = (0..700)
        .find(|&id| offset(&bytes, id) >= library::CHUNK)
        .unwrap();
    let prev = first - 1;
    let start = offset(&bytes, prev);
    let label_len = bytes[start + 1] as usize;
    let end = start + label_len + 8;
    assert!(end <= library::CHUNK);
    let at = start + library::CHUNK - end + shift;
    bytes.copy_within(start..end, at);
    let entry = directory + 4 * prev;
    bytes[entry..entry + 4].copy_from_slice(&(at as u32).to_le_bytes());
    reseal(&mut bytes);
    (bytes, prev)
}

#[test]
fn a_record_ending_at_the_last_byte_of_a_page_is_accepted_and_one_byte_more_is_refused() {
    let (bytes, id) = shifted_before_the_page(0);
    agree(&bytes, id, Ok(()), "last byte at $3FFF");
    let (bytes, id) = shifted_before_the_page(1);
    agree(&bytes, id, Err(KernelError::Malformed), "one byte over");
}

#[test]
fn a_library_that_ends_inside_a_record_is_judged_as_the_model_would() {
    let bytes = pack_all(&long_labels(700));
    let directory = u32_at(&bytes, 16);
    let last = u32_at(&bytes, directory + 4 * 699);
    let mut device = Device::new();
    for length in last.saturating_sub(2)..=bytes.len() {
        let mut cut = bytes.clone();
        cut[8..12].copy_from_slice(&(length as u32).to_le_bytes());
        reseal(&mut cut);
        if let Some(difference) = disagreement(&mut device, &cut, ASKED_UNBOUNDED, &mut [0; 16]) {
            panic!("length {length}: {difference}");
        }
    }
}

const REGISTERS: [u8; 10] = [0x57, 0x13, 0x68, 0x24, 0x9C, 0x36, 0xAE, 0x48, 0xC0, 0x5A];

fn failing_libraries() -> Vec<(Vec<u8>, usize)> {
    let mut theorems = String::new();
    for (i, proof) in [
        "vx vy vy vx ax-d ax-m",
        "vx ax-d",
        "vx vy ax-d vx",
        "vx vy ax-d",
        "( ax-d ) ABCD",
    ]
    .iter()
    .enumerate()
    {
        let conclusion = if i == 3 { "R y x" } else { "R x y" };
        let conclusion = if i == 0 { "R x x" } else { conclusion };
        theorems += &format!("${{ $d x y $. t{i} $p |- {conclusion} $= {proof} $. $}}\n");
    }
    let db = parse(&format!("{DB}\n{theorems}"));
    let bytes = pack_all(&db);
    let mut cases = vec![
        (bytes.clone(), number(&db, "good")),
        (bytes.clone(), number(&db, "bad")),
        (bytes.clone(), number(&db, "nodummy")),
        (bytes.clone(), assertions(&db).len()),
    ];
    cases.extend((0..5).map(|i| (bytes.clone(), number(&db, &format!("t{i}")))));
    let good = number(&db, "good");
    let mut forward = bytes.clone();
    let refs = fields(&forward, good).refs;
    forward[refs..refs + 2].copy_from_slice(&(good as u16).to_le_bytes());
    reseal(&mut forward);
    cases.push((forward, good));
    let mut record = bytes.clone();
    let dv = fields(&record, number(&db, "ax-d")).dv;
    record[dv + 1] += 1;
    reseal(&mut record);
    cases.push((record, good));
    let typecodes = parse(
        "$c wff set |- $. $v x ph $. vx $f set x $. wph $f wff ph $. ax $a |- ph $.
         wrong $p |- ph $= vx ax $.",
    );
    cases.push((pack_all(&typecodes), number(&typecodes, "wrong")));
    let mut magic = bytes;
    magic[0] = b'X';
    cases.push((magic, 0));
    cases
}

#[test]
fn registers_the_bank_and_interrupts_survive_every_kind_of_failure() {
    let marker = *b"flash page marker";
    let mut seen = std::collections::BTreeSet::new();
    for (bytes, id) in failing_libraries() {
        let want = model_one(&bytes, id, ASKED_UNBOUNDED);
        seen.insert(want);
        let mut device = Device::with_library(&bytes);
        device.m.load_flash(0x50, 0, &marker);
        device.m.load_ram(0x8000, &[0xA5; 64]);
        device.params(ASKED_UNBOUNDED);
        device
            .m
            .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
        device.run("ENTRY_REGS");
        assert_eq!(device.byte("TEST_VERIFY_STATUS"), want);
        let regs = device.m.read_ram(device.label("TEST_REGS"), 10);
        assert_eq!(regs, REGISTERS, "registers after status {want}");

        for page in [0x81u8, 0x50] {
            for interrupts in [0u8, 1] {
                device.m.out_port(7, page);
                device
                    .m
                    .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
                device.run("ENTRY_IFFV");
                assert_eq!(device.byte("TEST_VERIFY_STATUS"), want);
                assert_eq!(
                    device.byte("TEST_IFF"),
                    interrupts,
                    "interrupts, status {want}"
                );
                let bank = device.m.read_ram(0x8000, marker.len());
                let expect = if page == 0x81 {
                    vec![0xA5; marker.len()]
                } else {
                    marker.to_vec()
                };
                assert_eq!(
                    bank, expect,
                    "bank B after status {want} from page {page:#x}"
                );
            }
        }
    }
    for code in [0, 1, 2, 4, 6, 7, 8, 9, 10, 11, 12] {
        assert!(
            seen.contains(&(code as u8)),
            "no case ended in status {code}"
        );
    }
}

#[test]
fn the_substitution_serial_wraps_without_changing_a_result() {
    let db = parse(&copies_src(2, 6, false));
    let bytes = pack_all(&db);
    let id = number(&db, "big");
    let mut device = Device::with_library(&bytes);
    assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, status(Ok(())));
    let serial = device.label("KV_SERIAL");
    assert_eq!(device.m.read_ram(serial, 2), [11, 0], "eleven applies");
    device.m.load_ram(serial, &0xFFFDu16.to_le_bytes());
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.run("ENTRY_VERIFY");
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), status(Ok(())));
    assert_eq!(
        device.m.read_ram(serial, 2),
        [9, 0],
        "wrapped to 1 on the third apply"
    );
}

#[test]
fn enough_applies_in_one_initialisation_wrap_the_serial() {
    const DROPS: usize = 7000;
    const THEOREMS: usize = 10;
    let mut src = String::from(
        "$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\nn0 $a wff T $.\n",
    );
    for i in 0..THEOREMS {
        src += &format!(
            "big{i} $p wff T $= n0 n0 drop{} $.\n",
            " n0 drop".repeat(DROPS)
        );
    }
    let db = parse(&src);
    let bytes = pack_all(&db);
    let count = assertions(&db).len();
    let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    let mut device = Device::with_library(&bytes);
    let (init, got, _) = device.verify_all(count, ASKED_UNBOUNDED);
    assert_eq!((init, &got), (want_init, &want));
    assert_eq!(got[2..], [0; THEOREMS]);
    let applies = THEOREMS * (2 * DROPS + 3);
    assert!(applies > 2 * 0xFFFF);
    let serial = device.m.read_ram(device.label("KV_SERIAL"), 2);
    assert_eq!(serial, (((applies - 1) % 0xFFFF + 1) as u16).to_le_bytes());
}
