use mcpack::mm::{Database, Grammar, Kind, Parsed, StmtId, SyntaxError};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// A database with its grammar and the syntax trees of the assertions read so far.
pub struct Catalog<'a> {
    pub db: &'a Database,
    library: u32,
    grammar: &'a Grammar<'a>,
    syntax_dv: bool,
    parsed: RefCell<HashMap<StmtId, Result<Rc<Parsed>, SyntaxError>>>,
}

impl<'a> Catalog<'a> {
    /// `library` identifies the library the database was read from; a saved session carries it
    /// and only restores against the same value.
    pub fn new(db: &'a Database, grammar: &'a Grammar<'a>, library: u32) -> Rc<Self> {
        Rc::new(Catalog {
            db,
            library,
            grammar,
            syntax_dv: has_syntax_dv(db),
            parsed: RefCell::new(HashMap::new()),
        })
    }

    /// Whether some syntax axiom declares a disjoint pair.
    pub(crate) fn syntax_dv(&self) -> bool {
        self.syntax_dv
    }

    pub fn library(&self) -> u32 {
        self.library
    }

    pub fn parsed(&self, id: StmtId) -> Result<Rc<Parsed>, SyntaxError> {
        if let Some(found) = self.parsed.borrow().get(&id) {
            return found.clone();
        }
        let result = self.grammar.parse(id).map(Rc::new);
        self.parsed.borrow_mut().insert(id, result.clone());
        result
    }
}

fn has_syntax_dv(db: &Database) -> bool {
    let typed: HashSet<_> = db
        .stmts
        .iter()
        .filter(|s| s.kind == Kind::Float)
        .map(|s| s.expr[0])
        .collect();
    db.stmts
        .iter()
        .any(|s| s.kind == Kind::Axiom && !s.frame.dv.is_empty() && typed.contains(&s.expr[0]))
}
