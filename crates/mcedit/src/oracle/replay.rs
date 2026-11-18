use super::prooftree::{ProofNode, proof_tree};
use super::splice::{close_scopes, splice_at};
use crate::catalog::Catalog;
use crate::error::Error;
use crate::export::Finished;
use crate::session::{GoalId, Session, Sizes};
use crate::term::MetaId;
use mcpack::mm::{Database, Grammar, Kind, ProofError, StmtId, Tree, Verifier};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The reference verifier rejects the original proof.
    Original(ProofError),
    /// A public operation failed while rebuilding the proof.
    Session(Error),
    /// A metavariable is left that only a compound term could resolve.
    NeedsRefine,
    Splice(String),
    /// The database with the exported proof does not parse.
    Parse(String),
    /// The reference verifier rejects the exported proof.
    Rejected(ProofError),
}

pub struct Replayed {
    pub finished: Finished,
    pub ops: usize,
    pub assigns: usize,
    pub refines: usize,
    pub peak: Sizes,
    pub dummies: usize,
    /// The size of the session saved after the last operation, in bytes.
    pub saved: usize,
}

type Expected = HashMap<MetaId, Rc<Tree>>;

fn drive(
    session: &mut Session,
    goal: GoalId,
    node: &ProofNode,
    expected: &mut Expected,
) -> Result<(), Failure> {
    let db = session.db();
    if db.stmt(node.label).kind == Kind::Essential {
        session.close(goal, node.label).map_err(Failure::Session)?;
        return Ok(());
    }
    let outcome = session.apply(goal, node.label).map_err(Failure::Session)?;
    for (meta, tree) in outcome.metas.zip(&node.subst) {
        expected.insert(meta, tree.clone());
    }
    for (child, sub) in outcome.goals.zip(&node.children) {
        drive(session, child, sub, expected)?;
    }
    Ok(())
}

/// Rebuilds a theorem's verified proof top down with the public operations, then exports it.
/// With `refine` off, a metavariable no variable can resolve is a failure.
pub fn replay(catalog: &Rc<Catalog>, theorem: StmtId, refine: bool) -> Result<Replayed, Failure> {
    let db = catalog.db;
    let root = proof_tree(db, theorem).map_err(Failure::Original)?;
    let mut session = Session::start_in(catalog.clone(), theorem).map_err(Failure::Session)?;
    let mut expected = Expected::new();
    drive(&mut session, 0, &root, &mut expected)?;
    let (mut assigns, mut refines) = (0, 0);
    while let Some(&meta) = session.status().metas.first() {
        let tree = expected.get(&meta).ok_or(Failure::NeedsRefine)?.clone();
        let stmt = db.stmt(tree.stmt);
        if stmt.kind == Kind::Float {
            session
                .assign(meta, stmt.expr[1])
                .map_err(Failure::Session)?;
            assigns += 1;
        } else if refine {
            let outcome = session.refine(meta, tree.stmt).map_err(Failure::Session)?;
            for (new, child) in outcome.metas.zip(&tree.children) {
                expected.insert(new, child.clone());
            }
            refines += 1;
        } else {
            return Err(Failure::NeedsRefine);
        }
    }
    let finished = session.export().map_err(Failure::Session)?;
    Ok(Replayed {
        ops: session.ops().len(),
        assigns,
        refines,
        peak: session.peak(),
        dummies: finished.dummies.len(),
        saved: session.save().len(),
        finished,
    })
}

/// Writes the exported proof into the database text in place of the original and verifies it.
/// The text after the theorem is dropped; nothing there can affect its proof.
pub fn verify_spliced(src: &str, db: &Database, finished: &Finished) -> Result<(), Failure> {
    let label = &db.stmt(finished.theorem).label;
    let (text, cut) = splice_at(src, label, &finished.statement(db))
        .map_err(|e| Failure::Splice(format!("{e:?}")))?;
    let spliced = mcpack::mm::parse(close_scopes(&text, cut).into_bytes())
        .map_err(|e| Failure::Parse(e.to_string()))?;
    let id = spliced
        .find(label)
        .ok_or_else(|| Failure::Splice("label vanished".into()))?;
    Verifier::new(&spliced)
        .verify(id)
        .map_err(Failure::Rejected)
}

#[derive(Default)]
pub struct Report {
    /// Theorems rebuilt and exported.
    pub rebuilt: usize,
    /// Theorems whose exported proof the reference verifier accepted in the spliced database.
    pub verified: usize,
    /// Theorems whose original proof the verifier rejects, skipped.
    pub skipped: usize,
    pub failures: Vec<(String, Failure)>,
    pub ops: usize,
    pub assigns: usize,
    pub refines: usize,
    pub with_dummies: usize,
    pub with_dv: usize,
    pub peak: Sizes,
    pub peak_theorem: String,
    /// The peak sizes of every rebuilt theorem.
    pub samples: Vec<Sizes>,
    /// The saved size of every rebuilt theorem, in bytes.
    pub saved: Vec<usize>,
    pub rebuild_time: Duration,
    pub verify_time: Duration,
}

/// Rebuilds every theorem `select` keeps, given its position among the database's theorems, and
/// verifies the exported proofs `check` keeps.
pub fn replay_database(
    src: &str,
    select: impl Fn(usize) -> bool,
    check: impl Fn(usize, &Replayed) -> bool,
    refine: bool,
) -> Result<Report, String> {
    let db = mcpack::mm::parse(src.as_bytes().to_vec()).map_err(|e| e.to_string())?;
    let grammar = Grammar::new(&db, &[("|-", "wff")]).map_err(|e| format!("{e:?}"))?;
    let catalog = Catalog::new(&db, &grammar, mcpack::library::crc32(src.as_bytes()));
    let mut report = Report::default();
    for (n, theorem) in db.theorems().enumerate() {
        if !select(n) {
            continue;
        }
        let label = db.stmt(theorem).label.clone();
        let started = Instant::now();
        let replayed = replay(&catalog, theorem, refine);
        report.rebuild_time += started.elapsed();
        match replayed {
            Err(Failure::Original(_)) => report.skipped += 1,
            Err(e) => report.failures.push((label, e)),
            Ok(done) => {
                report.rebuilt += 1;
                if check(n, &done) {
                    let started = Instant::now();
                    let verified = verify_spliced(src, &db, &done.finished);
                    report.verify_time += started.elapsed();
                    match verified {
                        Ok(()) => report.verified += 1,
                        Err(e) => report.failures.push((label.clone(), e)),
                    }
                }
                report.ops += done.ops;
                report.assigns += done.assigns;
                report.refines += done.refines;
                report.with_dummies += usize::from(done.dummies > 0);
                report.with_dv += usize::from(!done.finished.dv.is_empty());
                if done.peak.terms.nodes > report.peak.terms.nodes {
                    report.peak_theorem = label;
                }
                report.samples.push(done.peak);
                report.saved.push(done.saved);
                let p = &mut report.peak;
                p.terms.nodes = p.terms.nodes.max(done.peak.terms.nodes);
                p.terms.args = p.terms.args.max(done.peak.terms.args);
                p.terms.metas = p.terms.metas.max(done.peak.terms.metas);
                p.goals = p.goals.max(done.peak.goals);
                p.conditions = p.conditions.max(done.peak.conditions);
                p.ops = p.ops.max(done.ops);
            }
        }
    }
    Ok(report)
}
