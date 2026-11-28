use crate::caps;
use crate::expr::{Expr, Shape};
use mcpack::mm::{Database, Kind, Node, StmtId, Sym, Tree};
use std::collections::{HashMap, HashSet};
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

/// Whether a proof tree is inside the caps of the walk the calculator reads a stored proof with:
/// distinct goals, subgoals of a goal, links and nodes.
pub(super) fn within_caps(root: &Rc<Node>, sizes: &HashMap<*const Node, u64>) -> bool {
    let mut seen: HashSet<*const Node> = HashSet::new();
    let mut todo = vec![root];
    let mut links = 0;
    while let Some(node) = todo.pop() {
        if !seen.insert(Rc::as_ptr(node)) {
            continue;
        }
        if seen.len() > caps::MAX_GOALS || node.children.len() > caps::MAX_SUBGOALS {
            return false;
        }
        links += node.children.len();
        if links > caps::MAX_LINKS {
            return false;
        }
        todo.extend(&node.children);
    }
    sizes[&Rc::as_ptr(root)] <= caps::MAX_PROOF_NODES
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

    fn node(children: Vec<Rc<Node>>) -> Rc<Node> {
        Rc::new(Node {
            label: 0,
            expr: Vec::new(),
            children,
        })
    }

    fn allowed(root: &Rc<Node>) -> bool {
        let mut sizes = HashMap::new();
        measure(root, &mut sizes);
        within_caps(root, &sizes)
    }

    /// A chain of `n` distinct goals, each taking the next.
    fn chain(n: usize) -> Rc<Node> {
        (1..n).fold(node(Vec::new()), |tail, _| node(vec![tail]))
    }

    #[test]
    fn a_tree_of_255_distinct_goals_is_inside_the_caps_and_one_of_256_is_not() {
        assert!(allowed(&chain(255)));
        assert!(!allowed(&chain(256)));
    }

    #[test]
    fn a_goal_with_31_subgoals_is_inside_the_caps_and_one_with_32_is_not() {
        let leaves = |n: usize| node((0..n).map(|_| node(Vec::new())).collect());
        assert!(allowed(&leaves(31)));
        assert!(!allowed(&leaves(32)));
    }

    /// A chain of goals that share one leaf: 31 links each, the last with `last`.
    fn links(whole: usize, last: usize) -> Rc<Node> {
        let leaf = node(Vec::new());
        let mut tail = node(vec![leaf.clone(); last]);
        for _ in 0..whole {
            let mut children = vec![leaf.clone(); 30];
            children.push(tail);
            tail = node(children);
        }
        tail
    }

    #[test]
    fn a_tree_of_857_links_is_inside_the_caps_and_one_of_858_is_not() {
        assert!(allowed(&links(27, 20)));
        assert!(!allowed(&links(27, 21)));
    }

    /// `levels` goals that each take the one below twice, over a leaf, and a root that takes the
    /// top one twice and `extra` more leaves: `2^(levels + 1) - 1` nodes for the top one.
    fn doubled(levels: usize, extra: usize) -> Rc<Node> {
        let leaf = node(Vec::new());
        let top = (0..levels).fold(leaf.clone(), |below, _| node(vec![below.clone(), below]));
        let mut children = vec![top.clone(), top];
        children.extend((0..extra).map(|_| leaf.clone()));
        node(children)
    }

    #[test]
    fn a_tree_of_16777215_nodes_is_inside_the_caps_and_one_of_16777216_is_not() {
        let at = doubled(22, 0);
        assert_eq!(at.children.len(), 2);
        let mut sizes = HashMap::new();
        assert_eq!(measure(&at, &mut sizes), 16_777_215);
        assert!(allowed(&at));
        let past = doubled(22, 1);
        let mut sizes = HashMap::new();
        assert_eq!(measure(&past, &mut sizes), 16_777_216);
        assert!(!allowed(&past));
    }
}
