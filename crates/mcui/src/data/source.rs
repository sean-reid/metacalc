use super::PROVABLE;
use mcedit::Catalog;
use mcpack::library::{self, Assertion};
use mcpack::mm::{self, Database, Grammar, StmtId};
use std::collections::HashMap;
use std::rc::Rc;

/// The library as a Metamath database, which is where parse trees and goal trees come from, and
/// the grammar built from it once.
pub(super) struct Source {
    pub(super) db: &'static Database,
    pub(super) ids: Vec<StmtId>,
    pub(super) index: HashMap<StmtId, usize>,
    pub(super) grammar: Option<&'static Grammar<'static>>,
    /// Whether the database holds assertion `i` with a proof of `?`, because the kernel rejects it.
    pub(super) unproved: Vec<bool>,
    /// The editor's parsed assertions, kept for as long as the library is.
    pub(super) catalog: Option<Rc<Catalog<'static>>>,
}

impl Source {
    pub(super) fn load(bytes: &[u8], assertions: &[Assertion], crc: u32) -> Option<Source> {
        let library::Export { text, rejected } = library::export_accepted(bytes).ok()?;
        let db = mm::parse(text.into_bytes()).ok()?;
        let ids: Vec<StmtId> = (0..db.stmts.len() as StmtId)
            .filter(|&id| db.stmt(id).kind.is_assertion())
            .collect();
        if ids.len() != assertions.len() {
            return None;
        }
        let unproved = assertions
            .iter()
            .map(|a| rejected.iter().any(|(label, _)| *label == a.label))
            .collect();
        let index = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let db: &'static Database = Box::leak(Box::new(db));
        let grammar = Grammar::new(db, &[(PROVABLE, "wff")])
            .ok()
            .map(|g| &*Box::leak(Box::new(g)));
        let catalog = grammar.map(|g| Catalog::new(db, g, crc));
        Some(Source {
            db,
            ids,
            index,
            grammar,
            unproved,
            catalog,
        })
    }
}
