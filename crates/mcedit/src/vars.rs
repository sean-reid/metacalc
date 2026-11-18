use mcpack::mm::{Database, Kind, StmtId, Sym};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// The variables a proof of one theorem can use and the typecode of each.
pub struct Vars {
    types: HashMap<Sym, Sym>,
    /// Variables whose `$v` is in force at the theorem and that have a floating hypothesis before it, in declaration order.
    declared: Vec<Sym>,
    /// The theorem's mandatory floating hypotheses.
    mandatory: Vec<(Sym, StmtId)>,
}

impl Vars {
    pub fn new(db: &Database, theorem: StmtId) -> Self {
        let mut types: HashMap<Sym, Sym> = HashMap::new();
        let mut declared = Vec::new();
        let mut active = Vec::new();
        for id in 0..theorem {
            let stmt = db.stmt(id);
            if stmt.kind != Kind::Float {
                continue;
            }
            let (typecode, var) = (stmt.expr[0], stmt.expr[1]);
            if let Entry::Vacant(slot) = types.entry(var) {
                slot.insert(typecode);
                declared.push(var);
            }
            if stmt.scope_end > theorem {
                active.push((var, typecode));
            }
        }
        declared.retain(|&v| db.var_active(v, theorem));
        for (var, typecode) in active {
            types.insert(var, typecode);
        }
        let mandatory = db
            .stmt(theorem)
            .frame
            .hyps
            .iter()
            .filter(|&&h| db.stmt(h).kind == Kind::Float)
            .map(|&h| (db.stmt(h).expr[1], h))
            .collect();
        Vars {
            types,
            declared,
            mandatory,
        }
    }

    pub fn types(&self) -> HashMap<Sym, Sym> {
        self.types.clone()
    }

    pub fn typecode(&self, var: Sym) -> Sym {
        self.types[&var]
    }

    pub fn is_mandatory(&self, var: Sym) -> bool {
        self.mandatory.iter().any(|&(v, _)| v == var)
    }

    pub fn mandatory_float(&self, var: Sym) -> Option<StmtId> {
        self.mandatory.iter().find(|&&(v, _)| v == var).map(|p| p.1)
    }

    /// The variables a metavariable of `typecode` may be assigned, in declaration order.
    pub fn of_typecode(&self, typecode: Sym) -> impl Iterator<Item = Sym> + '_ {
        self.declared
            .iter()
            .copied()
            .filter(move |v| self.types[v] == typecode)
    }
}

/// The floating hypothesis of `var` that is active at `theorem`, if it has one.
pub fn active_float(db: &Database, var: Sym, theorem: StmtId) -> Option<StmtId> {
    (0..theorem).rev().find(|&id| {
        let s = db.stmt(id);
        s.kind == Kind::Float && s.expr[1] == var && s.scope_end > theorem
    })
}
