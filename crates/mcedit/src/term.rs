use mcpack::mm::{Database, StmtId, Sym};
use std::collections::HashMap;

pub type TermId = u32;
pub type MetaId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    /// A variable of the theorem being proved, or a dummy picked for it. It is fixed like a constant.
    Var(Sym),
    /// An unknown that unification or an assignment determines.
    Meta(MetaId),
    /// A syntax axiom applied to one subterm per mandatory hypothesis.
    App(StmtId),
}

#[derive(Clone, Copy, Debug)]
struct Node {
    head: Head,
    start: u32,
    len: u32,
}

#[derive(Clone, Copy, Debug)]
struct Meta {
    typecode: Sym,
    bound: Option<TermId>,
    node: TermId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clash {
    Constructor,
    Occurs,
    Typecode,
    /// A term or metavariable id the store does not hold.
    Unknown,
}

/// A point to roll the store back to: terms, metavariables and bindings made after it are dropped.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    nodes: u32,
    args: u32,
    metas: u32,
    trail: u32,
}

/// One piece of a flattened term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Sym(Sym),
    Meta(MetaId),
}

/// High-water marks of the arena, in the units a fixed-memory port has to budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Peak {
    pub nodes: usize,
    pub args: usize,
    pub metas: usize,
}

/// Terms with metavariables, kept in append-only arenas so that rolling back is a truncation.
pub struct Store<'a> {
    db: &'a Database,
    var_types: HashMap<Sym, Sym>,
    nodes: Vec<Node>,
    args: Vec<TermId>,
    metas: Vec<Meta>,
    trail: Vec<MetaId>,
    peak: Peak,
}

impl<'a> Store<'a> {
    pub fn new(db: &'a Database, var_types: HashMap<Sym, Sym>) -> Self {
        Store {
            db,
            var_types,
            nodes: Vec::new(),
            args: Vec::new(),
            metas: Vec::new(),
            trail: Vec::new(),
            peak: Peak::default(),
        }
    }

    fn push(&mut self, head: Head, kids: &[TermId]) -> TermId {
        let start = self.args.len() as u32;
        self.args.extend_from_slice(kids);
        self.nodes.push(Node {
            head,
            start,
            len: kids.len() as u32,
        });
        self.peak.nodes = self.peak.nodes.max(self.nodes.len());
        self.peak.args = self.peak.args.max(self.args.len());
        self.nodes.len() as TermId - 1
    }

    pub fn var(&mut self, var: Sym) -> TermId {
        self.push(Head::Var(var), &[])
    }

    pub fn app(&mut self, syntax: StmtId, kids: &[TermId]) -> TermId {
        self.push(Head::App(syntax), kids)
    }

    /// A fresh unbound metavariable of `typecode`, and the term that stands for it.
    pub fn new_meta(&mut self, typecode: Sym) -> TermId {
        let id = self.metas.len() as MetaId;
        let node = self.push(Head::Meta(id), &[]);
        self.metas.push(Meta {
            typecode,
            bound: None,
            node,
        });
        self.peak.metas = self.peak.metas.max(self.metas.len());
        node
    }

    pub fn peak(&self) -> Peak {
        self.peak
    }

    /// The current sizes of the arenas.
    pub fn len(&self) -> Peak {
        Peak {
            nodes: self.nodes.len(),
            args: self.args.len(),
            metas: self.metas.len(),
        }
    }

    pub fn meta_count(&self) -> usize {
        self.metas.len()
    }

    pub fn meta_term(&self, meta: MetaId) -> Option<TermId> {
        self.metas.get(meta as usize).map(|m| m.node)
    }

    pub fn meta_typecode(&self, meta: MetaId) -> Option<Sym> {
        self.metas.get(meta as usize).map(|m| m.typecode)
    }

    pub fn is_unbound(&self, meta: MetaId) -> bool {
        self.metas
            .get(meta as usize)
            .is_some_and(|m| m.bound.is_none())
    }

    pub fn unbound(&self) -> impl Iterator<Item = MetaId> + '_ {
        (0..self.metas.len() as MetaId).filter(|&m| self.is_unbound(m))
    }

    pub fn mark(&self) -> Mark {
        Mark {
            nodes: self.nodes.len() as u32,
            args: self.args.len() as u32,
            metas: self.metas.len() as u32,
            trail: self.trail.len() as u32,
        }
    }

    pub fn rollback(&mut self, mark: Mark) {
        for meta in self.trail.drain(mark.trail as usize..) {
            self.metas[meta as usize].bound = None;
        }
        self.nodes.truncate(mark.nodes as usize);
        self.args.truncate(mark.args as usize);
        self.metas.truncate(mark.metas as usize);
    }

    /// Follows bound metavariables to the term they stand for.
    ///
    /// Term and metavariable ids are invalidated by `rollback`; the accessors treat an unknown id as
    /// no term rather than panic.
    pub fn walk(&self, mut term: TermId) -> TermId {
        while let Some(Node {
            head: Head::Meta(m),
            ..
        }) = self.nodes.get(term as usize)
        {
            match self.metas[*m as usize].bound {
                Some(next) => term = next,
                None => break,
            }
        }
        term
    }

    pub fn head(&self, term: TermId) -> Option<Head> {
        self.nodes.get(self.walk(term) as usize).map(|n| n.head)
    }

    /// The subterms of a term; its metavariables are not resolved.
    pub fn children(&self, term: TermId) -> &[TermId] {
        match self.nodes.get(self.walk(term) as usize) {
            Some(n) => &self.args[n.start as usize..(n.start + n.len) as usize],
            None => &[],
        }
    }

    pub fn typecode(&self, term: TermId) -> Option<Sym> {
        Some(match self.head(term)? {
            Head::Var(v) => *self.var_types.get(&v)?,
            Head::Meta(m) => self.metas[m as usize].typecode,
            Head::App(s) => self.db.stmt(s).expr[0],
        })
    }

    fn occurs(&self, meta: MetaId, term: TermId) -> bool {
        let term = self.walk(term);
        match self.head(term) {
            Some(Head::Meta(m)) => m == meta,
            Some(Head::App(_)) => self.children(term).iter().any(|&k| self.occurs(meta, k)),
            _ => false,
        }
    }

    pub fn bind(&mut self, meta: MetaId, term: TermId) -> Result<(), Clash> {
        let wanted = self.meta_typecode(meta).ok_or(Clash::Unknown)?;
        match self.typecode(term) {
            None => return Err(Clash::Unknown),
            Some(t) if t != wanted => return Err(Clash::Typecode),
            Some(_) => {}
        }
        if self.occurs(meta, term) {
            return Err(Clash::Occurs);
        }
        self.metas[meta as usize].bound = Some(term);
        self.trail.push(meta);
        Ok(())
    }

    /// First-order unification. On failure the bindings made so far stay; roll back to a mark.
    pub fn unify(&mut self, a: TermId, b: TermId) -> Result<(), Clash> {
        let (a, b) = (self.walk(a), self.walk(b));
        let (Some(&na), Some(&nb)) = (self.nodes.get(a as usize), self.nodes.get(b as usize))
        else {
            return Err(Clash::Unknown);
        };
        if a == b {
            return Ok(());
        }
        match (na.head, nb.head) {
            (Head::Meta(m), _) => self.bind(m, b),
            (_, Head::Meta(m)) => self.bind(m, a),
            (Head::Var(x), Head::Var(y)) if x == y => Ok(()),
            (Head::App(x), Head::App(y)) if x == y => {
                for i in 0..na.len {
                    let ka = self.args[(na.start + i) as usize];
                    let kb = self.args[(nb.start + i) as usize];
                    self.unify(ka, kb)?;
                }
                Ok(())
            }
            _ => Err(Clash::Constructor),
        }
    }

    pub fn is_ground(&self, term: TermId) -> bool {
        let term = self.walk(term);
        match self.head(term) {
            Some(Head::Var(_)) => true,
            Some(Head::App(_)) => self.children(term).iter().all(|&k| self.is_ground(k)),
            _ => false,
        }
    }

    /// Adds the variables of a term to `out`, once each.
    pub fn variables(&self, term: TermId, out: &mut Vec<Sym>) {
        let term = self.walk(term);
        match self.head(term) {
            Some(Head::Var(v)) => {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            Some(Head::App(_)) => {
                for &k in self.children(term) {
                    self.variables(k, out);
                }
            }
            _ => {}
        }
    }

    /// The term as symbols, a syntax axiom written out with its subterms in place of its variables.
    pub fn pieces(&self, term: TermId, out: &mut Vec<Piece>) {
        let term = self.walk(term);
        match self.head(term) {
            Some(Head::Var(v)) => out.push(Piece::Sym(v)),
            Some(Head::Meta(m)) => out.push(Piece::Meta(m)),
            Some(Head::App(s)) => {
                let stmt = self.db.stmt(s);
                let kids = self.children(term).to_vec();
                for &sym in &stmt.expr[1..] {
                    let slot = if self.db.syms[sym as usize].is_var {
                        stmt.frame
                            .hyps
                            .iter()
                            .position(|&h| self.db.stmt(h).expr[1] == sym)
                    } else {
                        None
                    };
                    match slot {
                        Some(i) => self.pieces(kids[i], out),
                        None => out.push(Piece::Sym(sym)),
                    }
                }
            }
            None => {}
        }
    }

    /// The term in Metamath notation with unbound metavariables written `?n`; nothing for an unknown id.
    pub fn show(&self, term: TermId) -> Option<String> {
        self.head(term)?;
        let mut pieces = Vec::new();
        self.pieces(term, &mut pieces);
        let words: Vec<String> = pieces
            .iter()
            .map(|p| match *p {
                Piece::Sym(s) => self.db.syms[s as usize].name.clone(),
                Piece::Meta(m) => format!("?{m}"),
            })
            .collect();
        Some(words.join(" "))
    }
}
