use crate::sections::*;
use crate::support::*;

/// Strings over `a` and `b`, shortest first: each is the body of its own syntax axiom, and no
/// string is read two ways.
fn strings(n: usize) -> Vec<String> {
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
fn axioms(n: usize) -> Vec<u8> {
    let mut src = String::from("$c |- wff a b $.\n");
    for (i, body) in strings(n).iter().enumerate() {
        src += &format!("w{i} $a wff {body} $.\nax-{i} $a |- {body} $.\n");
    }
    pack_with_sections(&src)
}

/// `n` variables, each the whole of its own statement.
fn variables(n: usize) -> Vec<u8> {
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
fn mixed() -> Vec<u8> {
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

const LONG: [usize; 6] = [100, 126, 127, 128, 200, 253];

/// 130 syntax axioms, so that the axiom for negation has an escaped index and costs two bytes a
/// node, and statements of `LONG` negations, whose trees run from 201 to 507 bytes. A second
/// negation declared first costs one byte a node, which makes a tree of 254 bytes, an even length.
fn long_trees() -> Vec<u8> {
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

fn number(bytes: &[u8], label: &str) -> u16 {
    decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .expect("the label exists") as u16
}

fn check_all(bytes: &[u8], what: &str) -> Rig {
    let mut rig = opened(bytes);
    check_sections(&mut rig, bytes, what, &ascending);
    rig
}

#[test]
fn syntax_indices_126_127_and_382_come_out_of_the_iterator_as_the_decoder_decodes_them() {
    let bytes = axioms(383);
    let mut rig = check_all(&bytes, "383 axioms");
    for (index, tree) in [
        (126u16, vec![0x7E]),
        (127, vec![0x7F, 0]),
        (128, vec![0x7F, 1]),
        (382, vec![0x7F, 255]),
    ] {
        let n = number(&bytes, &format!("ax-{index}"));
        let got = rig.tree(n, 0xFF);
        assert_eq!(
            (got.status, got.bytes.clone()),
            (ST_OK, tree.clone()),
            "index {index}"
        );
        assert_eq!(rig.walk(&tree), (ST_OK, vec![(0, index)]), "index {index}");
        let (status, assertion, arity) = rig.syntax(index);
        assert_eq!((status, arity), (ST_OK, 0));
        assert_eq!(assertion, number(&bytes, &format!("w{index}")));
    }
    assert_eq!(rig.syntax_count(), (ST_OK, 383));
    let bytes = axioms(127);
    let mut rig = check_all(&bytes, "127 axioms");
    assert_eq!(rig.syntax_count(), (ST_OK, 127));
    assert_eq!(rig.walk(&[0x7E]), (ST_OK, vec![(0, 126)]));
    assert_eq!(
        rig.walk(&[0x7F, 0]).0,
        ST_BAD,
        "index 127 is past a table of 127"
    );
}

#[test]
fn variable_numbers_126_and_127_come_out_of_the_iterator_as_the_decoder_decodes_them() {
    for n in [127, 130, 253] {
        let bytes = variables(n);
        let mut rig = check_all(&bytes, &format!("{n} variables"));
        let library = decoded(&bytes);
        let constants = library.constants.len() as u16;
        for (var, tree) in [
            (0u16, vec![0x80]),
            (126, vec![0xFE]),
            (127, vec![0xFF, 0]),
            (
                n as u16 - 1,
                if n > 127 {
                    vec![0xFF, (n - 128) as u8]
                } else {
                    vec![0xFE]
                },
            ),
        ] {
            if var >= n as u16 {
                continue;
            }
            let at = number(&bytes, &format!("ax-v{var}"));
            let got = rig.tree(at, 0xFF);
            assert_eq!(
                (got.status, got.bytes.clone()),
                (ST_OK, tree.clone()),
                "{n}: v{var}"
            );
            assert_eq!(
                rig.walk(&tree),
                (ST_OK, vec![(1, constants + var)]),
                "{n}: v{var}"
            );
        }
    }
}

#[test]
fn a_tree_with_an_escaped_axiom_and_two_escaped_variables_reads_back() {
    let bytes = mixed();
    let mut rig = check_all(&bytes, "mixed");
    let top = number(&bytes, "ax-top");
    let constants = decoded(&bytes).constants.len() as u16;
    let got = rig.tree(top, 0xFF);
    assert_eq!(got.bytes, [0x7F, 3, 0xFF, 1, 0xFF, 2]);
    assert_eq!(
        rig.walk(&got.bytes),
        (
            ST_OK,
            vec![(0, 130), (1, constants + 128), (1, constants + 129)]
        )
    );
}

#[test]
fn trees_of_every_length_to_507_bytes_copy_255_and_give_the_offset_of_the_rest() {
    let bytes = long_trees();
    let mut rig = check_all(&bytes, "long trees");
    for (label, len) in [
        ("ax-long-100", 201u16),
        ("ax-long-126", 253),
        ("ax-mixed", 254),
        ("ax-long-127", 255),
        ("ax-long-128", 257),
        ("ax-long-200", 401),
        ("ax-long-253", 507),
    ] {
        let at = number(&bytes, label);
        let got = rig.tree(at, 0xFF);
        assert_eq!((got.status, got.len), (ST_OK, len), "{label}");
        assert_eq!(got.bytes.len(), usize::from(len.min(255)), "{label}");
        let file = &bytes[got.offset as usize..got.offset as usize + usize::from(len)];
        assert_eq!(got.bytes, file[..file.len().min(255)], "{label}");
    }
    let with = number(&bytes, "ax-with");
    let hypothesis = rig.tree(with, 0);
    assert_eq!((hypothesis.status, hypothesis.len), (ST_OK, 7));
    assert_eq!(rig.tree(with, 1).bytes, [0x80]);
    assert_eq!(rig.tree(with, 0xFF).bytes, [0x80]);
    assert_eq!(rig.tree(with, 2).status, ST_INDEX);
}

#[test]
fn a_node_the_buffer_cuts_in_two_is_given_back_untouched_for_a_refill() {
    let bytes = axioms(383);
    let mut rig = opened(&bytes);
    let (status, kind, value, hl, left) = rig.node(&[0x7F], 0xA200);
    assert_eq!((status, hl, left), (ST_RANGE, 0xA200, 1));
    let _ = (kind, value);
    let (status, kind, value, hl, left) = rig.node(&[0x7F, 0xFF, 0x05], 0xA200);
    assert_eq!((status, kind, value, hl, left), (ST_OK, 0, 382, 0xA202, 1));
    assert_eq!(rig.node(&[], 0xA200).0, ST_RANGE, "no bytes left");
    let (status, kind, value, hl, left) = rig.node(&[0x05, 0x7F], 0xA200);
    assert_eq!((status, kind, value, hl, left), (ST_OK, 0, 5, 0xA201, 1));
    assert_eq!(
        rig.node(&[0xFF], 0xA200).0,
        ST_RANGE,
        "an escaped variable with no second byte"
    );
}

#[test]
fn an_index_or_variable_past_the_tables_is_refused_by_the_iterator() {
    let bytes = axioms(130);
    let mut rig = opened(&bytes);
    assert_eq!(rig.node(&[0x7F, 2], 0xA200).0, ST_OK);
    assert_eq!(rig.node(&[0x7F, 3], 0xA200).0, ST_BAD, "index 130 of 130");
    assert_eq!(rig.node(&[0x7E], 0xA200).0, ST_OK);
    assert_eq!(
        rig.node(&[0x80], 0xA200).0,
        ST_BAD,
        "no variable in this library"
    );
    let bytes = variables(130);
    let mut rig = opened(&bytes);
    let constants = decoded(&bytes).constants.len() as u16;
    assert_eq!(rig.node(&[0xFF, 2], 0xA200).2, constants + 129);
    assert_eq!(rig.node(&[0xFF, 3], 0xA200).0, ST_BAD, "symbol 132 of 132");
    assert_eq!(rig.node(&[0xFF, 0xFF], 0xA200).0, ST_BAD, "variable 382");
    assert_eq!(rig.node(&[0xFE], 0xA200).2, constants + 126);
    let bytes = variables(1);
    let mut rig = opened(&bytes);
    assert_eq!(rig.node(&[0x80], 0xA200).0, ST_OK);
    assert_eq!(rig.node(&[0x81], 0xA200).0, ST_BAD);
    let mut closed = Rig::with_library(&bytes);
    assert_eq!(closed.node(&[0x80], 0xA200).0, ST_CLOSED);
}
