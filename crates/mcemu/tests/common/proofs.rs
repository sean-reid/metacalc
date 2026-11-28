//! Libraries with proofs written for the proof tests, a mirror of a record's layout written from
//! the format's header in crates/mcpack/src/library/mod.rs, and what the host model says of a
//! theorem's goal tree.

use mcpack::library::{Step, crc32, decode, pack, pack_kernel};
use mcpack::mm::{self, StmtId};
use mcui::data::Lib;
use mcui::{Installed, Status};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Step `n` of a compressed proof written as the letters the mm format uses: A to T for the last
/// digit, U to Y for the higher ones.
pub fn letters(steps: &[Token]) -> String {
    let mut out = String::new();
    for step in steps {
        match *step {
            Token::Save => out.push('Z'),
            Token::Num(n) => {
                let mut high = n / 20;
                let mut digits = Vec::new();
                while high > 0 {
                    digits.push((b'U' + ((high - 1) % 5) as u8) as char);
                    high = (high - 1) / 5;
                }
                out.extend(digits.iter().rev());
                out.push((b'A' + (n % 20) as u8) as char);
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub enum Token {
    Num(usize),
    Save,
}

pub use Token::{Num, Save};

/// The statements every proof library starts with: a wff and a provable typecode, an implication
/// as syntax, and axioms that take no, one, two, three and many hypotheses.
pub const BASE: &str = "$c |- wff ( ) -> $.
$v p q r $.
wp $f wff p $.
wq $f wff q $.
wr $f wff r $.
wi $a wff ( p -> q ) $.
ax-1 $a |- ( p -> ( q -> p ) ) $.
${
  min $e |- p $.
  maj $e |- ( p -> q ) $.
  ax-mp $a |- q $.
$}
${
  c1 $e |- p $.
  c2 $e |- p $.
  ax-c $a |- p $.
$}
${
  b1 $e |- p $.
  b2 $e |- p $.
  two $a |- ( p -> p ) $.
$}
${
  t1 $e |- p $.
  t2 $e |- p $.
  t3 $e |- p $.
  tri $a |- ( p -> p ) $.
$}
";

/// A theorem of the base: its hypotheses as (label, statement), its statement with the typecode,
/// the labels its proof names in the order of its list, and the proof.
pub fn theorem(
    label: &str,
    hyps: &[(&str, &str)],
    statement: &str,
    names: &[&str],
    proof: &[Token],
) -> String {
    let mut out = String::from("${\n");
    for (h, s) in hyps {
        out += &format!("  {label}-{h} $e {s} $.\n");
    }
    out += &format!(
        "  {label} $p {statement} $= ( {} ) {} $.\n$}}\n",
        names.join(" "),
        letters(proof)
    );
    out
}

/// The theorems the tests know the goal trees of: label, steps, distinct goals, links, total.
pub const KNOWN: [(&str, usize, usize, usize, u64); 7] = [
    ("one", 1, 1, 0, 1),
    ("mp2", 9, 5, 4, 5),
    ("use2", 4, 2, 2, 3),
    ("use3", 5, 2, 3, 4),
    ("syn1", 3, 1, 0, 1),
    ("ref1", 3, 2, 1, 2),
    ("mp2s", 9, 5, 4, 5),
];

/// The base and the theorems of `KNOWN`, then `wtest`, which proves a syntax statement.
pub fn fixture_source() -> String {
    let mut src = String::from(BASE);
    let h = |l: &'static str, s: &'static str| (l, s);
    src += &theorem("one", &[h("h1", "|- p")], "|- p", &[], &[Num(1)]);
    src += &theorem(
        "mp2",
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
    );
    src += &theorem(
        "use2",
        &[h("u1", "|- p")],
        "|- ( p -> p )",
        &["two"],
        &[Num(0), Num(1), Save, Num(3), Num(2)],
    );
    src += &theorem(
        "use3",
        &[h("u1", "|- p")],
        "|- ( p -> p )",
        &["tri"],
        &[Num(0), Num(1), Save, Num(3), Num(3), Num(2)],
    );
    src += &theorem(
        "syn1",
        &[],
        "|- ( p -> ( p -> p ) )",
        &["ax-1"],
        &[Num(0), Save, Num(2), Num(1)],
    );
    src += &theorem(
        "ref1",
        &[h("r1", "|- p")],
        "|- p",
        &["one"],
        &[Num(0), Num(1), Num(2)],
    );
    src += &theorem(
        "mp2s",
        &[
            h("m1", "|- p"),
            h("m2", "|- ( p -> q )"),
            h("m3", "|- ( q -> r )"),
        ],
        "|- r",
        &["ax-mp"],
        &[
            Num(1),
            Save,
            Num(2),
            Num(0),
            Num(7),
            Num(3),
            Num(4),
            Num(6),
            Num(5),
            Num(6),
        ],
    );
    src += &theorem(
        "wtest",
        &[],
        "wff ( p -> ( q -> p ) )",
        &["wi"],
        &[Num(0), Num(1), Num(0), Num(2), Num(2)],
    );
    src
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

pub fn parse(src: &str) -> mm::Database {
    mm::parse(src.as_bytes().to_vec()).expect("the database parses")
}

/// A library of every statement of `src`, with sections.
pub fn packed(src: &str) -> Vec<u8> {
    let db = parse(src);
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("it packs")
}

/// The same without sections, so that the last record is the last byte of the file.
pub fn kernel_only(src: &str) -> Vec<u8> {
    let db = parse(src);
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack_kernel(&db, &all).expect("it packs")
}

pub fn fixture() -> Vec<u8> {
    packed(&fixture_source())
}

pub fn number_of(bytes: &[u8], label: &str) -> u16 {
    decode(bytes)
        .expect("the library decodes")
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}")) as u16
}

pub fn get32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

pub fn get16(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

/// Where the parts of assertion `n` lie in `bytes`, read from the format and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub record: usize,
    pub theorem: bool,
    pub hyps: usize,
    pub nhyps: usize,
    /// The offset of the count of disjoint pairs; the pairs follow it.
    pub pairs: usize,
    /// The offset of the reference count; the entries follow it.
    pub refs_count: usize,
    pub nrefs: usize,
    pub proof: usize,
    pub plen: usize,
}

pub fn layout(bytes: &[u8], n: usize) -> Layout {
    let directory = get32(bytes, 16) as usize;
    let record = get32(bytes, directory + 4 * n) as usize;
    let theorem = bytes[record] == 1;
    let mut at = record + 2 + usize::from(bytes[record + 1]);
    let nhyps = usize::from(bytes[at]);
    at += 1;
    let hyps = at;
    for _ in 0..nhyps {
        at += if bytes[at] == 0 {
            2
        } else {
            2 + usize::from(bytes[at + 1])
        };
    }
    let pairs = at;
    at += 2 + 2 * get16(bytes, at);
    at += 1 + usize::from(bytes[at]);
    let mut out = Layout {
        record,
        theorem,
        hyps,
        nhyps,
        pairs,
        refs_count: 0,
        nrefs: 0,
        proof: 0,
        plen: 0,
    };
    if theorem {
        at += 2 + 2 * get16(bytes, at);
        out.refs_count = at;
        out.nrefs = get16(bytes, at);
        at += 2 + 2 * out.nrefs;
        out.plen = get16(bytes, at);
        out.proof = at + 2;
    }
    out
}

/// A proof's bytes as the format stores them.
pub fn encode(steps: &[Step]) -> Vec<u8> {
    let mut out = Vec::new();
    for step in steps {
        match *step {
            Step::Save => out.push(0xFF),
            Step::Number(n) if n < 0xF0 => out.push(n as u8),
            Step::Number(n) => {
                let wide = n - 0xF0;
                out.extend([0xF0 + (wide >> 8) as u8, wide as u8]);
            }
        }
    }
    out
}

pub fn reseal(bytes: &mut [u8]) {
    let len = bytes.len() as u32;
    bytes[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

/// A copy of `bytes` with `change` applied and the length and checksum written again.
pub fn edited(bytes: &[u8], change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut out = bytes.to_vec();
    change(&mut out);
    reseal(&mut out);
    out
}

/// What the host model says of one theorem: steps from the stored proof, distinct goals by
/// pointer, links as the sum of children over the distinct goals, and the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub steps: usize,
    pub goals: usize,
    pub links: usize,
    pub total: u64,
}

/// The model's goal tree of assertion `n`, as the outline builds it, before the calculator's caps
/// on a tree apply: `None` for an axiom and for a syntax statement.
pub fn outline_of(bytes: &[u8], n: usize) -> Option<Rc<mm::Node>> {
    let text = mcpack::library::export(bytes).expect("the library exports");
    let db = mm::parse(text.into_bytes()).expect("the export parses");
    let id = (0..db.stmts.len() as StmtId)
        .filter(|&id| db.stmt(id).kind.is_assertion())
        .nth(n)
        .expect("an assertion of that number");
    mm::outline(&db, id, "|-").ok()
}

/// The nodes of the tree under `node`, a goal used twice counting twice.
pub fn nodes_under(node: &Rc<mm::Node>, sizes: &mut HashMap<*const mm::Node, u64>) -> u64 {
    if let Some(&size) = sizes.get(&Rc::as_ptr(node)) {
        return size;
    }
    let size = node
        .children
        .iter()
        .fold(1u64, |sum, c| sum.saturating_add(nodes_under(c, sizes)));
    sizes.insert(Rc::as_ptr(node), size);
    size
}

/// The distinct goals of a tree by pointer, the links over them, and the most subgoals of one.
pub fn tree_counts(root: &Rc<mm::Node>) -> (usize, usize, usize) {
    let mut seen = HashSet::new();
    let mut pending = vec![root.clone()];
    let (mut links, mut widest) = (0, 0);
    while let Some(node) = pending.pop() {
        if seen.insert(Rc::as_ptr(&node)) {
            links += node.children.len();
            widest = widest.max(node.children.len());
            pending.extend(node.children.iter().cloned());
        }
    }
    (seen.len(), links, widest)
}

/// What the model's outline says of assertion `n`, whatever the caps on a tree.
pub fn outline_shape(bytes: &[u8], n: usize) -> Shape {
    let root = outline_of(bytes, n).expect("the outline has a tree");
    let (goals, links, _) = tree_counts(&root);
    Shape {
        steps: decode(bytes).unwrap().assertions[n]
            .proof
            .iter()
            .filter(|s| matches!(s, Step::Number(_)))
            .count(),
        goals,
        links,
        total: nodes_under(&root, &mut HashMap::new()),
    }
}

/// The model's answer for every assertion: `None` for an axiom and a theorem the model refuses
/// (a syntax statement).
pub fn model_shapes(name: &str, bytes: &[u8]) -> Vec<Option<Shape>> {
    let library = decode(bytes).expect("the library decodes");
    let n = library.assertions.len();
    let lib = Lib::new(
        Installed {
            name: name.into(),
            bytes: bytes.to_vec(),
        },
        vec![Status::Unchecked; n],
    )
    .expect("the model loads the library");
    (0..n)
        .map(|i| {
            let goals = lib.goals(i).ok()?;
            let mut seen = HashSet::new();
            let mut pending = vec![goals.root.clone()];
            let mut links = 0;
            while let Some(node) = pending.pop() {
                if seen.insert(Rc::as_ptr(&node)) {
                    links += node.children.len();
                    pending.extend(node.children.iter().cloned());
                }
            }
            let steps = library.assertions[i]
                .proof
                .iter()
                .filter(|s| matches!(s, Step::Number(_)))
                .count();
            Some(Shape {
                steps,
                goals: seen.len(),
                links,
                total: goals.total(),
            })
        })
        .collect()
}
