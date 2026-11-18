mod support;

use mcpack::library;
use mcpack::mm;
use mcui::data::{By, Installed, Lib};
use mcui::{Key, LoadError, Mark, State, Status, Statuses, Ui};
use support::{LOGIC, db, logic_bytes, open, press, statuses};

fn lib(unchecked: &[&str], failed: &[&str]) -> Lib {
    let bytes = logic_bytes();
    let table = statuses(&bytes, unchecked, failed);
    Lib::new(
        Installed {
            name: "logic".into(),
            bytes,
        },
        table,
    )
    .unwrap()
}

fn index(lib: &Lib, label: &str) -> usize {
    (0..lib.len()).find(|&i| lib.label(i) == label).unwrap()
}

#[test]
fn a_checked_assertion_resting_on_an_unchecked_one_is_marked_so_all_the_way_up() {
    let lib = lib(&["a2i"], &[]);
    let mark = |label: &str| lib.mark(index(&lib, label));
    assert_eq!(mark("a2i"), Mark::Unchecked);
    assert_eq!(mark("a1i"), Mark::Checked);
    assert_eq!(mark("mpd"), Mark::Resting);
    assert_eq!(mark("syl"), Mark::Resting);
    assert_eq!(mark("id"), Mark::Resting);
    assert_eq!(mark("alid"), Mark::Checked);
    assert_eq!(mark("ax-2"), Mark::Checked);
}

#[test]
fn a_failure_is_a_cross_and_what_rests_on_it_rests() {
    let lib = lib(&[], &["a1i"]);
    assert_eq!(lib.mark(index(&lib, "a1i")), Mark::Failed);
    assert_eq!(lib.mark(index(&lib, "syl")), Mark::Resting);
    assert_eq!(lib.mark(index(&lib, "a2i")), Mark::Checked);
    assert_eq!(lib.failed(), 1);
}

#[test]
fn checked_counts_the_resting_and_leaves_out_unchecked_and_failed() {
    let lib = lib(&["a2i", "alid"], &["id"]);
    assert_eq!(lib.len(), 13);
    assert_eq!(lib.checked(), 10);
}

#[test]
fn a_status_table_of_the_wrong_length_is_refused() {
    let bytes = logic_bytes();
    let installed = Installed {
        name: "logic".into(),
        bytes,
    };
    let err = Lib::new(installed.clone(), vec![Status::Checked; 3]).err();
    assert!(matches!(err, Some(LoadError::Statuses(_))));
    let err = Ui::new(vec![installed], Statuses::default()).err();
    assert!(matches!(err, Some(LoadError::Statuses(_))));
}

#[test]
fn bytes_that_are_not_a_library_are_refused() {
    let installed = Installed {
        name: "junk".into(),
        bytes: b"MCL1 not a library".to_vec(),
    };
    let err = Lib::new(installed, Vec::new()).err();
    assert!(matches!(err, Some(LoadError::Library(name, _)) if name == "junk"));
}

#[test]
fn the_goal_tree_matches_the_reference_outline_of_the_source_database() {
    let lib = lib(&[], &[]);
    let syl = index(&lib, "syl");
    let goals = lib.goals(syl).unwrap();
    let source = db(LOGIC);
    let id = source.find("syl").unwrap();
    let reference = mm::outline(&source, id, "|-").unwrap();
    assert_eq!(
        goals.total(),
        reference.render(&source).lines().count() as u64
    );
    assert_eq!(goals.position(&[]), 1);
    assert_eq!(goals.position(&[0]), 2);
    assert_eq!(goals.position(&[1]), 3);
    assert_eq!(goals.position(&[1, 0]), 4);

    let root = lib.view(&goals, &goals.node(&[]));
    assert_eq!(root.by, By::Assertion(index(&lib, "mpd")));
    assert_eq!(root.subgoals.len(), 2);
    assert_eq!(root.goal.tokens(), ["|-", "(", "ph", "->", "ch", ")"]);
    let first = lib.view(&goals, &goals.node(&[0]));
    assert_eq!(first.by, By::Hypothesis(1));
    assert!(first.subgoals.is_empty());
    let second = lib.view(&goals, &goals.node(&[1]));
    assert_eq!(second.by, By::Assertion(index(&lib, "a1i")));
    let third = lib.view(&goals, &goals.node(&[1, 0]));
    assert_eq!(third.by, By::Hypothesis(2));
}

#[test]
fn an_axiom_has_no_goal_tree() {
    let lib = lib(&[], &[]);
    assert!(lib.goals(index(&lib, "ax-1")).is_err());
}

fn without_a_database() -> Ui {
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    assert!(library::read(&bytes).is_ok());
    assert!(library::export(&bytes).is_err());
    let table = statuses(&bytes, &[], &[]);
    Ui::new(
        vec![Installed {
            name: "logic".into(),
            bytes,
        }],
        Statuses {
            libraries: vec![table],
            my_proofs: 0,
        },
    )
    .unwrap()
}

#[test]
fn a_library_that_does_not_export_still_draws_every_screen() {
    let mut ui = without_a_database();
    press(&mut ui, &[Key::Enter]);
    for _ in 0..13 {
        for keys in [
            &[Key::Enter][..],
            &[Key::Down, Key::Right, Key::F1, Key::F2],
            &[Key::Clear, Key::Clear],
        ] {
            press(&mut ui, keys);
            let frame = ui.frame();
            assert_eq!((frame.width(), frame.height()), (96, 64));
        }
        press(&mut ui, &[Key::Down]);
    }
}

#[test]
fn without_a_database_a_statement_is_one_run_of_tokens_and_a_proof_cannot_be_read() {
    let mut ui = without_a_database();
    open(&mut ui, "syl");
    match ui.state() {
        State::Theorem { node, .. } => assert_eq!(node, 0),
        other => panic!("{other:?}"),
    }
    press(&mut ui, &[Key::Down]);
    match ui.state() {
        State::Theorem { node, .. } => assert_eq!(node, 0, "a flat statement has nothing to enter"),
        other => panic!("{other:?}"),
    }
    press(&mut ui, &[Key::F3]);
    assert_eq!(
        ui.state(),
        State::Notice {
            title: "Proof".into()
        }
    );
}

fn lib_from(bytes: Vec<u8>) -> Lib {
    let table = statuses(&bytes, &[], &[]);
    Lib::new(
        Installed {
            name: "patched".into(),
            bytes,
        },
        table,
    )
    .unwrap()
}

#[test]
fn a_proof_that_names_a_later_assertion_shows_as_failed_and_what_rests_on_it_rests() {
    let sound = logic_bytes();
    let reader = lib_from(sound.clone());
    let (a1i, alid) = (index(&reader, "a1i"), index(&reader, "alid"));
    let patched = lib_from(support::patch_reference(&sound, "a1i", 0, alid as u16));
    assert_eq!(patched.mark(a1i), Mark::Failed);
    assert_eq!(patched.mark(index(&patched, "syl")), Mark::Resting);
    assert_eq!(patched.mark(index(&patched, "ax-1")), Mark::Checked);
    assert_eq!(patched.failed(), 1);
    assert_eq!(patched.checked(), patched.len() - 1);
    assert_eq!(reader.failed(), 0);
}

#[test]
fn a_proof_that_names_itself_shows_as_failed() {
    let sound = logic_bytes();
    let reader = lib_from(sound.clone());
    let a1i = index(&reader, "a1i");
    let patched = lib_from(support::patch_reference(&sound, "a1i", 0, a1i as u16));
    assert_eq!(patched.mark(a1i), Mark::Failed);
}

#[test]
fn only_a_library_that_cannot_be_read_as_a_tree_is_degraded() {
    assert!(!lib(&[], &[]).degraded());
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    assert!(lib_from(bytes).degraded());
}

#[test]
fn an_empty_library_has_no_marks_to_count_and_is_not_degraded() {
    let bytes = library::pack(&db(LOGIC), &[]).unwrap();
    let lib = lib_from(bytes);
    assert_eq!(
        (lib.len(), lib.checked(), lib.failed(), lib.degraded()),
        (0, 0, 0, false)
    );
}
