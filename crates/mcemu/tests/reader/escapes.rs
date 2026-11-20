use crate::sections::*;
use crate::support::*;
use crate::treelibs::*;

fn check_all(bytes: &[u8], what: &str) -> Rig {
    let mut rig = opened(bytes);
    check_sections(&mut rig, bytes, what, &ascending);
    rig
}

#[test]
fn syntax_indices_126_127_and_382_come_out_of_the_reader_as_the_decoder_decodes_them() {
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
        let (status, assertion, arity) = rig.syntax(index);
        assert_eq!((status, arity), (ST_OK, 0));
        assert_eq!(assertion, number(&bytes, &format!("w{index}")));
    }
    assert_eq!(rig.syntax_count(), (ST_OK, 383));
    let bytes = axioms(127);
    let mut rig = check_all(&bytes, "127 axioms");
    assert_eq!(rig.syntax_count(), (ST_OK, 127));
}

#[test]
fn variable_numbers_126_and_127_come_out_of_the_reader_as_the_decoder_decodes_them() {
    for n in [127, 130, 253] {
        let bytes = variables(n);
        let mut rig = check_all(&bytes, &format!("{n} variables"));
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
        }
    }
}

#[test]
fn a_tree_with_an_escaped_axiom_and_two_escaped_variables_reads_back() {
    let bytes = mixed();
    let mut rig = check_all(&bytes, "mixed");
    let top = number(&bytes, "ax-top");
    let got = rig.tree(top, 0xFF);
    assert_eq!(got.bytes, [0x7F, 3, 0xFF, 1, 0xFF, 2]);
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
        let after = BUF + len.min(255);
        rig.poke(after, &[0xA5]);
        let got = rig.tree(at, 0xFF);
        assert_eq!((got.status, got.len), (ST_OK, len), "{label}");
        assert_eq!(
            rig.peek(after, 1),
            [0xA5],
            "{label}: nothing past the tree is copied"
        );
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
