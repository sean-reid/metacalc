use super::{Ctx, Cursor, Entry, SectionError, Shapes, TREES, Writer, within_chunk};
use crate::library::{Assertion, Hypothesis, PackError};
use crate::mm::{Database, Grammar, Kind, StmtId, Sym, Tree as Parsed};
use std::collections::{BTreeSet, HashMap};

/// Where an index stops fitting in the node byte itself and takes an escape byte after it.
const INLINE: usize = 127;
/// Syntax axioms a library's trees can use: 127 in the node byte, 256 more behind the escape.
pub const AXIOM_CAPACITY: usize = INLINE + 256;
/// Variables a library's trees can use; a library has fewer symbols than this.
pub const VARIABLE_CAPACITY: usize = INLINE + 256;
/// The typecode of a provable statement and the typecode its tree's root has.
pub(crate) const PROVABLE: (&str, &str) = ("|-", "wff");
const AXIOM_ESCAPE: u8 = 0x7F;
const VARIABLE: u8 = 0x80;
const VARIABLE_ESCAPE: u8 = 0xFF;
const NONE: u8 = 0xFE;
const LONG: u8 = 0xFF;
const MAX_INLINE: usize = 0xFD;

/// One piece of a syntax axiom's expression after its typecode: a constant token, or the place
/// of the child that stands for a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Token(String),
    Child(usize),
}

/// A syntax axiom the trees use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxAxiom {
    /// The axiom's number among the assertions.
    pub assertion: usize,
    /// How many children a node of it has: its hypotheses, all floating.
    pub arity: usize,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    /// An index into the syntax table.
    Axiom(usize),
    /// A symbol id.
    Variable(usize),
}

/// A statement's parse tree as its nodes in preorder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

/// The parse tree section: the syntax table and, per assertion, a tree for each essential
/// hypothesis in record order and one for the conclusion, or none where the grammar read nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trees {
    pub syntax: Vec<SyntaxAxiom>,
    pub items: Vec<Vec<Option<Tree>>>,
}

impl Trees {
    /// The shape bytes of every syntax axiom, in table order.
    pub fn shapes(&self) -> Shapes {
        Shapes {
            axioms: self.syntax.iter().map(|a| shape_bytes(&a.parts)).collect(),
        }
    }

    /// Statements the grammar could not read.
    pub fn without_tree(&self) -> usize {
        self.items.iter().flatten().filter(|t| t.is_none()).count()
    }

    /// The most nodes any tree has.
    pub fn largest(&self) -> usize {
        let sizes = self.items.iter().flatten().flatten();
        sizes.map(|t| t.nodes.len()).max().unwrap_or(0)
    }

    /// Folds `tree` from its leaves up: `variable` makes a value of a variable's symbol id, and
    /// `node` one of an axiom's parts and the values of its children in order.
    pub fn fold<'a, T>(
        &'a self,
        tree: &Tree,
        mut variable: impl FnMut(usize) -> T,
        mut node: impl FnMut(&'a [Part], Vec<T>) -> T,
    ) -> Option<T> {
        let mut values: Vec<T> = Vec::new();
        for n in tree.nodes.iter().rev() {
            match *n {
                Node::Variable(id) => values.push(variable(id)),
                Node::Axiom(index) => {
                    let axiom = self.syntax.get(index)?;
                    let at = values.len().checked_sub(axiom.arity)?;
                    let mut children = values.split_off(at);
                    children.reverse();
                    values.push(node(&axiom.parts, children));
                }
            }
        }
        let value = values.pop()?;
        values.is_empty().then_some(value)
    }
}

/// What the packer hands the encoder: the syntax table as (assertion number, arity) and the
/// preorder bytes of every statement's tree.
pub(super) struct Prepared {
    syntax: Vec<(u16, u8)>,
    shapes: Vec<Vec<u8>>,
    items: Vec<Vec<Option<Vec<u8>>>>,
}

/// Parses every statement of the library and encodes the trees. `None` when the database has no
/// grammar to parse with.
pub(super) fn prepare(
    db: &Database,
    assertions: &[StmtId],
    local_sym: &HashMap<Sym, u8>,
    local_stmt: &HashMap<StmtId, u16>,
    constants: usize,
) -> Result<Option<Prepared>, PackError> {
    let Ok(grammar) = Grammar::new(db, &[PROVABLE]) else {
        return Ok(None);
    };
    let mut parsed: Vec<Vec<Option<std::rc::Rc<Parsed>>>> = Vec::with_capacity(assertions.len());
    let mut used: BTreeSet<u16> = BTreeSet::new();
    for &id in assertions {
        let stmt = db.stmt(id);
        let floats: Vec<(Sym, StmtId)> = stmt
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Float)
            .map(|&h| (db.stmt(h).expr[1], h))
            .collect();
        let essentials = stmt
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Essential);
        let exprs = essentials.map(|&h| &db.stmt(h).expr).chain([&stmt.expr]);
        let trees: Vec<_> = exprs
            .map(|expr| grammar.parse_expr(expr, &floats).ok())
            .collect();
        for tree in trees.iter().flatten() {
            let mut pending = vec![tree];
            while let Some(node) = pending.pop() {
                if db.stmt(node.stmt).kind != Kind::Float {
                    let n =
                        local_stmt
                            .get(&node.stmt)
                            .ok_or_else(|| PackError::SyntaxAfterCut {
                                statement: stmt.label.clone(),
                                axiom: db.stmt(node.stmt).label.clone(),
                            })?;
                    used.insert(*n);
                }
                pending.extend(&node.children);
            }
        }
        parsed.push(trees);
    }
    if used.len() > AXIOM_CAPACITY {
        return Err(PackError::TreeCapacity("syntax axioms", used.len()));
    }
    let numbers: Vec<u16> = used.into_iter().collect();
    let index: HashMap<u16, usize> = numbers.iter().zip(0..).map(|(&n, i)| (n, i)).collect();
    let by_number: HashMap<u16, StmtId> = local_stmt.iter().map(|(&id, &n)| (n, id)).collect();
    let syntax = numbers
        .iter()
        .map(|n| (*n, db.stmt(by_number[n]).frame.hyps.len() as u8))
        .collect();
    let shapes = numbers
        .iter()
        .map(|n| shape_of(db, by_number[n]))
        .collect::<Result<Vec<_>, _>>()?;
    let mut items = Vec::with_capacity(parsed.len());
    for (&id, trees) in assertions.iter().zip(&parsed) {
        let mut item = Vec::with_capacity(trees.len());
        for tree in trees {
            let Some(tree) = tree else {
                item.push(None);
                continue;
            };
            let mut bytes = Vec::new();
            let mut pending = vec![tree];
            while let Some(node) = pending.pop() {
                let stmt = db.stmt(node.stmt);
                if stmt.kind == Kind::Float {
                    let var = usize::from(local_sym[&stmt.expr[1]]) - constants;
                    if var >= VARIABLE_CAPACITY {
                        return Err(PackError::TreeCapacity("variables", var + 1));
                    }
                    push_index(&mut bytes, var, VARIABLE, VARIABLE_ESCAPE);
                } else {
                    let axiom = index[&local_stmt[&node.stmt]];
                    push_index(&mut bytes, axiom, 0, AXIOM_ESCAPE);
                }
                pending.extend(node.children.iter().rev());
            }
            if bytes.len() > u16::MAX as usize - 1 {
                return Err(PackError::TooLarge(db.stmt(id).label.clone()));
            }
            item.push(Some(bytes));
        }
        items.push(item);
    }
    Ok(Some(Prepared {
        syntax,
        shapes,
        items,
    }))
}

impl Prepared {
    /// Each syntax axiom's shape bytes, in table order.
    pub(super) fn shapes(&self) -> &[Vec<u8>] {
        &self.shapes
    }
}

/// The shape bytes of a syntax axiom: 0 for a constant, else 1 + the position of the float
/// hypothesis whose variable stands there.
fn shape_of(db: &Database, axiom: StmtId) -> Result<Vec<u8>, PackError> {
    let stmt = db.stmt(axiom);
    let slots: Vec<Sym> = stmt
        .frame
        .hyps
        .iter()
        .map(|&h| db.stmt(h).expr[1])
        .collect();
    let shape: Vec<u8> = stmt.expr[1..]
        .iter()
        .map(|sym| {
            slots
                .iter()
                .position(|s| s == sym)
                .map_or(0, |k| k as u8 + 1)
        })
        .collect();
    if shape.is_empty() || shape.len() > usize::from(u8::MAX) {
        return Err(PackError::SectionTooLarge);
    }
    Ok(shape)
}

/// Writes `n` as `base + n` when it fits under the escape byte, else the escape and `n - 127`.
fn push_index(out: &mut Vec<u8>, n: usize, base: u8, escape: u8) {
    if n < INLINE {
        out.push(base + n as u8);
    } else {
        out.extend([escape, (n - INLINE) as u8]);
    }
}

pub(super) fn encode(w: &mut Writer, trees: &Prepared) -> Result<(), PackError> {
    let too_large = || PackError::SectionTooLarge;
    w.open(TREES);
    w.put(&(trees.syntax.len() as u16).to_le_bytes());
    for &(number, arity) in &trees.syntax {
        w.put(&number.to_le_bytes());
        w.put(&[arity]);
    }
    let index = w.len();
    w.put(&vec![0; 4 * trees.items.len()]);
    for (i, item) in trees.items.iter().enumerate() {
        let mut bytes = Vec::new();
        for tree in item {
            match tree {
                None => bytes.push(NONE),
                Some(tree) if tree.len() <= MAX_INLINE => {
                    bytes.push(tree.len() as u8);
                    bytes.extend_from_slice(tree);
                }
                Some(tree) => {
                    bytes.push(LONG);
                    bytes.extend((tree.len() as u16).to_le_bytes());
                    bytes.extend_from_slice(tree);
                }
            }
        }
        let offset = w.item(&bytes).map_err(|_| too_large())?;
        w.patch(index + 4 * i, &offset.to_le_bytes());
    }
    w.close();
    Ok(())
}

/// The shape bytes of `parts`: 0 for a token, 1 + k for child k.
pub(super) fn shape_bytes(parts: &[Part]) -> Vec<u8> {
    parts
        .iter()
        .map(|part| match part {
            Part::Token(_) => 0,
            Part::Child(k) => (k + 1) as u8,
        })
        .collect()
}

fn syntax_parts(assertion: &Assertion) -> Option<Vec<Part>> {
    let mut floats: Vec<&str> = Vec::new();
    for hypothesis in &assertion.hypotheses {
        match hypothesis {
            Hypothesis::Float { variable, .. } => floats.push(variable),
            Hypothesis::Essential(_) => return None,
        }
    }
    let parts = assertion
        .conclusion
        .get(1..)?
        .iter()
        .map(|token| match floats.iter().position(|f| f == token) {
            Some(slot) => Part::Child(slot),
            None => Part::Token(token.clone()),
        })
        .collect::<Vec<Part>>();
    let uses = |slot: usize| parts.iter().filter(|&p| *p == Part::Child(slot)).count();
    (0..floats.len())
        .all(|slot| uses(slot) == 1)
        .then_some(parts)
}

fn tree(bytes: &[u8], syntax: &[SyntaxAxiom], ctx: &Ctx) -> Option<Tree> {
    let mut nodes = Vec::with_capacity(bytes.len());
    let mut open = 1usize;
    let mut bytes = bytes.iter().copied();
    while let Some(byte) = bytes.next() {
        open = open.checked_sub(1)?;
        let mut number = |escape: u8, base: u8| -> Option<usize> {
            if byte == escape {
                Some(INLINE + bytes.next()? as usize)
            } else {
                Some((byte - base) as usize)
            }
        };
        if byte <= AXIOM_ESCAPE {
            let index = number(AXIOM_ESCAPE, 0)?;
            open += syntax.get(index)?.arity;
            nodes.push(Node::Axiom(index));
        } else {
            let id = ctx.constants + number(VARIABLE_ESCAPE, VARIABLE)?;
            if id >= ctx.names.len() {
                return None;
            }
            nodes.push(Node::Variable(id));
        }
    }
    (open == 0 && !nodes.is_empty()).then_some(Tree { nodes })
}

pub(super) fn decode(bytes: &[u8], entry: &Entry, ctx: &Ctx) -> Result<Trees, SectionError> {
    let bad = SectionError::Trees;
    let end = entry.offset + entry.length;
    let mut cur = Cursor::new(bytes, entry.offset, end);
    let count = cur.u16().ok_or(bad)?;
    if count > AXIOM_CAPACITY {
        return Err(bad);
    }
    let mut syntax: Vec<SyntaxAxiom> = Vec::with_capacity(count);
    for _ in 0..count {
        let assertion = cur.u16().ok_or(bad)?;
        if syntax
            .last()
            .is_some_and(|last| last.assertion >= assertion)
        {
            return Err(bad);
        }
        let arity = cur.u8().ok_or(bad)?;
        let source = ctx.assertions.get(assertion).ok_or(bad)?;
        let parts = syntax_parts(source).ok_or(bad)?;
        if source.hypotheses.len() != arity {
            return Err(bad);
        }
        syntax.push(SyntaxAxiom {
            assertion,
            arity,
            parts,
        });
    }
    let first_item = cur.pos + 4 * ctx.assertions.len();
    let mut items = Vec::with_capacity(ctx.assertions.len());
    for assertion in ctx.assertions {
        let at = cur.u32().ok_or(bad)?;
        if at < first_item {
            return Err(bad);
        }
        let mut item = Cursor::new(bytes, at, end);
        let statements = 1 + assertion
            .hypotheses
            .iter()
            .filter(|h| matches!(h, Hypothesis::Essential(_)))
            .count();
        let mut trees = Vec::with_capacity(statements);
        for _ in 0..statements {
            let len = match item.u8().ok_or(bad)? as u8 {
                NONE => {
                    trees.push(None);
                    continue;
                }
                LONG => item.u16().ok_or(bad)?,
                len => len as usize,
            };
            let body = item.take(len).ok_or(bad)?;
            trees.push(Some(tree(body, &syntax, ctx).ok_or(bad)?));
        }
        if !within_chunk(at, item.pos - at) {
            return Err(bad);
        }
        items.push(trees);
    }
    Ok(Trees { syntax, items })
}
