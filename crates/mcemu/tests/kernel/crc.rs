use crate::support::*;
use mcemu::Speed;
use mcpack::library::{CHUNK, Kernel, crc32};

const MORE: u8 = 0xFF;
const MALFORMED: u8 = 1;

struct Crc {
    init: u8,
    begin: u8,
    statuses: Vec<u8>,
    value: u32,
    tstates: u32,
}

impl Crc {
    /// The status the app would see: the first refusal, else the last chunk's.
    fn overall(&self) -> u8 {
        if self.init != 0 {
            self.init
        } else if self.begin != 0 {
            self.begin
        } else {
            *self.statuses.last().unwrap_or(&MORE)
        }
    }
}

fn run_entry(device: &mut Device, entry: &str, max_calls: u8) -> Crc {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_COUNT"), &[max_calls, 0]);
    let tstates = device.run(entry);
    let calls = device.byte("TEST_CRC_CALLS") as usize;
    let value = device.m.read_ram(device.label("KERNEL_CRC"), 4);
    Crc {
        init: device.byte("TEST_INIT_STATUS"),
        begin: device.byte("TEST_CRC_BEGIN"),
        statuses: device.m.read_ram(device.label("TEST_STATUSES"), calls),
        value: u32::from_le_bytes(value.try_into().unwrap()),
        tstates,
    }
}

fn run_crc(device: &mut Device) -> Crc {
    run_entry(device, "ENTRY_CRC", 255)
}

fn chunks(len: usize) -> usize {
    len.div_ceil(CHUNK)
}

fn model_accepts(bytes: &[u8]) -> bool {
    Kernel::new(bytes, clamped(ASKED_UNBOUNDED)).is_ok()
}

/// `base` with deterministic filler appended up to `len` bytes, sealed.
fn padded(base: &[u8], len: usize) -> Vec<u8> {
    let mut bytes = base.to_vec();
    let mut state = 0x2545_F491u32;
    while bytes.len() < len {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        bytes.push((state >> 11) as u8);
    }
    bytes[8..12].copy_from_slice(&(len as u32).to_le_bytes());
    reseal(&mut bytes);
    bytes
}

/// A library with no symbols and no assertions, `len` bytes long.
fn bare(len: usize) -> Vec<u8> {
    let mut bytes = vec![0xA5; len];
    bytes[..4].copy_from_slice(b"MCL1");
    bytes[8..12].copy_from_slice(&(len as u32).to_le_bytes());
    bytes[12..16].fill(0);
    bytes[16..20].copy_from_slice(&20u32.to_le_bytes());
    reseal(&mut bytes);
    bytes
}

fn sum(bytes: &[u8]) -> u32 {
    crc32(&bytes[8..])
}

#[test]
fn the_checksum_agrees_with_the_model_on_every_corpus_library() {
    let (_, small) = small_library();
    let mut libraries = vec![small];
    for name in [
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
    ] {
        libraries.push(pack_all(&corpus(name)));
    }
    for (n, bytes) in libraries.iter().enumerate() {
        let mut device = Device::with_library(bytes);
        let run = run_crc(&mut device);
        assert!(model_accepts(bytes), "library {n}: the model");
        assert_eq!(
            (run.init, run.begin, run.overall()),
            (0, 0, 0),
            "library {n}"
        );
        assert_eq!(
            run.statuses.len(),
            chunks(bytes.len()),
            "library {n}: calls"
        );
        assert!(
            run.statuses[..run.statuses.len() - 1]
                .iter()
                .all(|&s| s == MORE)
        );
        assert_eq!(run.value, sum(bytes), "library {n}: value");

        let mut wrong = bytes.clone();
        wrong[4] ^= 0x10;
        assert!(!model_accepts(&wrong), "library {n}: the model, wrong sum");
        device.load(&wrong);
        let run = run_crc(&mut device);
        assert_eq!(
            (run.init, run.overall()),
            (0, MALFORMED),
            "library {n}, wrong sum"
        );
        assert_eq!(run.value, sum(bytes), "library {n}: value, wrong sum");
    }
}

#[test]
fn each_bit_of_the_stored_checksum_matters() {
    let (_, bytes) = small_library();
    let mut device = Device::with_library(&bytes);
    for bit in 0..32 {
        let mut flipped = bytes.clone();
        flipped[4 + bit / 8] ^= 1 << (bit % 8);
        assert!(!model_accepts(&flipped), "bit {bit}: the model");
        device.load(&flipped);
        let run = run_crc(&mut device);
        assert_eq!((run.init, run.overall()), (0, MALFORMED), "bit {bit}");
        assert_eq!(run.value, sum(&bytes), "bit {bit}: value");
    }
    device.load(&bytes);
    assert_eq!(run_crc(&mut device).overall(), 0);
}

#[test]
fn a_changed_byte_in_any_chunk_is_refused() {
    let (_, small) = small_library();
    let len = 3 * CHUNK + 5000;
    let bytes = padded(&small, len);
    let mut device = Device::with_library(&bytes);
    let run = run_crc(&mut device);
    assert_eq!((run.overall(), run.statuses.len()), (0, 4));
    let mut at = vec![bytes.len() - 1, bytes.len() - 5000];
    at.extend((0..4).map(|chunk| chunk * CHUNK + 4000 + chunk));
    at.extend([
        CHUNK - 1,
        CHUNK,
        2 * CHUNK - 1,
        2 * CHUNK,
        3 * CHUNK - 1,
        3 * CHUNK,
    ]);
    for offset in at {
        let mut changed = bytes.clone();
        changed[offset] ^= 0x01;
        assert!(!model_accepts(&changed), "offset {offset}: the model");
        device.load(&changed);
        let run = run_crc(&mut device);
        assert_eq!((run.init, run.overall()), (0, MALFORMED), "offset {offset}");
        assert_eq!(run.value, sum(&changed), "offset {offset}: value");
    }
}

#[test]
fn the_value_is_right_at_every_edge_of_a_chunk() {
    let (_, small) = small_library();
    let mut lengths = vec![small.len() + 1, 1000];
    for k in 1..=3 {
        lengths.extend([k * CHUNK - 1, k * CHUNK, k * CHUNK + 1]);
    }
    for len in lengths {
        let bytes = padded(&small, len);
        assert!(model_accepts(&bytes), "length {len}: the model");
        let mut device = Device::with_library(&bytes);
        let run = run_crc(&mut device);
        assert_eq!((run.init, run.overall()), (0, 0), "length {len}");
        assert_eq!(run.statuses.len(), chunks(len), "length {len}: calls");
        assert_eq!(run.value, sum(&bytes), "length {len}: value");
    }
}

#[test]
fn the_smallest_libraries_are_checked() {
    for len in [20, 21, 22, 27, 28, 29] {
        let bytes = bare(len);
        let loads = model_accepts(&bytes);
        assert!(loads, "length {len}: the model");
        let mut device = Device::with_library(&bytes);
        let run = run_crc(&mut device);
        assert_eq!((run.init, run.overall()), (0, 0), "length {len}");
        assert_eq!(run.statuses, [0], "length {len}");
        assert_eq!(run.value, sum(&bytes), "length {len}: value");

        let mut wrong = bytes.clone();
        wrong[7] ^= 0x80;
        assert!(!model_accepts(&wrong), "length {len}: the model, wrong sum");
        device.load(&wrong);
        let run = run_crc(&mut device);
        assert_eq!(
            (run.init, run.overall()),
            (0, MALFORMED),
            "length {len}, wrong sum"
        );
    }
}

#[test]
fn the_checksum_needs_a_successful_init() {
    let (_, bytes) = small_library();
    let marker = *b"flash page marker";
    let mut device = Device::with_library(&bytes);
    device.m.load_flash(0x50, 0, &marker);
    device.m.out_port(PORT_BANK_B, 0x50);
    let run = run_entry(&mut device, "ENTRY_CRC_RAW", 4);
    assert_eq!((run.begin, run.statuses), (MALFORMED, vec![MALFORMED]));
    assert_eq!(device.m.read_ram(0x8000, marker.len()), marker);

    let mut bad = bytes.clone();
    bad[0] = b'X';
    device.load(&bad);
    let run = run_crc(&mut device);
    assert_eq!((run.init, run.begin), (MALFORMED, MALFORMED));
    assert_eq!(run.statuses, [MALFORMED]);
    assert_eq!(device.m.read_ram(0x8000, marker.len()), marker);

    let mut short = bytes.clone();
    short.truncate(19);
    short[8..12].copy_from_slice(&19u32.to_le_bytes());
    device.load(&short);
    let run = run_crc(&mut device);
    assert_eq!(
        (run.init, run.begin, run.statuses),
        (MALFORMED, MALFORMED, vec![MALFORMED])
    );
}

#[test]
fn a_run_is_ordered_begin_then_chunks_and_a_new_init_ends_it() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let mut device = Device::with_library(&bytes);

    let run = run_entry(&mut device, "ENTRY_CRC_REINIT", 0);
    assert_eq!(run.init, 0);
    let both = device.m.read_ram(device.label("TEST_STATUSES"), 2);
    assert_eq!(both, [MORE, MALFORMED]);

    let run = run_entry(&mut device, "ENTRY_CRC_RESTART", 255);
    assert_eq!((run.begin, run.overall(), run.value), (0, 0, sum(&bytes)));
    assert_eq!(run.statuses.len(), 3);

    run_entry(&mut device, "ENTRY_CRC_AGAIN", 255);
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), MALFORMED);
}

const REGISTERS: [u8; 10] = [0x57, 0x13, 0x68, 0x24, 0x9C, 0x36, 0xAE, 0x48, 0xC0, 0x5A];

#[test]
fn the_checksum_restores_registers_the_bank_the_interrupts_and_all_other_memory() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let marker = *b"flash page marker";
    assert!(label("KERNEL_RAM_END") <= label("TEST_STATUSES"));
    let free_start = label("TEST_STATUSES") + 16;
    let free_len = usize::from(label("TEST_BLOCK") - free_start);
    let stack = usize::from(label("KSTACK_TOP") - label("KSTACK"));
    for page in [RAM_PAGE, 0x50] {
        for interrupts in [0u8, 1] {
            let mut device = Device::with_library(&bytes);
            device.m.load_flash(0x50, 0, &marker);
            device.m.load_ram(0x8000, &vec![0xA5; PAGE]);
            device.m.load_ram(free_start, &vec![0x5A; free_len]);
            device.m.load_ram(label("KSTACK"), &vec![0xCC; stack]);
            device.m.out_port(PORT_BANK_B, page);
            device
                .m
                .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
            let run = run_entry(&mut device, "ENTRY_CRC_ENV", 255);
            assert_eq!((run.init, run.overall(), run.value), (0, 0, sum(&bytes)));
            assert_eq!(device.byte("TEST_IFF"), interrupts, "interrupts");
            let regs = device.m.read_ram(device.label("TEST_REGS"), 10);
            assert_eq!(regs, REGISTERS, "registers");
            let bank = device.m.read_ram(0x8000, marker.len());
            let expect = if page == RAM_PAGE {
                vec![0xA5; marker.len()]
            } else {
                marker.to_vec()
            };
            assert_eq!(bank, expect, "bank B from page {page:#x}");
            if page == RAM_PAGE {
                assert_eq!(device.m.read_ram(0x8000, PAGE), vec![0xA5; PAGE]);
            }
            assert!(
                device.m.read_ram(free_start, free_len) == vec![0x5A; free_len],
                "memory above the block"
            );
            let used = device.m.read_ram(label("KSTACK"), stack);
            let deepest = used.iter().position(|&b| b != 0xCC).unwrap_or(stack);
            assert!(
                deepest >= 128,
                "the kernel stack comes within 128 bytes of its end"
            );
        }
    }
}

fn vector(device: &mut Device, data: &[u8]) -> u32 {
    device.m.load_ram(label("TEST_BUF"), data);
    device
        .m
        .load_ram(label("TEST_BUF_LEN"), &(data.len() as u16).to_le_bytes());
    device.run("ENTRY_CRC_VECTOR");
    let raw = device.m.read_ram(label("KERNEL_CRC"), 4);
    !u32::from_le_bytes(raw.try_into().unwrap())
}

#[test]
fn the_core_reproduces_the_published_test_vector_and_table() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    let mut device = Device::new();
    assert_eq!(vector(&mut device, b"123456789"), 0xCBF4_3926);
    let table = device.m.read_ram(label("CRC_TABLE"), 1024);
    let entry =
        |i: usize| u32::from_le_bytes([table[i], table[256 + i], table[512 + i], table[768 + i]]);
    assert_eq!(entry(0), 0);
    assert_eq!(entry(1), 0x7707_3096);
    assert_eq!(entry(2), 0xEE0E_612C);
    assert_eq!(entry(255), 0x2D02_EF8D);
    assert_eq!(vector(&mut device, b""), 0);
    let data: Vec<u8> = (0..2000u32).map(|i| (i * 167 + 13) as u8).collect();
    for len in (0..=70).chain([255, 256, 257, 1000, 2000]) {
        assert_eq!(
            vector(&mut device, &data[..len]),
            crc32(&data[..len]),
            "length {len}"
        );
    }
}

#[test]
fn a_chunk_stays_under_half_a_second_at_6_mhz() {
    let (_, small) = small_library();
    let full = 369_178;
    let bytes = padded(&small, full);
    assert!(model_accepts(&bytes));
    let mut results = Vec::new();
    for speed in [Speed::Mhz15, Speed::Mhz6] {
        let mut device = Device::with_library(&bytes);
        device.m.set_speed(speed);
        let [none, one, two] =
            [0, 1, 2].map(|calls| run_entry(&mut device, "ENTRY_CRC", calls).tstates);
        let chunk = two - one;
        let init = device.verify_all(0, ASKED_UNBOUNDED).2;
        let setup = none - init;
        let whole = run_crc(&mut device);
        assert_eq!(whole.overall(), 0);
        assert_eq!(whole.statuses.len(), chunks(full));
        assert_eq!(whole.value, sum(&bytes));
        println!(
            "{speed:?}: begin {setup} T-states, a full chunk {chunk} ({:.1} T-states per byte), \
             the {full}-byte library {} T-states",
            f64::from(chunk) / CHUNK as f64,
            whole.tstates
        );
        results.push(chunk);
    }
    let six = f64::from(results[1]) / 6_000_000.0;
    println!("a chunk takes {six:.3} s at 6 MHz");
    assert!(six < 0.5, "{six} s");
}
