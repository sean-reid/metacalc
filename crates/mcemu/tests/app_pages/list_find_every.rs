//! A prefix for every label of the proofs library, typed in full or to the cap, on each stage under
//! each kind, and the title at the widths where the prefix is cut: every frame is the model's.

use crate::list_find_rig::*;
use crate::list_kind_rig::*;
use crate::list_rig::*;
use mcui::Kind;

#[test]
fn every_label_typed_to_the_cap_lands_where_the_model_lands_it_under_each_kind() {
    let built = proofs_three();
    let labels = labels_of(built);
    let marks = every_mark(built);
    for (kind, table) in [
        (Kind::All, false),
        (Kind::Theorems, false),
        (Kind::Axioms, false),
        (Kind::Unchecked, true),
        (Kind::Unchecked, false),
        (Kind::All, true),
        (Kind::Theorems, true),
        (Kind::Axioms, true),
    ] {
        let stages: &[usize] = if kind == Kind::All {
            &[0, 1, 2]
        } else {
            &[0, 2]
        };
        let (mut hits, mut misses) = (0, 0);
        for &stage in stages {
            let mut list = list_under(built, stage, kind, table.then_some(&marks));
            let items = list.items();
            for label in &labels {
                list.f2();
                let typed: String = label.chars().take(8).collect();
                list.type_text(&typed);
                let owned = items.iter().any(|&i| labels[i].starts_with(&typed));
                assert_eq!(
                    list.find().missed,
                    !owned,
                    "{kind:?} stage {stage}: {typed}"
                );
                if owned {
                    hits += 1;
                } else {
                    misses += 1;
                }
            }
            list.skip_the_wait();
            list.settle();
        }
        println!("{kind:?} table {table}: {hits} hits, {misses} misses");
        let floor = if kind == Kind::Axioms { 15 } else { 20 };
        assert!(
            hits >= floor && misses >= 20,
            "{kind:?}: {hits} hits, {misses} misses"
        );
    }
}

#[test]
fn the_title_shows_the_prefix_whole_or_cut_where_it_stops_fitting_after_a_miss() {
    let width = |s: &str| mcui::canvas::text_width(s);
    let mut cases: Vec<String> = Vec::new();
    let alphabet = ['z', 'm', 'c', 'j', 'i', '.'];
    let mut stack = vec![String::from("z")];
    while let Some(s) = stack.pop() {
        if s.len() >= 5
            && [40, 41, 42, 43, 47].contains(&width(&s))
            && cases
                .iter()
                .filter(|c| width(c) == width(&s) && c.len() == s.len())
                .count()
                < 2
        {
            cases.push(s.clone());
        }
        if s.len() < 8 {
            for c in alphabet {
                stack.push(format!("{s}{c}"));
            }
        }
    }
    cases.push("mmmmmmmm".to_string());
    let widths: std::collections::BTreeSet<usize> = cases.iter().map(|c| width(c)).collect();
    assert!(widths.is_superset(&[40, 41, 42, 43].into()), "{widths:?}");
    assert!(cases.iter().any(|c| width(c) == 47), "the widest eight");
    let mut list = Lister::open(proofs_three(), None, 1);
    list.settle();
    for case in &cases {
        list.f2();
        list.type_text(case);
        assert!(list.find().missed, "{case}");
        let (_, missed) = list.model_find();
        assert!(missed);
    }
    assert!(cases.len() >= 8, "{} cases", cases.len());
    println!("{} prefixes at widths {widths:?}", cases.len());
}
