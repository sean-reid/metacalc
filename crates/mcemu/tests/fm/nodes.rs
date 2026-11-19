use crate::fixture;
use crate::rig::*;
use crate::support::*;
use mcui::Status;
use mcui::data::{Installed, Lib};
use mcui::expr::Expr;
use mcui::fold::{Folds, layout};

/// (first token, last token, depth) of each node.
pub type Table = Vec<(usize, usize, usize)>;

/// The host model's expressions for a library's statements.
pub struct Model {
    lib: Lib,
}

impl Model {
    pub fn new(bytes: &[u8]) -> Model {
        let status = vec![Status::Unchecked; decoded(bytes).assertions.len()];
        let installed = Installed {
            name: "library".into(),
            bytes: bytes.to_vec(),
        };
        Model {
            lib: Lib::new(installed, status).expect("the model loads the library"),
        }
    }

    /// The nodes of statement `k` of assertion `n`, from the model's `Expr`.
    pub fn table(&self, n: u16, k: u8) -> Table {
        let statement = self.lib.statement(usize::from(n));
        let expr = match k {
            0xFF => &statement.conclusion,
            _ => &statement.hyps[usize::from(k)],
        };
        spans(expr)
    }
}

/// Each node's span as the layout of the unfolded expression gives it: the tokens its subtree
/// owns, the lead being node 0's.
fn spans(expr: &Expr) -> Table {
    let laid = layout(expr, &Folds::none(expr), 94);
    let mut spans = vec![(usize::MAX, 0); expr.len()];
    for piece in 0..laid.pieces.len() {
        let span = &mut spans[laid.owner(piece)];
        *span = (span.0.min(piece), span.1.max(piece));
    }
    for node in (0..expr.len()).rev() {
        for &child in expr.children(node) {
            let inner = spans[child];
            spans[node] = (spans[node].0.min(inner.0), spans[node].1.max(inner.1));
        }
    }
    (0..expr.len())
        .map(|n| (spans[n].0, spans[n].1, expr.depth(n)))
        .collect()
}

impl Fm {
    /// Loads the statement, builds its nodes, and returns the table fm holds.
    pub fn built(&mut self, n: u16, k: u8) -> Table {
        let what = format!("statement {k} of assertion {n}");
        assert_eq!(self.statement(n, k).status(), 0, "{what}");
        let out = self.nodes();
        assert_eq!(out.status(), 0, "{what}: fm_nodes");
        self.table(count(out))
    }
}

#[test]
fn the_node_table_is_the_models_for_every_statement_of_the_fixture() {
    let bytes = fixture::nodes_library();
    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let all = statements(&decoded(&bytes));
    for (n, k, tokens) in &all {
        let want = model.table(*n, *k);
        let got = fm.built(*n, *k);
        assert_eq!(got, want, "statement {k} of assertion {n}: {tokens:?}");
        let again = fm.nodes();
        assert_eq!((again.status(), count(again)), (0, want.len()));
    }
    assert!(all.len() > 90, "{} statements", all.len());
}

#[test]
fn the_fixture_holds_every_structure_the_table_has_to_get_right() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let shapes = library.sections.as_ref().unwrap().shapes.as_ref().unwrap();
    let children = |shape: &Vec<u8>| {
        shape
            .iter()
            .copied()
            .filter(|&p| p != 0)
            .collect::<Vec<_>>()
    };
    assert!(
        shapes.axioms.iter().any(|s| {
            let order = children(s);
            order.windows(2).any(|w| w[0] > w[1])
        }),
        "an axiom whose slots are out of child order"
    );
    let wrappers = shapes
        .axioms
        .iter()
        .filter(|s| s.len() == 1 && s[0] != 0)
        .count();
    assert!(
        wrappers >= 2,
        "{wrappers} wrapper axioms of one slot and no constant"
    );
    let one_constant = shapes.axioms.iter().filter(|s| **s == [0]).count();
    assert!(
        one_constant > 100,
        "{one_constant} axioms of one constant and no slot"
    );

    let model = Model::new(&bytes);
    let mut fm = Fm::new(&bytes);
    let all = statements(&library);
    let at = |label: &str| {
        library
            .assertions
            .iter()
            .position(|a| a.label == label)
            .unwrap() as u16
    };
    let trees = library.sections.as_ref().unwrap().trees.as_ref().unwrap();
    let (mut tall, mut long, mut without, mut mixed) = (0, 0, 0, 0);
    for (n, k, _) in &all {
        let tree = fm.rig.tree(*n, *k);
        without += usize::from(tree.status == 9);
        long += usize::from(tree.len > 255);
        tall += usize::from(model.table(*n, *k).iter().any(|row| row.2 > 13));
        let items = &trees.items[usize::from(*n)];
        let missing = items.iter().any(|t| t.is_none());
        mixed += usize::from(missing && tree.status == 0);
    }
    assert!(without >= 1, "a statement with no tree");
    assert!(
        mixed >= 1,
        "a statement with a tree in an assertion with a statement that has none"
    );
    assert_eq!(long, 3, "trees over 255 bytes: 256, 257 and 507");
    assert!(tall >= 2, "{tall} statements deeper than 13");
    let deepest = all
        .iter()
        .flat_map(|(n, k, _)| model.table(*n, *k))
        .map(|row| row.2);
    assert_eq!(deepest.max(), Some(fixture::CHAIN), "the deepest node");
    let root = at("nest-root");
    assert_eq!(
        fm.rig.tree(root, 0xFF).len,
        3,
        "a wrapper of a wrapper over a variable"
    );
    assert_eq!(model.table(root, 0xFF), [(0, 1, 0)]);
    let below = at("nest-below");
    assert_eq!(
        model.table(below, 0xFF),
        [(0, 5, 0), (2, 2, 1), (4, 4, 1)],
        "a wrapper chain below a root"
    );
}

#[test]
fn a_library_without_trees_and_shapes_opens_and_every_statement_is_one_node() {
    let bytes = fixture::unstructured();
    let mut fm = Fm::new(&bytes);
    assert_eq!((fm.rig.shapes().0, fm.rig.syntax_count().0), (9, 9));
    assert_eq!(fm.rig.sections().1 & 2, 2, "it has a glyph map");
    for (n, k, tokens) in statements(&decoded(&bytes)) {
        let want = vec![(0, tokens.len() - 1, 0)];
        assert_eq!(fm.built(n, k), want, "statement {k} of assertion {n}");
    }
}

#[test]
fn a_statement_the_library_has_no_tree_for_is_one_node_over_every_token() {
    let bytes = fixture::library();
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    let wide = library
        .assertions
        .iter()
        .position(|a| a.label == "wide")
        .unwrap() as u16;
    assert_eq!(fm.rig.tree(wide, 0xFF).status, 9);
    let tokens = library.assertions[usize::from(wide)].conclusion.len();
    assert_eq!(fm.built(wide, 0xFF), [(0, tokens - 1, 0)]);
}

#[test]
fn the_table_is_built_once_for_a_loaded_statement_and_a_new_statement_unbuilds_it() {
    let bytes = fixture::library();
    let library = decoded(&bytes);
    let mut fm = Fm::new(&bytes);
    let built = |fm: &mut Fm| fm.memory("FV_NODES", 1)[0];
    let (n, k, _) = statements(&library)
        .into_iter()
        .find(|s| s.2.len() > 9)
        .unwrap();
    assert_eq!(fm.nodes().status(), code("FM_ERR_INDEX"));
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(built(&mut fm), 0);
    let first = fm.nodes();
    assert_eq!((first.status(), built(&mut fm)), (0, count(first) as u8));
    fm.rows(94);
    fm.draw(1, 0, 94, (0, 8));
    let table = fm.table(count(first));
    let second = fm.nodes();
    assert!(
        second.tstates < first.tstates / 4,
        "the second call reads nothing"
    );
    assert_eq!(
        (count(second), fm.table(count(second))),
        (count(first), table)
    );
    assert_eq!(fm.statement(n, k).status(), 0);
    assert_eq!(built(&mut fm), 0);
    assert_eq!(fm.open().status(), 0);
    assert_eq!(
        fm.nodes().status(),
        code("FM_ERR_INDEX"),
        "open leaves no statement"
    );
}

#[test]
fn the_deepest_tree_leaves_six_bytes_of_the_explicit_stack_and_writes_only_fms_memory() {
    let bytes = fixture::nodes_library();
    let library = decoded(&bytes);
    let deep = library
        .assertions
        .iter()
        .position(|a| a.label == "deep")
        .unwrap() as u16;
    let mut fm = Fm::opened_primed(&bytes);
    assert_eq!(fm.statement(deep, 0xFF).status(), 0);
    let pile = usize::from(fm.rig.label("FM_PILE") - SPARE_BASE);
    let size = constant_of(WITH_FM.labels, "FM_PILE_SIZE") as usize;
    fm.rig.m.ram_page(SPARE_PAGE)[pile..pile + size].fill(0xA5);
    let out = fm.nodes();
    assert_eq!((out.status(), count(out)), (0, fixture::CHAIN + 1));
    let page = fm.rig.m.ram_page(SPARE_PAGE);
    let used = page[pile..pile + size].iter().rposition(|&b| b != 0xA5);
    assert_eq!(used.map(|at| at + 1), Some(2 * fixture::CHAIN));
    let found = crate::guard::stray(&mut fm.rig, &crate::session::allowed());
    assert!(found.is_empty(), "stray writes at {found:x?}");
}

fn named(fm: &mut Fm, bytes: &[u8], label: &str) -> Table {
    let n = decoded(bytes)
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap() as u16;
    fm.built(n, 0xFF)
}

/// The nodes of a statement that is `operators` prefix operators in front of a variable.
fn chain(operators: usize) -> Table {
    let last = operators + 1;
    let mut rows = vec![(0, last, 0)];
    rows.extend((1..=operators).map(|j| (j + 1, last, j)));
    rows
}

#[test]
fn a_wrapper_has_no_node_and_a_one_constant_axiom_has_one() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    assert_eq!(named(&mut fm, &bytes, "nest-root"), [(0, 1, 0)]);
    assert_eq!(
        named(&mut fm, &bytes, "nest-below"),
        [(0, 5, 0), (2, 2, 1), (4, 4, 1)]
    );
    assert_eq!(
        named(&mut fm, &bytes, "onec"),
        [(0, 5, 0), (2, 2, 1), (4, 4, 1)],
        "F3 is an axiom of one constant and no slot, so a node"
    );
    assert_eq!(
        named(&mut fm, &bytes, "wrapc"),
        [
            (0, 13, 0),
            (2, 10, 1),
            (3, 3, 2),
            (5, 9, 2),
            (6, 6, 3),
            (8, 8, 3),
            (12, 12, 1)
        ],
        "a class abstraction under an implication, its wrapper collapsed, then its sibling"
    );
}

#[test]
fn trees_of_255_256_and_257_bytes_split_across_the_buffer_where_the_chain_says() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    for (label, bytes_long, operators) in
        [("t255", 255, 127), ("t256", 256, 128), ("t257", 257, 129)]
    {
        let n = decoded(&bytes)
            .assertions
            .iter()
            .position(|a| a.label == label)
            .unwrap() as u16;
        assert_eq!(fm.rig.tree(n, 0xFF).len, bytes_long, "{label}");
        assert_eq!(fm.built(n, 0xFF), chain(operators), "{label}");
    }
}

#[test]
fn the_symbol_id_of_a_variable_in_a_tree_is_always_one_byte() {
    let bytes = fixture::nodes_library();
    let mut fm = Fm::new(&bytes);
    let counts = fm.rig.counts();
    let symbols = u16::from(counts.0) + counts.1;
    let mut seen = 0;
    for first in 0x80..=0xFFu8 {
        for second in 0..=255u8 {
            let (status, kind, value, ..) = fm.rig.node(&[first, second], 0xA100);
            if status == 0 && kind == 1 {
                assert!(
                    value < symbols && value < 256,
                    "{first:x} {second:x}: {value}"
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 100, "{seen} variable nodes read");
}

#[test]
fn a_refill_that_ends_where_the_library_does_is_read_and_one_byte_short_is_not() {
    let bytes = fixture::nodes_library();
    let n = decoded(&bytes)
        .assertions
        .iter()
        .position(|a| a.label == "deep")
        .unwrap() as u16;
    for (cut, status) in [(0, 0), (1, code("FM_ERR_READ"))] {
        let mut fm = Fm::new(&bytes);
        assert_eq!(fm.statement(n, 0xFF).status(), 0);
        let end = fm.rig.tree(n, 0xFF).offset + 2 * fixture::CHAIN as u32 + 1 - cut;
        let at = fm.rig.label("LR_LENGTH");
        fm.rig.poke(at, &end.to_le_bytes()[..3]);
        assert_eq!(fm.nodes().status(), status, "{cut} bytes short");
    }
}

#[test]
fn a_shapes_section_of_2048_bytes_is_not_too_big_and_one_of_2049_is() {
    let bytes = fixture::library();
    let state = |length: u16| {
        let mut fm = Fm::bare(&bytes);
        let row = fm.rig.label("LS_SH") + 3;
        fm.rig.poke(row, &length.to_le_bytes());
        fm.open().status()
    };
    assert_eq!(state(2049), code("FM_ERR_TOO_BIG"));
    assert_eq!(state(0xFFFF), code("FM_ERR_TOO_BIG"));
    assert_ne!(state(2048), code("FM_ERR_TOO_BIG"));
}
