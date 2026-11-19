use crate::support::*;
use mcpack::library::{ENTRY_LEN, GLYPHS, SHAPES, TRAILER_LEN, TREES, crc32, pack_kernel};
use mcpack::mm::StmtId;
use std::sync::OnceLock;

const HEAD: &str = "$c ( ) -> -. <-> /\\ \\/ A. E. = e. { } | , wff |- setvar class $.\n\
    $v ph ps ch th ta x y z A B $.\n\
    wph $f wff ph $.\nwps $f wff ps $.\nwch $f wff ch $.\nwth $f wff th $.\nwta $f wff ta $.\n\
    vx $f setvar x $.\nvy $f setvar y $.\nvz $f setvar z $.\n\
    cA $f class A $.\ncB $f class B $.\n\
    wn $a wff -. ph $.\nwi $a wff ( ph -> ps ) $.\nwb $a wff ( ph <-> ps ) $.\n\
    wa $a wff ( ph /\\ ps ) $.\nwo $a wff ( ph \\/ ps ) $.\n\
    wal $a wff A. x ph $.\nwex $a wff E. x ph $.\n\
    weq $a wff x = y $.\nwceq $a wff A = B $.\nwcel $a wff A e. B $.\n\
    cv $a class x $.\ncab $a class { x | ph } $.\n";

const AXIOMS: &str = "ax-1 $a |- ( ph -> ( ps -> ph ) ) $.\n\
    ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.\n\
    ax-3 $a |- ( ( -. ph -> -. ps ) -> ( ps -> ph ) ) $.\n\
    ${\nmin $e |- ph $.\nmaj $e |- ( ph -> ps ) $.\nax-mp $a |- ps $.\n$}\n\
    ${\nh1 $e |- ( ph -> ps ) $.\nh2 $e |- ( ps -> ch ) $.\nh3 $e |- ( ch -> th ) $.\n\
    syl3 $a |- ( ph -> th ) $.\n$}\n\
    df-an $a |- ( ( ph /\\ ps ) <-> -. ( ph -> -. ps ) ) $.\n\
    df-or $a |- ( ( ph \\/ ps ) <-> ( -. ph -> ps ) ) $.\n\
    alnex $a |- ( A. x -. ph <-> -. E. x ph ) $.\n\
    ${\nalimi.1 $e |- ( ph -> ps ) $.\nalimi $a |- ( A. x ph -> A. x ps ) $.\n$}\n\
    df-clab $a |- ( y e. { x | ph } <-> ( ph /\\ ( ph -> ph ) ) ) $.\n\
    ${\nh10 $e |- ( ( ( ph -> ps ) -> ( ch -> th ) ) -> ( ( ta -> ph ) -> ( ps -> ch ) ) ) $.\n\
    h11 $e |- ( ph <-> ps ) $.\nh12 $e |- A = B $.\nh13 $e |- ( ph -> ( ps -> ( ch -> ( th -> ta ) ) ) ) $.\n\
    h14 $e |- ph $.\nh15 $e |- ( A. x ph /\\ E. y ps ) $.\nh16 $e |- ( x = y -> ( ph <-> ps ) ) $.\n\
    h17 $e |- A e. { x | ph } $.\nh18 $e |- ( ph \\/ ps ) $.\nh19 $e |- ( ph /\\ ps ) $.\n\
    many $a |- ( ( ph /\\ ps ) -> ( ( ch \\/ th ) <-> ( ta /\\ ph ) ) ) $.\n$}\n\
    wide $a |- ( setvar class wff x A e. B ) $.\n\
    ax-one $a |- ph $.\n\
    ax-class $a |- ( A = B -> ( x e. A <-> x e. B ) ) $.\n";

/// A conclusion of the 255 symbols the format allows: the typecode, a negation and 63 nested
/// conjunctions.
fn longest() -> String {
    let nested = 63;
    format!(
        "longest $a |- -. {}ps {}$.\n",
        "( ph /\\ ".repeat(nested),
        ") ".repeat(nested)
    )
}

/// The fixture's header and syntax axioms followed by `extra`, packed with a glyph map and trees.
pub fn custom(extra: &str) -> Vec<u8> {
    pack_all(&parse(&format!("{HEAD}{extra}")))
}

/// Statements of set.mm's shapes: short and long, many hypotheses, a token wider than a narrow
/// line, and one of 255 symbols.
pub fn source() -> String {
    format!("{HEAD}{AXIOMS}{}", longest())
}

/// Constants that are each a syntax axiom of their own, so that a later syntax axiom has an index
/// of 127 or more and its nodes take two bytes in a tree.
pub const FILLERS: usize = 130;

/// The most operators a statement of 255 symbols can hold in front of a variable.
pub const CHAIN: usize = 253;

/// What the base fixture lacks: a wrapper over a wrapper, a statement that is only a wrapper
/// chain, a wrapper below a root, a prefix operator whose index takes two bytes in a tree and a
/// statement of it 253 deep with a tree of 507 bytes, which fm reads in three pieces.
fn structure() -> String {
    let mut src = format!("$c ~ {} $.\n", names("F", FILLERS).join(" "));
    src += "wcl $a wff A $.\n";
    for i in 0..FILLERS {
        src += &format!("wf{i} $a wff F{i} $.\n");
    }
    src += "wun $a wff ~ ph $.\n";
    src += "nest-root $a |- x $.\nnest-below $a |- ( ph -> x ) $.\n";
    src += "onec $a |- ( F3 -> ph ) $.\n";
    src += "wrapc $a |- ( ( ch -> { x | ps } ) -> th ) $.\n";
    let fillers = names("F", FILLERS);
    let wide: Vec<&str> = (0..250).map(|i| fillers[i % FILLERS].as_str()).collect();
    let ten = fillers[..10].join(" ");
    src += &format!("wten $a wff {ten} $.\nten $a |- {ten} $.\n");
    src += "wbig $a wff ";
    src += &wide.join(" ");
    src += " $.\n";
    src += &format!("big $a |- {} $.\n", wide.join(" "));
    for (label, lead) in [("t255", ""), ("t256", "-. "), ("t257", "-. -. ")] {
        src += &format!("{label} $a |- {lead}{}ph $.\n", "~ ".repeat(127));
    }
    for (n, chunk) in names("F", FILLERS).chunks(13).enumerate() {
        let nested = chunk
            .iter()
            .rev()
            .skip(1)
            .fold(chunk.last().unwrap().clone(), |inner, f| {
                format!("( {f} /\\ {inner} )")
            });
        src += &format!("fill{n} $a |- {nested} $.\n");
    }
    src + &format!("deep $a |- {}ph $.\n", "~ ".repeat(CHAIN))
}

fn names(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{prefix}{i}")).collect()
}

/// The base fixture and `structure`.
pub fn nodes_library() -> Vec<u8> {
    static PACKED: OnceLock<Vec<u8>> = OnceLock::new();
    PACKED
        .get_or_init(|| pack_all(&parse(&format!("{}{}", source(), structure()))))
        .clone()
}

pub fn library() -> Vec<u8> {
    pack_all(&parse(&source()))
}

/// A second library over the same symbols, so with a glyph section somewhere else.
pub fn other() -> Vec<u8> {
    pack_all(&parse(&format!("{HEAD}ax-a $a |- ( ph -> ph ) $.\n")))
}

/// The file offset of the section table's row for `id`.
pub fn row_of(bytes: &[u8], id: u8) -> usize {
    let count = usize::from(bytes[bytes.len() - TRAILER_LEN]);
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    (0..count)
        .map(|i| first + i * ENTRY_LEN)
        .find(|&at| bytes[at] == id)
        .expect("the library has the row")
}

/// `bytes` with `change` applied and the length and checksum of the header made right again.
pub fn with(bytes: &[u8], change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut out = bytes.to_vec();
    change(&mut out);
    let len = out.len() as u32;
    out[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = crc32(&out[8..]);
    out[4..8].copy_from_slice(&crc.to_le_bytes());
    out
}

/// The fixture with its trees and shapes sections renamed to ids nobody reads: a glyph map and
/// neither of the two.
pub fn unstructured() -> Vec<u8> {
    let good = library();
    let (trees, shapes) = (row_of(&good, TREES), row_of(&good, SHAPES));
    with(&good, |b| {
        b[trees] = 40;
        b[shapes] = 41;
    })
}

/// The fixture packed without sections.
pub fn plain() -> Vec<u8> {
    let db = parse(&source());
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack_kernel(&db, &all).unwrap()
}

pub fn section(bytes: &[u8]) -> usize {
    let sections = decoded(bytes).sections.unwrap();
    sections
        .table
        .iter()
        .find(|e| e.id == GLYPHS)
        .unwrap()
        .offset
}

/// Where the glyph entry of `symbol` starts in the library.
pub fn entry(bytes: &[u8], symbol: usize) -> usize {
    let at = section(bytes) + 3 + 2 * symbol;
    section(bytes) + usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

/// The fixture with `change` applied to its bytes.
pub fn patched(change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut bytes = library();
    change(&mut bytes);
    bytes
}
