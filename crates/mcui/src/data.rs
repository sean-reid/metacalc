//! What the calculator has: packed libraries, read back into the statements and proofs the
//! screens show.

use crate::expr::{Expr, Shape};
use crate::status::{Mark, Status, marks};
use mcpack::library::{self, Assertion, Hypothesis, ReadError};
use mcpack::mm::{self, Database, Grammar, Kind, Node, StmtId, Sym, Tree};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// A library to install: its name and its packed bytes.
#[derive(Debug, Clone)]
pub struct Installed {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The status table the caller keeps: one entry per assertion of each library, and the number of
/// proofs the user has saved.
#[derive(Debug, Clone, Default)]
pub struct Statuses {
    pub libraries: Vec<Vec<Status>>,
    pub my_proofs: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The bytes of the named library do not decode.
    Library(String, ReadError),
    /// The status table does not match the libraries.
    Statuses(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Library(name, e) => write!(f, "library {name}: {e}"),
            LoadError::Statuses(why) => write!(f, "status table: {why}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// A theorem's hypotheses and conclusion as trees.
#[derive(Debug)]
pub struct Statement {
    pub hyps: Vec<Expr>,
    pub conclusion: Expr,
}

impl Statement {
    /// Statement `i` counting the hypotheses first and the conclusion last.
    pub fn at(&self, i: usize) -> &Expr {
        self.hyps.get(i).unwrap_or(&self.conclusion)
    }

    /// The hypotheses, then the conclusion.
    pub fn all(&self) -> Vec<&Expr> {
        self.hyps.iter().chain([&self.conclusion]).collect()
    }
}

/// The library as a Metamath database, which is where parse trees and goal trees come from, and
/// the grammar built from it once. The database is leaked so the grammar can borrow it for as
/// long as the process runs; a library is loaded once.
struct Source {
    db: &'static Database,
    ids: Vec<StmtId>,
    index: HashMap<StmtId, usize>,
    grammar: Option<Grammar<'static>>,
}

pub struct Lib {
    pub name: String,
    bytes: Vec<u8>,
    assertions: Vec<Assertion>,
    status: Vec<Status>,
    marks: Vec<Mark>,
    source: OnceCell<Option<Source>>,
    statements: RefCell<HashMap<usize, Rc<Statement>>>,
}

impl Lib {
    pub fn new(installed: Installed, status: Vec<Status>) -> Result<Lib, LoadError> {
        let assertions = library::read(&installed.bytes)
            .map_err(|e| LoadError::Library(installed.name.clone(), e))?;
        if assertions.len() != status.len() {
            return Err(LoadError::Statuses(format!(
                "{} has {} assertions and {} statuses",
                installed.name,
                assertions.len(),
                status.len()
            )));
        }
        Ok(Lib {
            name: installed.name,
            bytes: installed.bytes,
            marks: marks(&assertions, &status),
            assertions,
            status,
            source: OnceCell::new(),
            statements: RefCell::default(),
        })
    }

    pub fn len(&self) -> usize {
        self.assertions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assertions.is_empty()
    }

    pub fn label(&self, i: usize) -> &str {
        &self.assertions[i].label
    }

    pub fn is_theorem(&self, i: usize) -> bool {
        self.assertions[i].theorem
    }

    pub fn status(&self, i: usize) -> Status {
        self.status[i]
    }

    pub fn mark(&self, i: usize) -> Mark {
        self.marks[i]
    }

    /// Assertions that count as checked: marked checked, whether or not they rest on others.
    pub fn checked(&self) -> usize {
        self.marks
            .iter()
            .filter(|&&m| matches!(m, Mark::Checked | Mark::Resting))
            .count()
    }

    /// Assertions that failed, or whose proof uses itself or a later assertion.
    pub fn failed(&self) -> usize {
        self.marks.iter().filter(|&&m| m == Mark::Failed).count()
    }

    /// The disjoint variable pairs the statement itself requires.
    pub fn dist(&self, i: usize) -> &[(String, String)] {
        &self.assertions[i].disjoint
    }

    fn source(&self) -> Option<&Source> {
        self.source.get_or_init(|| self.load()).as_ref()
    }

    fn load(&self) -> Option<Source> {
        let text = library::export(&self.bytes).ok()?;
        let db = mm::parse(text.into_bytes()).ok()?;
        let ids: Vec<StmtId> = (0..db.stmts.len() as StmtId)
            .filter(|&id| db.stmt(id).kind.is_assertion())
            .collect();
        if ids.len() != self.assertions.len() {
            return None;
        }
        let index = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let db: &'static Database = Box::leak(Box::new(db));
        let grammar = Grammar::new(db, &[("|-", "wff")]).ok();
        Some(Source {
            db,
            ids,
            index,
            grammar,
        })
    }

    /// Whether statements fall back to flat runs of tokens because the library does not export or
    /// its grammar cannot be built. A library with no statements has nothing to show flat.
    pub fn degraded(&self) -> bool {
        !self.is_empty() && self.source().is_none_or(|s| s.grammar.is_none())
    }

    /// The statement of assertion `i`, with parse trees when the grammar reads it and one flat
    /// run of tokens when it does not. Whether a statement is a tree or flat decides where the
    /// cursor can go and how it folds, so it is part of what the frames specify.
    pub fn statement(&self, i: usize) -> Rc<Statement> {
        if let Some(found) = self.statements.borrow().get(&i) {
            return found.clone();
        }
        let built = Rc::new(
            self.parse_statement(i)
                .unwrap_or_else(|| self.flat_statement(i)),
        );
        self.statements.borrow_mut().insert(i, built.clone());
        built
    }

    fn essentials(&self, i: usize) -> impl Iterator<Item = &Vec<String>> {
        self.assertions[i]
            .hypotheses
            .iter()
            .filter_map(|h| match h {
                Hypothesis::Essential(expr) => Some(expr),
                Hypothesis::Float { .. } => None,
            })
    }

    fn flat_statement(&self, i: usize) -> Statement {
        let flat = |expr: &Vec<String>| Expr::flat(vec![expr[0].clone()], &expr[1..]);
        Statement {
            hyps: self.essentials(i).map(flat).collect(),
            conclusion: flat(&self.assertions[i].conclusion),
        }
    }

    fn parse_statement(&self, i: usize) -> Option<Statement> {
        let source = self.source()?;
        let parsed = source.grammar.as_ref()?.parse(source.ids[i]).ok()?;
        let build =
            |lead: &str, tree: &Tree| Expr::build(vec![lead.to_string()], &shape(source.db, tree));
        let hyps = self
            .essentials(i)
            .zip(&parsed.hyps)
            .map(|(expr, tree)| build(&expr[0], tree))
            .collect();
        let conclusion = build(&self.assertions[i].conclusion[0], &parsed.conclusion);
        Some(Statement { hyps, conclusion })
    }

    /// The goal tree of theorem `i`.
    pub fn goals(&self, i: usize) -> Result<Goals, &'static str> {
        if !self.assertions[i].theorem {
            return Err("an axiom has no proof");
        }
        let source = self.source().ok_or("the library does not export")?;
        let id = source.ids[i];
        let root = mm::outline(source.db, id, "|-").map_err(|e| match e {
            mm::OutlineError::NotProvable => "it proves a syntax statement",
            _ => "the proof does not verify",
        })?;
        let stmt = source.db.stmt(id);
        let mut floats: Vec<(Sym, StmtId)> = Vec::new();
        let mut hyps = Vec::new();
        let used = match &stmt.proof {
            Some(Ok(proof)) => match &proof.body {
                mm::ProofBody::Normal(labels) | mm::ProofBody::Compressed { labels, .. } => {
                    labels.clone()
                }
            },
            _ => Vec::new(),
        };
        for &h in &stmt.frame.hyps {
            let hyp = source.db.stmt(h);
            match hyp.kind {
                Kind::Float => floats.push((hyp.expr[1], h)),
                _ => hyps.push(h),
            }
        }
        for &h in &used {
            let hyp = source.db.stmt(h);
            if hyp.kind == Kind::Float && floats.iter().all(|&(v, _)| v != hyp.expr[1]) {
                floats.push((hyp.expr[1], h));
            }
        }
        let mut sizes = HashMap::new();
        measure(&root, &mut sizes);
        Ok(Goals {
            theorem: i,
            root,
            floats,
            hyps,
            sizes,
        })
    }

    /// What the screen shows of one goal: its statement, how it was proved, and its subgoals.
    pub fn view(&self, goals: &Goals, node: &Node) -> GoalView {
        let source = self.source().expect("goals exist only with a source");
        let db = source.db;
        let grammar = source.grammar.as_ref();
        let expr = |syms: &[Sym]| {
            let name = |s: Sym| db.syms[s as usize].name.clone();
            let lead = vec![name(syms[0])];
            let parsed = grammar.and_then(|g| g.parse_expr(syms, &goals.floats).ok());
            match parsed {
                Some(tree) => Expr::build(lead, &shape(db, &tree)),
                None => {
                    let tokens: Vec<String> = syms[1..].iter().map(|&s| name(s)).collect();
                    Expr::flat(lead, &tokens)
                }
            }
        };
        let by = match source.index.get(&node.label) {
            Some(&i) => By::Assertion(i),
            None => By::Hypothesis(
                goals
                    .hyps
                    .iter()
                    .position(|&h| h == node.label)
                    .map_or(0, |n| n + 1),
            ),
        };
        GoalView {
            goal: expr(&node.expr),
            by,
            subgoals: node.children.iter().map(|c| expr(&c.expr)).collect(),
        }
    }
}

fn shape(db: &Database, tree: &Tree) -> Shape {
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

fn measure(node: &Rc<Node>, sizes: &mut HashMap<*const Node, u64>) -> u64 {
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
    floats: Vec<(Sym, StmtId)>,
    hyps: Vec<StmtId>,
    sizes: HashMap<*const Node, u64>,
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
