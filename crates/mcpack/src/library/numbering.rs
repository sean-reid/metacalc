use super::PackError;
use crate::mm::{Database, Kind, Stmt, StmtId, Sym};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Orders `variables`, given in `$v` order, as `build` numbers them.
pub(super) fn order(
    db: &Database,
    assertions: &[(StmtId, &Stmt)],
    variables: &[Sym],
) -> Result<Vec<Sym>, PackError> {
    let count = variables.len();
    let index: HashMap<Sym, usize> = variables.iter().zip(0..).map(|(&v, i)| (v, i)).collect();
    let mut first: HashMap<Sym, (bool, usize)> = HashMap::new();
    for (at, stmt) in db.stmts.iter().enumerate() {
        if stmt.kind != Kind::Float {
            continue;
        }
        let global = stmt.scope_end == StmtId::MAX;
        let entry = first.entry(stmt.expr[1]).or_insert((global, at));
        if global && !entry.0 {
            *entry = (true, at);
        }
    }
    let key = |v: usize| (first.get(&variables[v]).map_or(usize::MAX, |e| e.1), v);

    let mut after: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut waiting = vec![0usize; count];
    let mut witness: HashMap<(usize, usize), usize> = HashMap::new();
    for (at, (_, stmt)) in assertions.iter().enumerate() {
        let floats: Vec<usize> = stmt
            .frame
            .hyps
            .iter()
            .map(|&h| db.stmt(h))
            .filter(|h| h.kind == Kind::Float)
            .filter_map(|h| index.get(&h.expr[1]).copied())
            .collect();
        for pair in floats.windows(2) {
            if witness.insert((pair[0], pair[1]), at).is_none() {
                after[pair[0]].push(pair[1]);
                waiting[pair[1]] += 1;
            }
        }
    }
    let mut ready: BinaryHeap<Reverse<((usize, usize), usize)>> = (0..count)
        .filter(|&v| waiting[v] == 0)
        .map(|v| Reverse((key(v), v)))
        .collect();
    let mut sorted = Vec::with_capacity(count);
    while let Some(Reverse((_, var))) = ready.pop() {
        sorted.push(variables[var]);
        for &next in &after[var] {
            waiting[next] -= 1;
            if waiting[next] == 0 {
                ready.push(Reverse((key(next), next)));
            }
        }
    }
    if sorted.len() == count {
        return Ok(sorted);
    }
    Err(conflict(assertions, &witness, &waiting))
}

/// The error for edges with a cycle: the assertions of the edges on one. `waiting` is nonzero
/// exactly for the variables the sort could not place.
fn conflict(
    assertions: &[(StmtId, &Stmt)],
    witness: &HashMap<(usize, usize), usize>,
    waiting: &[usize],
) -> PackError {
    let stuck = |v: usize| waiting[v] > 0;
    let mut earlier: HashMap<usize, usize> = HashMap::new();
    let mut edges: Vec<_> = witness.keys().collect();
    edges.sort_unstable();
    for &&(a, b) in &edges {
        if stuck(a) && stuck(b) {
            earlier.entry(b).or_insert(a);
        }
    }
    let mut at = edges.iter().map(|e| e.1).find(|&v| stuck(v)).unwrap_or(0);
    let mut seen: Vec<usize> = Vec::new();
    while !seen.contains(&at) {
        seen.push(at);
        at = earlier.get(&at).copied().unwrap_or(at);
    }
    let cycle = &seen[seen.iter().position(|&v| v == at).unwrap_or(0)..];
    let mut labels: Vec<String> = Vec::new();
    for (i, &var) in cycle.iter().enumerate() {
        let next = cycle[(i + 1) % cycle.len()];
        let record = witness.get(&(next, var)).copied().unwrap_or(0);
        let label = &assertions[record].1.label;
        if !labels.contains(label) {
            labels.push(label.clone());
        }
    }
    PackError::FloatOrder(labels)
}
