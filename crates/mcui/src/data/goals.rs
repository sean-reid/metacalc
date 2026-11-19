use crate::expr::{Expr, Shape};
use mcpack::mm::{Database, Kind, Node, StmtId, Sym, Tree};
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn shape(db: &Database, tree: &Tree) -> Shape {
    let stmt = db.stmt(tree.stmt);
    let name = |s: Sym| db.syms[s as usize].name.as_str();
    if stmt.kind == Kind::Float {
        return Shape::leaf(name(stmt.expr[1]));
    }
    let parts = stmt.expr[1..]
        .iter()
        .map(|&sym| {
            let slot = if db.syms[sym as usize].is_var {
                stmt.frame.hyps.iter().position(|&h| {
                    let h = db.stmt(h);
                    h.kind == Kind::Float && h.expr[1] == sym
                })
            } else {
                None
            };
            match slot {
                Some(slot) => shape(db, &tree.children[slot]),
                None => Shape::tok(name(sym)),
            }
        })
        .collect();
    Shape::node(parts)
}

pub(super) fn measure(node: &Rc<Node>, sizes: &mut HashMap<*const Node, u64>) -> u64 {
    if let Some(&size) = sizes.get(&Rc::as_ptr(node)) {
        return size;
    }
    let size = node
        .children
        .iter()
        .fold(1u64, |sum, c| sum.saturating_add(measure(c, sizes)));
    sizes.insert(Rc::as_ptr(node), size);
    size
}

/// How a goal was proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// The theorem's own hypothesis, numbered from one.
    Hypothesis(usize),
    /// An assertion of the library, by number.
    Assertion(usize),
}

pub struct GoalView {
    pub goal: Expr,
    pub by: By,
    pub subgoals: Vec<Expr>,
}

/// The goal tree of a theorem and what is needed to read its nodes.
pub struct Goals {
    pub theorem: usize,
    pub root: Rc<Node>,
    pub(super) floats: Vec<(Sym, StmtId)>,
    pub(super) hyps: Vec<StmtId>,
    pub(super) sizes: HashMap<*const Node, u64>,
}

impl Goals {
    /// Nodes in the tree, a node used twice counting twice.
    pub fn total(&self) -> u64 {
        self.sizes[&Rc::as_ptr(&self.root)]
    }

    /// The node reached by taking child `path[0]`, then child `path[1]`, and so on.
    pub fn node(&self, path: &[usize]) -> Rc<Node> {
        path.iter()
            .fold(self.root.clone(), |node, &k| node.children[k].clone())
    }

    /// The place of the node at `path` in reading order, from one.
    pub fn position(&self, path: &[usize]) -> u64 {
        let mut node = self.root.clone();
        let mut at: u64 = 1;
        for &k in path {
            at = node.children[..k]
                .iter()
                .fold(at.saturating_add(1), |sum, c| {
                    sum.saturating_add(self.sizes[&Rc::as_ptr(c)])
                });
            node = node.children[k].clone();
        }
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_over_a_tree_that_shares_its_children_saturate() {
        let mut node = Rc::new(Node {
            label: 0,
            expr: Vec::new(),
            children: Vec::new(),
        });
        for _ in 0..80 {
            node = Rc::new(Node {
                label: 0,
                expr: Vec::new(),
                children: vec![node.clone(), node],
            });
        }
        let mut sizes = HashMap::new();
        assert_eq!(measure(&node, &mut sizes), u64::MAX);
    }
}
