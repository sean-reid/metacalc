//! The folding properties over every assertion of the checked-in corpus databases.

mod support;

use mcglyph::Font;
use mcpack::library::{self, Hypothesis};
use mcpack::mm::{self, Kind};
use mcui::data::{Installed, Lib};
use mcui::expr::Expr;
use mcui::fold::{Fit, Folds, fit, layout};
use mcui::{Key, State, Status, Statuses, Ui};
use std::path::Path;

fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mcpack/tests/corpus");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "mm"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(p).unwrap())
        })
        .collect()
}

fn rows(expr: &Expr, folds: &Folds, width: usize) -> usize {
    layout(expr, folds, width).rows()
}

/// What must hold of the folds `fit` chose for one statement set.
fn check_fit(exprs: &[&Expr], what: &str) {
    for (width, want) in [(94, 1), (94, 6), (60, 6), (30, 3)] {
        let items: Vec<Fit> = exprs.iter().map(|&expr| Fit { expr, width }).collect();
        let folds = fit(&items, want);
        let used: usize = exprs
            .iter()
            .zip(&folds)
            .map(|(e, f)| rows(e, f, width))
            .sum();
        let all: usize = exprs
            .iter()
            .map(|e| {
                let mut f = Folds::none(e);
                f.toggle(e, 0);
                rows(e, &f, width)
            })
            .sum();
        if all <= want {
            assert!(
                used <= want,
                "{what}: {used} rows against {want} at {width}"
            );
        }
        for (expr, mut folds) in exprs.iter().zip(folds) {
            for n in 0..expr.len() {
                if folds.is_folded(n) {
                    folds.toggle(expr, n);
                }
            }
            let open = layout(expr, &folds, width);
            let tokens = expr.tokens();
            assert_eq!(open.lines, font_lines(&tokens, width), "{what}");
        }
    }
}

/// The font's own layout, for tokens it knows; the corpus has tokens it does not.
fn font_lines(tokens: &[&str], width: usize) -> Vec<mcglyph::Line> {
    let font = Font::builtin();
    let items: Vec<mcglyph::Item> = tokens
        .iter()
        .map(|t| match (font.token_width(t), font.token_pad(t)) {
            (Some(width), Some(pad)) => mcglyph::Item { width, pad },
            _ => mcglyph::Item {
                width: font.text_width(t),
                pad: mcglyph::Pad::default(),
            },
        })
        .collect();
    mcglyph::place(&items, width)
}

#[test]
fn every_assertion_of_every_corpus_database_folds_without_panicking_as_a_flat_run_of_tokens() {
    let mut seen = 0;
    for (name, bytes) in corpus() {
        let Ok(db) = mm::parse(bytes) else { continue };
        for id in 0..db.stmts.len() as mm::StmtId {
            let stmt = db.stmt(id);
            if !stmt.kind.is_assertion() {
                continue;
            }
            let tokens: Vec<String> = stmt.expr[1..]
                .iter()
                .map(|&s| db.syms[s as usize].name.clone())
                .collect();
            let lead = vec![db.syms[stmt.expr[0] as usize].name.clone()];
            let expr = Expr::flat(lead, &tokens);
            check_fit(&[&expr], &format!("{name} {}", stmt.label));
            seen += 1;
        }
    }
    assert!(seen >= 50, "only {seen} assertions");
}

#[test]
fn every_assertion_of_the_packable_databases_folds_as_a_tree_and_matches_its_packed_statement() {
    let mut tree_libs = 0;
    for (name, bytes) in corpus() {
        let Ok(db) = mm::parse(bytes) else { continue };
        let Some(packed) = support::pack_all(&db) else {
            continue;
        };
        let assertions = library::read(&packed).unwrap();
        let table = vec![Status::Checked; assertions.len()];
        let lib = Lib::new(
            Installed {
                name: name.clone(),
                bytes: packed,
            },
            table,
        )
        .unwrap();
        for (i, assertion) in assertions.iter().enumerate() {
            let statement = lib.statement(i);
            let want_hyps: Vec<&Vec<String>> = assertion
                .hypotheses
                .iter()
                .filter_map(|h| match h {
                    Hypothesis::Essential(e) => Some(e),
                    Hypothesis::Float { .. } => None,
                })
                .collect();
            assert_eq!(statement.hyps.len(), want_hyps.len());
            for (expr, want) in statement
                .all()
                .into_iter()
                .zip(want_hyps.into_iter().chain([&assertion.conclusion]))
            {
                assert_eq!(
                    expr.tokens(),
                    want.iter().map(String::as_str).collect::<Vec<_>>(),
                    "{name} {}",
                    assertion.label
                );
            }
            check_fit(&statement.all(), &format!("{name} {}", assertion.label));
        }
        tree_libs += 1;
    }
    assert!(tree_libs >= 3, "{tree_libs} packable databases");
}

#[test]
fn every_screen_opens_for_every_assertion_of_the_packable_databases() {
    let (mut exercised, mut sound) = (0, 0);
    for (name, bytes) in corpus() {
        let Ok(db) = mm::parse(bytes) else { continue };
        let Some(packed) = support::pack_all(&db) else {
            continue;
        };
        let assertions = library::read(&packed).unwrap();
        let count = assertions.len();
        let accepted = library::check(&packed).is_ok();
        let mut ui = Ui::new(
            vec![Installed {
                name: name.clone(),
                bytes: packed,
            }],
            Statuses {
                libraries: vec![vec![Status::Checked; count]],
                my_proofs: 0,
            },
        )
        .unwrap();
        support::press(&mut ui, &[Key::Enter]);
        let mut proofs = 0;
        for assertion in &assertions {
            let show = |ui: &mut Ui, keys: &[Key]| {
                support::press(ui, keys);
                let frame = ui.frame();
                assert_eq!((frame.width(), frame.height()), (96, 64), "{name}");
            };
            show(&mut ui, &[Key::Enter]);
            show(&mut ui, &[Key::F2, Key::Clear]);
            show(
                &mut ui,
                &[
                    Key::Down,
                    Key::Right,
                    Key::Enter,
                    Key::Up,
                    Key::Left,
                    Key::F1,
                ],
            );
            show(&mut ui, &[Key::F3]);
            let opened = matches!(ui.state(), State::Proof { .. });
            let provable = assertion.theorem && assertion.conclusion[0] == "|-";
            if accepted && provable {
                assert!(opened, "{name} {}: no proof opened", assertion.label);
            }
            proofs += usize::from(opened);
            show(
                &mut ui,
                &[
                    Key::Down,
                    Key::Enter,
                    Key::Down,
                    Key::Enter,
                    Key::Up,
                    Key::Up,
                ],
            );
            show(
                &mut ui,
                &[Key::F1, Key::Left, Key::Left, Key::F2, Key::Right, Key::F1],
            );
            show(&mut ui, &[Key::Clear, Key::Clear]);
            support::press(&mut ui, &[Key::Down]);
            assert!(matches!(ui.state(), State::List { .. }), "{name}");
        }
        exercised += 1;
        if accepted {
            sound += 1;
            assert!(proofs > 0, "{name} opened no proof");
        }
        assert!(db.stmts.iter().any(|s| s.kind == Kind::Theorem));
    }
    assert!(exercised >= 3, "{exercised} databases walked");
    assert!(sound >= 2, "{sound} databases the kernel accepts");
}

/// The packable corpus databases and the support logics, with a name each.
fn databases() -> Vec<(String, mm::Database)> {
    let mut all: Vec<(String, mm::Database)> = corpus()
        .into_iter()
        .filter_map(|(name, bytes)| Some((name, mm::parse(bytes).ok()?)))
        .collect();
    all.push(("logic".into(), support::db(support::LOGIC)));
    all.push((
        "extra".into(),
        support::db(&format!("{}{}", support::LOGIC, support::EXTRA)),
    ));
    all
}

#[test]
fn the_stored_trees_give_every_statement_the_tree_the_export_and_parse_path_gives() {
    let mut compared = 0;
    for (name, db) in databases() {
        let (Some(with), Some(without)) = (support::pack_all(&db), support::pack_all_kernel(&db))
        else {
            continue;
        };
        if library::check(&with).is_err() {
            continue;
        }
        let load = |bytes: Vec<u8>| {
            let table = vec![Status::Checked; library::read(&bytes).unwrap().len()];
            Lib::new(
                Installed {
                    name: name.clone(),
                    bytes,
                },
                table,
            )
            .unwrap()
        };
        let (stored, parsed) = (load(with), load(without));
        assert_eq!(stored.len(), parsed.len());
        for i in 0..stored.len() {
            let (a, b) = (stored.statement(i), parsed.statement(i));
            assert_eq!(a.all(), b.all(), "{name} {}", stored.label(i));
            compared += a.all().len();
        }
    }
    assert!(compared > 50, "{compared} statements compared");
}
