//! Hand-built libraries that break the rules of a reference list, shared by the model's tests and
//! the emulator's differential tests so both sides see the same bytes.
#![allow(dead_code)]

use mcpack::library::{KernelError, crc32, pack};
use mcpack::mm::{self, StmtId};

const SOURCE: &str = "
$c wff |- R $.
$v x y $.
vx $f wff x $.
vy $f wff y $.
ax-s $a |- R $.
${ s.1 $e |- R $. ax-t $a |- R $. $}
${ u.1 $e |- R $. ax-u $a |- R $. $}
ax-w $a |- R x $.
${ v.1 $e |- R x $. ax-v $a |- R $. $}
${ vv.1 $e |- R y $. ax-vv $a |- R x $. $}
th $p |- R $= ax-s ax-t ax-u $.
th2 $p |- R $= vx vx ax-w ax-v $.
th3 $p |- R x $= vx vy vy ax-w ax-vv $.
";

/// Assertion numbers in `SOURCE`.
pub const TH: usize = 6;
pub const TH2: usize = 7;
pub const TH3: usize = 8;

const DUMMY: u16 = 0x8000;
const X: u16 = DUMMY + 3;
const Y: u16 = DUMMY + 4;

pub struct Case {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub id: usize,
    pub expected: Result<(), KernelError>,
}

pub fn pack_source(src: &str) -> Vec<u8> {
    let db = mm::parse(src.as_bytes().to_vec()).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("it packs")
}

pub fn base() -> Vec<u8> {
    pack_source(SOURCE)
}

/// Where a theorem's reference list and proof sit, read from the documented layout.
pub struct Spot {
    pub refs: usize,
    pub count: usize,
    pub proof: usize,
}

pub fn spot(bytes: &[u8], id: usize) -> Spot {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = u32_at(u32_at(16) + 4 * id);
    at += 2 + bytes[at + 1] as usize;
    let hyps = bytes[at];
    at += 1;
    for _ in 0..hyps {
        at += if bytes[at] == 0 {
            2
        } else {
            2 + bytes[at + 1] as usize
        };
    }
    at += 2 + 2 * u16_at(at);
    at += 1 + bytes[at] as usize;
    let count_at = at + 2 + 2 * u16_at(at);
    let count = u16_at(count_at);
    let refs = count_at + 2;
    Spot {
        refs,
        count,
        proof: refs + 2 * count + 2,
    }
}

pub fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

/// `base` with a theorem's reference entries and proof bytes replaced.
fn edited(id: usize, refs: &[(usize, u16)], proof: &[(usize, u8)]) -> Vec<u8> {
    let mut bytes = base();
    let at = spot(&bytes, id);
    for &(slot, value) in refs {
        bytes[at.refs + 2 * slot..][..2].copy_from_slice(&value.to_le_bytes());
    }
    for &(step, value) in proof {
        bytes[at.proof + step] = value;
    }
    reseal(&mut bytes);
    bytes
}

/// A theorem naming `entries` distinct axioms and one rule that joins them, `entries - 1` of
/// the first and the rule itself being the list.
pub fn long_source(entries: usize) -> String {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    let mut proof = String::from("n0");
    for i in 0..entries - 1 {
        src += &format!("n{i} $a wff T $.\n");
        if i > 0 {
            proof += &format!(" n{i} drop");
        }
    }
    src + &format!("big $p wff T $= {proof} $.\n")
}

/// The theorem `big` of `long_source(entries)`, with entry `j` made equal to entry `i` for a
/// `repeat` of `(i, j)`.
fn long(entries: usize, repeat: Option<(usize, usize)>) -> (Vec<u8>, usize) {
    let mut bytes = pack_source(&long_source(entries));
    let id = entries;
    let at = spot(&bytes, id);
    assert_eq!(at.count, entries);
    if let Some((i, j)) = repeat {
        bytes.copy_within(at.refs + 2 * i..at.refs + 2 * i + 2, at.refs + 2 * j);
        reseal(&mut bytes);
    }
    (bytes, id)
}

pub fn cases() -> Vec<Case> {
    let ok = Ok(());
    let case = |name, bytes, id, expected| Case {
        name,
        bytes,
        id,
        expected,
    };
    let loose = Err(KernelError::LooseReference);
    let bad = Err(KernelError::BadRecord);
    let forward = Err(KernelError::ForwardReference);
    let mut cases = vec![
        case("th as packed", base(), TH, ok),
        case("th2 as packed", base(), TH2, ok),
        case("th3 as packed", base(), TH3, ok),
        case(
            "entries in the order of first use, not of assertion number",
            edited(TH, &[(1, 2), (2, 1)], &[]),
            TH,
            ok,
        ),
        case(
            "unused entry at the end",
            edited(TH, &[], &[(2, 1)]),
            TH,
            loose,
        ),
        case(
            "unused entry in the middle",
            edited(TH, &[], &[(1, 2), (2, 2)]),
            TH,
            loose,
        ),
        case(
            "two entries used in the wrong order",
            edited(TH, &[], &[(1, 2), (2, 1)]),
            TH,
            loose,
        ),
        case(
            "duplicate assertion entry",
            edited(TH, &[(2, 1)], &[]),
            TH,
            bad,
        ),
        case(
            "duplicate assertion entry, the copy unused",
            edited(TH, &[(2, 1)], &[(2, 1)]),
            TH,
            bad,
        ),
        case(
            "duplicate dummy entry",
            edited(TH2, &[(1, X)], &[]),
            TH2,
            bad,
        ),
        case(
            "dummy entry for a mandatory variable",
            edited(TH3, &[(0, X)], &[]),
            TH3,
            bad,
        ),
        case(
            "dummy entry for another variable",
            edited(TH3, &[(0, Y)], &[]),
            TH3,
            ok,
        ),
        case(
            "dummy entry for a constant",
            edited(TH2, &[(0, DUMMY)], &[]),
            TH2,
            Err(KernelError::Malformed),
        ),
        case(
            "unused forward reference",
            edited(TH, &[(2, TH2 as u16)], &[(2, 1)]),
            TH,
            loose,
        ),
        case(
            "unused reference to the theorem itself",
            edited(TH, &[(2, TH as u16)], &[(2, 1)]),
            TH,
            loose,
        ),
        case(
            "used forward reference",
            edited(TH, &[(2, TH2 as u16)], &[]),
            TH,
            forward,
        ),
        case(
            "forward reference named after an unused entry",
            edited(TH, &[(2, TH2 as u16)], &[(1, 2), (2, 1)]),
            TH,
            loose,
        ),
    ];
    let limit = Err(KernelError::WorkLimit);
    // Comparison number 131072 is the last the limit allows; entry 489 against entry 509 is
    // comparison 131072 of 513 entries, against entry 510 comparison 131073.
    for (name, repeat, expected) in [
        ("512 entries are within the work limit", None, ok),
        (
            "a duplicate is found before the work limit",
            Some((0, 1)),
            bad,
        ),
        (
            "a duplicate at the last comparison the limit allows",
            Some((489, 509)),
            bad,
        ),
        (
            "a duplicate one comparison past the limit",
            Some((489, 510)),
            limit,
        ),
    ] {
        let entries = if repeat.is_none() { 512 } else { 513 };
        let (bytes, id) = long(entries, repeat);
        cases.push(case(name, bytes, id, expected));
    }
    let (bytes, id) = long(513, None);
    cases.push(case("513 entries pass the work limit", bytes, id, limit));
    cases
}
