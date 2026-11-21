use crate::fixture::{self, patched};
use crate::rig::*;
use crate::support::*;
use crate::ui_support::same_frame;

const BLANK: [u8; FRAME_LEN] = [0; FRAME_LEN];

fn malformed() -> u8 {
    code("ST_MALFORMED")
}

fn owned() -> u16 {
    constant_of(WITH_FM.labels, "FM_OWNED") as u16
}

fn ready(fm: &mut Fm) -> u16 {
    let at = fm.rig.label("KV_READY") - 0xC000;
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    u16::from_le_bytes([page[usize::from(at)], page[usize::from(at) + 1]])
}

/// The assertion whose conclusion is the longest and the statement that makes the draw clear.
fn pick(bytes: &[u8]) -> (u16, u8) {
    let all = statements(&decoded(bytes));
    let (n, k, _) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    (*n, *k)
}

#[test]
fn an_fm_session_after_a_good_kernel_init_makes_the_next_verify_refuse() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(ready(&mut fm), 0x5AA5);
    let good = fm.rig.kernel_verify(0);
    assert_ne!(
        good,
        malformed(),
        "the kernel accepts it before an fm session"
    );
    assert_eq!(fm.rig.kernel_verify(0), good);
    assert_eq!(ready(&mut fm), 0x5AA5);
    fm.rows(94);
    assert_eq!(ready(&mut fm), owned());
    assert_eq!(fm.rig.kernel_verify(0), malformed());
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.rig.kernel_verify(0), good);
}

#[test]
fn every_routine_takes_the_kernels_mark_whatever_it_returns() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    for routine in ["open", "statement", "rows", "draw"] {
        let mut fm = Fm::new(&bytes);
        fm.statement(n, k);
        assert_eq!(fm.rig.kernel_init(), 0, "{routine}");
        match routine {
            "open" => fm.open(),
            "statement" => fm.statement(n, k),
            "rows" => fm.rows(94),
            _ => fm.draw(1, 0, 94, (0, 8)),
        };
        assert_eq!(ready(&mut fm), owned(), "{routine}");
        assert_eq!(fm.rig.kernel_verify(0), malformed(), "{routine}");
    }
    let mut fm = Fm::bare(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    fm.statement(0, 0xFF);
    assert_eq!(ready(&mut fm), owned(), "a refused call too");
}

#[test]
fn after_a_kernel_init_fm_is_closed_and_draws_nothing_until_fm_open_runs_again() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    let before = fm.rows(94);
    assert_eq!(before.status(), 0);
    assert_eq!(fm.rig.kernel_init(), 0);
    let closed = code("FM_ERR_NOT_OPEN");
    for out in [fm.rows(94), fm.rows(94)] {
        assert_eq!((out.status(), out.bc >> 8), (closed, 0));
    }
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), closed);
    assert_eq!(fm.frame(), BLANK, "a closed fm draws nothing");
    assert_eq!(fm.statement(*n, *k).status(), closed);
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.rows(94).status(), 0, "open leaves no statement");
    assert_eq!(fm.rows(94).bc >> 8, 0);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    assert_eq!(fm.rows(94).bc, before.bc);
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    let want = model_frame(tokens, 94, 1, 0, 0..8);
    same_frame("after reopening", &fm.frame(), &want);
}

#[test]
fn fm_stays_open_across_sessions_while_the_kernel_is_not_initialised() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let (n, k, tokens) = all.iter().max_by_key(|(_, _, t)| t.len()).unwrap();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(*n, *k).status(), 0);
    for _ in 0..3 {
        assert_eq!(fm.rows(94).status(), 0);
    }
    assert_eq!(fm.draw(1, 0, 94, (0, 8)).status(), 0);
    same_frame(
        "a later session",
        &fm.frame(),
        &model_frame(tokens, 94, 1, 0, 0..8),
    );
}

#[test]
fn a_kernel_reset_between_two_sessions_closes_fm() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(fm.rows(94).status(), 0);
    assert_eq!(fm.rig.call("KERNEL_RESET", Regs::default()).status(), 0);
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.rows(94).status(), 0);
}

#[test]
fn a_good_init_a_verify_and_a_refused_init_leave_fm_closed() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_ne!(fm.rig.kernel_verify(0), malformed());
    fm.rig.poke(KERNEL_PARAMS, &[0x90, 0, 0, 0, 0, 0, 0, 0, 0]);
    let regs = Regs {
        hl: KERNEL_PARAMS,
        ..Regs::default()
    };
    assert_eq!(fm.rig.call("KERNEL_INIT", regs).status(), malformed());
    assert_eq!(ready(&mut fm), 0, "a refused init leaves the mark clear");
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    assert_eq!(fm.open().status(), 0);
    assert_eq!(fm.statement(n, k).status(), 0);
}

fn high(fm: &mut Fm) -> Vec<u8> {
    let start = usize::from(fm.rig.label("KERNEL_RAM_HIGH") - 0xC000);
    let size = usize::from(fm.rig.label("HIGH_SIZE"));
    fm.rig.m.ram_page(SPARE_PAGE)[start..start + size].to_vec()
}

#[test]
fn a_refused_kernel_init_writes_nothing_in_the_high_region_whichever_check_refuses() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let first = |fm: &mut Fm| {
        fm.rig.poke(KERNEL_PARAMS, &[0x90, 0, 0, 0, 0, 0, 0, 0, 0]);
        let regs = Regs {
            hl: KERNEL_PARAMS,
            ..Regs::default()
        };
        fm.rig.call("KERNEL_INIT", regs).status()
    };
    let last = |fm: &mut Fm| {
        let twin = patched(|b| {
            let at = b
                .windows(6)
                .position(|w| w == [0xFF, 1, b'(', 0xFF, 1, b')'])
                .unwrap();
            b[at + 5] = b'(';
        });
        fm.rig.load(&twin);
        fm.rig.kernel_init()
    };
    for (what, refuse, status) in [
        (
            "the first check",
            &first as &dyn Fn(&mut Fm) -> u8,
            "ST_MALFORMED",
        ),
        ("the last check", &last, "ST_BAD_SYMBOL"),
    ] {
        let mut fm = Fm::new(&bytes);
        assert_eq!(fm.statement(n, k).status(), 0);
        let before = high(&mut fm);
        assert_eq!(refuse(&mut fm), code(status), "{what}");
        assert_eq!(high(&mut fm), before, "{what}: the high region");
    }
}

#[test]
fn an_fm_session_between_a_crc_begin_and_its_chunk_ends_the_run() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.rig.call("KERNEL_CRC_BEGIN", Regs::default()).status(), 0);
    let alone = fm.rig.call("KERNEL_CRC_CHUNK", Regs::default()).status();
    assert_eq!(alone, 0, "the run ends on a match without fm");
    assert_eq!(fm.rig.call("KERNEL_CRC_BEGIN", Regs::default()).status(), 0);
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    let chunk = fm.rig.call("KERNEL_CRC_CHUNK", Regs::default()).status();
    assert_eq!(chunk, malformed());
}

#[test]
fn the_status_numbers_are_the_ones_the_header_lists() {
    for (name, number) in STATUSES {
        assert_eq!(
            constant_of(WITH_FM.labels, name),
            u32::from(number),
            "{name}"
        );
    }
}

#[test]
fn a_verify_run_over_the_whole_library_closes_fm_and_a_new_open_repairs_it() {
    let bytes = fixture::library();
    let all = statements(&decoded(&bytes));
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    for id in 0..decoded(&bytes).assertions.len() {
        fm.rig.kernel_verify(id);
    }
    assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"));
    assert_eq!(fm.open().status(), 0);
    for (n, k, tokens) in all.iter().step_by(5) {
        assert_eq!(fm.statement(*n, *k).status(), 0);
        fm.clear();
        assert_eq!(fm.draw(3, 0, 92, (0, 8)).status(), 0);
        same_frame(
            "after a verify run",
            &fm.frame(),
            &model_frame(tokens, 92, 3, 0, 0..8),
        );
    }
}

#[test]
fn only_the_whole_word_fm_owned_keeps_fm_open() {
    let bytes = fixture::library();
    let (n, k) = pick(&bytes);
    let word = owned();
    for (what, mark) in [
        ("the high byte alone", (word & 0xFF00) | 0x11),
        ("the low byte alone", 0x1100 | (word & 0x00FF)),
        ("the kernel's magic", 0x5AA5),
    ] {
        let mut fm = Fm::new(&bytes);
        assert_eq!(fm.statement(n, k).status(), 0);
        let at = usize::from(fm.rig.label("KV_READY") - 0xC000);
        fm.rig.m.ram_page(SPARE_PAGE)[at..at + 2].copy_from_slice(&mark.to_le_bytes());
        assert_eq!(fm.rows(94).status(), code("FM_ERR_NOT_OPEN"), "{what}");
        assert_eq!(ready(&mut fm), word, "{what}: the session takes the mark");
    }
}

/// What a kernel run on the loaded library says: the init status, the verdict of every proof, the
/// CRC run's begin status, each chunk's status and the CRC the last chunk returns.
#[derive(Debug, PartialEq)]
pub(crate) struct KernelRun {
    pub(crate) init: u8,
    /// The typecode table and the part of the check cache kernel_init clears, read back after it.
    tables: Vec<u8>,
    pub(crate) verdicts: Vec<u8>,
    begin: u8,
    chunks: Vec<u8>,
    crc: u32,
}

fn init_tables(rig: &mut Rig, assertions: usize) -> Vec<u8> {
    let at = |name: &str| usize::from(rig.label(name) - SPARE_BASE);
    let ids = usize::from(rig.label("CACHE_IDS"));
    let cleared = assertions.min(ids).div_ceil(8);
    let (typecodes, cache) = (at("TYPECODES"), at("CACHE"));
    let page = rig.m.ram_page(SPARE_PAGE);
    let mut out = page[typecodes..typecodes + 256].to_vec();
    out.extend_from_slice(&page[cache..cache + cleared]);
    out
}

pub(crate) fn kernel_run(rig: &mut Rig, assertions: usize) -> KernelRun {
    let init = rig.kernel_init();
    let tables = init_tables(rig, assertions);
    let verdicts = (0..assertions).map(|id| rig.kernel_verify(id)).collect();
    let begin = rig.call("KERNEL_CRC_BEGIN", Regs::default()).status();
    let mut chunks = Vec::new();
    let crc = loop {
        let out = rig.call("KERNEL_CRC_CHUNK", Regs::default());
        chunks.push(out.status());
        if out.status() != 0xFF {
            break (u32::from(out.de) << 16) | u32::from(out.hl);
        }
    };
    KernelRun {
        init,
        tables,
        verdicts,
        begin,
        chunks,
        crc,
    }
}

pub(crate) fn clean_run(bytes: &[u8]) -> KernelRun {
    let mut rig = Rig::with_image(WITH_FM);
    rig.load(bytes);
    rig.open_ok();
    let run = kernel_run(&mut rig, decoded(bytes).assertions.len());
    assert_eq!(run.init, 0);
    assert_eq!(run.chunks.last(), Some(&0), "the CRC matches");
    run
}

/// The assertion with the most tokens in all, which fills the most of fm's memory.
pub(crate) fn fullest(bytes: &[u8]) -> u16 {
    let mut total = std::collections::BTreeMap::new();
    for (n, _, tokens) in statements(&decoded(bytes)) {
        *total.entry(n).or_insert(0) += tokens.len();
    }
    total.into_iter().max_by_key(|&(_, t)| t).unwrap().0
}

pub(crate) fn scribble(fm: &mut Fm, region: &str, size: &str, byte: impl Fn(usize) -> u8) {
    let at = usize::from(fm.rig.label(region) - SPARE_BASE);
    let len = usize::from(fm.rig.label(size));
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    for (i, b) in page[at..at + len].iter_mut().enumerate() {
        *b = byte(i);
    }
}

fn the_kernel_after_fm(bytes: &[u8]) {
    let assertions = decoded(bytes).assertions.len();
    let want = clean_run(bytes);
    let seeded = |i: usize| (i.wrapping_mul(2_654_435_761) >> 13) as u8;
    let patterns: [(&str, &dyn Fn(usize) -> u8); 3] =
        [("FF", &|_| 0xFF), ("00", &|_| 0), ("seeded", &seeded)];
    for (what, byte) in patterns {
        let mut fm = Fm::new(bytes);
        assert_eq!(fm.assertion(fullest(bytes)).status(), 0);
        scribble(&mut fm, "FM_LOW", "FM_LOW_SIZE", byte);
        scribble(&mut fm, "FM_RAM", "FM_RAM_SIZE", byte);
        let got = kernel_run(&mut fm.rig, assertions);
        assert_eq!(got, want, "after fm and a write of {what}");
    }
}

#[test]
fn the_kernel_gives_the_same_verdicts_and_crc_after_fm_filled_its_regions_and_a_test_overwrote_them()
 {
    let bytes = fixture::library();
    assert!(decoded(&bytes).assertions.len() > 10);
    the_kernel_after_fm(&bytes);
}

#[test]
fn the_kernel_gives_the_same_verdicts_and_crc_after_fm_on_a_library_with_proofs_to_check() {
    let bytes = fixture::proofs();
    let labels: Vec<String> = decoded(&bytes)
        .assertions
        .iter()
        .map(|a| a.label.clone())
        .collect();
    let verdicts = clean_run(&bytes).verdicts;
    for (label, verdict) in labels.iter().zip(&verdicts) {
        assert_eq!(*verdict == 0, label != "bad", "{label}: {verdict}");
    }
    assert!(labels.len() > 50, "{} assertions", labels.len());
    the_kernel_after_fm(&bytes);
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib>"]
fn the_kernel_gives_the_same_verdicts_and_crc_on_prop_after_fm_and_a_write_over_its_regions() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/prop.mclib")).expect("library reads");
    the_kernel_after_fm(&bytes);
}

type Entry<'a> = (&'a str, &'a dyn Fn(&mut Fm));

#[test]
fn after_any_kernel_entry_that_may_write_the_regions_fm_assertion_and_fm_select_are_closed() {
    let bytes = fixture::library();
    let n = fullest(&bytes);
    let closed = code("FM_ERR_NOT_OPEN");
    let entries: [Entry; 4] = [
        ("a good init", &|fm| assert_eq!(fm.rig.kernel_init(), 0)),
        ("a refused init", &|fm| {
            fm.rig.poke(KERNEL_PARAMS, &[0x90, 0, 0, 0, 0, 0, 0, 0, 0]);
            let regs = Regs {
                hl: KERNEL_PARAMS,
                ..Regs::default()
            };
            assert_eq!(fm.rig.call("KERNEL_INIT", regs).status(), malformed());
        }),
        ("a reset", &|fm| {
            assert_eq!(fm.rig.call("KERNEL_RESET", Regs::default()).status(), 0);
        }),
        ("an init and a verify", &|fm| {
            assert_eq!(fm.rig.kernel_init(), 0);
            fm.rig.kernel_verify(0);
        }),
    ];
    for (what, entry) in entries {
        let mut fm = Fm::new(&bytes);
        assert_eq!(fm.assertion(n).status(), 0, "{what}");
        entry(&mut fm);
        let out = fm.assertion(n);
        assert_eq!((out.status(), count(out)), (closed, 0), "{what}: assertion");
        let out = fm.select(0xFF);
        assert_eq!((out.status(), count(out)), (closed, 0), "{what}: select");
        assert_eq!(fm.open().status(), 0, "{what}: open");
        assert_eq!(fm.assertion(n).status(), 0, "{what}: assertion after open");
        assert_eq!(fm.select(0xFF).status(), 0, "{what}: select after open");
    }
}

#[test]
fn a_verify_or_crc_call_the_kernel_refuses_while_fm_owns_the_mark_writes_only_its_stack_page() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.assertion(fullest(&bytes)).status(), 0);
    let (stack, top) = (fm.rig.label("KSTACK"), fm.rig.label("KSTACK_TOP"));
    let (scratch, size) = (fm.rig.label("FM_TCNT"), fm.rig.label("FM_TREE_SIZE"));
    assert!(
        scratch <= stack && top <= scratch + size,
        "the kernel's stack page is inside FM_TCNT, which holds nothing between fm calls"
    );
    let regions = |fm: &mut Fm| {
        let mut all = Vec::new();
        for (region, size) in [("FM_LOW", "FM_LOW_SIZE"), ("FM_RAM", "FM_RAM_SIZE")] {
            let at = usize::from(fm.rig.label(region) - SPARE_BASE);
            let len = usize::from(fm.rig.label(size));
            let mut bytes = fm.rig.m.ram_page(SPARE_PAGE)[at..at + len].to_vec();
            let (low, high) = (stack, top);
            for addr in low..high {
                let i = usize::from(addr - SPARE_BASE);
                if (at..at + len).contains(&i) {
                    bytes[i - at] = 0;
                }
            }
            all.extend(bytes);
        }
        all
    };
    let before = regions(&mut fm);
    assert_eq!(fm.rig.kernel_verify(0), malformed());
    for entry in ["KERNEL_CRC_BEGIN", "KERNEL_CRC_CHUNK"] {
        assert_eq!(fm.rig.call(entry, Regs::default()).status(), malformed());
    }
    assert!(
        regions(&mut fm) == before,
        "a refused kernel call wrote outside its stack"
    );
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    let at = usize::from(top - SPARE_BASE);
    assert!(
        page[at - 8..at].iter().any(|&b| b != 0),
        "the stack page was used"
    );
    assert_eq!(fm.select(0xFF).status(), 0, "fm is still open");
    let out = fm.nodes();
    assert_eq!(out.status(), 0, "and its headers are intact");
}

#[test]
fn an_fm_call_takes_the_reference_list_the_last_verify_left() {
    let bytes = fixture::library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(fm.rig.kernel_init(), 0);
    assert_eq!(fm.rig.kernel_verify(0), 0);
    let held = fm.rig.call("KERNEL_REFS", Regs::default());
    assert_eq!((held.status(), held.bc), (0, 0), "an axiom's list, held");
    fm.rows(94);
    assert_eq!(ready(&mut fm), owned());
    let gone = fm.rig.call("KERNEL_REFS", Regs::default());
    assert_eq!(
        (gone.status(), gone.de, gone.hl),
        (code("ST_NO_REFS"), 0, 0)
    );
}
