use super::{Database, Grammar, Kind, ProofBody, StmtId, SyntaxError, Tree};

/// Every statement the roots depend on, roots included, in database order.
///
/// Dependencies run through proofs, mandatory hypotheses, and the syntax axioms that build each kept statement.
/// A statement the grammar cannot read stops the walk and is returned as the error.
pub fn closure(
    db: &Database,
    grammar: &Grammar,
    roots: &[StmtId],
) -> Result<Vec<StmtId>, (StmtId, SyntaxError)> {
    let mut seen = vec![false; db.stmts.len()];
    let mut pending: Vec<StmtId> = Vec::new();
    let mut visit = |id: StmtId, pending: &mut Vec<StmtId>| {
        if !std::mem::replace(&mut seen[id as usize], true) {
            pending.push(id);
        }
    };
    for &root in roots {
        visit(root, &mut pending);
    }
    while let Some(id) = pending.pop() {
        let stmt = db.stmt(id);
        for &hyp in &stmt.frame.hyps {
            visit(hyp, &mut pending);
        }
        if let Some(Ok(proof)) = &stmt.proof {
            let (ProofBody::Normal(refs) | ProofBody::Compressed { labels: refs, .. }) =
                &proof.body;
            for &dep in refs {
                visit(dep, &mut pending);
            }
        }
        if matches!(stmt.kind, Kind::Axiom | Kind::Theorem) {
            let parsed = grammar.parse(id).map_err(|e| (id, e))?;
            let mut trees: Vec<&Tree> = parsed.hyps.iter().map(|tree| &**tree).collect();
            trees.push(&parsed.conclusion);
            while let Some(tree) = trees.pop() {
                visit(tree.stmt, &mut pending);
                trees.extend(tree.children.iter().map(|child| &**child));
            }
        }
    }
    Ok((0..db.stmts.len() as StmtId)
        .filter(|&id| seen[id as usize])
        .collect())
}
