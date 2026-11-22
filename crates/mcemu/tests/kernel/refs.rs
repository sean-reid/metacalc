use crate::oracle::{bytes_of, stored};
use crate::references::cases;
use crate::support::*;
use crate::work::press_on_during;
use mcemu::{Machine, Stop};

const SCRIPT: u16 = 0x9000;
const OUT: u16 = 0x9800;
const BATCH: usize = 500;

#[derive(Clone, Copy)]
enum Op {
    Init,
    Reset,
    Verify(usize),
    Refs,
}

/// What one op left: A, BC, and DE:HL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    status: u8,
    bc: u16,
    de: u16,
    hl: u16,
}

impl Slot {
    /// The 24-bit library offset in E:HL, with D as the call returns it.
    fn offset(&self) -> usize {
        assert_eq!(self.de >> 8, 0, "D is 0");
        usize::from(self.de) << 16 | usize::from(self.hl)
    }
}

fn run_script(device: &mut Device, ops: &[Op]) -> Vec<Slot> {
    run_script_timed(device, ops).0
}

fn run_script_timed(device: &mut Device, ops: &[Op]) -> (Vec<Slot>, u32) {
    let mut bytes = Vec::new();
    for op in ops {
        match *op {
            Op::Init => bytes.push(0),
            Op::Reset => bytes.push(1),
            Op::Verify(id) => bytes.extend([2, id as u8, (id >> 8) as u8]),
            Op::Refs => bytes.push(3),
        }
    }
    bytes.push(255);
    device.params(ASKED_UNBOUNDED);
    device.m.load_ram(SCRIPT, &bytes);
    let tstates = device.run("ENTRY_SCRIPT");
    (read_slots(device, ops.len()), tstates)
}

fn read_slots(device: &mut Device, n: usize) -> Vec<Slot> {
    let out = device.m.read_ram(OUT, 8 * n);
    out.chunks(8)
        .map(|s| Slot {
            status: s[0],
            bc: u16::from_le_bytes([s[1], s[2]]),
            de: u16::from_le_bytes([s[3], s[4]]),
            hl: u16::from_le_bytes([s[5], s[6]]),
        })
        .collect()
}

fn st(name: &str) -> u8 {
    label(name) as u8
}

/// Verifies and asks for the list of each of `ids`, after one init; returns the verdict and the
/// refs slot of each, and compares the list with the one mcpack's reader gives.
fn check_ids(
    device: &mut Device,
    bytes: &[u8],
    ids: &[usize],
    want: &[Vec<u16>],
) -> (usize, usize) {
    let mut ops = vec![Op::Init];
    for &id in ids {
        ops.extend([Op::Verify(id), Op::Refs]);
    }
    let slots = run_script(device, &ops);
    assert_eq!(slots[0].status, 0, "init");
    let (mut lists, mut entries) = (0, 0);
    for (n, &id) in ids.iter().enumerate() {
        let refs = slots[2 + 2 * n];
        assert_eq!(refs.status, 0, "assertion {id}: kernel_refs");
        let count = usize::from(refs.bc);
        assert_eq!(count, want[id].len(), "assertion {id}: count");
        let at = refs.offset();
        let got = &bytes[at..at + 2 * count];
        assert_eq!(
            got,
            bytes_of(&want[id]),
            "assertion {id}: entries at {at:#x}"
        );
        lists += 1;
        entries += count;
    }
    (lists, entries)
}

#[test]
fn kernel_refs_gives_each_assertions_list_as_mcpacks_reader_has_it() {
    let (db, bytes) = small_library();
    let want = stored(&bytes);
    let ids: Vec<usize> = (0..assertions(&db).len()).collect();
    let mut device = Device::with_library(&bytes);
    let (lists, entries) = check_ids(&mut device, &bytes, &ids, &want);
    let total: usize = want.iter().map(Vec::len).sum();
    println!(
        "fixture: {lists} lists, {entries} entries, library totals {} and {total}",
        want.len()
    );
    assert_eq!((lists, entries), (want.len(), total));
    assert!(
        want.iter().any(|l| l.iter().any(|&e| e & 0x8000 != 0)),
        "a dummy entry"
    );
    assert!(want.iter().any(Vec::is_empty), "an axiom");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn kernel_refs_gives_every_list_of_the_shipped_libraries_as_mcpacks_reader_has_it() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let want = stored(&bytes);
        let mut device = Device::with_library(&bytes);
        let (mut lists, mut entries) = (0, 0);
        let ids: Vec<usize> = (0..want.len()).collect();
        for batch in ids.chunks(BATCH) {
            let (l, e) = check_ids(&mut device, &bytes, batch, &want);
            lists += l;
            entries += e;
        }
        let theorems = want.iter().filter(|l| !l.is_empty()).count();
        let assertion_entries = want.iter().flatten().filter(|&&e| e & 0x8000 == 0).count();
        println!(
            "{name}: {lists} assertions, {theorems} with a list, {entries} entries, {assertion_entries} of them assertions"
        );
        assert_eq!(lists, want.len(), "{name}");
        assert_eq!(entries, want.iter().map(Vec::len).sum::<usize>(), "{name}");
    }
}

/// The record's first byte, which is a kind of 0 or 1.
fn damage(bytes: &[u8], id: usize) -> Vec<u8> {
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let record = u32_at(u32_at(16) + 4 * id) & 0xFF_FFFF;
    let mut damaged = bytes.to_vec();
    damaged[record] = 2;
    reseal(&mut damaged);
    damaged
}

#[test]
fn kernel_refs_says_there_is_no_list_unless_the_last_verify_located_one() {
    let none = st("ST_NO_REFS");
    assert_eq!(none, 17);
    let (db, bytes) = small_library();
    let want = stored(&bytes);
    let dummy = number(&db, "dummy");
    let both = number(&db, "both");
    let bad = number(&db, "bad");
    let wrong = number(&db, "same");
    assert!(want[dummy].len() > 1 && want[both].len() > 1);
    let count = assertions(&db).len();
    let no_list = |slot: Slot| {
        assert_eq!((slot.status, slot.de, slot.hl), (none, 0, 0), "{slot:?}");
    };
    let located = |slot: Slot, id: usize| {
        assert_eq!(slot.status, 0, "assertion {id}");
        assert_eq!(usize::from(slot.bc), want[id].len(), "assertion {id}");
    };

    let mut device = Device::with_library(&bytes);
    let slots = run_script(&mut device, &[Op::Init, Op::Refs]);
    no_list(slots[1]);

    let slots = run_script(
        &mut device,
        &[Op::Verify(dummy), Op::Refs, Op::Reset, Op::Refs],
    );
    located(slots[1], dummy);
    no_list(slots[3]);

    let slots = run_script(
        &mut device,
        &[Op::Init, Op::Verify(dummy), Op::Refs, Op::Init, Op::Refs],
    );
    located(slots[2], dummy);
    no_list(slots[4]);

    let slots = run_script(
        &mut device,
        &[Op::Verify(dummy), Op::Verify(count), Op::Refs],
    );
    assert_eq!(slots[1].status, st("ST_MALFORMED"));
    no_list(slots[2]);

    let slots = run_script(&mut device, &[Op::Verify(dummy), Op::Verify(bad), Op::Refs]);
    assert_eq!(slots[1].status, st("ST_DISJOINT"));
    located(slots[2], bad);
    let slots = run_script(
        &mut device,
        &[Op::Verify(both), Op::Verify(wrong), Op::Refs],
    );
    assert_eq!(slots[1].status, st("ST_DISJOINT"));
    located(slots[2], wrong);

    let damaged = damage(&bytes, dummy);
    let mut device = Device::with_library(&damaged);
    let slots = run_script(
        &mut device,
        &[
            Op::Init,
            Op::Verify(both),
            Op::Refs,
            Op::Verify(dummy),
            Op::Refs,
        ],
    );
    located(slots[2], both);
    assert_eq!(slots[3].status, st("ST_MALFORMED"));
    no_list(slots[4]);

    let slots = run_script(&mut device, &[Op::Init, Op::Refs, Op::Refs]);
    no_list(slots[1]);
    assert_eq!(slots[1], slots[2]);
}

#[test]
fn an_axiom_has_a_list_of_none_and_a_verdict_does_not_change_the_list() {
    let (db, bytes) = small_library();
    let axiom = number(&db, "ax-m");
    let mut device = Device::with_library(&bytes);
    let slots = run_script(&mut device, &[Op::Init, Op::Verify(axiom), Op::Refs]);
    assert_eq!(slots[1].status, 0);
    assert_eq!(
        (slots[2].status, slots[2].bc, slots[2].de, slots[2].hl),
        (0, 0, 0, 0)
    );
}

#[test]
fn a_verify_stopped_by_the_on_key_leaves_the_list_it_located() {
    let bytes = cases::pack_source(&cases::long_source(511));
    let want = stored(&bytes);
    let mut device = Device::with_library(&bytes);
    press_on_during(&mut device, 511, 2_000_000);
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), st("ST_INTERRUPTED"));
    device.m.release_key(Machine::KEY_ON);
    let slots = run_script(&mut device, &[Op::Refs, Op::Refs]);
    assert_eq!(slots[0], slots[1]);
    assert_eq!(slots[0].status, 0);
    assert_eq!(usize::from(slots[0].bc), 511);
    let at = slots[0].offset();
    assert_eq!(bytes[at..at + 1022], bytes_of(&want[511]));
}

#[test]
fn kernel_refs_changes_only_the_session_save_area_and_its_return_address_and_nothing_else() {
    let (db, bytes) = small_library();
    let id = number(&db, "dummy");
    let mut device = Device::with_library(&bytes);
    run_script(&mut device, &[Op::Init, Op::Verify(id)]);
    fill_outside(device.m.ram_page(SPARE_PAGE), DEFAULT, 0x5A);
    let before = device.kram(0xC000, 0x4000);
    let first = run_script(&mut device, &[Op::Refs]);
    let second = run_script(&mut device, &[Op::Refs]);
    assert_eq!(first, second, "the list is the same on a second call");
    assert_eq!(first[0].status, 0);
    let after = device.kram(0xC000, 0x4000);
    let low = usize::from(label("KERNEL_RAM_LOW") - 0xC000);
    let top = usize::from(label("KSTACK_TOP") - 0xC000);
    let allowed = [low + 0x10..low + 0x15, top - 4..top];
    let changed: Vec<usize> = (0..before.len())
        .filter(|&at| before[at] != after[at] && !allowed.iter().any(|r| r.contains(&at)))
        .collect();
    assert!(changed.is_empty(), "kernel_refs wrote at {changed:x?}");
}

/// The three figures the header gives for kernel_refs, in the order it gives them.
fn header_figures() -> [u32; 4] {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/kernel.z80");
    let text = std::fs::read_to_string(path).unwrap();
    let text: Vec<&str> = text
        .lines()
        .map(|l| l.trim_start_matches(';').trim())
        .collect();
    let text = text.join(" ");
    let at = text
        .find("kernel_refs costs ")
        .expect("the header gives the cost");
    let rest = &text[at..text.find(", entry and exit").unwrap()];
    let numbers: Vec<u32> = rest
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .collect();
    numbers.try_into().expect("four figures")
}

#[test]
fn kernel_refs_costs_what_the_header_says_and_the_ceiling_allows() {
    const CEILING: u32 = 780;
    let bytes = cases::pack_source(&cases::long_source(79));
    let mut device = Device::with_library(&bytes);
    run_script(&mut device, &[Op::Init, Op::Verify(79)]);
    let call = device.run("ENTRY_CALL_ONLY");
    let refs = device.run("ENTRY_REFS_ONLY") - call;
    run_script(&mut device, &[Op::Verify(0)]);
    let axiom = device.run("ENTRY_REFS_ONLY") - call;
    run_script(&mut device, &[Op::Init]);
    let unlisted = device.run("ENTRY_REFS_ONLY") - call;
    run_script(&mut device, &[Op::Reset]);
    let none = device.run("ENTRY_REFS_ONLY") - call;
    println!(
        "kernel_refs: {refs} with a list, {axiom} after an axiom, {unlisted} with the mark set and no list, {none} with the mark clear"
    );
    assert_eq!([refs, axiom, unlisted, none], header_figures());
    assert!(axiom < refs && unlisted < axiom && none < unlisted);
    assert!(
        refs <= CEILING,
        "{refs} T-states, over the ceiling of {CEILING}"
    );
}

const KV_READY: &str = "KV_READY";

#[test]
fn kernel_refs_needs_the_ready_mark_and_a_flag_of_exactly_one() {
    let none = st("ST_NO_REFS");
    let (db, bytes) = small_library();
    let id = number(&db, "dummy");
    let want = stored(&bytes);
    let magic = constant("READY_MAGIC") as u16;
    let no_list =
        |slot: Slot| assert_eq!((slot.status, slot.de, slot.hl), (none, 0, 0), "{slot:?}");

    for fill in [0xFFu8, 0x00, 0x01] {
        let mut device = Device::with_library(&bytes);
        device.m.ram_page(SPARE_PAGE).fill(fill);
        no_list(run_script(&mut device, &[Op::Refs])[0]);
    }
    let mut device = Device::with_library(&bytes);
    for (at, byte) in device.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
        *byte = (at.wrapping_mul(31) ^ (at >> 8).wrapping_mul(7)) as u8 | 1;
    }
    device.kram_write(device.label("KV_LISTED"), &[1]);
    device.kram_write(device.label(KV_READY), &[0xA5, 0x00]);
    no_list(run_script(&mut device, &[Op::Refs])[0]);

    let mut device = Device::with_library(&bytes);
    let slots = run_script(&mut device, &[Op::Init, Op::Verify(id), Op::Refs]);
    assert_eq!(usize::from(slots[2].bc), want[id].len());
    for fm_value in [0x1234u16, 0, 0x5AA4, 0xA55A] {
        device.kram_write(device.label(KV_READY), &fm_value.to_le_bytes());
        no_list(run_script(&mut device, &[Op::Refs])[0]);
    }
    device.kram_write(device.label(KV_READY), &magic.to_le_bytes());
    let slots = run_script(&mut device, &[Op::Refs]);
    assert_eq!(
        usize::from(slots[0].bc),
        want[id].len(),
        "the mark alone hid the list"
    );
    for flag in [0u8, 2, 0xFF, 0x81] {
        device.kram_write(device.label("KV_LISTED"), &[flag]);
        no_list(run_script(&mut device, &[Op::Refs])[0]);
    }
}

#[test]
fn a_verify_refused_for_want_of_the_ready_mark_leaves_no_list() {
    let (db, bytes) = small_library();
    let (first, second) = (number(&db, "dummy"), number(&db, "both"));
    let magic = constant("READY_MAGIC") as u16;
    let mut device = Device::with_library(&bytes);
    run_script(&mut device, &[Op::Init, Op::Verify(first)]);
    device.kram_write(device.label(KV_READY), &[0, 0]);
    let slots = run_script(&mut device, &[Op::Verify(second)]);
    assert_eq!(slots[0].status, st("ST_MALFORMED"));
    device.kram_write(device.label(KV_READY), &magic.to_le_bytes());
    let slots = run_script(&mut device, &[Op::Refs]);
    assert_eq!(
        (slots[0].status, slots[0].de, slots[0].hl),
        (st("ST_NO_REFS"), 0, 0)
    );
}

#[test]
fn a_verify_that_returns_malformed_after_the_record_was_parsed_leaves_no_list() {
    let mut bytes = cases::base();
    let spot = cases::spot(&bytes, cases::TH);
    let len = usize::from(u16::from_le_bytes([
        bytes[spot.proof - 2],
        bytes[spot.proof - 1],
    ]));
    bytes[spot.proof + len - 1] = 0xF1;
    reseal(&mut bytes);
    assert_eq!(
        model_one(&bytes, cases::TH, ASKED_UNBOUNDED),
        st("ST_MALFORMED"),
        "a wide step cut off by the end of the proof"
    );
    let want = stored(&cases::base());
    let mut device = Device::with_library(&bytes);
    let slots = run_script(
        &mut device,
        &[
            Op::Init,
            Op::Verify(cases::TH2),
            Op::Refs,
            Op::Verify(cases::TH),
            Op::Refs,
        ],
    );
    assert_eq!(slots[1].status, 0);
    assert_eq!(usize::from(slots[2].bc), want[cases::TH2].len());
    assert_eq!(slots[3].status, st("ST_MALFORMED"));
    assert_eq!(
        (slots[4].status, slots[4].de, slots[4].hl),
        (st("ST_NO_REFS"), 0, 0)
    );
}

#[test]
fn the_on_key_during_kernel_refs_gives_the_list_or_status_14_and_never_a_wrong_count() {
    let (db, bytes) = small_library();
    let id = number(&db, "dummy");
    let want = stored(&bytes);
    let mut device = Device::with_library(&bytes);
    run_script(&mut device, &[Op::Init, Op::Verify(id)]);
    let (clean, total) = run_script_timed(&mut device, &[Op::Refs]);
    let clean = clean[0];
    assert_eq!(usize::from(clean.bc), want[id].len());
    let interrupted = st("ST_INTERRUPTED");
    let (mut lists, mut stops) = (0, 0);
    for tap in 0..=total {
        device.params(ASKED_UNBOUNDED);
        device.m.load_ram(SCRIPT, &[3, 255]);
        let mut run = device.m.run(device.label("ENTRY_SCRIPT"), tap);
        if run.stop == Stop::Limit {
            device.m.press_key(Machine::KEY_ON);
            run = device.m.resume(RUN_LIMIT);
        }
        assert_eq!(run.stop, Stop::Halted, "tap at {tap}");
        device.m.release_key(Machine::KEY_ON);
        let got = read_slots(&mut device, 1)[0];
        if got.status == interrupted {
            stops += 1;
        } else {
            assert_eq!(got, clean, "tap at {tap}");
            lists += 1;
        }
    }
    assert!(lists > 0 && stops > 0, "{lists} lists, {stops} stops");
    println!("{total} T-states swept: {lists} lists, {stops} stops");
    device.m.out_port(3, 0);
    for tap in total.saturating_sub(700)..=total {
        for hold in 1..=30 {
            device.params(ASKED_UNBOUNDED);
            device.m.load_ram(SCRIPT, &[3, 255]);
            let mut run = device.m.run(device.label("ENTRY_SCRIPT"), tap);
            if run.stop == Stop::Limit {
                device.m.press_key(Machine::KEY_ON);
                run = device.m.resume(hold);
                device.m.release_key(Machine::KEY_ON);
                if run.stop == Stop::Limit {
                    run = device.m.resume(RUN_LIMIT);
                }
            }
            assert_eq!(run.stop, Stop::Halted, "tap at {tap} for {hold}");
            let got = read_slots(&mut device, 1)[0];
            assert!(
                got.status == interrupted || got == clean,
                "ON down at {tap} for {hold} T-states: {got:?}"
            );
        }
    }
    device.m.press_key(Machine::KEY_ON);
    let held = run_script(&mut device, &[Op::Refs])[0];
    device.m.release_key(Machine::KEY_ON);
    assert_eq!(held.status, interrupted, "ON held across the call");
}
