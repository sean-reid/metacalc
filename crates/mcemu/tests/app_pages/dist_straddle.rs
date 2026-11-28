//! The pointer's carry: a pair list that crosses a multiple of 10000h, once inside a 16-pair read and once
//! between two reads. mcpack never lays a record across a 16 KB chunk, so the libraries here move one
//! record back over the zero padding that precedes it, which the reader and the model both accept.

use crate::app::*;
use crate::dist_lib::{Case, all_pairs, record_offset, source, variables};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcui::Key;
use std::sync::OnceLock;

const BOUNDARY: usize = 0x10000;
const LABEL: &str = "dvcarry";

/// The fixture's header and `count` long assertions, then a tuning assertion whose label is `tune` dashes
/// longer than its stem, then the case: the tuning moves the case's record against the boundary a byte at a time.
fn source_with(count: usize, tune: usize, case: &Case) -> String {
    let nested = 63;
    let mut src = String::new();
    for i in 0..count {
        src += &format!(
            "fill{i} $a |- -. {}ps {}$.\n",
            "( ph /\\ ".repeat(nested),
            ") ".repeat(nested)
        );
    }
    src += &format!("tune{} $a |- ph $.\n", "-".repeat(tune));
    src += &source(std::slice::from_ref(case));
    src
}

fn word(lib: &[u8], at: usize) -> usize {
    u32::from_le_bytes(lib[at..at + 4].try_into().unwrap()) as usize
}

/// The size of the axiom record at `start` and the offset of its pair list.
pub fn axiom(lib: &[u8], start: usize) -> (usize, usize) {
    let mut at = start + 2 + usize::from(lib[start + 1]);
    let hyps = usize::from(lib[at]);
    at += 1;
    for _ in 0..hyps {
        at += if lib[at] == 0 {
            2
        } else {
            2 + usize::from(lib[at + 1])
        };
    }
    let pairs = usize::from(u16::from_le_bytes([lib[at], lib[at + 1]]));
    let list = at + 2;
    let end = list + 2 * pairs + 1 + usize::from(lib[list + 2 * pairs]);
    (end - start, list)
}

/// What a library that has a record across the boundary holds.
pub struct Crossing {
    pub library: Vec<u8>,
    pub n: usize,
    pub first: usize,
    pub last: usize,
}

/// The offset after the axiom record `n` of `lib`.
fn end_of(lib: &[u8], n: usize) -> usize {
    let start = record_offset(lib, n) as usize;
    start + axiom(lib, start).0
}

/// A library whose record `LABEL` has its pair list start `back` bytes before the boundary, the case
/// having `pairs`. The packer starts that record on the boundary after zero padding; the tuning record
/// before it is as long as leaves the padding the room the move back takes, and 4 bytes over.
pub fn crossing(back: usize, pairs: Vec<(&'static str, &'static str)>) -> Crossing {
    let case = Case {
        label: LABEL.into(),
        pairs,
    };
    for count in (200..=275).rev() {
        let probe = crate::fixture::custom(&source_with(count, 0, &case));
        let n = word_count(&probe) - 1;
        let start = record_offset(&probe, n) as usize;
        let shift = back + axiom(&probe, start).1 - start;
        let want = BOUNDARY - shift - 4;
        let ends = end_of(&probe, n - 1);
        if ends > want {
            continue;
        }
        let lib = crate::fixture::custom(&source_with(count, want - ends, &case));
        let start = record_offset(&lib, n) as usize;
        if start == BOUNDARY && end_of(&lib, n - 1) == want {
            return patched(lib, n, back);
        }
    }
    panic!("no count of long assertions puts the record against the boundary");
}

fn word_count(lib: &[u8]) -> usize {
    usize::from(u16::from_le_bytes([lib[14], lib[15]]))
}

/// `lib` with record `n`, which starts at the boundary after zero padding, moved back so that its pair
/// list starts `back` bytes before the boundary, with the directory and the checksum made right.
fn patched(mut lib: Vec<u8>, n: usize, back: usize) -> Crossing {
    let start = record_offset(&lib, n) as usize;
    let (len, list) = axiom(&lib, start);
    let new = start - (back + list - start);
    assert!(
        lib[new..start].iter().all(|&b| b == 0),
        "the bytes the record moves over are padding"
    );
    lib.copy_within(start..start + len, new);
    lib[new + len..start + len].fill(0);
    let directory = word(&lib, 16);
    lib[directory + 4 * n..directory + 4 * n + 4].copy_from_slice(&(new as u32).to_le_bytes());
    let crc = mcpack::library::crc32(&lib[8..]);
    lib[4..8].copy_from_slice(&crc.to_le_bytes());
    let (_, first) = axiom(&lib, new);
    let pairs = usize::from(u16::from_le_bytes([lib[first - 2], lib[first - 1]]));
    Crossing {
        library: lib,
        n,
        first,
        last: first + 2 * pairs - 1,
    }
}

fn pairs(count: usize) -> Vec<(&'static str, &'static str)> {
    all_pairs(&variables()[..14])[..count].to_vec()
}

/// The pair list starts 16 bytes before the boundary: the first read of 16 pairs crosses it.
pub fn inside() -> &'static (Crossing, &'static Built) {
    static CELL: OnceLock<(Crossing, &'static Built)> = OnceLock::new();
    CELL.get_or_init(|| {
        let c = crossing(16, pairs(40));
        let built = around("dist-carry-inside", &c.library);
        (c, built)
    })
}

/// The pair list starts 32 bytes before the boundary: the first read ends on it and the second starts on it.
pub fn between() -> &'static (Crossing, &'static Built) {
    static CELL: OnceLock<(Crossing, &'static Built)> = OnceLock::new();
    CELL.get_or_init(|| {
        let c = crossing(32, pairs(40));
        let built = around("dist-carry-between", &c.library);
        (c, built)
    })
}

fn scrolls_with_the_models_frames((c, built): &'static (Crossing, &'static Built), what: &str) {
    let lib = lib_of(built);
    let n = c.n;
    assert_eq!(lib.label(n), LABEL);
    assert_eq!(
        lib.dist(n).len(),
        40,
        "{what}: the model reads the list across the boundary"
    );
    let offset = record_offset(&c.library, n) as usize;
    assert!(
        offset < BOUNDARY,
        "{what}: the record starts at {offset:#x}"
    );
    assert!(
        c.first < BOUNDARY && c.last >= BOUNDARY,
        "{what}: the list's first byte {:#x} and last {:#x} are on opposite sides of {BOUNDARY:#x}",
        c.first,
        c.last
    );
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    open_dist(&mut walker);
    let rows = rows_of(&lib, n).len();
    assert_eq!(app_rows(&mut walker), rows, "{what}");
    assert_eq!(walker.var16("DV_NPAIRS"), 40, "{what}");
    for step in 1..=last_of(rows) {
        assert!(press(&mut walker, DOWN, Key::Down), "{what}: Down {step}");
    }
    assert!(last_of(rows) >= 1, "{what}: the list scrolls");
    leave(&mut walker);
}

#[test]
fn a_pair_list_across_10000h_inside_a_read_is_read_whole() {
    let c = inside();
    assert_eq!(BOUNDARY - c.0.first, 16);
    scrolls_with_the_models_frames(c, "inside a read");
}

#[test]
fn a_pair_list_across_10000h_between_two_reads_is_read_whole() {
    let c = between();
    assert_eq!(BOUNDARY - c.0.first, 32);
    scrolls_with_the_models_frames(c, "between two reads");
}
