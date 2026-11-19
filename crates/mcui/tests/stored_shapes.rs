//! The shapes section gives every statement the same subexpression tree as the parts derived from
//! the syntax axioms' records.

mod support;

use mcpack::library::{self, Part, Trees};
use mcui::expr::Shape;
use support::{LOGIC, db, extra_bytes, pack_all};

/// The parts of the syntax axiom at `index`, read from the stored shape and the axiom's
/// conclusion in the record.
fn from_shape(library: &library::Library, trees: &Trees, index: usize) -> Vec<Part> {
    let shapes = library.sections.as_ref().unwrap().shapes.as_ref().unwrap();
    let axiom = &trees.syntax[index];
    let tokens = &library.assertions[axiom.assertion].conclusion[1..];
    assert_eq!(tokens.len(), shapes.axioms[index].len());
    shapes.axioms[index]
        .iter()
        .zip(tokens)
        .map(|(&byte, token)| match byte {
            0 => Part::Token(token.clone()),
            k => Part::Child(usize::from(k) - 1),
        })
        .collect()
}

fn shape_of(
    library: &library::Library,
    trees: &Trees,
    tree: &library::Tree,
    parts: impl Fn(usize) -> Vec<Part>,
) -> Shape {
    let names: Vec<&String> = library
        .constants
        .iter()
        .chain(library.variables.iter().map(|v| &v.name))
        .collect();
    let order: Vec<usize> = tree
        .nodes
        .iter()
        .filter_map(|n| match n {
            library::Node::Axiom(i) => Some(*i),
            library::Node::Variable(_) => None,
        })
        .collect();
    let mut index = 0;
    trees
        .fold(
            tree,
            |id| Shape::leaf(names[id]),
            |_, mut children| {
                index += 1;
                let axiom = order[order.len() - index];
                let parts = parts(axiom)
                    .iter()
                    .map(|part| match part {
                        Part::Token(token) => Shape::tok(token),
                        Part::Child(k) => {
                            std::mem::replace(&mut children[*k], Shape::Node(Vec::new()))
                        }
                    })
                    .collect();
                Shape::node(parts)
            },
        )
        .expect("the tree folds")
}

fn compare(bytes: &[u8]) -> usize {
    let library = library::decode(bytes).unwrap();
    let trees = library.sections.as_ref().unwrap().trees.as_ref().unwrap();
    let mut compared = 0;
    for (assertion, item) in library.assertions.iter().zip(&trees.items) {
        for tree in item.iter().flatten() {
            let derived = shape_of(&library, trees, tree, |i| trees.syntax[i].parts.clone());
            let stored = shape_of(&library, trees, tree, |i| from_shape(&library, trees, i));
            assert_eq!(stored, derived, "{}", assertion.label);
            compared += 1;
        }
    }
    compared
}

#[test]
fn the_stored_shapes_give_every_statement_the_derived_shape() {
    let logic = pack_all(&db(LOGIC)).expect("the logic packs");
    assert!(compare(&logic) > 20);
    assert!(compare(&extra_bytes()) > 25);
}
