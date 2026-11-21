use crate::fit::*;
use crate::fixture;
use crate::nodes::Model;
use crate::rig::*;
use crate::support::*;
use mcpack::library::Library;
use mcui::canvas::{CONTENT_ROWS, STATEMENT_WIDTH};
use mcui::fold::{Folds, fit_one};

/// `vars` variables joined by conjunctions, nested to the right.
fn nested(vars: usize) -> String {
    let names = ["ph", "ps", "ch", "th", "ta"];
    let open: String = (0..vars - 1)
        .map(|i| format!("( {} /\\ ", names[i % 5]))
        .collect();
    format!("{open}{}{}", names[(vars - 1) % 5], " )".repeat(vars - 1))
}

/// `count` negations in front of a variable.
fn negations(count: usize) -> String {
    format!("{}ph", "-. ".repeat(count))
}

fn cases_source() -> String {
    let mut src = String::from("ax-id $a |- ( ph -> ph ) $.\n");
    src += &format!(
        "${{\nf1.1 $e |- {} $.\nf1.2 $e |- ( ph -> ( ps -> ch ) ) $.\nf1 $a |- ph $.\n$}}\n",
        nested(22)
    );
    src += &format!(
        "${{\ng1.1 $e |- {0} $.\ng1.2 $e |- {0} $.\ng1 $a |- ps $.\n$}}\n",
        nested(16)
    );
    src += &format!(
        "${{\nf2.1 $e |- {} $.\nf2.2 $e |- {} $.\nf2 $a |- ph $.\n$}}\n",
        negations(8),
        nested(22)
    );
    src += "${\n";
    for h in 1..=7 {
        src += &format!("e8.{h} $e |- {} $.\n", nested(5));
    }
    src += &format!("e8 $a |- {} $.\n$}}\n", nested(14));
    src += "${\nflatone.1 $e |- ph $.\nflatone $a |- ( ph -> ps ) $.\n$}\n";
    for (label, hyps, size) in [("big3", 3, 130), ("big2", 2, 100)] {
        src += "${\n";
        for h in 1..=hyps {
            src += &format!("{label}.{h} $e |- {} $.\n", negations(size));
        }
        src += &format!("{label} $a |- {} $.\n$}}\n", negations(size));
    }
    src
}

pub fn cases_library() -> Vec<u8> {
    fixture::custom(&cases_source())
}

fn index_of(library: &Library, label: &str) -> u16 {
    library
        .assertions
        .iter()
        .position(|a| a.label == label)
        .unwrap_or_else(|| panic!("no assertion {label}")) as u16
}

struct Named {
    fm: Fm,
    model: Model,
    library: Library,
}

impl Named {
    fn new(bytes: &[u8]) -> Named {
        Named {
            fm: Fm::new(bytes),
            model: Model::new(bytes),
            library: decoded(bytes),
        }
    }

    /// Compares fm with the model for the assertion at every width and returns the outcome and
    /// the oracle at the screen's width.
    fn at(&mut self, label: &str, rows: usize) -> (Outcome, Oracle) {
        let n = index_of(&self.library, label);
        let mut tally = Tally::default();
        let mut first = None;
        for width in WIDTHS {
            let outcome = compare(
                &mut self.fm,
                &self.model,
                &self.library,
                (n, rows, width),
                FULL,
                &mut tally,
            );
            first.get_or_insert(outcome);
        }
        let hyps = essentials(&self.library, n);
        (
            first.unwrap(),
            oracle(&self.model, n, hyps, rows, STATEMENT_WIDTH),
        )
    }
}

fn folded(o: &Oracle) -> Vec<usize> {
    o.folds.iter().map(Folds::count).collect()
}

#[test]
fn a_statement_that_already_fits_is_left_unfolded() {
    let mut named = Named::new(&cases_library());
    for label in ["ax-id", "flatone"] {
        let (outcome, o) = named.at(label, CONTENT_ROWS);
        assert_eq!(outcome, Outcome::Unfolded, "{label}");
        assert_eq!(folded(&o).iter().sum::<usize>(), 0);
        assert!(o.total() <= CONTENT_ROWS);
    }
}

#[test]
fn one_long_statement_is_folded_one_level_further_for_each_line_taken_away() {
    let bytes = fixture::nodes_library();
    let mut named = Named::new(&bytes);
    let mut sets = Vec::new();
    for rows in (1..=CONTENT_ROWS).rev() {
        let (_, o) = named.at("longest", rows);
        assert!(o.total() <= rows.max(1), "{rows} rows: {}", o.total());
        sets.push((rows, folded(&o)[0]));
    }
    println!("longest, folds by rows: {sets:?}");
    assert!(sets.windows(2).all(|w| w[0].1 <= w[1].1), "{sets:?}");
    let mut distinct: Vec<usize> = sets.iter().map(|s| s.1).collect();
    distinct.dedup();
    assert!(distinct.len() >= 4, "{sets:?}");
    assert!(
        sets[0].1 > 0,
        "the statement does not fit six lines as it is"
    );
}

#[test]
fn folding_the_first_statement_is_enough_and_a_line_of_one_is_skipped() {
    let mut named = Named::new(&cases_library());
    for rows in [CONTENT_ROWS, 3] {
        let (outcome, o) = named.at("f1", rows);
        assert_eq!(outcome, Outcome::Fitted);
        let counts = folded(&o);
        assert!(
            counts[0] > 0 && counts[1] == 0 && counts[2] == 0,
            "{counts:?}"
        );
        assert_eq!(o.rows_of(1), 1, "the second hypothesis takes a line");
        assert!(
            (1..o.exprs[1].len()).any(|n| o.exprs[1].foldable(n)),
            "and it has a node below its root to fold"
        );
    }
}

#[test]
fn a_line_of_one_ahead_of_a_long_statement_is_skipped_at_every_depth() {
    let mut named = Named::new(&cases_library());
    for rows in [3, 4] {
        let (outcome, o) = named.at("f2", rows);
        assert_eq!(outcome, Outcome::Fitted);
        let counts = folded(&o);
        assert_eq!((counts[0], counts[2]), (0, 0), "{counts:?}");
        assert!(counts[1] > 0);
        assert_eq!(o.rows_of(0), 1);
        let eager = without_skip(&o, rows);
        assert_ne!(eager, o.folds, "folding it too would show at {rows} rows");
    }
}

#[test]
fn folding_the_first_statement_is_not_enough_so_the_second_folds_too() {
    let mut named = Named::new(&cases_library());
    let (outcome, o) = named.at("g1", CONTENT_ROWS);
    assert_eq!(outcome, Outcome::Fitted);
    let counts = folded(&o);
    assert!(counts[0] > 0 && counts[1] > 0, "{counts:?}");
    assert!(o.total() <= CONTENT_ROWS);
}

#[test]
fn more_statements_than_rows_fit_alone_to_a_line_and_the_conclusion_to_all_the_rows() {
    let mut named = Named::new(&cases_library());
    let (outcome, o) = named.at("e8", CONTENT_ROWS);
    assert_eq!(outcome, Outcome::Alone);
    assert!(o.exprs.len() > CONTENT_ROWS);
    for s in 0..o.exprs.len() - 1 {
        assert_eq!(o.rows_of(s), 1, "hypothesis {s}");
    }
    let last = o.exprs.len() - 1;
    let one_line = fit_one(&o.exprs[last], o.widths[last], 1);
    assert_ne!(
        o.folds[last], one_line,
        "the conclusion keeps more than a line"
    );
    assert!(o.rows_of(last) > 1 && o.rows_of(last) <= CONTENT_ROWS);
    for rows in [1, 2, 3, 8] {
        named.at("e8", rows);
    }
}

#[test]
fn a_flat_statement_has_nothing_to_fold() {
    let bytes = fixture::unstructured();
    let library = decoded(&bytes);
    let all = statements(&library);
    let mut fm = Fm::new(&bytes);
    let mut long = 0;
    for n in 0..library.assertions.len() {
        let mine: Vec<_> = all
            .iter()
            .filter(|(a, _, _)| usize::from(*a) == n)
            .collect();
        let hyps = mine.len() - 1;
        let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
        for rows in [CONTENT_ROWS, 1] {
            assert_eq!(fm.assertion(n as u16).status(), 0);
            let out = fm.fit(rows, hyp, concl);
            assert_eq!(out.status(), 0, "assertion {n}");
            let want: usize = mine
                .iter()
                .enumerate()
                .map(|(s, (_, _, t))| model_rows(t, if s == hyps { concl } else { hyp }))
                .sum();
            assert_eq!(usize::from(out.hl), want, "assertion {n}, {rows} rows");
            long += usize::from(want > rows);
            for (s, (_, k, _)) in mine.iter().enumerate() {
                assert_eq!(fm.select(*k).status(), 0, "{s}");
                assert_eq!(fm.nodes().status(), 0);
                assert_eq!(
                    fm.flags(),
                    vec![false],
                    "statement {s} is one node, unfolded"
                );
            }
        }
    }
    assert!(long > 3, "{long} flat assertions longer than the rows");
}

#[test]
fn an_assertion_with_no_hypotheses_fits_and_folds_by_its_conclusion_alone() {
    let mut named = Named::new(&fixture::nodes_library());
    let (outcome, o) = named.at("ax-2", 1);
    assert_eq!(outcome, Outcome::Fitted);
    assert_eq!(o.exprs.len(), 1);
    assert_eq!(o.rows_of(0), 1);
    assert!(folded(&o)[0] > 0);
}

#[test]
fn the_too_large_assertions_are_refused_with_no_fold_changed() {
    let bytes = cases_library();
    let mut named = Named::new(&bytes);
    let mut refused = 0;
    for label in ["big3", "big2"] {
        let n = index_of(&named.library, label);
        let hyps = essentials(&named.library, n);
        let large = !named.model.within(n);
        assert_eq!(named.fm.assertion(n).status(), 0, "{label}");
        assert_eq!(named.fm.select(0xFF).status(), 0);
        let hand = named.fm.fold(0, 1);
        assert_eq!((hand.status(), count(hand)), (0, 1));
        let (hyp, concl) = widths_of(hyps, STATEMENT_WIDTH);
        let out = named.fm.fit(CONTENT_ROWS, hyp, concl);
        let status = out.status();
        println!("{label}: model too large {large}, fm_fit status {status}");
        assert_eq!(status == code("FM_ERR_LIMIT"), large, "{label}");
        if large {
            refused += 1;
            assert_eq!(named.fm.select(0xFF).status(), 0);
            let flags = named.fm.flags();
            assert!(
                flags[0] && flags[1..].iter().all(|&f| !f),
                "the hand fold stays"
            );
            let rows = named.fm.rows(concl);
            assert_eq!(count(rows), 1, "and the statement still shows it");
        }
    }
    assert_eq!(refused, 1, "big3 is past the 384 nodes and big2 is not");
}

/// The model's fold_levels with the rule that skips a statement of one line taken out.
fn without_skip(o: &Oracle, rows: usize) -> Vec<Folds> {
    use mcui::fold::layout as lay;
    let total = |f: &[Folds]| -> usize {
        (0..f.len())
            .map(|s| lay(&o.exprs[s], &f[s], o.widths[s]).rows())
            .sum()
    };
    let mut folds: Vec<Folds> = o.exprs.iter().map(Folds::none).collect();
    if total(&folds) <= rows {
        return folds;
    }
    let deepest = o
        .exprs
        .iter()
        .flat_map(|e| (0..e.len()).map(|n| e.depth(n)))
        .max()
        .unwrap();
    for depth in (0..=deepest).rev() {
        for s in 0..o.exprs.len() {
            let mut changed = false;
            for n in (0..o.exprs[s].len()).filter(|&n| o.exprs[s].depth(n) == depth) {
                if !folds[s].is_folded(n) {
                    changed |= folds[s].toggle(&o.exprs[s], n);
                }
            }
            if changed && total(&folds) <= rows {
                return folds;
            }
        }
    }
    folds
}
