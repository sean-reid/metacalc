//! Screen 5: proving a theorem backwards, driven by keys only, against the real editor.

mod support;

use mcedit::oracle::verify_spliced;
use mcedit::{Catalog, Session};
use mcpack::library;
use mcpack::mm::{self, Grammar};
use mcui::Key::{self, *};
use mcui::{SavedProof, State, Ui, View};
use support::{bare_bytes, check_frame, index_of, open, press, proving_bytes, ui_over};

fn check(ui: &Ui, name: &str) {
    check_frame(&ui.frame(), name);
}

struct Where {
    open: usize,
    finished: bool,
    view: View,
    steps: usize,
    candidate: Option<String>,
    goal: Option<u32>,
}

fn at(ui: &Ui) -> Where {
    match ui.state() {
        State::Proving {
            open,
            finished,
            view,
            steps,
            candidate,
            goal,
            ..
        } => Where {
            open,
            finished,
            view,
            steps,
            candidate,
            goal,
        },
        other => panic!("not the proving screen: {other:?}"),
    }
}

fn replayed(ui: &Ui) -> usize {
    match ui.state() {
        State::Proving { replayed, .. } => replayed,
        other => panic!("not the proving screen: {other:?}"),
    }
}

fn notice(ui: &Ui) -> String {
    match ui.state() {
        State::Notice { title } => title,
        other => panic!("not a notice: {other:?}"),
    }
}

/// Opens theorem `label` of `bytes` and presses Try.
fn prove(bytes: &[u8], label: &str) -> Ui {
    let mut ui = ui_over(bytes.to_vec(), &[], &[]);
    open(&mut ui, label);
    ui.press(F4);
    ui
}

/// Moves the highlight to candidate `label`.
fn select(ui: &mut Ui, label: &str) {
    for _ in 0..300 {
        if at(ui).candidate.as_deref() == Some(label) {
            return;
        }
        ui.press(Down);
    }
    panic!("no candidate {label}");
}

/// A proof in progress. After every operation, while the candidates are showing and their
/// highlight is on the first row, a second interface over the same library, given only the saved
/// form, must open on the same frame. A restore reproduces the proof and the goal on screen; the
/// cursor and scroll of a list are not saved and start from the top.
struct Prover {
    ui: Ui,
    bytes: Vec<u8>,
    label: String,
    rounds: usize,
}

impl Prover {
    fn new(bytes: Vec<u8>, label: &str) -> Prover {
        Prover {
            ui: prove(&bytes, label),
            bytes,
            label: label.into(),
            rounds: 0,
        }
    }

    fn find_saved(&self) -> Option<SavedProof> {
        let theorem = index_of(&self.bytes, &self.label);
        self.ui
            .saved_proofs()
            .find(|p| p.theorem == theorem)
            .cloned()
    }

    fn saved(&self) -> SavedProof {
        self.find_saved().expect("the proof is saved")
    }

    fn round_trip(&mut self) {
        let saved = self.find_saved();
        let mut fresh = ui_over(self.bytes.clone(), &[], &[]);
        if let Some(saved) = &saved {
            fresh
                .restore_proof(saved.clone())
                .expect("the form restores");
        }
        open(&mut fresh, &self.label);
        fresh.press(F4);
        assert!(
            fresh.frame() == self.ui.frame(),
            "{} after {:?}\n{}\n{}",
            self.label,
            saved.as_ref().map(|s| s.steps),
            self.ui.frame(),
            fresh.frame()
        );
        assert_eq!(fresh.saved_proofs().next(), saved.as_ref());
        self.rounds += 1;
    }

    /// Presses `keys`, and when the screen is back on its candidates checks the round trip.
    fn keys(&mut self, keys: &[Key]) {
        press(&mut self.ui, keys);
        if at(&self.ui).view == View::Candidates {
            self.round_trip();
        }
    }

    fn aply(&mut self, label: &str) {
        select(&mut self.ui, label);
        self.keys(&[F1]);
    }

    /// Closes the goal by hypothesis `n`, counting from one.
    fn hyp(&mut self, n: usize) {
        press(&mut self.ui, &[F2]);
        press(&mut self.ui, &vec![Down; n - 1]);
        self.keys(&[F1]);
    }

    /// Jumps to row `row` of the outline.
    fn tree(&mut self, row: usize) {
        press(&mut self.ui, &[F5]);
        press(&mut self.ui, &vec![Down; row]);
        self.keys(&[Enter]);
    }

    fn finished(&self) {
        let now = at(&self.ui);
        assert!(now.finished, "{} is not finished", self.label);
        assert_eq!(now.open, 0);
        assert_eq!(now.goal, None);
        self.verify();
    }

    /// The exported proof, spliced in place of the original, passes the reference verifier.
    fn verify(&self) {
        let text = library::export(&self.bytes).expect("the library exports");
        let db = mm::parse(text.clone().into_bytes()).expect("the export parses");
        let grammar = Grammar::new(&db, &[("|-", "wff")]).expect("the grammar builds");
        let session = Session::restore(Catalog::new(&db, &grammar), &self.saved().session)
            .expect("the saved form restores");
        let finished = session.export().expect("the session exports");
        assert_eq!(finished.theorem, db.find(&self.label).unwrap());
        assert_eq!(
            verify_spliced(&text, &db, &finished),
            Ok(()),
            "{}",
            self.label
        );
    }
}

#[test]
fn try_opens_the_proving_screen_from_the_list_and_the_theorem() {
    let bytes = proving_bytes();
    let mut ui = ui_over(bytes.clone(), &[], &[]);
    press(&mut ui, &[Enter, F2]);
    support::typed(&mut ui, "a1i");
    ui.press(F4);
    assert!(matches!(ui.state(), State::Proving { ref label, .. } if label == "a1i"));
    assert_eq!(ui.depth(), 3);
    ui.press(Clear);
    ui.press(Enter);
    assert!(matches!(ui.state(), State::Theorem { .. }));
    ui.press(F4);
    assert!(matches!(ui.state(), State::Proving { ref label, .. } if label == "a1i"));
    assert_eq!(ui.depth(), 4);
}

#[test]
fn an_axiom_and_a_syntax_theorem_cannot_be_proved() {
    let mut ui = ui_over(proving_bytes(), &[], &[]);
    open(&mut ui, "ax-1");
    ui.press(F4);
    assert_eq!(notice(&ui), "Prove");
    check(&ui, "proving-notice-axiom");
    home(&mut ui);
    open(&mut ui, "wtwice");
    ui.press(F4);
    assert_eq!(notice(&ui), "Prove");
    check(&ui, "proving-notice-syntax");
}

#[test]
fn proving_a1i_with_aply_and_hyp() {
    let mut p = Prover::new(proving_bytes(), "a1i");
    check(&p.ui, "proving-root");
    assert_eq!(at(&p.ui).open, 1);
    assert_eq!(at(&p.ui).candidate.as_deref(), Some("ax-mp"));
    p.aply("ax-mp");
    check(&p.ui, "proving-after-apply");
    assert_eq!(at(&p.ui).open, 2);
    assert_eq!(at(&p.ui).candidate.as_deref(), Some("ax-1"));
    p.aply("ax-1");
    assert_eq!(at(&p.ui).open, 1);
    press(&mut p.ui, &[F2]);
    assert_eq!(at(&p.ui).view, View::Hyps);
    check(&p.ui, "proving-hyps");
    p.keys(&[F1]);
    check(&p.ui, "proving-finished");
    p.finished();
    assert!(p.rounds >= 3);
}

#[test]
fn proving_mpd_with_the_tree_and_undo() {
    let mut p = Prover::new(proving_bytes(), "mpd");
    p.aply("ax-mp");
    assert_eq!(at(&p.ui).open, 2);
    p.aply("a2i");
    assert_eq!(at(&p.ui).open, 2);
    p.keys(&[F5]);
    assert_eq!(at(&p.ui).view, View::Tree);
    check(&p.ui, "proving-tree");
    p.keys(&[Clear]);
    p.hyp(2);
    assert_eq!(at(&p.ui).open, 1);
    p.keys(&[F4]);
    assert_eq!(at(&p.ui).open, 2);
    p.hyp(2);
    p.hyp(1);
    p.finished();
    p.keys(&[F4]);
    assert!(!at(&p.ui).finished);
    assert_eq!(at(&p.ui).open, 1);
    p.hyp(1);
    p.finished();
}

#[test]
fn the_tree_jumps_to_an_open_goal_that_is_not_next() {
    let mut p = Prover::new(proving_bytes(), "syl");
    p.aply("mpd");
    assert_eq!(at(&p.ui).open, 2);
    let first = at(&p.ui).goal;
    p.tree(2);
    assert_ne!(at(&p.ui).goal, first);
    assert_eq!(at(&p.ui).view, View::Candidates);
    check(&p.ui, "proving-after-jump");
    p.aply("a1i");
    p.hyp(2);
    assert_eq!(at(&p.ui).open, 1);
    p.hyp(1);
    p.finished();
}

#[test]
fn auto_proves_a_goal_that_one_assertion_settles() {
    let mut p = Prover::new(proving_bytes(), "alid");
    p.keys(&[F3]);
    p.finished();
    p.keys(&[F4]);
    assert_eq!(at(&p.ui).open, 1);
    p.keys(&[F3]);
    p.finished();
    let mut p = Prover::new(proving_bytes(), "foo");
    p.keys(&[F3]);
    p.finished();
}

#[test]
fn a_goal_with_no_candidate_is_closed_by_its_hypothesis() {
    let mut p = Prover::new(bare_bytes(), "th");
    check(&p.ui, "proving-no-candidates");
    p.hyp(1);
    p.finished();
}

#[test]
fn candidates_scroll_in_the_order_the_editor_returns_them() {
    let mut ui = prove(&proving_bytes(), "tmany");
    check(&ui, "proving-candidates-top");
    let mut seen = Vec::new();
    for _ in 0..30 {
        let now = at(&ui).candidate.expect("a candidate");
        if seen.last() != Some(&now) {
            seen.push(now);
        }
        ui.press(Down);
    }
    assert_eq!(
        seen,
        [
            "ax-mp", "a1i", "mpd", "syl", "id", "foo", "axa1", "axa2", "axa3", "axa4", "axa5",
            "axa6"
        ]
    );
    check(&ui, "proving-candidates-scrolled");
    press(&mut ui, &[Left, Left, Left]);
    assert_eq!(at(&ui).candidate.as_deref(), Some("ax-mp"));
    press(&mut ui, &[Right]);
    assert_eq!(at(&ui).candidate.as_deref(), Some("id"));
}

#[test]
fn a_long_goal_folds_to_two_rows() {
    let ui = prove(&proving_bytes(), "tlong");
    check(&ui, "proving-long-goal");
}

#[test]
fn a_hypothesis_that_does_not_match_is_refused() {
    let mut ui = prove(&proving_bytes(), "a1i");
    press(&mut ui, &[F2, F1]);
    assert_eq!(notice(&ui), "Hyp");
    check(&ui, "proving-notice-no-match");
    ui.press(Clear);
    assert_eq!(at(&ui).steps, 0);
    assert_eq!(ui.saved_proofs().count(), 0);
}

#[test]
fn undo_with_nothing_done_is_refused() {
    let mut ui = prove(&proving_bytes(), "a1i");
    ui.press(F4);
    assert_eq!(notice(&ui), "Undo");
    check(&ui, "proving-notice-nothing-to-undo");
}

#[test]
fn auto_that_finds_nothing_or_meets_unknowns_is_refused() {
    let mut ui = prove(&proving_bytes(), "a1i");
    ui.press(F3);
    assert_eq!(notice(&ui), "Auto");
    check(&ui, "proving-notice-auto-nothing");
    ui.press(Clear);
    select(&mut ui, "ax-mp");
    press(&mut ui, &[F1, F3]);
    assert_eq!(notice(&ui), "Auto");
    press(&mut ui, &[Clear, F4, F3]);
    assert_eq!(notice(&ui), "Auto");
}

#[test]
fn auto_on_a_goal_with_unknowns_says_so() {
    let mut ui = prove(&proving_bytes(), "mpd");
    select(&mut ui, "ax-mp");
    ui.press(F1);
    select(&mut ui, "a2i");
    ui.press(F1);
    ui.press(F3);
    assert_eq!(notice(&ui), "Auto");
    check(&ui, "proving-notice-auto-unknowns");
}

#[test]
fn an_application_that_breaks_a_distinct_variable_rule_is_refused() {
    let mut ui = prove(&proving_bytes(), "tq");
    select(&mut ui, "ax-5");
    ui.press(F1);
    assert_eq!(notice(&ui), "Apply");
    check(&ui, "proving-notice-distinct");
    ui.press(Clear);
    assert_eq!(at(&ui).steps, 0);
    assert_eq!(at(&ui).open, 1);
}

#[test]
fn a_goal_that_is_still_unknown_waits_and_cannot_be_applied_to() {
    let mut ui = prove(&proving_bytes(), "a1i");
    select(&mut ui, "ax-mp");
    ui.press(F1);
    press(&mut ui, &[F5, Up, Up]);
    ui.press(Down);
    ui.press(Enter);
    assert_eq!(at(&ui).goal, Some(1));
    check(&ui, "proving-unknown");
    ui.press(F1);
    assert_eq!(notice(&ui), "Apply");
    check(&ui, "proving-notice-unknown");
}

#[test]
fn the_tree_marks_open_and_closed_goals() {
    let mut p = Prover::new(proving_bytes(), "syl");
    p.aply("mpd");
    p.hyp(1);
    p.keys(&[F5]);
    assert_eq!(at(&p.ui).view, View::Tree);
    check(&p.ui, "proving-tree-marks");
    let before = (at(&p.ui).goal, at(&p.ui).steps);
    press(&mut p.ui, &[Up, Enter]);
    assert_eq!(at(&p.ui).view, View::Tree);
    assert_eq!((at(&p.ui).goal, at(&p.ui).steps), before);
    check(&p.ui, "proving-tree-closed-row");
    press(&mut p.ui, &[Down, Enter]);
    assert_eq!(at(&p.ui).view, View::Candidates);
    assert_eq!(at(&p.ui).goal, before.0);
    assert_eq!(at(&p.ui).steps, before.1);
}

#[test]
fn clear_leaves_each_list_before_it_leaves_the_screen() {
    let mut ui = prove(&proving_bytes(), "syl");
    press(&mut ui, &[F2]);
    assert_eq!(at(&ui).view, View::Hyps);
    check(&ui, "proving-hyps-two");
    ui.press(Clear);
    assert_eq!(at(&ui).view, View::Candidates);
    press(&mut ui, &[F5, Clear]);
    assert_eq!(at(&ui).view, View::Candidates);
    ui.press(Clear);
    assert_eq!(ui.depth(), 3);
}

#[test]
fn the_hyp_key_collapses_the_list_again() {
    let mut ui = prove(&proving_bytes(), "syl");
    let before = ui.frame();
    press(&mut ui, &[F2, F2]);
    assert!(ui.frame() == before);
}

#[test]
fn a_finished_proof_names_itself_and_only_undo_and_tree_remain() {
    let mut p = Prover::new(proving_bytes(), "alid");
    p.keys(&[F3]);
    p.keys(&[F1, F2, F3]);
    p.finished();
    p.keys(&[F5]);
    assert_eq!(at(&p.ui).view, View::Tree);
    check(&p.ui, "proving-finished-tree");
}

/// The libraries screen of an interface whose status table counts `count` proofs.
fn libraries_with(bytes: &[u8], count: usize) -> mcglyph::Bitmap {
    let table = support::statuses(bytes, &[], &[]);
    Ui::new(
        vec![mcui::Installed {
            name: "logic".into(),
            bytes: bytes.to_vec(),
        }],
        mcui::Statuses {
            libraries: vec![table],
            my_proofs: count,
        },
    )
    .unwrap()
    .frame()
}

fn home(ui: &mut Ui) -> mcglyph::Bitmap {
    while ui.depth() > 1 {
        ui.press(Clear);
    }
    ui.frame()
}

#[test]
fn my_proofs_follows_the_proofs_in_progress() {
    let bytes = proving_bytes();
    let mut p = Prover::new(bytes.clone(), "a1i");
    assert!(home(&mut p.ui) == libraries_with(&bytes, 2));
    open(&mut p.ui, "a1i");
    p.keys(&[F4]);
    p.aply("ax-mp");
    assert!(home(&mut p.ui) == libraries_with(&bytes, 3));
    open(&mut p.ui, "a1i");
    p.keys(&[F4, F4]);
    assert_eq!(p.ui.saved_proofs().count(), 0);
    assert!(home(&mut p.ui) == libraries_with(&bytes, 2));
}

#[test]
fn a_restored_proof_is_already_in_the_count() {
    let bytes = proving_bytes();
    let mut p = Prover::new(bytes.clone(), "a1i");
    p.aply("ax-mp");
    let saved = p.saved();
    let mut fresh = ui_over(bytes.clone(), &[], &[]);
    fresh.restore_proof(saved).unwrap();
    assert!(home(&mut fresh) == libraries_with(&bytes, 2));
    open(&mut fresh, "a1i");
    press(&mut fresh, &[F4, F4, F4]);
    assert!(home(&mut fresh) == libraries_with(&bytes, 2));
    assert_eq!(fresh.saved_proofs().count(), 1);
}

#[test]
fn a_saved_form_that_does_not_fit_is_refused() {
    let bytes = proving_bytes();
    let mut p = Prover::new(bytes.clone(), "a1i");
    p.aply("ax-mp");
    let saved = p.saved();
    let mut ui = ui_over(bytes.clone(), &[], &[]);
    let wrong = |f: &dyn Fn(&mut SavedProof)| {
        let mut s = saved.clone();
        f(&mut s);
        s
    };
    use mcui::RestoreError::*;
    assert_eq!(
        ui.restore_proof(wrong(&|s| s.library = 4)),
        Err(NoSuchLibrary)
    );
    assert_eq!(
        ui.restore_proof(wrong(&|s| s.theorem = 9999)),
        Err(NoSuchTheorem)
    );
    assert!(matches!(
        ui.restore_proof(wrong(&|s| s.session[0] = 9)),
        Err(Unreadable(_))
    ));
    assert!(matches!(
        ui.restore_proof(wrong(&|s| s.theorem = index_of(&bytes, "syl"))),
        Err(Unreadable(_))
    ));
    assert!(matches!(
        ui.restore_proof(wrong(&|s| s.theorem = index_of(&bytes, "ax-1"))),
        Err(Unreadable(_))
    ));
    assert_eq!(ui.saved_proofs().count(), 0);
}

#[test]
fn proving_id_binds_the_unknown_left_over_and_one_undo_takes_it_back() {
    let mut p = Prover::new(proving_bytes(), "id");
    p.aply("mpd");
    p.aply("ax-1");
    assert_eq!(at(&p.ui).open, 1);
    let steps = at(&p.ui).steps;
    p.aply("ax-1");
    assert_eq!(
        at(&p.ui).steps,
        steps + 2,
        "the application and one assignment"
    );
    check(&p.ui, "proving-finished-bound");
    p.finished();
    p.keys(&[F4]);
    assert_eq!(at(&p.ui).steps, steps);
    assert_eq!(at(&p.ui).open, 1);
    p.aply("ax-1");
    p.finished();
}

#[test]
fn a_variable_a_distinct_variable_rule_refuses_is_passed_over() {
    let mut p = Prover::new(proving_bytes(), "tdv2");
    p.aply("axu");
    p.aply("ax-5");
    p.finished();
    let saved = p.saved();
    assert_eq!(
        saved.steps, 3,
        "two applications and the one assignment that was taken"
    );
    assert_eq!(saved.groups, [1, 2]);
    let text = library::export(&p.bytes).unwrap();
    let db = mm::parse(text.into_bytes()).unwrap();
    let grammar = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let session = Session::restore(Catalog::new(&db, &grammar), &saved.session).unwrap();
    let bound = session.ops().iter().find_map(|op| match op {
        mcedit::Op::Assign { var, .. } => Some(db.syms[*var as usize].name.clone()),
        _ => None,
    });
    assert_eq!(
        bound.as_deref(),
        Some("z"),
        "x and y are refused, z is declared disjoint"
    );
}

#[test]
fn one_undo_takes_back_an_auto_that_made_two_operations() {
    let mut p = Prover::new(proving_bytes(), "ta1");
    p.keys(&[F3]);
    p.finished();
    assert_eq!(p.saved().groups, [2]);
    p.keys(&[F4]);
    assert_eq!(at(&p.ui).steps, 0);
    assert_eq!(at(&p.ui).open, 1);
}

#[test]
fn a_group_of_two_operations_comes_back_from_a_restore_as_one_undo() {
    let mut p = Prover::new(proving_bytes(), "ta1");
    p.keys(&[F3]);
    let saved = p.saved();
    assert_eq!((saved.steps, saved.groups.clone()), (2, vec![2]));
    let mut fresh = ui_over(p.bytes.clone(), &[], &[]);
    fresh.restore_proof(saved).unwrap();
    open(&mut fresh, "ta1");
    press(&mut fresh, &[F4, F4]);
    assert_eq!(at(&fresh).steps, 0);
    assert_eq!(at(&fresh).goal, Some(0));
}

/// A saved proof built straight from mcedit: goal 0 of `label` gets each of `apply` in turn.
fn crafted(bytes: &[u8], label: &str, apply: &[&str], groups: Vec<usize>) -> SavedProof {
    let text = library::export(bytes).unwrap();
    let db = mm::parse(text.into_bytes()).unwrap();
    let grammar = Grammar::new(&db, &[("|-", "wff")]).unwrap();
    let mut session =
        Session::start_in(Catalog::new(&db, &grammar), db.find(label).unwrap()).unwrap();
    for (goal, name) in apply.iter().enumerate() {
        session.apply(goal as u32, db.find(name).unwrap()).unwrap();
    }
    SavedProof {
        library: 0,
        crc: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        theorem: index_of(bytes, label),
        session: session.save(),
        goal: Some(0),
        steps: apply.len(),
        groups,
    }
}

#[test]
fn groups_that_do_not_describe_the_operations_are_refused() {
    let bytes = proving_bytes();
    for (apply, groups) in [
        (vec![], vec![0]),
        (vec!["ax-mp", "ax-mp"], vec![2, 0]),
        (vec!["ax-mp", "ax-mp"], vec![0, 2]),
        (vec!["ax-mp"], vec![usize::MAX, 2]),
        (vec!["ax-mp"], vec![3]),
    ] {
        let saved = crafted(&bytes, "a1i", &apply, groups.clone());
        let mut ui = ui_over(bytes.clone(), &[], &[]);
        assert!(
            matches!(
                ui.restore_proof(saved),
                Err(mcui::RestoreError::Unreadable(_))
            ),
            "{groups:?}"
        );
        open(&mut ui, "a1i");
        ui.press(F4);
        ui.press(F4);
        assert_eq!(notice(&ui), "Undo", "{groups:?}");
    }
}

#[test]
fn a_restored_proof_with_a_broken_distinct_variable_rule_is_not_finished() {
    let bytes = proving_bytes();
    let mut ui = ui_over(bytes.clone(), &[], &[]);
    ui.restore_proof(crafted(&bytes, "tq", &["ax-5"], vec![1]))
        .unwrap();
    open(&mut ui, "tq");
    ui.press(F4);
    let now = at(&ui);
    assert_eq!((now.open, now.goal, now.finished), (0, None, false));
}

#[test]
fn a_goal_that_no_longer_exists_falls_back_to_the_first_open_goal() {
    let mut p = Prover::new(proving_bytes(), "a1i");
    p.aply("ax-mp");
    for goal in [Some(9999), Some(0), None] {
        let mut saved = p.saved();
        saved.goal = goal;
        let mut fresh = ui_over(p.bytes.clone(), &[], &[]);
        fresh.restore_proof(saved).unwrap();
        open(&mut fresh, "a1i");
        fresh.press(F4);
        assert_eq!(at(&fresh).goal, Some(2), "{goal:?}");
    }
}

#[test]
fn a_saved_proof_from_another_version_of_the_library_is_refused() {
    let mut p = Prover::new(proving_bytes(), "a1i");
    p.aply("ax-mp");
    let saved = p.saved();
    let mut older = ui_over(support::logic_bytes(), &[], &[]);
    assert_eq!(
        older.restore_proof(saved.clone()),
        Err(mcui::RestoreError::Unreadable("the library has changed"))
    );
    let mut same = ui_over(proving_bytes(), &[], &[]);
    assert_eq!(same.restore_proof(saved.clone()), Ok(()));
    let mut damaged = saved;
    damaged.groups = vec![5];
    assert!(matches!(
        same.restore_proof(damaged),
        Err(mcui::RestoreError::Unreadable(_))
    ));
}

#[test]
fn a_hypothesis_list_longer_than_the_screen_scrolls_and_pages() {
    let mut ui = prove(&proving_bytes(), "many2");
    ui.press(F2);
    check(&ui, "proving-many-hyps-top");
    let selected = |ui: &Ui| match ui.state() {
        State::Proving { selected, .. } => selected,
        other => panic!("{other:?}"),
    };
    press(&mut ui, &[Down, Down, Down, Down, Down]);
    assert_eq!(selected(&ui), 5);
    check(&ui, "proving-many-hyps-scrolled");
    press(&mut ui, &[Down, Down, Down, Down, Down]);
    assert_eq!(selected(&ui), 7, "stops at the last hypothesis");
    press(&mut ui, &[Left]);
    assert_eq!(selected(&ui), 7 - at_rows(&ui));
    press(&mut ui, &[Left, Left, Left]);
    assert_eq!(selected(&ui), 0);
    ui.press(Right);
    assert_eq!(selected(&ui), at_rows(&ui));
    press(&mut ui, &[Down, Down, Down, Down, Down, Down]);
    ui.press(F1);
    assert_eq!(notice(&ui), "Hyp");
}

/// The rows a list under the goal has on the proving screen of the hypothesis test.
fn at_rows(_ui: &Ui) -> usize {
    4
}

#[test]
fn a_deep_tree_scrolls_and_its_indent_stops_at_the_maximum_depth() {
    let bytes = support::chain_bytes(12);
    let mut p = Prover::new(bytes, "chain");
    for _ in 0..12 {
        p.aply("axs");
    }
    assert_eq!(at(&p.ui).open, 1);
    press(&mut p.ui, &[F5]);
    let selected = |ui: &Ui| match ui.state() {
        State::Proving { selected, .. } => selected,
        other => panic!("{other:?}"),
    };
    assert_eq!(selected(&p.ui), 12);
    check(&p.ui, "proving-tree-deep-bottom");
    press(&mut p.ui, &[Up, Up, Up]);
    assert_eq!(selected(&p.ui), 9);
    ui_press_n(&mut p.ui, Up, 20);
    assert_eq!(selected(&p.ui), 0);
    check(&p.ui, "proving-tree-deep-top");
    p.ui.press(Right);
    assert_eq!(selected(&p.ui), 6);
    p.ui.press(Right);
    p.ui.press(Right);
    assert_eq!(selected(&p.ui), 12, "stops at the last goal");
    p.ui.press(Left);
    assert_eq!(selected(&p.ui), 6);
    p.ui.press(Enter);
    assert_eq!(
        at(&p.ui).view,
        View::Tree,
        "a goal that was applied is not jumped to"
    );
}

fn ui_press_n(ui: &mut Ui, key: Key, n: usize) {
    for _ in 0..n {
        ui.press(key);
    }
}

#[test]
fn no_key_press_replays_the_operations_before_it() {
    use std::time::Instant;
    let depth = 45;
    let bytes = support::chain_bytes(depth);
    let mut ui = prove(&bytes, "chain");
    let mut times = Vec::new();
    for _ in 0..depth {
        let started = Instant::now();
        ui.press(F1);
        times.push(started.elapsed());
        assert_eq!(replayed(&ui), 0);
    }
    let started = Instant::now();
    press(&mut ui, &[F2, F1]);
    times.push(started.elapsed());
    assert_eq!(replayed(&ui), 0);
    assert_eq!(at(&ui).steps, depth + 1);
    assert!(at(&ui).finished);
    let saved = ui.saved_proofs().next().unwrap().clone();
    let mut fresh = ui_over(bytes.clone(), &[], &[]);
    fresh.restore_proof(saved).unwrap();
    open(&mut fresh, "chain");
    fresh.press(F4);
    assert_eq!(
        replayed(&fresh),
        depth + 1,
        "the proof is replayed once, on opening"
    );
    press(&mut fresh, &[F4, F4, F1]);
    assert_eq!(replayed(&fresh), depth + 1);
    let ms = |d: &std::time::Duration| d.as_secs_f64() * 1e3;
    let total: f64 = times.iter().map(ms).sum();
    let max = times.iter().map(ms).fold(0.0, f64::max);
    let (first, last): (f64, f64) = (
        times[..10].iter().map(ms).sum(),
        times[depth - 10..depth].iter().map(ms).sum(),
    );
    println!(
        "{} keys: total {total:.1} ms, worst {max:.1} ms, first ten {first:.1} ms, last ten {last:.1} ms",
        times.len()
    );
}
