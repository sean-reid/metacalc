//! A statement as a tree of subexpressions, the unit the cursor moves over and folding hides.

/// The shape an [`Expr`] is built from: a constant token, or a subexpression of parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Tok(String),
    Node(Vec<Shape>),
}

impl Shape {
    /// A subexpression that is a single token, such as a variable.
    pub fn leaf(token: &str) -> Shape {
        Shape::Node(vec![Shape::Tok(token.to_string())])
    }

    pub fn node(parts: Vec<Shape>) -> Shape {
        Shape::Node(parts)
    }

    pub fn tok(token: &str) -> Shape {
        Shape::Tok(token.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Part {
    Tok(String),
    Child(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub parts: Vec<Part>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub depth: usize,
}

/// A statement's subexpressions in preorder, node 0 being the whole expression. `lead` holds the
/// tokens before it, the typecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub(crate) lead: Vec<String>,
    pub(crate) nodes: Vec<Node>,
}

impl Expr {
    pub fn build(lead: Vec<String>, root: &Shape) -> Expr {
        let mut expr = Expr {
            lead,
            nodes: Vec::new(),
        };
        match root {
            Shape::Node(parts) => {
                expr.add(parts, None, 0);
            }
            Shape::Tok(_) => {
                expr.add(std::slice::from_ref(root), None, 0);
            }
        }
        expr
    }

    /// An expression with no structure: one node holding every token.
    pub fn flat(lead: Vec<String>, tokens: &[String]) -> Expr {
        let root = Shape::Node(tokens.iter().map(|t| Shape::Tok(t.clone())).collect());
        Expr::build(lead, &root)
    }

    fn add(&mut self, parts: &[Shape], parent: Option<usize>, depth: usize) -> usize {
        if let [Shape::Node(inner)] = parts {
            return self.add(inner, parent, depth);
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            parts: Vec::new(),
            parent,
            children: Vec::new(),
            depth,
        });
        let mut built = Vec::with_capacity(parts.len());
        for part in parts {
            match part {
                Shape::Tok(t) => built.push(Part::Tok(t.clone())),
                Shape::Node(inner) => {
                    let child = self.add(inner, Some(id), depth + 1);
                    self.nodes[id].children.push(child);
                    built.push(Part::Child(child));
                }
            }
        }
        self.nodes[id].parts = built;
        id
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn parent(&self, node: usize) -> Option<usize> {
        self.nodes[node].parent
    }

    /// The subexpressions directly inside `node`, left to right.
    pub fn children(&self, node: usize) -> &[usize] {
        &self.nodes[node].children
    }

    pub fn depth(&self, node: usize) -> usize {
        self.nodes[node].depth
    }

    /// Whether folding `node` hides anything.
    pub fn foldable(&self, node: usize) -> bool {
        !self.nodes[node].children.is_empty()
    }

    /// The node after `node` among the children of its parent, or the previous one.
    pub fn sibling(&self, node: usize, forward: bool) -> Option<usize> {
        let siblings = self.children(self.parent(node)?);
        let at = siblings.iter().position(|&n| n == node)?;
        if forward {
            siblings.get(at + 1).copied()
        } else {
            at.checked_sub(1).map(|i| siblings[i])
        }
    }

    /// Every token in reading order, the lead first.
    pub fn tokens(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.lead.iter().map(String::as_str).collect();
        self.collect(0, &mut out);
        out
    }

    fn collect<'a>(&'a self, node: usize, out: &mut Vec<&'a str>) {
        for part in &self.nodes[node].parts {
            match part {
                Part::Tok(t) => out.push(t),
                Part::Child(c) => self.collect(*c, out),
            }
        }
    }
}
