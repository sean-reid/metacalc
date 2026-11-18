use crate::isolation::{expected, offset, pattern, primed, run_one, stray_writes};
use crate::limits::copies_src;
use crate::support::*;
use mcpack::library::KernelError;

/// Bytes either side of the two regions, which belong to TI-OS.
const EDGES: [u16; 8] = [
    0xC000, 0xC4FF, 0xD700, 0xDEFF, 0xFF00, 0xFF01, 0xFFFE, 0xFFFF,
];
const CANARY: u8 = 0xCC;

struct Run {
    status: u8,
    stack_used: usize,
    device: Device,
}

/// Runs init and one verify on a primed spare page and fails on any write outside the regions.
fn run_checked(bytes: &[u8], id: usize, what: &str) -> Run {
    let mut device = primed(DEFAULT, bytes);
    let stack = usize::from(label("KSTACK_TOP") - label("KSTACK"));
    device.kram_write(label("KSTACK"), &vec![CANARY; stack]);
    let (init, status) = run_one(&mut device, id);
    assert_eq!(init, 0, "{what}: init");
    let stray = stray_writes(&mut device, 0, expected());
    assert!(stray.is_empty(), "{what}: {stray:x?}");
    for addr in EDGES {
        assert_eq!(
            device.kram(addr, 1)[0],
            pattern(SPARE_PAGE, offset(addr)),
            "{what}: the byte at {addr:04X}"
        );
    }
    let used = device.kram(label("KSTACK"), stack);
    let deepest = used.iter().position(|&b| b != CANARY).unwrap_or(stack);
    Run {
        status,
        stack_used: stack - deepest,
        device,
    }
}

fn cap(name: &str) -> usize {
    usize::from(label(name))
}

/// The copies theorem of limits.rs, run at its cap and one past it.
fn at_and_past(size: usize, fit: usize, saving: bool, what: &str) -> [Run; 2] {
    [fit, fit + 1].map(|copies| {
        let db = parse(&copies_src(size, copies, saving));
        let bytes = pack_all(&db);
        run_checked(
            &bytes,
            number(&db, "big"),
            &format!("{what}, {copies} copies"),
        )
    })
}

fn assert_ok_then_oom(runs: &[Run; 2], what: &str) {
    assert_eq!(runs[0].status, status(Ok(())), "{what}, at the capacity");
    assert_eq!(
        runs[1].status,
        status(Err(KernelError::OutOfMemory)),
        "{what}, one past it"
    );
}

fn last_byte_written(run: &mut Run, end: u16) {
    let at = end - 1;
    assert_ne!(
        run.device.kram(at, 1)[0],
        pattern(SPARE_PAGE, offset(at)),
        "the last byte of the buffer, {at:04X}, was written"
    );
}

#[test]
fn full_and_overfull_stack_symbols_stay_inside_the_high_region() {
    let size = 96;
    let fit = cap("CAP_STACK_SYMS") / size - 1;
    let runs = at_and_past(size, fit, false, "stack symbols");
    assert_ok_then_oom(&runs, "stack symbols");
    let end = label("STACK_BUF") + cap("CAP_STACK_SYMS") as u16;
    assert_eq!(
        end,
        label("SAVED_BUF"),
        "the buffer ends where the saved symbols begin"
    );
}

#[test]
fn full_and_overfull_saved_symbols_stay_inside_the_high_region() {
    let size = 96;
    let fit = cap("CAP_SAVED_SYMS") / size;
    let mut runs = at_and_past(size, fit, true, "saved symbols");
    assert_ok_then_oom(&runs, "saved symbols");
    let end = label("SAVED_BUF") + cap("CAP_SAVED_SYMS") as u16;
    assert_eq!(end, 0xFF00, "the buffer ends at the first byte TI-OS owns");
    last_byte_written(&mut runs[0], end);
}

#[test]
fn a_full_and_an_overfull_stack_of_entries_stay_inside_the_low_region() {
    let entries = cap("CAP_STACK_ENTRIES");
    let mut runs = at_and_past(2, entries, false, "stack entries");
    assert_ok_then_oom(&runs, "stack entries");
    let end = label("STARTS_BUF") + 2 * entries as u16;
    assert_eq!(end, label("SAVED_STARTS_BUF"));
    last_byte_written(&mut runs[0], end);
}

#[test]
fn a_full_and_an_overfull_list_of_saved_entries_stay_inside_the_low_region() {
    let entries = cap("CAP_SAVED_ENTRIES");
    let mut runs = at_and_past(2, entries, true, "saved entries");
    assert_ok_then_oom(&runs, "saved entries");
    let end = label("SAVED_STARTS_BUF") + 2 * entries as u16;
    assert_eq!(end, label("CRC_TABLE"));
    last_byte_written(&mut runs[0], end);
}

/// A library of `count` axioms and a theorem that uses the two axioms with ids `first` and `first + 1`.
fn cache_library(count: usize, first: usize) -> (mcpack::mm::Database, Vec<u8>) {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    for i in 0..count {
        src += &format!("n{i} $a wff T $.\n");
    }
    src += &format!("tc $p wff T $= n{} n{} drop $.\n", first - 1, first);
    let db = parse(&src);
    let bytes = pack_all(&db);
    (db, bytes)
}

#[test]
fn the_check_cache_filled_to_its_last_bit_stays_inside_the_low_region() {
    assert_eq!(cap("CACHE_IDS"), 8192);
    let cache = label("CACHE");
    assert_eq!(cache + 0x400, label("STARTS_BUF"));
    // Ids 8191 and 8192: the first is the last bit the cache holds, the second is past it.
    let (db, bytes) = cache_library(8300, 8191);
    let id = number(&db, "tc");
    let mut run = run_checked(&bytes, id, "cache edge");
    assert_eq!(run.status, status(Ok(())));
    let last = run.device.kram(cache + 0x3FF, 1)[0];
    assert_eq!(last & 0x80, 0x80, "the bit for id 8191 is set");
    // Ids 2 and 3 land in the first byte.
    let (db, bytes) = cache_library(8300, 2);
    let mut run = run_checked(&bytes, number(&db, "tc"), "cache start");
    assert_eq!(run.status, status(Ok(())));
    assert_eq!(run.device.kram(cache, 1)[0] & 0x0C, 0x0C);
}

/// A theorem that applies an axiom of `vars` mandatory variables to each variable in turn, so the
/// substitution table gets an entry for every variable.
fn subst_library(vars: usize) -> (mcpack::mm::Database, Vec<u8>) {
    let names: Vec<String> = (0..vars).map(|i| format!("v{i}")).collect();
    let mut src = format!("$c wff |- $. $v {} $.\n", names.join(" "));
    for n in &names {
        src += &format!("f{n} $f wff {n} $.\n");
    }
    src += &format!("ax $a |- {} $.\n", names.join(" "));
    let proof: Vec<String> = names.iter().map(|n| format!("f{n}")).collect();
    src += &format!("t $p |- {} $= {} ax $.\n", names.join(" "), proof.join(" "));
    let db = parse(&src);
    let bytes = pack_all(&db);
    (db, bytes)
}

#[test]
fn a_full_substitution_table_stays_inside_the_high_region() {
    let vars = 253;
    let (db, bytes) = subst_library(vars);
    let mut run = run_checked(&bytes, number(&db, "t"), "substitution table");
    assert_eq!(run.status, status(Ok(())));
    let table = label("SUBST");
    let last = run.device.kram(table + 254 * 8, 8);
    assert_ne!(
        last, [0; 8],
        "the entry of the highest variable id is bound"
    );
    assert_eq!(table + 0x800, label("STACK_BUF"));
}

#[test]
fn the_private_stack_never_comes_near_its_end() {
    let mut deepest = 0;
    for name in ["big-unifier.mm", "demo0.mm", "miu.mm", "anatomy.mm"] {
        let db = corpus(name);
        let bytes = pack_all(&db);
        for local in 0..assertions(&db).len() {
            let run = run_checked(&bytes, local, name);
            deepest = deepest.max(run.stack_used);
        }
    }
    let (db, bytes) = subst_library(253);
    deepest = deepest.max(run_checked(&bytes, number(&db, "t"), "subst").stack_used);
    println!("deepest private stack use: {deepest} of 256 bytes");
    assert!(deepest <= 128, "{deepest} bytes of the private stack");
}
