use crate::isolation::{Allowed, pattern, prime, stray};
use crate::proofs::*;
use crate::support::*;
use mcpack::library::decode;

pub const PROOF: &str = "LIB_PROOF";
pub const MARK: u8 = 0xE5;
/// The statuses of the header, written out here and not read from the image.
pub const ST_NONE: u8 = 9;
/// Bytes of stack the header says the deepest exit of lib_proof uses below its return address.
pub const STACK_DEPTH: usize = 8;

pub fn proof_regs(n: u16) -> Regs {
    Regs {
        hl: n,
        ix: 0x1357,
        iy: 0x2468,
        a: 0x5A,
        bc: 0x7777,
        de: 0x6666,
    }
}

/// The block lib_proof fills, as the header lays it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub page: u8,
    pub addr: u16,
    pub refs: u32,
    pub nrefs: u16,
    pub proof: u32,
    pub plen: u16,
}

impl Rig {
    pub fn block_at(&self) -> u16 {
        self.label("LP_PAGE")
    }

    pub fn seed_block(&mut self) {
        let at = self.block_at();
        self.poke(at, &[MARK; 13]);
        let base = self.label("LR_DCBASE");
        self.poke(base, &[0xFF, 0xFF]);
    }

    pub fn block_bytes(&mut self) -> Vec<u8> {
        let at = self.block_at();
        self.peek(at, 13)
    }

    pub fn block(&mut self) -> Block {
        let b = self.block_bytes();
        let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let t = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], 0]);
        Block {
            page: b[0],
            addr: w(1),
            refs: t(3),
            nrefs: w(6),
            proof: t(8),
            plen: w(11),
        }
    }

    pub fn proof(&mut self, n: u16) -> Out {
        self.call(PROOF, proof_regs(n))
    }
}

/// What the block must hold for assertion `n` of `bytes`, from the format.
pub fn expected(bytes: &[u8], n: usize, lib_page: u8) -> Block {
    let l = layout(bytes, n);
    assert!(l.theorem);
    let chunk = (l.record >> 14) as u8;
    assert_eq!(chunk, ((l.proof + l.plen - 1) >> 14) as u8, "one chunk");
    Block {
        page: lib_page - chunk,
        addr: 0x8000 | (l.record & 0x3FFF) as u16,
        refs: (l.refs_count + 2) as u32,
        nrefs: l.nrefs as u16,
        proof: l.proof as u32,
        plen: l.plen as u16,
    }
}

/// More statements than one chunk holds, in three shapes of theorem.
pub fn multi_source(count: usize) -> String {
    let mut src = String::from(BASE);
    let h = |l: &'static str, s: &'static str| (l, s);
    for i in 0..count {
        let pad = "x".repeat(i % 17);
        src += &match i % 3 {
            0 => theorem(
                &format!("one{i}{pad}"),
                &[h("h1", "|- p")],
                "|- p",
                &[],
                &[Num(1)],
            ),
            1 => theorem(
                &format!("use{i}{pad}"),
                &[h("u1", "|- p")],
                "|- ( p -> p )",
                &["tri"],
                &[Num(0), Num(1), Save, Num(3), Num(3), Num(2)],
            ),
            _ => theorem(
                &format!("mp{i}{pad}"),
                &[
                    h("m1", "|- p"),
                    h("m2", "|- ( p -> q )"),
                    h("m3", "|- ( q -> r )"),
                ],
                "|- r",
                &["ax-mp"],
                &[
                    Num(1),
                    Num(2),
                    Num(0),
                    Num(1),
                    Num(3),
                    Num(4),
                    Num(6),
                    Num(5),
                    Num(6),
                ],
            ),
        };
    }
    src
}

/// Every assertion of `bytes` through lib_proof against the format and the decoder. Returns
/// (theorems, axioms).
pub fn sweep(bytes: &[u8], what: &str, every: usize) -> (usize, usize) {
    let library = decode(bytes).expect("it decodes");
    let mut rig = Rig::with_library(bytes);
    rig.open_ok();
    let (mut theorems, mut axioms) = (0, 0);
    for (n, a) in library.assertions.iter().enumerate() {
        if !a.theorem && n % every != 0 && n + 1 != library.assertions.len() {
            continue;
        }
        rig.seed_block();
        let out = rig.proof(n as u16);
        if a.theorem {
            assert_eq!(out.status(), ST_OK, "{what}: {n}");
            let got = rig.block();
            assert_eq!(got, expected(bytes, n, rig.lib_page), "{what}: {n}");
            assert_eq!(
                usize::from(got.nrefs),
                a.references.len(),
                "{what}: {n}: the decoder's reference count"
            );
            let stored = &bytes[got.proof as usize..got.proof as usize + usize::from(got.plen)];
            assert_eq!(
                stored,
                encode(&a.proof),
                "{what}: {n}: the bytes at the proof's offset are the decoder's proof"
            );
            let l = layout(bytes, n);
            assert_eq!(l.nhyps, a.hypotheses.len(), "{what}: {n}");
            assert_eq!(
                rig.state_word("LR_DCBASE", 2),
                0xFFFF,
                "{what}: {n}: the cache is dropped"
            );
            assert_eq!(rig.state_word("LR_DN", 2), n as u32, "{what}: {n}");
            theorems += 1;
        } else {
            assert_eq!(out.status(), ST_NONE, "{what}: {n}");
            axioms += 1;
        }
        assert_eq!(out.af & 0x40 != 0, out.status() == 0, "{what}: Z flag");
    }
    (theorems, axioms)
}

#[test]
fn every_assertion_of_the_fixtures_gives_the_offsets_the_format_and_the_decoder_give() {
    let mut theorems = 0;
    let mut axioms = 0;
    let mut libraries: Vec<(String, Vec<u8>, usize)> = vec![
        ("fixture".into(), fixture(), 1),
        ("multi".into(), packed(&multi_source(2400)), 7),
        ("big".into(), big_library(), 211),
    ];
    for name in ["anatomy.mm", "demo0.mm", "miu.mm", "big-unifier.mm"] {
        libraries.push((name.into(), pack_all(&corpus(name)), 1));
    }
    for (name, bytes, every) in &libraries {
        let (t, a) = sweep(bytes, name, *every);
        println!("{name}: {t} theorems and {a} axioms");
        theorems += t;
        axioms += a;
    }
    assert!(
        libraries[1].1.len() > 4 * CHUNK,
        "the multi library spans chunks"
    );
    assert!(theorems >= 2400 + 8, "{theorems} theorems");
    assert!(axioms >= 30, "{axioms} axioms");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_give_the_offsets_the_format_gives() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let (t, a) = sweep(&bytes, name, 1);
        println!("{name}: {t} theorems and {a} axioms");
        let want = match name {
            "prop" => (756, 0),
            "pred" => (1_125, 0),
            _ => (3_049, 0),
        };
        assert_eq!((t, a != 0), (want.0, true), "{name}");
    }
}

#[test]
fn a_number_past_the_count_and_a_closed_reader_are_refused_with_the_outputs_untouched() {
    let bytes = fixture();
    let count = decode(&bytes).unwrap().assertions.len() as u16;
    let mut rig = Rig::with_library(&bytes);
    rig.seed_block();
    let closed = rig.proof(0);
    assert_eq!(closed.status(), ST_CLOSED);
    assert_eq!(rig.block_bytes(), [MARK; 13], "closed");
    rig.open_ok();
    for n in [count, count + 1, 0xFFFF] {
        rig.seed_block();
        let out = rig.proof(n);
        assert_eq!(out.status(), ST_INDEX, "{n}");
        assert_eq!(rig.block_bytes(), [MARK; 13], "{n}");
        assert_eq!(rig.state_word("LR_DN", 2), 0xFFFF, "{n}: no descriptor");
    }
    rig.seed_block();
    assert_eq!(rig.proof(count - 1).status(), ST_OK, "the last assertion");
    let n = number_of(&bytes, "ax-mp");
    assert_eq!(rig.proof(n).status(), ST_NONE, "an axiom");
}

#[test]
fn a_proof_that_ends_at_the_library_or_the_chunk_is_read_and_one_byte_more_is_refused() {
    let base = kernel_only(&packed_source_for_edges());
    let last = decode(&base).unwrap().assertions.len() - 1;
    let l = layout(&base, last);
    assert!(l.theorem);
    let room = base.len() - l.proof;
    for (what, plen, status) in [
        ("up to the library's last byte", room, ST_OK),
        ("one past it", room + 1, ST_BAD),
        ("far past it", 60_000, ST_BAD),
        ("empty", 0, ST_OK),
    ] {
        let bytes = edited(&base, |b| {
            b[l.proof - 2..l.proof].copy_from_slice(&(plen as u16).to_le_bytes());
        });
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        rig.seed_block();
        let out = rig.proof(last as u16);
        assert_eq!(out.status(), status, "{what}");
        if status == ST_OK {
            assert_eq!(usize::from(rig.block().plen), plen, "{what}");
        }
    }
    let multi = kernel_only(&multi_source(2400));
    let library = decode(&multi).unwrap();
    assert!(multi.len() > 3 * CHUNK);
    let in_first = (0..library.assertions.len())
        .filter(|&n| {
            let l = layout(&multi, n);
            l.theorem && l.proof + l.plen <= CHUNK
        })
        .next_back()
        .expect("a theorem ends in the first chunk");
    let l = layout(&multi, in_first);
    assert!(l.record >> 14 == 0 && CHUNK - (l.proof + l.plen) < 100);
    let fits = CHUNK - l.proof;
    for (what, plen, status) in [
        ("ending on the chunk's last byte", fits, ST_OK),
        ("one byte into the next chunk", fits + 1, ST_BAD),
        ("a whole chunk long", 0xFFFF, ST_BAD),
    ] {
        let bytes = edited(&multi, |b| {
            b[l.proof - 2..l.proof].copy_from_slice(&(plen as u16).to_le_bytes());
        });
        let mut rig = Rig::with_library(&bytes);
        rig.open_ok();
        rig.seed_block();
        let out = rig.proof(in_first as u16);
        assert_eq!(out.status(), status, "{what}");
        if status == ST_OK {
            let got = rig.block();
            assert_eq!(usize::from(got.plen), plen);
            assert_eq!(got.page, rig.lib_page);
        }
    }
}

fn packed_source_for_edges() -> String {
    fixture_source()
}

/// The five outcomes of lib_proof on the fixture: ok, an axiom, a number past the count, a
/// library that is not open and a record whose proof runs past the library.
fn outcomes() -> Vec<(&'static str, Vec<u8>, bool, u16, u8)> {
    let good = fixture();
    let count = decode(&good).unwrap().assertions.len() as u16;
    let cut = kernel_only(&fixture_source());
    let last = decode(&cut).unwrap().assertions.len() - 1;
    let l = layout(&cut, last);
    let broken = edited(&cut, |b| {
        b[l.proof - 2..l.proof].copy_from_slice(&60_000u16.to_le_bytes())
    });
    vec![
        (
            "theorem",
            good.clone(),
            true,
            number_of(&good, "mp2"),
            ST_OK,
        ),
        (
            "axiom",
            good.clone(),
            true,
            number_of(&good, "ax-1"),
            ST_NONE,
        ),
        ("past the count", good.clone(), true, count, ST_INDEX),
        ("closed", good, false, 0, ST_CLOSED),
        ("cut", broken, true, last as u16, ST_BAD),
    ]
}

fn run_outcome(rig: &mut Rig, open: bool, bytes: &[u8]) {
    rig.load(bytes);
    if open {
        rig.open_ok();
    }
}

#[test]
fn ports_the_interrupt_state_and_the_index_registers_come_back_at_every_exit() {
    let mut seen = 0;
    for (port5, port7, ei) in [
        (0u8, 0x81u8, false),
        (2, 0x81, true),
        (7, 0x84, true),
        (4, 0x87, false),
    ] {
        for (what, bytes, open, n, status) in outcomes() {
            let mut rig = Rig::new();
            rig.m.out_port(5, port5);
            rig.m.out_port(7, port7);
            rig.ei = ei;
            run_outcome(&mut rig, open, &bytes);
            let out = rig.proof(n);
            let what = format!("{what} at {port5}/{port7:#x}/{ei}");
            assert_eq!(out.status(), status, "{what}");
            assert_eq!(
                (out.ix, out.iy),
                (0x1357, 0x2468),
                "{what}: index registers"
            );
            assert_eq!((out.port5, out.port7), (port5, port7), "{what}: ports");
            assert_eq!(out.iff, u8::from(ei), "{what}: interrupt flag");
            seen += 1;
        }
    }
    assert_eq!(seen, 20);
}

#[test]
fn the_shadow_registers_are_touched_only_by_a_read_and_only_with_interrupts_off() {
    let seed = Shadows {
        af: 0x1122,
        bc: 0x3344,
        de: 0x5566,
        hl: 0x7788,
    };
    for (what, bytes, open, n, status) in outcomes() {
        let mut rig = Rig::new();
        run_outcome(&mut rig, open, &bytes);
        let (out, shadows) = rig.call_shadowed(PROOF, proof_regs(n), seed);
        assert_eq!(out.status(), status, "{what}");
        let reads = matches!(status, ST_OK | ST_NONE | ST_BAD);
        assert_eq!(shadows != seed, reads, "{what}: {shadows:x?}");
    }
}

fn stack_used(bytes: &[u8], open: bool, n: u16) -> usize {
    let mut most = 0;
    for flip in [0u8, 0xFF] {
        let mut rig = Rig::new();
        run_outcome(&mut rig, open, bytes);
        for (at, byte) in rig.m.ram_page(OS_PAGE).iter_mut().enumerate() {
            *byte = pattern(OS_PAGE, at) ^ flip;
        }
        rig.proof(n);
        let page = rig.m.ram_page(OS_PAGE).to_vec();
        let lowest = (0..page.len())
            .find(|&at| page[at] != pattern(OS_PAGE, at) ^ flip)
            .unwrap();
        most = most.max(page.len() - lowest);
    }
    most
}

#[test]
fn the_stack_goes_exactly_as_deep_as_the_header_says_on_each_exit() {
    let mut table = Vec::new();
    for (what, bytes, open, n, _) in outcomes() {
        table.push((what, stack_used(&bytes, open, n)));
    }
    println!("{table:?}");
    let deepest = table.iter().map(|&(_, used)| used).max().unwrap();
    assert_eq!(deepest, 2 + STACK_DEPTH);
    for (what, used) in &table {
        let want = match *what {
            "closed" => 2 + 4,
            "past the count" => 2 + 4,
            _ => 2 + STACK_DEPTH,
        };
        assert_eq!(*used, want, "{what}");
    }
}

#[test]
fn it_writes_only_the_state_the_bounce_buffer_and_the_callers_bytes() {
    let bytes = packed(&multi_source(2400));
    let mut rig = Rig::new();
    rig.m.out_port(5, 0);
    prime(&mut rig);
    rig.load(&bytes);
    rig.open_ok();
    let state = rig.label("LIB_STATE");
    let size = usize::from(rig.label("LIB_STATE_SIZE"));
    let facts = |rig: &mut Rig| {
        let mut all = rig.peek(state + 0x10, 0x10);
        all.extend(rig.peek(state + 0xA8, size - 0xA8));
        all
    };
    let before = facts(&mut rig);
    let count = decode(&bytes).unwrap().assertions.len();
    let mut ran = 0;
    for n in (0..count).step_by(37).chain([0, count - 1]) {
        let out = rig.proof(n as u16);
        assert!(matches!(out.status(), ST_OK | ST_NONE), "{n}");
        ran += 1;
    }
    assert_eq!(rig.proof(count as u16).status(), ST_INDEX);
    let found = stray(&mut rig, &Allowed::of(DEFAULT));
    assert!(found.is_empty(), "{found:x?}");
    assert_eq!(
        facts(&mut rig),
        before,
        "the facts and the sections are kept"
    );
    assert!(ran > 60);
}

#[test]
fn it_keeps_the_descriptor_and_the_window_and_drops_the_cache_only_when_it_fills_the_block() {
    let bytes = fixture();
    let mut rig = Rig::with_library(&bytes);
    rig.open_ok();
    let n = number_of(&bytes, "mp2");
    rig.hypothesis(n, 3);
    let held = |rig: &mut Rig| {
        (
            rig.state_word("LR_DN", 2),
            rig.state_word("LR_DREC", 3),
            rig.state_word("LR_DNHYPS", 1),
            rig.state_word("LR_DCONC", 3),
            rig.state_word("LR_DCLEN", 1),
        )
    };
    let before = held(&mut rig);
    assert_eq!(before.0, u32::from(n));
    rig.label_of(0);
    assert_ne!(rig.state_word("LR_DCBASE", 2), 0xFFFF, "a cache is held");
    rig.hypothesis(n, 3);
    assert_eq!(rig.proof(n).status(), ST_OK);
    assert_eq!(held(&mut rig), before, "the descriptor is kept");
    assert_eq!(rig.state_word("LR_DCBASE", 2), 0xFFFF);
    let (status, symbols, kind) = rig.hypothesis(n, 3);
    assert_eq!(
        (status, kind),
        (ST_OK, 0xFF),
        "the next call still reads the record"
    );
    assert!(!symbols.is_empty());
}

#[test]
fn an_interrupt_at_any_point_changes_nothing() {
    let bytes = fixture();
    let n = number_of(&bytes, "mp2");
    let mut rig = Rig::with_library(&bytes);
    rig.install_handler();
    rig.ei = true;
    rig.open_ok();
    let bounce = usize::from(rig.label("LIB_BOUNCE") - 0xC000);
    for (at, byte) in rig.m.ram_page(SPARE_PAGE).iter_mut().enumerate() {
        *byte = pattern(SPARE_PAGE, at);
    }
    let quiet = rig.proof(n);
    assert_eq!(quiet.status(), ST_OK);
    let block = rig.block();
    let (mut fired, mut after) = (0, 1u32);
    loop {
        rig.seed_block();
        let (out, taken) = rig.call_interrupted(PROOF, proof_regs(n), after);
        fired += usize::from(taken > 0);
        assert_eq!(out.status(), ST_OK, "after {after}");
        assert_eq!(rig.block(), block, "after {after}");
        let page = rig.m.ram_page(SPARE_PAGE).to_vec();
        let stray: Vec<usize> = (0..page.len())
            .filter(|&at| {
                page[at] != pattern(SPARE_PAGE, at) && !(bounce..bounce + 256).contains(&at)
            })
            .collect();
        assert!(stray.is_empty(), "after {after}: {stray:x?}");
        if taken == 0 && after > 50 {
            break;
        }
        after += 53;
    }
    println!("{fired} interrupts landed inside lib_proof");
    assert!(fired > 8, "{fired} interrupts landed");
}
