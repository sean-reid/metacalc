use crate::stubos::{TIMER_PERIOD_15MHZ, kbd_key, kbd_scan_code};
use crate::support::*;
use mcemu::FLASH_TEST_PAGE;
use mcpack::library::{KernelError, Limits};
use mcpack::mm::Database;
use std::collections::BTreeSet;

const CODE_BASE: u16 = 0x4000;

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
    let len = usize::from(device.label("STACK_BUF") - device.label("SUBST"));
    device.kram(device.label("SUBST"), len)
}

fn table_len(device: &mut Device) -> usize {
    table(device).len()
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
            vec![0; table_len(&mut device)],
            "{layout}: the first proof leaves bindings"
        );
        assert_eq!(init_only(&mut device), 0, "{layout}");
        assert_eq!(
            table(&mut device),
            vec![0; table_len(&mut device)],
            "{layout}: bindings after init"
        );
        assert_eq!(verify_only(&mut device, good), want, "{layout}");
        assert_eq!(table(&mut device), fresh_table, "{layout}: table");
        assert_eq!(serial(&mut device), fresh_serial, "{layout}: serial");
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
        assert_eq!(
            device.kram(copy, device.param_block(limits(6)).len()),
            device.param_block(limits(6))
        );

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
        assert_eq!(
            verify_only(&mut device, id),
            status(Err(KernelError::Malformed)),
            "{layout}: other page"
        );
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
const SLICE: u32 = 20_000;
const _: () = assert!(
    SLICE < TIMER_PERIOD_15MHZ,
    "a slice is shorter than a timer period"
);
const MAX_SLICES: usize = 20_000;
const PORT_MASK: u8 = 3;
const MASK_ON_AND_TIMER: u8 = 0x0B;
const KEY_2: u8 = 0x1A;

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
        if page[kbd_scan_code()] != 0 {
            ticks += 1;
            page[kbd_scan_code()] = 0;
            page[kbd_key()] = 0;
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
    let periods = busy.tstates / u64::from(TIMER_PERIOD_15MHZ);
    assert!(periods >= 20, "the run spans {periods} timer periods");
    assert!(
        busy.interrupts >= THEOREMS,
        "{} interrupts taken in {periods} timer periods",
        busy.interrupts
    );
}

const SYNC_CODE: u16 = 0x9000;
const SLED: u16 = 0x9010;
const SLED_LEN: usize = 8000;
const STEP: usize = 5;
const IRQ_VECTOR: u16 = 0x38;
const STEPS_IN_WINDOW: usize = 150;
const LD_A_I: [u8; 2] = [0xED, 0x57];
const LD_A_I_LEN: u16 = LD_A_I.len() as u16;
const JP_PE: u8 = 0xEA;
const FLAG_PV: u16 = 0x04;

struct Call {
    status: u8,
    work: Vec<u8>,
    stack: usize,
}

fn assert_kernel_enter_opens_with_ld_a_i_and_jp_pe(device: &Device) {
    let enter = usize::from(device.label("KERNEL_ENTER") - CODE_BASE);
    let code = &device.image.code;
    assert_eq!(
        code[enter..enter + 2],
        LD_A_I,
        "kernel_enter starts with ld a,i"
    );
    assert_eq!(code[enter + 2], JP_PE, "then jp pe");
}

fn call(device: &mut Device, size: usize) -> Call {
    let used = device.kram(device.label("KSTACK"), size);
    Call {
        status: device.byte("TEST_VERIFY_STATUS"),
        work: device.kram_label("KV_WORK", 3),
        stack: size - used.iter().position(|&b| b != 0xCC).unwrap_or(size),
    }
}

/// The halt that ends `ENTRY_VERIFY`: `ei`, `ld hl,(nn)`, `call nn`, `ld (nn),a` come first.
fn verify_end(device: &Device) -> u16 {
    device.label("ENTRY_VERIFY") + 10
}

/// Makes `ENTRY_VERIFY` start with `ei` instead of `di`, and end in a jump to itself, since with
/// interrupts on a halt would run on into the next entry point at the first tick.
fn interrupts_on_through_the_entry(device: &mut Device) {
    let entry = device.label("ENTRY_VERIFY");
    let at = usize::from(entry - CODE_BASE);
    assert_eq!(device.image.code[at], 0xF3, "the entry starts with di");
    device.m.load_flash(FLASH_TEST_PAGE, at, &[0xFB]);
    let end = usize::from(verify_end(device) - CODE_BASE);
    assert_eq!(device.image.code[end], 0x76, "the entry ends with halt");
    device.m.load_flash(FLASH_TEST_PAGE, end, &[0x18, 0xFE]);
}

/// Starts `ENTRY_VERIFY` with interrupts on, `burn` T-states after a timer tick, and steps one
/// instruction at a time until `kernel_enter` has turned them off. Returns where the first
/// interrupt was taken from, if it was taken in that stretch.
fn entry_with_tick_at(device: &mut Device, burn: u32) -> Option<u16> {
    let (entry, done) = (device.label("ENTRY_VERIFY"), verify_end(device));
    device.m.run(SYNC_CODE, 2 * TIMER_PERIOD_15MHZ);
    if burn > 0 {
        device.m.run(SLED, burn);
    }
    let last = device.label("KE_SAVE");
    let mut taken = None;
    device.m.run(entry, 1);
    for _ in 0..STEPS_IN_WINDOW {
        if device.m.pc() == IRQ_VECTOR {
            let sp = device.m.regs().sp;
            let ret = device.m.read_ram(sp, 2);
            taken = Some(u16::from_le_bytes([ret[0], ret[1]]));
            break;
        }
        if device.m.pc() > last {
            break;
        }
        device.m.resume(1);
    }
    for _ in 0..MAX_SLICES {
        if (done..=done + 1).contains(&device.m.pc()) {
            return taken;
        }
        device.m.resume(SLICE);
    }
    panic!("no end, at pc {:04x}", device.m.pc());
}

#[test]
fn an_interrupt_taken_inside_kernel_enter_changes_neither_the_verdict_nor_the_work_nor_the_stack() {
    let (db, bytes) = small_library();
    let id = number(&db, "both");
    let mut device = Device::with_library(&bytes);
    assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, status(Ok(())));
    let size = usize::from(device.label("KSTACK_TOP") - device.label("KSTACK"));
    let fill = |device: &mut Device| device.kram_write(device.label("KSTACK"), &vec![0xCC; size]);
    fill(&mut device);
    assert_eq!(verify_only(&mut device, id), status(Ok(())));
    let quiet = call(&mut device, size);
    assert_ne!(quiet.work, [0; 3]);

    interrupts_on_through_the_entry(&mut device);
    device.m.load_ram(SYNC_CODE, &[0xFB, 0x76, 0xF3, 0x76]);
    device.m.load_ram(SLED, &vec![0; SLED_LEN]);
    device.m.out_port(PORT_MASK, MASK_ON_AND_TIMER);
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());

    assert_kernel_enter_opens_with_ld_a_i_and_jp_pe(&device);
    let (first, second) = (
        device.label("KERNEL_ENTER"),
        device.label("KERNEL_ENTER") + LD_A_I_LEN,
    );
    let window = first..=device.label("KE_SAVE");
    let mut boundaries = BTreeSet::from([first, second, device.label("KE_ON"), *window.end()]);
    let mut inside = BTreeSet::new();
    let mut burn = 0;
    loop {
        assert!(
            burn < TIMER_PERIOD_15MHZ,
            "the tick never moved ahead of kernel_enter"
        );
        fill(&mut device);
        let taken = entry_with_tick_at(&mut device, burn);
        let got = call(&mut device, size);
        assert_eq!(got.status, quiet.status, "status, tick {burn} T-states in");
        assert_eq!(got.work, quiet.work, "work, tick {burn} T-states in");
        assert_eq!(got.stack, quiet.stack, "stack, tick {burn} T-states in");
        match taken {
            Some(pc) if window.contains(&pc) => {
                inside.insert(pc);
            }
            Some(pc) if pc < first => break,
            _ => {}
        }
        burn += STEP as u32;
    }
    println!("interrupts taken inside kernel_enter from {inside:x?}");
    boundaries.retain(|pc| !inside.contains(pc));
    assert!(
        boundaries.is_empty(),
        "no interrupt was taken from {boundaries:x?}, only from {inside:x?}"
    );
}

#[test]
fn kernel_enter_reads_the_interrupt_flag_again_when_the_first_read_comes_back_clear() {
    let (db, bytes) = small_library();
    let id = number(&db, "both");
    let mut device = Device::with_library(&bytes);
    assert_eq!(device.verify(id, ASKED_UNBOUNDED).verify, status(Ok(())));
    interrupts_on_through_the_entry(&mut device);
    device.m.out_port(PORT_MASK, 0);
    assert_kernel_enter_opens_with_ld_a_i_and_jp_pe(&device);
    let (first_jp, entry) = (
        device.label("KERNEL_ENTER") + LD_A_I_LEN,
        device.label("ENTRY_VERIFY"),
    );
    device
        .m
        .load_ram(device.label("TEST_ASSERTION"), &(id as u16).to_le_bytes());
    device.m.run(entry, 1);
    for _ in 0..STEPS_IN_WINDOW {
        if device.m.pc() == first_jp {
            break;
        }
        device.m.resume(1);
    }
    assert_eq!(device.m.pc(), first_jp, "the first ld a,i has run");
    let mut regs = device.m.regs();
    assert_eq!(regs.iff1, 1, "interrupts are on");
    assert_eq!(regs.iff2, 1, "iff2 is set");
    assert_ne!(regs.af & FLAG_PV, 0, "the first ld a,i set P/V");
    regs.af &= !FLAG_PV;
    device.m.set_regs(&regs);
    let done = verify_end(&device);
    for _ in 0..MAX_SLICES {
        if (done..=done + 1).contains(&device.m.pc()) {
            break;
        }
        device.m.resume(SLICE);
    }
    assert_eq!(device.byte("TEST_VERIFY_STATUS"), status(Ok(())));
    assert_eq!(device.m.regs().iff1, 1, "interrupts are back on at exit");
}
