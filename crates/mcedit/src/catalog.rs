use mcpack::mm::{Database, Grammar, Parsed, StmtId, SyntaxError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A database with its grammar and the syntax trees of the assertions read so far.
pub struct Catalog<'a> {
    pub db: &'a Database,
    library: u32,
    grammar: &'a Grammar<'a>,
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
            parsed: RefCell::new(HashMap::new()),
        })
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
