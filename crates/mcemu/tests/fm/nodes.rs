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
