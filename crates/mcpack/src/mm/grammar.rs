use super::{Database, Kind, StmtId, Sym};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarError {
    UnknownSymbol(String),
    /// The syntax axiom has an essential hypothesis, an empty body or a repeated variable.
    UnsupportedRule(String),
    /// The syntax axiom can begin with its own typecode without consuming a symbol.
    LeftRecursion(String),
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, name) = match self {
            GrammarError::UnknownSymbol(name) => ("UnknownSymbol", name),
            GrammarError::UnsupportedRule(name) => ("UnsupportedRule", name),
            GrammarError::LeftRecursion(name) => ("LeftRecursion", name),
        };
        write!(f, "{kind}(\"{}\")", super::shown(name.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxError {
    NoParse,
    Ambiguous,
}

/// A statement's structure: a `$f` leaf, or a syntax axiom applied to one subtree per mandatory hypothesis, in hypothesis order.
#[derive(Debug, PartialEq, Eq)]
pub struct Tree {
    pub stmt: StmtId,
    pub children: Vec<Rc<Tree>>,
}

impl Tree {
    /// The tree as a proof in reverse Polish notation.
    pub fn rpn(&self, out: &mut Vec<StmtId>) {
        for child in &self.children {
            child.rpn(out);
        }
        out.push(self.stmt);
    }
}

/// The trees of an assertion: one per essential hypothesis in frame order, then the conclusion.
#[derive(Debug)]
pub struct Parsed {
    pub hyps: Vec<Rc<Tree>>,
    pub conclusion: Rc<Tree>,
}

struct Rule {
    stmt: StmtId,
    /// The axiom's symbols after the typecode; a variable carries its typecode and its place among the hypotheses.
    body: Vec<Part>,
}

#[derive(Clone, Copy)]
enum Part {
    Const(Sym),
    Var { typecode: Sym, slot: usize },
}

type Ends = Rc<[(usize, Rc<Tree>)]>;

/// The database a grammar reads: borrowed for the grammar's life, or shared with its owner.
enum Db<'a> {
    Borrowed(&'a Database),
    Shared(Rc<Database>),
}

/// The context-free grammar formed by a database's syntax axioms.
pub struct Grammar<'a> {
    db: Db<'a>,
    /// Typecodes of provable statements and the typecode their bodies parse as.
    provable: Vec<(Sym, Sym)>,
    by_const: HashMap<(Sym, Sym), Vec<Rule>>,
    by_var: HashMap<Sym, Vec<Rule>>,
}

struct Run<'g, 'a> {
    grammar: &'g Grammar<'a>,
    expr: &'g [Sym],
    floats: &'g [(Sym, StmtId)],
    memo: HashMap<(Sym, usize), Ends>,
}

impl Grammar<'static> {
    /// Like [`Grammar::new`], but the grammar keeps its own handle on the database, so nothing has
    /// to outlive it.
    pub fn shared(
        db: Rc<Database>,
        provable: &[(&str, &str)],
    ) -> Result<Grammar<'static>, GrammarError> {
        Grammar::build(Db::Shared(db), provable)
    }
}

impl<'a> Grammar<'a> {
    /// Builds the grammar; each pair in `provable` names a typecode such as `|-` and the typecode its statements parse as.
    pub fn new(db: &'a Database, provable: &[(&str, &str)]) -> Result<Self, GrammarError> {
        Grammar::build(Db::Borrowed(db), provable)
    }

    pub(crate) fn db(&self) -> &Database {
        match &self.db {
            Db::Borrowed(db) => db,
            Db::Shared(db) => db,
        }
    }

    fn build(source: Db<'a>, provable: &[(&str, &str)]) -> Result<Self, GrammarError> {
        let provable = {
            let db = match &source {
                Db::Borrowed(db) => *db,
                Db::Shared(db) => &**db,
            };
            let constant = |name: &str| {
                db.syms
                    .iter()
                    .position(|s| !s.is_var && s.name == name)
                    .map(|i| i as Sym)
                    .ok_or_else(|| GrammarError::UnknownSymbol(name.to_string()))
            };
            provable
                .iter()
                .map(|&(from, to)| Ok((constant(from)?, constant(to)?)))
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut grammar = Grammar {
            db: source,
            provable,
            by_const: HashMap::new(),
            by_var: HashMap::new(),
        };
        for stmt in 0..grammar.db().stmts.len() as StmtId {
            let (kind, typecode) = {
                let s = grammar.db().stmt(stmt);
                (s.kind, s.expr[0])
            };
            if kind != Kind::Axiom || grammar.provable.iter().any(|&(from, _)| from == typecode) {
                continue;
            }
            let body = grammar.rule_body(stmt).ok_or_else(|| {
                GrammarError::UnsupportedRule(grammar.db().stmt(stmt).label.clone())
            })?;
            let rule = Rule { stmt, body };
            match rule.body[0] {
                Part::Const(first) => grammar.by_const.entry((typecode, first)).or_default(),
                Part::Var { .. } => grammar.by_var.entry(typecode).or_default(),
            }
            .push(rule);
        }
        grammar.check_left_recursion()?;
        Ok(grammar)
    }

    fn rule_body(&self, stmt: StmtId) -> Option<Vec<Part>> {
        let s = self.db().stmt(stmt);
        let mut used = vec![false; s.frame.hyps.len()];
        let body = s.expr[1..]
            .iter()
            .map(|&sym| {
                if !self.db().syms[sym as usize].is_var {
                    return Some(Part::Const(sym));
                }
                let slot = s.frame.hyps.iter().position(|&hyp| {
                    let hyp = self.db().stmt(hyp);
                    hyp.kind == Kind::Float && hyp.expr[1] == sym
                })?;
                if std::mem::replace(&mut used[slot], true) {
                    return None;
                }
                let typecode = self.db().stmt(s.frame.hyps[slot]).expr[0];
                Some(Part::Var { typecode, slot })
            })
            .collect::<Option<Vec<_>>>()?;
        (!body.is_empty() && used.iter().all(|&u| u)).then_some(body)
    }

    fn check_left_recursion(&self) -> Result<(), GrammarError> {
        let leads = |typecode: Sym| {
            self.by_var
                .get(&typecode)
                .into_iter()
                .flatten()
                .map(|rule| {
                    let Part::Var { typecode, .. } = rule.body[0] else {
                        unreachable!()
                    };
                    (typecode, rule.stmt)
                })
        };
        for &start in self.by_var.keys() {
            let mut seen = vec![start];
            let mut pending: Vec<(Sym, StmtId)> = leads(start).collect();
            while let Some((typecode, via)) = pending.pop() {
                if typecode == start {
                    let label = self.db().stmt(via).label.clone();
                    return Err(GrammarError::LeftRecursion(label));
                }
                if !seen.contains(&typecode) {
                    seen.push(typecode);
                    pending.extend(leads(typecode));
                }
            }
        }
        Ok(())
    }

    /// Parses an axiom or theorem together with its essential hypotheses.
    pub fn parse(&self, id: StmtId) -> Result<Parsed, SyntaxError> {
        let stmt = self.db().stmt(id);
        let mut floats = Vec::new();
        let mut essentials = Vec::new();
        for &hyp in &stmt.frame.hyps {
            let h = self.db().stmt(hyp);
            if h.kind == Kind::Float {
                floats.push((h.expr[1], hyp));
            } else {
                essentials.push(hyp);
            }
        }
        let hyps = essentials
            .iter()
            .map(|&hyp| self.parse_expr(&self.db().stmt(hyp).expr, &floats))
            .collect::<Result<Vec<_>, _>>()?;
        let conclusion = self.parse_expr(&stmt.expr, &floats)?;
        Ok(Parsed { hyps, conclusion })
    }

    /// Parses one expression, typecode first, reading each variable through the float hypothesis `floats` pairs it with.
    pub fn parse_expr(
        &self,
        expr: &[Sym],
        floats: &[(Sym, StmtId)],
    ) -> Result<Rc<Tree>, SyntaxError> {
        let typecode = self
            .provable
            .iter()
            .find(|&&(from, _)| from == expr[0])
            .map_or(expr[0], |&(_, to)| to);
        let mut run = Run {
            grammar: self,
            expr: &expr[1..],
            floats,
            memo: HashMap::new(),
        };
        let ends = run.parse(typecode, 0);
        let mut whole = ends.iter().filter(|(end, _)| *end == run.expr.len());
        match (whole.next(), whole.next()) {
            (Some((_, tree)), None) => Ok(tree.clone()),
            (None, _) => Err(SyntaxError::NoParse),
            (Some(_), Some(_)) => Err(SyntaxError::Ambiguous),
        }
    }

    /// The statement a tree stands for, typecode first.
    pub fn expr(&self, tree: &Tree) -> Vec<Sym> {
        let stmt = self.db().stmt(tree.stmt);
        let mut out = vec![stmt.expr[0]];
        self.unparse(tree, &mut out);
        out
    }

    fn unparse(&self, tree: &Tree, out: &mut Vec<Sym>) {
        let stmt = self.db().stmt(tree.stmt);
        if stmt.kind == Kind::Float {
            out.push(stmt.expr[1]);
            return;
        }
        for &sym in &stmt.expr[1..] {
            let child = stmt.frame.hyps.iter().position(|&hyp| {
                self.db().syms[sym as usize].is_var && self.db().stmt(hyp).expr[1] == sym
            });
            match child {
                Some(slot) => self.unparse(&tree.children[slot], out),
                None => out.push(sym),
            }
        }
    }
}

impl Run<'_, '_> {
    /// Every way a prefix of the input from `pos` reads as `typecode`, as end positions with their trees.
    fn parse(&mut self, typecode: Sym, pos: usize) -> Ends {
        if let Some(ends) = self.memo.get(&(typecode, pos)) {
            return ends.clone();
        }
        let grammar = self.grammar;
        let mut ends = Vec::new();
        if let Some(&token) = self.expr.get(pos) {
            if grammar.db().syms[token as usize].is_var {
                let float = self.floats.iter().find(|&&(var, _)| var == token);
                if let Some(&(_, stmt)) = float
                    && grammar.db().stmt(stmt).expr[0] == typecode
                {
                    let children = Vec::new();
                    ends.push((pos + 1, Rc::new(Tree { stmt, children })));
                }
            } else if let Some(rules) = grammar.by_const.get(&(typecode, token)) {
                for rule in rules {
                    let mut children = vec![None; grammar.db().stmt(rule.stmt).frame.hyps.len()];
                    self.finish(rule, 1, pos + 1, &mut children, &mut ends);
                }
            }
        }
        for rule in grammar.by_var.get(&typecode).into_iter().flatten() {
            let mut children = vec![None; grammar.db().stmt(rule.stmt).frame.hyps.len()];
            self.finish(rule, 0, pos, &mut children, &mut ends);
        }
        let ends: Ends = ends.into();
        self.memo.insert((typecode, pos), ends.clone());
        ends
    }

    fn finish(
        &mut self,
        rule: &Rule,
        at: usize,
        pos: usize,
        children: &mut Vec<Option<Rc<Tree>>>,
        ends: &mut Vec<(usize, Rc<Tree>)>,
    ) {
        match rule.body.get(at) {
            None => {
                let children = children
                    .iter()
                    .map(|child| child.clone().expect("every slot is filled"))
                    .collect();
                let stmt = rule.stmt;
                ends.push((pos, Rc::new(Tree { stmt, children })));
            }
            Some(&Part::Const(sym)) => {
                if self.expr.get(pos) == Some(&sym) {
                    self.finish(rule, at + 1, pos + 1, children, ends);
                }
            }
            Some(&Part::Var { typecode, slot }) => {
                for (end, tree) in self.parse(typecode, pos).iter() {
                    children[slot] = Some(tree.clone());
                    self.finish(rule, at + 1, *end, children, ends);
                }
                children[slot] = None;
            }
        }
    }
}
