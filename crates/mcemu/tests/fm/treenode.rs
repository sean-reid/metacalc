use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use crate::treelibs::*;
use mcpack::library::{Library, Node, TREES, decode};

const TREE_ERR: u8 = 9;
const AT: u16 = 0xA200;

/// The whole of a stored tree: what lib_tree copies and the rest read at its offset.
fn full_tree(rig: &mut Rig, n: u16, k: u8) -> Option<Vec<u8>> {
    let tree = rig.tree(n, k);
    if tree.status != ST_OK {
        return None;
    }
    let mut bytes = tree.bytes;
    while bytes.len() < usize::from(tree.len) {
        let take = (usize::from(tree.len) - bytes.len()).min(256) as u16;
        let (status, more) = rig.read(tree.offset + bytes.len() as u32, take);
        assert_eq!(status, ST_OK);
        bytes.extend(&more[..usize::from(take)]);
    }
    Some(bytes)
}

/// Every stored tree of the open library, or of assertion `only`, walked through fm_tree_node
/// against the nodes the host decoder reads from it, with the facts the host gives when
/// `host_facts` and those fm_open left otherwise. Returns how many trees it walked.
fn check_nodes(
    rig: &mut Rig,
    library: &Library,
    what: &str,
    only: Option<usize>,
    host_facts: bool,
) -> usize {
    if host_facts {
        rig.set_facts_of(library);
    }
    let Some(trees) = library.sections.as_ref().and_then(|s| s.trees.as_ref()) else {
        return 0;
    };
    let mut walked = 0;
    for (n, item) in trees.items.iter().enumerate() {
        if only.is_some_and(|only| only != n) {
            continue;
        }
        for (k, stored) in item.iter().enumerate() {
            let Some(tree) = stored else { continue };
            let asked = if k + 1 == item.len() { 0xFF } else { k as u8 };
            let bytes = full_tree(rig, n as u16, asked).expect("the reader gives the tree");
            let want: Vec<(u8, u16)> = tree
                .nodes
                .iter()
                .map(|node| match *node {
                    Node::Axiom(i) => (0, i as u16),
                    Node::Variable(id) => (1, id as u16),
                })
                .collect();
            assert_eq!(rig.walk(&bytes), (0, want), "{what}: tree {k} of {n}");
            walked += 1;
        }
    }
    walked
}

fn opened_fm(bytes: &[u8]) -> Fm {
    Fm::bare(bytes)
}

#[test]
fn every_tree_of_the_corpus_and_the_fixture_walks_to_the_nodes_the_decoder_reads() {
    let mut libraries: Vec<(String, Vec<u8>)> = CORPUS
        .iter()
        .map(|name| (name.to_string(), pack_all(&corpus(name))))
        .collect();
    libraries.push(("fixture".into(), fixture::nodes_library()));
    let mut total = 0;
    for (name, bytes) in &libraries {
        let mut fm = opened_fm(bytes);
        total += check_nodes(&mut fm.rig, &decoded(bytes), name, None, true);
    }
    assert!(total > 200, "{total} trees walked");
    let fixture = fixture::nodes_library();
    let mut fm = Fm::new(&fixture);
    let walked = check_nodes(&mut fm.rig, &decoded(&fixture), "fixture", None, false);
    assert!(walked > 100, "{walked} trees walked on what fm_open left");
}

#[test]
fn trees_of_201_to_507_bytes_walk_across_the_refill_as_the_decoder_reads_them() {
    let bytes = long_trees();
    let mut fm = opened_fm(&bytes);
    let walked = check_nodes(&mut fm.rig, &decoded(&bytes), "long trees", None, true);
    assert!(walked > 10, "{walked} trees walked");
    for (label, len, between) in [
        ("ax-long-100", 201usize, None),
        ("ax-mixed", 254, None),
        ("ax-long-127", 255, None),
        ("ax-shift", 256, Some(true)),
        ("ax-long-128", 257, Some(false)),
        ("ax-long-200", 401, Some(false)),
        ("ax-long-253", 507, Some(false)),
    ] {
        let tree = full_tree(&mut fm.rig, number(&bytes, label), 0xFF).unwrap();
        assert_eq!(tree.len(), len, "{label}");
        let (mut at, mut bounds) = (0, Vec::new());
        while at < tree.len() {
            let end = tree.len().min(at + 2);
            let node = fm.rig.node(&tree[at..end], AT);
            assert_eq!(node.0, 0, "{label}");
            at = end - usize::from(node.4);
            bounds.push(at);
        }
        if let Some(between) = between {
            assert_eq!(bounds.contains(&255), between, "{label}: the cut at 255");
        }
    }
}

#[test]
fn fm_nodes_reads_trees_of_255_256_and_257_bytes_that_the_refill_cuts_at_a_node_and_inside_one() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    for label in ["t255", "t256", "t257"] {
        let n = number(&bytes, label);
        assert_eq!(fm.statement(n, 0xFF).status(), 0, "{label}");
        assert_eq!(
            fm.nodes().status(),
            0,
            "{label}: fm_nodes through the refill"
        );
        let walked = check_nodes(&mut fm.rig, &library, label, Some(usize::from(n)), false);
        assert_eq!(walked, 1, "{label}");
    }
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_tree_of_the_shipped_libraries_walks_to_the_nodes_the_decoder_reads() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let mut fm = Fm::new(&bytes);
        let (_, _, constants, variables) = fm.rig.counts();
        assert_eq!(fm.memory("FV_CONSTS", 1), [constants], "{name}");
        assert_eq!(fm.memory("FV_NSYMS", 1), [constants + variables], "{name}");
        let walked = check_nodes(&mut fm.rig, &decoded(&bytes), name, None, false);
        println!("{name}: {walked} trees walked");
        assert!(walked > 100, "{name}: {walked}");
    }
}

#[test]
fn fm_open_leaves_the_arities_and_the_counts_a_tree_is_decoded_against() {
    for bytes in [fixture::nodes_library(), fixture::library()] {
        let mut fm = Fm::new(&bytes);
        let arities: Vec<u8> = decoded(&bytes)
            .sections
            .unwrap()
            .trees
            .unwrap()
            .syntax
            .iter()
            .map(|s| s.arity as u8)
            .collect();
        assert_eq!(fm.memory("FM_PILE", arities.len()), arities);
        let (_, _, constants, variables) = fm.rig.counts();
        let (_, syntax) = fm.rig.syntax_count();
        assert!(syntax > 3 && constants > 3 && variables > 3);
        assert_eq!(fm.memory("FV_NS", 2), syntax.to_le_bytes());
        assert_eq!(fm.memory("FV_CONSTS", 1), [constants]);
        assert_eq!(
            fm.memory("FV_NSYMS", 1),
            [constants + variables],
            "the glyph map's symbol count"
        );
    }
}

#[test]
fn syntax_indices_126_127_128_and_382_decode_as_the_decoder_decodes_them() {
    let bytes = axioms(383);
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&decoded(&bytes));
    for (index, tree) in [
        (126u16, vec![0x7E]),
        (127, vec![0x7F, 0]),
        (128, vec![0x7F, 1]),
        (382, vec![0x7F, 255]),
    ] {
        let n = number(&bytes, &format!("ax-{index}"));
        assert_eq!(full_tree(&mut fm.rig, n, 0xFF), Some(tree.clone()));
        assert_eq!(fm.rig.walk(&tree), (0, vec![(0, index)]), "index {index}");
    }
    let bytes = axioms(127);
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&decoded(&bytes));
    assert_eq!(fm.rig.walk(&[0x7E]), (0, vec![(0, 126)]));
    assert_eq!(
        fm.rig.walk(&[0x7F, 0]).0,
        TREE_ERR,
        "index 127 is past a table of 127"
    );
}

#[test]
fn variable_numbers_126_and_127_decode_as_the_decoder_decodes_them() {
    for n in [127usize, 130, 253] {
        let bytes = variables(n);
        let library = decoded(&bytes);
        let constants = library.constants.len() as u16;
        let mut fm = opened_fm(&bytes);
        fm.rig.set_facts_of(&library);
        for var in [0u16, 126, 127, n as u16 - 1] {
            if var >= n as u16 {
                continue;
            }
            let at = number(&bytes, &format!("ax-v{var}"));
            let tree = full_tree(&mut fm.rig, at, 0xFF).unwrap();
            assert_eq!(
                fm.rig.walk(&tree),
                (0, vec![(1, constants + var)]),
                "{n}: v{var}"
            );
        }
    }
}

#[test]
fn a_tree_with_an_escaped_axiom_and_two_escaped_variables_decodes() {
    let bytes = mixed();
    let library = decoded(&bytes);
    let constants = library.constants.len() as u16;
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&library);
    let tree = full_tree(&mut fm.rig, number(&bytes, "ax-top"), 0xFF).unwrap();
    assert_eq!(tree, [0x7F, 3, 0xFF, 1, 0xFF, 2]);
    assert_eq!(
        fm.rig.walk(&tree),
        (
            0,
            vec![(0, 130), (1, constants + 128), (1, constants + 129)]
        )
    );
}

#[test]
fn a_node_the_buffer_cuts_in_two_is_refused_and_one_that_fits_is_read() {
    let bytes = axioms(383);
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&decoded(&bytes));
    let rig = &mut fm.rig;
    for (what, bytes) in [
        ("an escape with no second byte", &[0x7F][..]),
        ("an escaped variable with no second byte", &[0xFF]),
        ("no bytes left", &[]),
    ] {
        let out = rig.node_out(bytes, AT);
        assert_eq!(out.status(), TREE_ERR, "{what}");
        assert_eq!(out.af & 0x40, 0, "{what}: the zero flag is clear");
        assert_eq!(out.hl, AT, "{what}: HL is as it came in");
        assert_eq!(
            usize::from(out.bc as u8),
            bytes.len(),
            "{what}: C is as it came in"
        );
    }
    assert_eq!(rig.node(&[0x7F, 0xFF, 0x05], AT), (0, 0, 382, AT + 2, 1));
    assert_eq!(rig.node(&[0x05, 0x7F], AT), (0, 0, 5, AT + 1, 1));
    let good = rig.node_out(&[0x05], AT);
    assert_eq!((good.status(), good.af & 0x40), (0, 0x40), "a node: Z set");
    let bad = rig.node_out(&[0x80], AT);
    assert_eq!(
        (bad.status(), bad.af & 0x40),
        (TREE_ERR, 0),
        "a variable in a library with none: Z clear"
    );
}

#[test]
fn an_index_or_variable_past_the_counts_is_refused() {
    let bytes = axioms(130);
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&decoded(&bytes));
    let rig = &mut fm.rig;
    assert_eq!(rig.node(&[0x7F, 2], AT).0, 0);
    assert_eq!(rig.node(&[0x7F, 3], AT).0, TREE_ERR, "index 130 of 130");
    assert_eq!(rig.node(&[0x7F, 4], AT).0, TREE_ERR, "index 131 of 130");
    assert_eq!(rig.node(&[0x7F, 255], AT).0, TREE_ERR, "index 382 of 130");
    let bytes = axioms(10);
    let mut small = opened_fm(&bytes);
    small.rig.set_facts_of(&decoded(&bytes));
    assert_eq!(small.rig.node(&[0x09], AT).0, 0, "index 9 of 10");
    assert_eq!(small.rig.node(&[0x0A], AT).0, TREE_ERR, "index 10 of 10");
    assert_eq!(small.rig.node(&[0x0B], AT).0, TREE_ERR, "index 11 of 10");
    assert_eq!(small.rig.node(&[0x05], AT).0, 0, "index 5 of 10");
    assert_eq!(small.rig.node(&[0x7E], AT).0, TREE_ERR, "index 126 of 10");
    assert_eq!(
        small.rig.node(&[0x7F, 4], AT).0,
        TREE_ERR,
        "index 131 of 10"
    );
    let rig = &mut fm.rig;
    assert_eq!(rig.node(&[0x7E], AT).0, 0);
    assert_eq!(
        rig.node(&[0x80], AT).0,
        TREE_ERR,
        "no variable in this library"
    );
    let bytes = variables(130);
    let library = decoded(&bytes);
    let constants = library.constants.len() as u16;
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&library);
    let rig = &mut fm.rig;
    assert_eq!(rig.node(&[0xFF, 2], AT).2, constants + 129);
    assert_eq!(rig.node(&[0xFF, 3], AT).0, TREE_ERR, "symbol 132 of 132");
    assert_eq!(rig.node(&[0xFF, 0xFF], AT).0, TREE_ERR, "variable 382");
    assert_eq!(rig.node(&[0xFE], AT).2, constants + 126);
    let bytes = variables(1);
    let mut fm = opened_fm(&bytes);
    fm.rig.set_facts_of(&decoded(&bytes));
    assert_eq!(fm.rig.node(&[0x80], AT).0, 0);
    assert_eq!(fm.rig.node(&[0x81], AT).0, TREE_ERR);
}

#[test]
fn a_variable_number_that_added_to_the_constants_passes_255_is_refused() {
    let mut fm = opened_fm(&axioms(3));
    fm.rig.set_tree_facts(3, 250, 255);
    assert_eq!(fm.rig.node(&[0x84], AT).2, 254);
    assert_eq!(fm.rig.node(&[0x85], AT).0, TREE_ERR, "symbol 255 of 255");
    assert_eq!(fm.rig.node(&[0x8A], AT).0, TREE_ERR, "250 and 10 pass 255");
    assert_eq!(fm.rig.node(&[0xFE], AT).0, TREE_ERR, "250 and 126 pass 255");
}

/// The bytes of the trees section that hold trees, and where each assertion's item starts.
fn tree_items(bytes: &[u8]) -> (usize, usize, Vec<usize>) {
    let library = decoded(bytes);
    let sections = library.sections.as_ref().unwrap();
    let row = sections.table.iter().find(|e| e.id == TREES).unwrap();
    let syntax = sections.trees.as_ref().unwrap().syntax.len();
    let index = row.offset + 2 + 3 * syntax;
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let starts = (0..library.assertions.len())
        .map(|i| word(index + 4 * i))
        .collect();
    (
        index + 4 * library.assertions.len(),
        row.offset + row.length,
        starts,
    )
}

#[test]
fn a_changed_byte_in_a_tree_walks_to_the_hosts_nodes_or_is_refused_by_fm_nodes() {
    let good = pack_all(&parse(
        "$c wff |- ( ) -> & -. $.\n$v p q r s $.\n\
         wp $f wff p $.\nwq $f wff q $.\nwr $f wff r $.\nws $f wff s $.\n\
         wn $a wff -. p $.\nwi $a wff ( p -> q ) $.\nwa $a wff ( p & q ) $.\n\
         ax1 $a |- ( p -> q ) $.\nax2 $a |- ( p & ( q -> r ) ) $.\n\
         ${\nh $e |- ( p -> q ) $.\nax3 $a |- -. ( p & -. q ) $.\n$}\n",
    ));
    let (first, end, starts) = tree_items(&good);
    let all = statements(&decoded(&good));
    let model = Model::new(&good);
    let library = decoded(&good);
    let syntax = library
        .sections
        .as_ref()
        .unwrap()
        .trees
        .as_ref()
        .unwrap()
        .syntax
        .len();
    let consts = library.constants.len();
    let symbols = consts + library.variables.len();
    let mut fm = Fm::new(&good);
    let (mut host_ok, mut host_refused, mut outcomes) = (0, 0, [0usize; 4]);
    for at in first..end {
        let item = starts.iter().rposition(|&s| s <= at).unwrap();
        for edit in EDITS {
            let new = edit.apply(good[at]);
            if new == good[at] {
                continue;
            }
            let bytes = fixture::with(&good, |b| b[at] = new);
            let what = format!("byte {at:#x} {:#04x} -> {new:#04x}", good[at]);
            fm.rig.load(&bytes);
            assert_eq!(fm.rig.open(), 0, "{what}: the reader opens");
            match decode(&bytes) {
                Ok(library) => {
                    host_ok += 1;
                    check_nodes(&mut fm.rig, &library, &what, Some(item), false);
                }
                Err(_) => {
                    host_refused += 1;
                    for (n, k, _) in all.iter().filter(|(n, ..)| usize::from(*n) == item) {
                        let tree = full_tree(&mut fm.rig, *n, *k);
                        let mut walked = 0;
                        if let Some(tree) = &tree {
                            let (_, nodes) = fm.rig.walk(tree);
                            walked = nodes.len();
                            for (kind, value) in nodes {
                                let ok = match kind {
                                    0 => usize::from(value) < syntax,
                                    _ => (consts..symbols).contains(&usize::from(value)),
                                };
                                assert!(ok, "{what}: node ({kind}, {value}) is out of range");
                            }
                        }
                        assert_eq!(fm.statement(*n, *k).status(), 0, "{what}");
                        let out = fm.nodes();
                        let status = out.status();
                        if status == 0 {
                            outcomes[0] += 1;
                            assert_eq!(
                                fm.table(count(out)),
                                model.table(*n, *k),
                                "{what}: fm builds what it built before"
                            );
                        } else if status == TREE_ERR {
                            outcomes[1] += 1;
                            assert!(tree.is_some(), "{what}: the reader refuses this tree");
                        } else if status == code("FM_ERR_READ") {
                            outcomes[2] += 1;
                            assert!(tree.is_none(), "{what}: the reader gives this tree");
                        } else if status == code("FM_ERR_LIMIT") {
                            outcomes[3] += 1;
                            assert!(walked > 255, "{what}: only {walked} nodes");
                        } else {
                            panic!("{what}: status {status}");
                        }
                    }
                }
            }
        }
    }
    println!(
        "{host_ok} mutants the host decodes, {host_refused} it refuses; fm_nodes built {} tables, refused {} with FM_ERR_TREE, {} with FM_ERR_READ, {} with FM_ERR_LIMIT",
        outcomes[0], outcomes[1], outcomes[2], outcomes[3]
    );
    assert_eq!((host_ok, host_refused), (14, 107));
    assert_eq!(outcomes, [28, 86, 27, 0]);
}
