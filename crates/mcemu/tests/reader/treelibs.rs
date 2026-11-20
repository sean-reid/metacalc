use crate::support::*;
use mcpack::mm;

pub fn pack_with_sections(src: &str) -> Vec<u8> {
    let db = mm::parse(src.as_bytes().to_vec()).expect("it parses");
    pack_all(&db)
}

/// Strings over `a` and `b`, shortest first: each is the body of its own syntax axiom, and no
/// string is read two ways.
pub fn strings(n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for len in 1..=9usize {
        for bits in 0..1usize << len {
            let text: Vec<&str> = (0..len)
                .map(|i| if bits >> i & 1 == 0 { "a" } else { "b" })
                .collect();
            out.push(text.join(" "));
        }
    }
    out.truncate(n);
    out
}

/// `n` syntax axioms, each used by its own statement, so a statement's tree is one node and the
/// axiom's index in the syntax table is its number.
pub fn axioms(n: usize) -> Vec<u8> {
    let mut src = String::from("$c |- wff a b $.\n");
    for (i, body) in strings(n).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\nax-{i} $a |- {body} $.\n");
    }
    pack_with_sections(&src)
}

/// `n` variables, each the whole of its own statement.
pub fn variables(n: usize) -> Vec<u8> {
    let vars: Vec<String> = (0..n).map(|i| format!("v{i}")).collect();
    let mut src = format!("$c |- wff $.\n$v {} $.\n", vars.join(" "));
    for v in &vars {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    for v in &vars {
        src += &format!("ax-{v} $a |- {v} $.\n");
    }
    pack_with_sections(&src)
}

/// 130 syntax axioms, then `wi` as the 131st, then a statement over the last two of 130
/// variables: a tree with an escaped axiom and two escaped variables.
pub fn mixed() -> Vec<u8> {
    let mut src = String::from("$c ( ) |- wff a b $.\n");
    let vars: Vec<String> = (0..130).map(|i| format!("v{i}")).collect();
    src += &format!("$v {} $.\n", vars.join(" "));
    for v in &vars {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    for (i, body) in strings(130).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\nax-f{i} $a |- {body} $.\n");
    }
    src += "wi $a wff ( v0 v1 ) $.\n";
    src += "ax-top $a |- ( v128 v129 ) $.\n";
    pack_with_sections(&src)
}

pub const LONG: [usize; 6] = [100, 126, 127, 128, 200, 253];

/// 130 syntax axioms, so that the axiom for negation has an escaped index and costs two bytes a
/// node, and statements of `LONG` negations, whose trees run from 201 to 507 bytes. A second
/// negation declared first costs one byte a node, which makes a tree of 254 bytes, an even length.
pub fn long_trees() -> Vec<u8> {
    let mut src = String::from("$c |- wff a b -. -.. $.\n$v p $.\nwp $f wff p $.\n");
    src += "wm $a wff -.. p $.\n";
    for (i, body) in strings(130).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\nax-f{i} $a |- {body} $.\n");
    }
    src += "wn $a wff -. p $.\n";
    for n in LONG {
        src += &format!("ax-long-{n} $a |- {}p $.\n", "-. ".repeat(n));
    }
    src += &format!("ax-mixed $a |- -.. {}p $.\n", "-. ".repeat(126));
    src += "${\nh $e |- -. -. -. p $.\nax-with $a |- p $.\n$}\n";
    pack_with_sections(&src)
}

pub fn number(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .expect("the label exists") as u16
}
