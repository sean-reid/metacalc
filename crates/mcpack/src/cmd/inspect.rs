use super::{fail, grammar};
use mcpack::measure::measure;
use mcpack::mm::{self, Database, Kind, StmtId, Verifier, shown};
use std::process::ExitCode;

fn find_all(db: &Database, labels: &[String]) -> Result<Vec<StmtId>, ExitCode> {
    labels
        .iter()
        .map(|label| {
            db.find(label)
                .ok_or_else(|| super::unusable(format_args!("no statement is labelled {label}")))
        })
        .collect()
}

pub(super) fn kept(db: &Database, roots: &[String]) -> Result<Vec<StmtId>, ExitCode> {
    let roots = find_all(db, roots)?;
    mm::closure(db, &grammar(db)?, &roots).map_err(|(id, e)| {
        fail(format_args!(
            "{}: {e:?}",
            shown(db.stmt(id).label.as_bytes())
        ))
    })
}

pub fn closure(db: &Database, roots: &[String], splits: &[String]) -> Result<ExitCode, ExitCode> {
    let splits = find_all(db, splits)?;
    let kept = kept(db, roots)?;
    let report = measure(db, &kept, &splits);
    print!("{report}");
    for (label, e) in &report.failures {
        eprintln!("{}: {e:?}", shown(label.as_bytes()));
    }
    Ok(if report.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub fn parse(db: &Database) -> Result<ExitCode, ExitCode> {
    let grammar = grammar(db)?;
    let (mut parsed, mut failed) = (0usize, 0usize);
    for id in 0..db.stmts.len() as StmtId {
        let stmt = db.stmt(id);
        if !matches!(stmt.kind, Kind::Axiom | Kind::Theorem) {
            continue;
        }
        parsed += 1;
        match grammar.parse(id) {
            Ok(parsed) => {
                let essentials = stmt.frame.hyps.iter().map(|&hyp| db.stmt(hyp));
                let essentials = essentials.filter(|hyp| hyp.kind == Kind::Essential);
                let spelled = essentials
                    .chain([stmt])
                    .zip(parsed.hyps.iter().chain([&parsed.conclusion]))
                    .all(|(stmt, tree)| grammar.expr(tree)[1..] == stmt.expr[1..]);
                if !spelled {
                    failed += 1;
                    eprintln!(
                        "{}: the parse does not spell the statement",
                        shown(stmt.label.as_bytes())
                    );
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e:?}", shown(stmt.label.as_bytes()));
            }
        }
    }
    println!("{parsed} assertions parsed, {failed} failed");
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub fn verify(db: &Database) -> ExitCode {
    let mut verifier = Verifier::new(db);
    let (mut checked, mut failed) = (0usize, 0usize);
    for id in db.theorems() {
        checked += 1;
        if let Err(e) = verifier.verify(id) {
            failed += 1;
            eprintln!("{}: {e:?}", shown(db.stmt(id).label.as_bytes()));
        }
    }
    println!("{checked} proofs checked, {failed} failed");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

pub fn outline(db: &Database, label: &str) -> Result<ExitCode, ExitCode> {
    let id = db
        .find(label)
        .ok_or_else(|| super::unusable(format_args!("no statement is labelled {label}")))?;
    let tree = mm::outline(db, id, "|-").map_err(|e| fail(format_args!("{label}: {e}")))?;
    print!("{}", tree.render(db));
    Ok(ExitCode::SUCCESS)
}
