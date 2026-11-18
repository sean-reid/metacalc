use super::{Database, ProofBody, StmtId};

/// Every statement the roots depend on through proofs and mandatory hypotheses, roots included, in database order.
pub fn closure(db: &Database, roots: &[StmtId]) -> Vec<StmtId> {
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
    }
    (0..db.stmts.len() as StmtId)
        .filter(|&id| seen[id as usize])
        .collect()
}
