use crate::support::*;
use mcpack::library::{GLYPHS, pack_kernel};
use mcpack::mm::StmtId;

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

/// Statements of set.mm's shapes: short and long, many hypotheses, a token wider than a narrow
/// line, and one of 255 symbols.
pub fn source() -> String {
    format!("{HEAD}{AXIOMS}{}", longest())
}

pub fn library() -> Vec<u8> {
    pack_all(&parse(&source()))
}

/// A second library over the same symbols, so with a glyph section somewhere else.
pub fn other() -> Vec<u8> {
    pack_all(&parse(&format!("{HEAD}ax-a $a |- ( ph -> ph ) $.\n")))
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
