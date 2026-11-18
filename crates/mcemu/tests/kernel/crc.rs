use crate::support::*;
use mcemu::Speed;
use mcpack::library::{CHUNK, Kernel, crc32};

const MORE: u8 = 0xFF;
const MALFORMED: u8 = 1;

pub(crate) struct Crc {
    pub(crate) init: u8,
    pub(crate) begin: u8,
    pub(crate) statuses: Vec<u8>,
    pub(crate) value: u32,
    pub(crate) tstates: u32,
}

impl Crc {
    /// The status the app would see: the first refusal, else the last chunk's.
    pub(crate) fn overall(&self) -> u8 {
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
    let value = device.m.read_ram(device.label("TEST_CRC_RESULT"), 4);
    Crc {
        init: device.byte("TEST_INIT_STATUS"),
        begin: device.byte("TEST_CRC_BEGIN"),
        statuses: device.m.read_ram(device.label("TEST_STATUSES"), calls),
        value: u32::from_le_bytes(value.try_into().unwrap()),
        tstates,
    }
}

pub(crate) fn run_crc(device: &mut Device) -> Crc {
    run_entry(device, "ENTRY_CRC", 255)
}

fn chunks(len: usize) -> usize {
    len.div_ceil(CHUNK)
}

fn model_accepts(bytes: &[u8]) -> bool {
    Kernel::new(bytes, clamped(ASKED_UNBOUNDED)).is_ok()
}

/// `base` with deterministic filler appended up to `len` bytes, sealed.
pub(crate) fn padded(base: &[u8], len: usize) -> Vec<u8> {
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
        assert_eq!(run.value, 0, "library {n}: value, wrong sum");
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
        assert_eq!(run.value, 0, "bit {bit}: value");
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
        assert_eq!(run.value, 0, "offset {offset}: value");
    }
}

#[test]
fn the_value_is_right_at_every_edge_of_a_chunk() {
    let (_, small) = small_library();
    let mut lengths = vec![small.len() + 1, 1000];
    for k in 1..=3 {
        lengths.extend([k * CHUNK - 1, k * CHUNK, k * CHUNK + 1]);
    }
    lengths.extend([65_536, 65_537, 131_071, 131_072, 131_073]);
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
    let stale = device.label("KERNEL_CRC");
    device.kram_write(stale, &[0xA5; 4]);
    let run = run_entry(&mut device, "ENTRY_CRC_RAW", 4);
    assert_eq!((run.begin, run.statuses), (MALFORMED, vec![MALFORMED]));
    assert_eq!(run.value, 0);
    assert_eq!(device.m.read_ram(0x8000, marker.len()), marker);

    let mut bad = bytes.clone();
    bad[0] = b'X';
    device.load(&bad);
    device.kram_write(stale, &[0xA5; 4]);
    let run = run_crc(&mut device);
    assert_eq!((run.init, run.begin), (MALFORMED, MALFORMED));
    assert_eq!((run.statuses, run.value), (vec![MALFORMED], 0));
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

fn crc_before(device: &mut Device) -> u32 {
    let raw = device.m.read_ram(device.label("TEST_CRC_BEFORE"), 4);
    u32::from_le_bytes(raw.try_into().unwrap())
}

#[test]
fn a_successful_init_ends_a_run() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let mut device = Device::with_library(&bytes);
    let run = run_entry(&mut device, "ENTRY_CRC_SECOND_INIT", 0);
    assert_eq!(run.init, 0);
    let both = device.m.read_ram(device.label("TEST_STATUSES"), 2);
    assert_eq!(both, [MORE, MALFORMED]);
    assert_ne!(crc_before(&mut device), 0);
    assert_eq!(run.value, 0);
}

#[test]
fn begin_again_restarts_a_run() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let mut device = Device::with_library(&bytes);
    let run = run_entry(&mut device, "ENTRY_CRC_RESTART", 255);
    assert_eq!((run.begin, run.overall(), run.value), (0, 0, sum(&bytes)));
    assert_eq!(run.statuses.len(), 3);
}

#[test]
fn a_chunk_after_a_finished_run_is_refused_and_clears_the_value() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let mut device = Device::with_library(&bytes);
    let run = run_entry(&mut device, "ENTRY_CRC_AGAIN", 255);
    assert_eq!(device.byte("TEST_CRC_EXTRA"), MALFORMED);
    assert_eq!(crc_before(&mut device), sum(&bytes));
    assert_eq!(run.value, 0);
}

#[test]
fn a_failed_init_ends_a_run() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let mut device = Device::with_library(&bytes);
    let run = run_entry(&mut device, "ENTRY_CRC_FAILED_INIT", 0);
    assert_eq!(run.init, MALFORMED);
    let both = device.m.read_ram(device.label("TEST_STATUSES"), 2);
    assert_eq!(both, [MORE, MALFORMED]);
    assert_eq!(run.value, 0);
}

#[test]
fn verify_between_chunks_changes_neither_answer() {
    let (db, small) = small_library();
    let count = assertions(&db).len();
    let bytes = padded(&small, 6 * CHUNK + 77);
    assert!(model_accepts(&bytes));
    let mut device = Device::with_library(&bytes);
    let run = run_entry(&mut device, "ENTRY_CRC_BETWEEN_VERIFIES", 255);
    assert_eq!((run.init, run.begin, run.overall()), (0, 0, 0));
    assert_eq!(run.statuses.len(), 7);
    assert_eq!(run.value, sum(&bytes));
    let verified = device.m.read_ram(device.label("TEST_STATUSES") + 0x80, 7);
    let (_, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    assert_eq!(want.len(), count);
    assert_eq!(verified, want[..7]);
}

#[test]
fn the_checksum_runs_at_another_address() {
    let (_, small) = small_library();
    let bytes = padded(&small, 3 * CHUNK + 1);
    let mut device = Device::with_image(MOVED);
    device.load(&bytes);
    let run = run_crc(&mut device);
    assert_eq!((run.init, run.begin, run.overall()), (0, 0, 0));
    assert_eq!(run.statuses.len(), 4);
    assert_eq!(run.value, sum(&bytes));
    let table = device.label("CRC_TABLE");
    assert_eq!(table, device.label("KERNEL_RAM_LOW") + 0xE00);
    assert_eq!(device.kram(table + 1, 1), [0x96]);
    let mut wrong = bytes.clone();
    wrong[CHUNK + 5] ^= 4;
    device.load(&wrong);
    let run = run_crc(&mut device);
    assert_eq!((run.overall(), run.value), (MALFORMED, 0));
}

fn refused(device: &mut Device, interrupts: u8) {
    device
        .m
        .load_ram(device.label("TEST_IFF_IN"), &[interrupts]);
    let run = run_entry(device, "ENTRY_CRC_REFUSED_IFF", 0);
    assert_eq!(run.begin, MALFORMED, "begin");
    assert_eq!(device.byte("TEST_CRC_EXTRA"), MALFORMED, "chunk");
    assert_eq!(run.value, 0, "value");
    let after_begin = device.m.read_ram(device.label("TEST_IFF"), 2);
    assert_eq!(after_begin[1], interrupts, "interrupts after begin");
    assert_eq!(after_begin[0], interrupts, "interrupts after chunk");
}

#[test]
fn a_refused_call_restores_everything_whatever_the_page_held() {
    let marker = *b"flash page marker";
    let low = label("KERNEL_RAM_LOW");
    let saved = usize::from(low - 0xC000) + 0x10..usize::from(low - 0xC000) + 0x15;
    let crc_at = usize::from(label("KERNEL_CRC") - 0xC000);
    let stack = usize::from(label("KSTACK") - 0xC000)..usize::from(label("KSTACK_TOP") - 0xC000);
    let mut state = 0x9E37_79B9u32;
    let random: Vec<u8> = (0..PAGE)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 9) as u8
        })
        .collect();
    let ready = usize::from(label("KERNEL_RAM_LOW") + 0xA0 - 0xC000);
    for pattern in [vec![0xFF; PAGE], vec![0x01; PAGE], random] {
        assert_ne!(pattern[ready..ready + 2], [0xA5, 0x5A]);
        for page in [RAM_PAGE, 0x50] {
            for interrupts in [0u8, 1] {
                let mut device = Device::new();
                device.m.load_flash(0x50, 0, &marker);
                device.m.load_ram(0x8000, &[0xA5; 64]);
                device.m.out_port(PORT_BANK_B, page);
                device.m.ram_page(SPARE_PAGE).copy_from_slice(&pattern);
                refused(&mut device, interrupts);
                let bank = device.m.read_ram(0x8000, marker.len());
                let expect = if page == RAM_PAGE {
                    vec![0xA5; marker.len()]
                } else {
                    marker.to_vec()
                };
                assert_eq!(bank, expect, "bank B from page {page:#x}");
                let now = device.m.ram_page(SPARE_PAGE).to_vec();
                for at in (0..PAGE).filter(|at| !stack.contains(at) && !saved.contains(at)) {
                    let crc = (crc_at..crc_at + 4).contains(&at);
                    assert!(crc || now[at] == pattern[at], "page byte {at:#x} changed");
                }
            }
        }
    }
}

const REGISTERS: [u8; 10] = [0x57, 0x13, 0x68, 0x24, 0x9C, 0x36, 0xAE, 0x48, 0xC0, 0x5A];

#[test]
fn the_checksum_restores_registers_the_bank_the_interrupts_and_all_other_memory() {
    let (_, small) = small_library();
    let bytes = padded(&small, 2 * CHUNK + 100);
    let marker = *b"flash page marker";
    let stack = usize::from(label("KSTACK_TOP") - label("KSTACK"));
    for page in [RAM_PAGE, 0x50] {
        for interrupts in [0u8, 1] {
            let mut device = Device::with_library(&bytes);
            device.m.load_flash(0x50, 0, &marker);
            device.m.load_ram(0x8000, &vec![0xA5; PAGE]);
            fill_outside(device.m.ram_page(SPARE_PAGE), DEFAULT, 0x5A);
            device.kram_write(label("KSTACK"), &vec![0xCC; stack]);
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
            let outside = device.m.ram_page(SPARE_PAGE);
            assert_eq!(first_outside_not(outside, DEFAULT, 0x5A), None);
            let used = device.kram(label("KSTACK"), stack);
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
    let raw = device.kram_label("KERNEL_CRC", 4);
    !u32::from_le_bytes(raw.try_into().unwrap())
}

#[test]
fn the_core_reproduces_the_published_test_vector_and_table() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    let mut device = Device::new();
    assert_eq!(vector(&mut device, b"123456789"), 0xCBF4_3926);
    let table = device.kram_label("CRC_TABLE", 1024);
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

/// The figures in the header of kernel.z80.
const BEGIN_TSTATES: u32 = 307_000;
const CALL_TSTATES: u32 = 2_027_000;
const LIBRARY_TSTATES: u32 = 46_100_000;

#[test]
fn the_documented_costs_hold() {
    let (_, small) = small_library();
    let full = 369_178;
    let bytes = padded(&small, full);
    let three = padded(&small, 3 * CHUNK);
    assert!(model_accepts(&bytes));
    for speed in [Speed::Mhz15, Speed::Mhz6] {
        let mut device = Device::with_library(&three);
        device.m.set_speed(speed);
        let [none, one, two, all] =
            [0, 1, 2, 3].map(|calls| run_entry(&mut device, "ENTRY_CRC", calls).tstates);
        let init = device.verify_all(0, ASKED_UNBOUNDED).2;
        let setup = none - init;
        let longest = (two - one).max(all - two);

        let mut device = Device::with_library(&bytes);
        device.m.set_speed(speed);
        let whole = run_crc(&mut device);
        assert_eq!(whole.overall(), 0);
        assert_eq!(whole.statuses.len(), chunks(full));
        assert_eq!(whole.value, sum(&bytes));
        println!(
            "{speed:?}: begin {setup} T-states, longest chunk {longest} ({:.2} T-states per byte), \
             the {full}-byte library {} T-states",
            f64::from(longest) / CHUNK as f64,
            whole.tstates
        );
        assert!(setup <= BEGIN_TSTATES, "begin {setup}");
        assert!(longest <= CALL_TSTATES, "a chunk call {longest}");
        assert!(
            whole.tstates <= LIBRARY_TSTATES,
            "the library {}",
            whole.tstates
        );
    }
    assert!(f64::from(CALL_TSTATES) / 6_000_000.0 < 0.34);
}
