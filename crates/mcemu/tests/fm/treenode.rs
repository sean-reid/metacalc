use crate::fixture;
use crate::rig::*;
use crate::support::*;
use crate::treelibs::*;
use mcpack::library::{Library, Node};

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

/// Every stored tree of the open library, walked through fm_tree_node against the nodes the
/// host decoder reads from it. Returns how many trees it walked.
fn check_nodes(rig: &mut Rig, library: &Library, what: &str) -> usize {
    rig.set_facts_of(library);
    let Some(trees) = library.sections.as_ref().and_then(|s| s.trees.as_ref()) else {
        return 0;
    };
    let mut walked = 0;
    for (n, item) in trees.items.iter().enumerate() {
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
        total += check_nodes(&mut fm.rig, &decoded(bytes), name);
    }
    assert!(total > 200, "{total} trees walked");
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn every_tree_of_the_shipped_libraries_walks_to_the_nodes_the_decoder_reads() {
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    for name in ["prop", "pred", "ring"] {
        let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let mut fm = opened_fm(&bytes);
        let walked = check_nodes(&mut fm.rig, &decoded(&bytes), name);
        println!("{name}: {walked} trees walked");
        assert!(walked > 100, "{name}: {walked}");
    }
}

#[test]
fn fm_open_leaves_the_arities_and_the_counts_a_tree_is_decoded_against() {
    for bytes in [fixture::nodes_library(), fixture::library()] {
        let mut fm = Fm::new(&bytes);
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
    assert_eq!(
        rig.node(&[0x7F], AT).0,
        TREE_ERR,
        "an escape with no second byte"
    );
    assert_eq!(rig.node(&[0x7F, 0xFF, 0x05], AT), (0, 0, 382, AT + 2, 1));
    assert_eq!(rig.node(&[], AT).0, TREE_ERR, "no bytes left");
    assert_eq!(rig.node(&[0x05, 0x7F], AT), (0, 0, 5, AT + 1, 1));
    assert_eq!(
        rig.node(&[0xFF], AT).0,
        TREE_ERR,
        "an escaped variable with no second byte"
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
