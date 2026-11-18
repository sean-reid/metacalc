//! Hand-built libraries that break the rules of a reference list, shared by the model's tests and
//! the emulator's differential tests so both sides see the same bytes.
#![allow(dead_code)]

use mcpack::library::{KernelError, WORK_LIMIT, crc32, pack};
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
${ j.1 $e |- R $. j.2 $e |- R $. ax-j $a |- R $. $}
th $p |- R $= ax-s ax-t ax-u $.
th2 $p |- R $= vx vx ax-w ax-v $.
th3 $p |- R x $= vx vy vy ax-w ax-vv $.
th4 $p |- R $= ax-s ax-t ax-s ax-j $.
";

/// Assertion numbers in `SOURCE`.
pub const TH: usize = 7;
pub const TH2: usize = 8;
pub const TH3: usize = 9;
pub const TH4: usize = 10;

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
    pub hyps: usize,
    pub refs: usize,
    pub count: usize,
    pub proof: usize,
}

pub fn spot(bytes: &[u8], id: usize) -> Spot {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = u32_at(u32_at(16) + 4 * id);
    at += 2 + bytes[at + 1] as usize;
    let hyps = bytes[at] as usize;
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
        hyps,
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

fn letters(mut number: usize) -> String {
    let mut out = vec![b'A' + (number % 20) as u8];
    number /= 20;
    while number > 0 {
        number -= 1;
        out.push(b'U' + (number % 5) as u8);
        number /= 5;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

pub const HYPOTHESES: usize = 130;
pub const SAVES: usize = 500;

/// A theorem `big` with `HYPOTHESES` hypotheses, two references and `SAVES` saved steps, the last
/// of which it names at step number 631, a four-letter number in a compressed proof.
/// A theorem `big` after 260 axioms whose four references are assertions 1, 259, 0 and 3, so
/// two entries share a low byte, 0103h and 0003h.
pub fn same_low_source() -> String {
    let mut src =
        String::from("$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n");
    for i in 0..260 {
        src += &format!("n{i} $a wff T $.\n");
    }
    src + "big $p wff T $= n0 n258 drop n2 drop $.\n"
}

/// A theorem `big` of `wff x` whose reference list is `entries` dummy entries for `y`, the last
/// for `x` when `mandatory`, which `big` has a float for. Its proof is not a proof.
pub fn many_dummies(entries: usize, mandatory: bool) -> (Vec<u8>, usize) {
    let mut src = String::from(
        "$c T wff $.\n$v x y $.\nvx $f wff x $.\nvy $f wff y $.\n\
         ${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\n",
    );
    let mut proof = String::from("n0");
    for i in 0..entries - 1 {
        src += &format!("n{i} $a wff T $.\n");
        if i > 0 {
            proof += &format!(" n{i} drop");
        }
    }
    src += &format!("big $p wff x $= {proof} $.\n");
    let mut bytes = pack_source(&src);
    let id = entries;
    let at = spot(&bytes, id);
    assert_eq!(at.count, entries);
    for slot in 0..entries {
        let var = if mandatory && slot == entries - 1 {
            2
        } else {
            3
        };
        bytes[at.refs + 2 * slot..][..2].copy_from_slice(&(DUMMY + var).to_le_bytes());
    }
    reseal(&mut bytes);
    (bytes, id)
}

pub fn saved_source() -> String {
    let mut src = String::from(
        "$c T wff $.\n${ d.1 $e wff T $. d.2 $e wff T $. drop $a wff T $. $}\na $a wff T $.\n${\n",
    );
    for i in 0..HYPOTHESES {
        src += &format!("h{i} $e wff T $.\n");
    }
    let (a, drop) = (HYPOTHESES, HYPOTHESES + 1);
    let mut proof = format!("{}Z", letters(a));
    for _ in 1..SAVES {
        proof += &format!("{}{}Z", letters(a), letters(drop));
    }
    proof += &letters(HYPOTHESES + 2 + SAVES - 1);
    proof += &letters(drop);
    src + &format!("big $p wff T $= ( a drop ) {proof} $.\n$}}\n")
}

/// The theorem `big` of `long_source(entries)` with its reference entries edited by `edit`.
fn long_with(entries: usize, edit: impl FnOnce(&mut [u16])) -> (Vec<u8>, usize) {
    let mut bytes = pack_source(&long_source(entries));
    let id = entries;
    let at = spot(&bytes, id);
    assert_eq!(at.count, entries);
    let mut list: Vec<u16> = bytes[at.refs..at.refs + 2 * entries]
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    edit(&mut list);
    for (slot, entry) in list.iter().enumerate() {
        bytes[at.refs + 2 * slot..][..2].copy_from_slice(&entry.to_le_bytes());
    }
    reseal(&mut bytes);
    (bytes, id)
}

/// The theorem `big` of `long_source(entries)`, with entry `j` made equal to entry `i` for a
/// `repeat` of `(i, j)`.
fn long(entries: usize, repeat: Option<(usize, usize)>) -> (Vec<u8>, usize) {
    long_with(entries, |list| {
        if let Some((i, j)) = repeat {
            list[j] = list[i];
        }
    })
}

/// The pair of entries of a list of `n` whose comparison is the last the work limit allows once
/// the entries themselves have been charged.
fn last_allowed(n: usize) -> (usize, usize) {
    let budget = WORK_LIMIT as usize - n;
    let mut start = 0;
    for i in 0..n {
        let row = n - 1 - i;
        if budget <= start + row {
            return (i, i + budget - start);
        }
        start += row;
    }
    unreachable!()
}

pub struct Mutant {
    pub name: String,
    pub bytes: Vec<u8>,
    pub id: usize,
    /// What the kernel must say when the mutation keeps the proof's meaning, from the rule alone.
    pub expected: Option<Result<(), KernelError>>,
}

/// Whether a list of steps names the `entries` reference entries in order of first use.
fn in_first_use_order(steps: &[u8], hyps: usize, entries: usize) -> bool {
    let mut next = 0;
    for &step in steps {
        let Some(entry) = (step as usize).checked_sub(hyps).filter(|&e| e < entries) else {
            continue;
        };
        if entry > next {
            return false;
        }
        if entry == next {
            next += 1;
        }
    }
    next == entries
}

/// Structured edits of the theorems of `base`: any two entries renamed together with the steps
/// that name them, any two steps swapped, and any entry made equal to another.
pub fn mutants() -> Vec<Mutant> {
    let base = base();
    let mut out = Vec::new();
    for (label, id) in [("th", TH), ("th2", TH2), ("th3", TH3), ("th4", TH4)] {
        let at = spot(&base, id);
        let n = at.count;
        let len = u16::from_le_bytes([base[at.proof - 2], base[at.proof - 1]]) as usize;
        let steps = base[at.proof..at.proof + len].to_vec();
        let mut push = |name: String, entries: Vec<u8>, steps: Vec<u8>, expected| {
            let mut bytes = base.clone();
            bytes[at.refs..at.refs + 2 * n].copy_from_slice(&entries);
            bytes[at.proof..at.proof + len].copy_from_slice(&steps);
            reseal(&mut bytes);
            out.push(Mutant {
                name,
                bytes,
                id,
                expected,
            });
        };
        let entries = base[at.refs..at.refs + 2 * n].to_vec();
        for a in 0..n {
            for b in 0..n {
                if a < b {
                    let mut renamed = entries.clone();
                    for k in 0..2 {
                        renamed.swap(2 * a + k, 2 * b + k);
                    }
                    let steps: Vec<u8> = steps
                        .iter()
                        .map(|&s| match (s as usize).wrapping_sub(at.hyps) {
                            x if x == a => (at.hyps + b) as u8,
                            x if x == b => (at.hyps + a) as u8,
                            _ => s,
                        })
                        .collect();
                    let ordered = in_first_use_order(&steps, at.hyps, n);
                    let expected = if ordered {
                        Ok(())
                    } else {
                        Err(KernelError::LooseReference)
                    };
                    push(
                        format!("{label}: entries {a} and {b} renamed"),
                        renamed,
                        steps,
                        Some(expected),
                    );
                }
                if a != b {
                    let mut repeated = entries.clone();
                    repeated.copy_within(2 * b..2 * b + 2, 2 * a);
                    push(
                        format!("{label}: entry {a} made equal to entry {b}"),
                        repeated,
                        steps.clone(),
                        Some(Err(KernelError::BadRecord)),
                    );
                }
            }
        }
        for p in 0..len {
            for q in p + 1..len {
                let mut swapped = steps.clone();
                swapped.swap(p, q);
                push(
                    format!("{label}: steps {p} and {q} swapped"),
                    entries.clone(),
                    swapped,
                    None,
                );
            }
        }
    }
    out
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
    let malformed = Err(KernelError::Malformed);
    let (i, j) = last_allowed(513);
    for (name, entries, repeat, expected) in [
        ("511 entries are within the work limit", 511, None, ok),
        ("512 entries pass the work limit", 512, None, limit),
        ("513 entries pass the work limit", 513, None, limit),
        (
            "a duplicate is found before the work limit",
            513,
            Some((0, 1)),
            bad,
        ),
        (
            "a duplicate at the last comparison the limit allows",
            513,
            Some((i, j)),
            bad,
        ),
        (
            "a duplicate one comparison past the limit",
            513,
            Some((i, j + 1)),
            limit,
        ),
    ] {
        let (bytes, id) = long(entries, repeat);
        cases.push(case(name, bytes, id, expected));
    }
    for r in [4, 362, 363, 511] {
        for (name, repeat) in [
            ("a duplicate in the first row, first comparison", (0, 1)),
            ("a duplicate at the end of the first row", (0, r - 1)),
            ("a duplicate at the end of a middle row", (r / 2, r - 1)),
            ("a duplicate in the last row", (r - 2, r - 1)),
        ] {
            let (bytes, id) = long(r, Some(repeat));
            cases.push(case(name, bytes, id, bad));
        }
        let (bytes, id) = long_with(r, |list| list[1] = DUMMY + 3);
        cases.push(case(
            "a dummy entry and an assertion with the same low byte",
            bytes,
            id,
            malformed,
        ));
        let (bytes, id) = long(r, None);
        cases.push(case("a list with no duplicate", bytes, id, ok));
    }
    cases.push(case(
        "assertions with the same low byte in a list of four",
        pack_source(&same_low_source()),
        261,
        ok,
    ));
    cases.push(case(
        "a saved step named at a four-letter step number",
        pack_source(&saved_source()),
        2,
        ok,
    ));
    cases
}
