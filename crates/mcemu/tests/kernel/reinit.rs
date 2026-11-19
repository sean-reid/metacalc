use crate::support::*;
use mcemu::FLASH_TEST_PAGE;
use mcpack::library::{KernelError, Limits};
use mcpack::mm::Database;

const CODE_BASE: u16 = 0x4000;
const SUBST_LEN: usize = 2048;
const TABLE_ENTRY: usize = 8;

fn init_only(device: &mut Device) -> u8 {
    device.params(ASKED_UNBOUNDED);
    device.run("ENTRY_IFF");
    device.byte("TEST_INIT_STATUS")
}

fn verify_only(device: &mut Device, id: usize) -> u8 {
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.run("ENTRY_VERIFY");
    device.byte("TEST_VERIFY_STATUS")
}

fn table(device: &mut Device) -> Vec<u8> {
    device.kram(device.label("SUBST"), SUBST_LEN)
}

fn serial(device: &mut Device) -> u16 {
    let bytes = device.kram_label("KV_SERIAL", 2);
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// The table and serial a fresh device holds after proving `id` once.
fn after_one_proof(image: Image, bytes: &[u8], id: usize) -> (Vec<u8>, u16) {
    let mut fresh = Device::with_image_library(image, bytes);
    assert_eq!(fresh.verify(id, ASKED_UNBOUNDED).init, 0);
    (table(&mut fresh), serial(&mut fresh))
}

#[test]
fn init_leaves_no_binding_from_an_earlier_run_in_the_substitution_table() {
    let (db, bytes) = small_library();
    let (both, good) = (number(&db, "both"), number(&db, "good"));
    let want = model_one(&bytes, good, ASKED_UNBOUNDED);
    for (layout, image) in LAYOUTS {
        let (fresh_table, fresh_serial) = after_one_proof(image, &bytes, good);
        let mut device = Device::with_image_library(image, &bytes);
        assert_eq!(device.verify(both, ASKED_UNBOUNDED).verify, status(Ok(())));
        assert_ne!(
            table(&mut device),
            vec![0; SUBST_LEN],
            "{layout}: the first proof leaves bindings"
        );
        assert_eq!(init_only(&mut device), 0, "{layout}");
        assert_eq!(
            table(&mut device),
            vec![0; SUBST_LEN],
            "{layout}: bindings after init"
        );
        assert_eq!(verify_only(&mut device, good), want, "{layout}");
        assert_eq!(table(&mut device), fresh_table, "{layout}: table");
        assert_eq!(serial(&mut device), fresh_serial, "{layout}: serial");
        let z = 5 * TABLE_ENTRY;
        assert_eq!(table(&mut device)[z..z + TABLE_ENTRY], [0; TABLE_ENTRY]);
    }
}

#[test]
fn init_restarts_the_substitution_serial_wherever_the_last_run_left_it() {
    let (db, bytes) = small_library();
    let both = number(&db, "both");
    let good = both;
    let want = model_one(&bytes, good, ASKED_UNBOUNDED);
    for (layout, image) in LAYOUTS {
        let (fresh_table, fresh_serial) = after_one_proof(image, &bytes, good);
        assert!(fresh_serial >= 2, "{layout}: the proof applies twice");
        let mut device = Device::with_image_library(image, &bytes);
        assert_eq!(device.verify(both, ASKED_UNBOUNDED).verify, status(Ok(())));
        let near_wrap = 0u16.wrapping_sub(fresh_serial);
        device.kram_write(device.label("KV_SERIAL"), &near_wrap.to_le_bytes());
        assert_eq!(init_only(&mut device), 0, "{layout}");
        assert_eq!(serial(&mut device), 0, "{layout}: serial after init");
        assert_eq!(verify_only(&mut device, good), want, "{layout}");
        assert_eq!(serial(&mut device), fresh_serial, "{layout}: serial");
        assert_eq!(table(&mut device), fresh_table, "{layout}: table");
    }
}

#[test]
fn a_run_that_follows_init_agrees_with_the_model_on_every_assertion_whatever_ran_before() {
    let (db, bytes) = small_library();
    let count = assertions(&db).len();
    let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, &bytes);
        let (_, first, _) = device.verify_all(count, ASKED_UNBOUNDED);
        assert_eq!(first, want, "{layout}: first run");
        device.kram_write(device.label("KV_SERIAL"), &0xFFF0u16.to_le_bytes());
        let (init, second, _) = device.verify_all(count, ASKED_UNBOUNDED);
        assert_eq!((init, second), (want_init, want.clone()), "{layout}");
    }
}

fn twice() -> (Database, Vec<u8>) {
    let db = parse(
        "$c ( ) -> wff $.
         $v p q $.
         wp $f wff p $.
         wq $f wff q $.
         wi $a wff ( p -> q ) $.
         twice $p wff ( ( p -> p ) -> ( p -> p ) ) $= ( wi ) AABZCB $.",
    );
    let bytes = pack_all(&db);
    (db, bytes)
}

#[test]
fn verify_reads_the_page_and_limits_init_validated_and_not_the_block_it_copied_from() {
    let (db, bytes) = twice();
    let id = number(&db, "twice");
    let limits = |saved_syms| Limits {
        saved_syms,
        saved_entries: 1,
        ..ASKED_UNBOUNDED
    };
    let (ok, oom) = (status(Ok(())), status(Err(KernelError::OutOfMemory)));
    assert_eq!(model_one(&bytes, id, limits(6)), ok);
    assert_eq!(model_one(&bytes, id, limits(5)), oom);
    for (layout, image) in LAYOUTS {
        let mut device = Device::with_image_library(image, &bytes);
        assert_eq!(device.verify(id, limits(6)).verify, ok, "{layout}");
        let copy = device.label("KERNEL_LIB_PAGE");
        assert_eq!(device.kram(copy, 9), device.param_block(limits(6)));

        device.kram_write(copy, &device.param_block(limits(5)));
        device.kram_write(copy, &[0x20]);
        assert_eq!(verify_only(&mut device, id), ok, "{layout}: the copy");

        let limit = device.label("KV_SAVED_SYM_LIMIT");
        assert_eq!(device.kram(limit, 2), [6, 0], "{layout}: validated limit");
        device.kram_write(limit, &[5, 0]);
        assert_eq!(verify_only(&mut device, id), oom, "{layout}: limit 5");
        device.kram_write(limit, &[6, 0]);
        assert_eq!(verify_only(&mut device, id), ok, "{layout}: limit 6");

        let page = device.label("KV_LIB_PAGE");
        assert_eq!(device.kram(page, 1), [device.lib_page]);
        device.kram_write(page, &[0x20]);
        assert_ne!(verify_only(&mut device, id), ok, "{layout}: other page");
        device.kram_write(page, &[device.lib_page]);
        assert_eq!(verify_only(&mut device, id), ok, "{layout}: page back");
    }
}

#[test]
fn a_library_wrong_in_any_one_magic_byte_is_refused() {
    let (db, bytes) = small_library();
    let good = number(&db, "good");
    for at in 0..4 {
        let mut bad = bytes.clone();
        bad[at] ^= 0x01;
        reseal(&mut bad);
        for (layout, image) in LAYOUTS {
            let model = mcpack::library::Kernel::new(&bad, clamped(ASKED_UNBOUNDED)).map(|_| ());
            assert_eq!(model, Err(KernelError::Malformed), "magic byte {at}");
            let got = Device::with_image_library(image, &bad).verify(good, ASKED_UNBOUNDED);
            let refused = status(Err(KernelError::Malformed));
            assert_eq!(got.init, refused, "{layout}: init, magic byte {at}");
            assert_eq!(got.verify, refused, "{layout}: verify, magic byte {at}");
        }
    }
}

const THEOREMS: usize = 6;
const DROPS: usize = 150;
/// The test image's timer ticks every 1953 us, 29,295 T-states at 15 MHz.
const SLICE: u32 = 20_000;
const MAX_SLICES: usize = 20_000;
const PORT_MASK: u8 = 3;
const MASK_ON_AND_TIMER: u8 = 0x0B;
const KEY_2: u8 = 0x1A;
const KBD_SCAN_CODE: usize = 0x043F;
const KBD_KEY: usize = 0x0445;

fn long_proofs() -> (Database, Vec<u8>) {
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
    (db, bytes)
}

struct Outcome {
    init: u8,
    statuses: Vec<u8>,
    work: Vec<u8>,
    stack: usize,
    iff: u8,
    interrupts: usize,
    tstates: u64,
}

fn start(device: &mut Device, count: usize) -> usize {
    device.params(ASKED_UNBOUNDED);
    device
        .m
        .load_ram(device.label("TEST_COUNT"), &(count as u16).to_le_bytes());
    let size = usize::from(device.label("KSTACK_TOP") - device.label("KSTACK"));
    device.kram_write(device.label("KSTACK"), &vec![0xCC; size]);
    size
}

fn finish(device: &mut Device, size: usize, count: usize, tstates: u64, ticks: usize) -> Outcome {
    let used = device.kram(device.label("KSTACK"), size);
    Outcome {
        init: device.byte("TEST_INIT_STATUS"),
        statuses: device.m.read_ram(device.label("TEST_STATUSES"), count),
        work: device.kram_label("KV_WORK", 3),
        stack: size - used.iter().position(|&b| b != 0xCC).unwrap_or(size),
        iff: device.m.regs().iff1,
        interrupts: ticks,
        tstates,
    }
}

fn masked(bytes: &[u8], count: usize) -> Outcome {
    let mut device = Device::with_library(bytes);
    let size = start(&mut device, count);
    let tstates = u64::from(device.run("ENTRY_ALL"));
    finish(&mut device, size, count, tstates, 0)
}

/// Runs the test image's whole-library entry with interrupts on from its first instruction, the
/// timer unmasked and a key held so that each run of the stub's interrupt handler leaves a mark.
/// The final halt becomes a jump to itself, since with interrupts on a halt would run on into
/// the next entry point at the first tick.
fn interrupted(bytes: &[u8], count: usize) -> Outcome {
    let mut device = Device::with_library(bytes);
    let entry = device.label("ENTRY_ALL");
    let at = usize::from(entry - CODE_BASE);
    assert_eq!(device.image.code[at], 0xF3, "the entry starts with di");
    device.m.load_flash(FLASH_TEST_PAGE, at, &[0xFB]);
    let done = device.label("ALL_DONE");
    let end = usize::from(done - CODE_BASE);
    assert_eq!(device.image.code[end], 0x76, "the entry ends with halt");
    device.m.load_flash(FLASH_TEST_PAGE, end, &[0x18, 0xFE]);
    let size = start(&mut device, count);
    device.m.out_port(PORT_MASK, MASK_ON_AND_TIMER);
    device.m.press_key(KEY_2);
    let (mut tstates, mut ticks) = (0, 0);
    let mut run = device.m.run(entry, SLICE);
    for _ in 0..MAX_SLICES {
        tstates += u64::from(run.tstates);
        let page = device.m.ram_page(1);
        if page[KBD_SCAN_CODE] != 0 {
            ticks += 1;
            page[KBD_SCAN_CODE] = 0;
            page[KBD_KEY] = 0;
        }
        if (done..=done + 1).contains(&device.m.pc()) {
            return finish(&mut device, size, count, tstates, ticks);
        }
        run = device.m.resume(SLICE);
    }
    panic!(
        "no end after {MAX_SLICES} slices, at pc {:04x}",
        device.m.pc()
    );
}

#[test]
fn interrupts_taken_around_the_kernel_calls_change_neither_a_verdict_nor_the_work_nor_the_stack() {
    let (db, bytes) = long_proofs();
    let count = assertions(&db).len();
    let (want_init, want) = model_statuses(&bytes, ASKED_UNBOUNDED);
    let quiet = masked(&bytes, count);
    let busy = interrupted(&bytes, count);
    assert_eq!((quiet.init, &quiet.statuses), (want_init, &want));
    assert_eq!((busy.init, &busy.statuses), (want_init, &want), "statuses");
    assert_eq!(busy.work, quiet.work, "work count");
    assert_ne!(busy.work, [0; 3], "the last proof charged work");
    assert_eq!(busy.stack, quiet.stack, "kernel stack used");
    assert_eq!(
        (quiet.iff, busy.iff),
        (0, 1),
        "interrupts as they were at entry"
    );
    let periods = busy.tstates / 29_295;
    assert!(periods >= 20, "the run spans {periods} timer periods");
    assert!(
        busy.interrupts >= THEOREMS,
        "{} interrupts taken in {periods} timer periods",
        busy.interrupts
    );
}
