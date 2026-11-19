mod support;

use mcpack::library;
use mcpack::mm;
use mcui::data::{By, Installed, Lib};
use mcui::{Key, LoadError, Mark, State, Status, Statuses, Ui};
use support::{LOGIC, db, kernel_only_bytes, logic_bytes, open, press, statuses};

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
    let mut bytes = kernel_only_bytes();
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
    let patched = lib_from(support::refreshed(support::patch_reference(
        &sound,
        "a1i",
        0,
        alid as u16,
    )));
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
    let patched = lib_from(support::refreshed(support::patch_reference(
        &sound, "a1i", 0, a1i as u16,
    )));
    assert_eq!(patched.mark(a1i), Mark::Failed);
}

#[test]
fn only_a_library_that_cannot_be_read_as_a_tree_is_degraded() {
    assert!(!lib(&[], &[]).degraded());
    let mut bytes = kernel_only_bytes();
    assert!(!lib_from(bytes.clone()).degraded());
    bytes[4] ^= 0xFF;
    assert!(lib_from(bytes).degraded());
}

#[test]
fn a_library_with_its_own_trees_needs_no_export_to_show_a_statement_as_a_tree() {
    let mut bytes = logic_bytes();
    bytes[4] ^= 0xFF;
    assert!(library::export(&bytes).is_err());
    let damaged = lib_from(bytes);
    assert!(!damaged.degraded());
    let sound = lib(&[], &[]);
    for i in 0..sound.len() {
        assert_eq!(
            damaged.statement(i).all(),
            sound.statement(i).all(),
            "{}",
            sound.label(i)
        );
    }
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

#[test]
fn marks_updated_one_status_at_a_time_equal_a_full_recompute() {
    let sound = logic_bytes();
    let at = |label: &str| {
        library::read(&sound)
            .unwrap()
            .iter()
            .position(|a| a.label == label)
            .unwrap() as u16
    };
    let patched = support::patch_reference(&sound, "a1i", 0, at("alid"));
    let patched = support::refreshed(support::patch_reference(&patched, "mpd", 1, at("ax-many")));
    let first = statuses(&patched, &[], &[]);
    let mut live = Lib::new(
        Installed {
            name: "logic".into(),
            bytes: patched.clone(),
        },
        first.clone(),
    )
    .unwrap();
    let mut table = first;
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as usize
    };
    for step in 0..400 {
        let i = next() % live.len();
        let status = [Status::Checked, Status::Failed, Status::Unchecked][next() % 3];
        live.set_status(i, status);
        table[i] = status;
        let full = Lib::new(
            Installed {
                name: "logic".into(),
                bytes: patched.clone(),
            },
            table.clone(),
        )
        .unwrap();
        for j in 0..live.len() {
            assert_eq!(live.mark(j), full.mark(j), "step {step}, {}", live.label(j));
        }
    }
}

#[test]
fn a_library_gives_its_section_titles() {
    let rule = "=-".repeat(40)[..79].to_string();
    let head = |title: &str| format!("$(\n{rule}\n  {title}\n{rule}\n$)\n");
    let src = format!(
        "$c wff |- $.\n$v ph $.\nwph $f wff ph $.\n{}ax-a $a |- ph $.\n{}ax-b $a |- ph $.\nax-c $a |- ph $.\n",
        head("First"),
        head("Second")
    );
    let bytes = support::pack_all(&db(&src)).unwrap();
    let titled = lib_from(bytes);
    let got: Vec<(usize, u8, &str)> = titled
        .titles()
        .iter()
        .map(|t| (t.assertion, t.depth, t.text.as_str()))
        .collect();
    assert_eq!(got, [(0, 3, "First"), (1, 3, "Second")]);
    assert_eq!(titled.label(1), "ax-b");
    assert!(lib(&[], &[]).titles().is_empty());
    assert!(lib_from(kernel_only_bytes()).titles().is_empty());
}

/// 130 syntax axioms and 130 variables, so the last statement's tree needs both escapes.
fn escaped_source() -> String {
    let vars: Vec<String> = (0..130).map(|i| format!("v{i}")).collect();
    let mut src = format!("$c ( ) |- wff a b $.\n$v {} $.\n", vars.join(" "));
    for v in &vars {
        src += &format!("f{v} $f wff {v} $.\n");
    }
    let mut made = 0;
    for len in 1..=8usize {
        for bits in 0..1usize << len {
            if made == 130 {
                break;
            }
            let body: Vec<&str> = (0..len)
                .map(|i| if bits >> i & 1 == 0 { "a" } else { "b" })
                .collect();
            src += &format!("w{made} $a wff {} $.\n", body.join(" "));
            made += 1;
        }
    }
    src += "wi $a wff ( v0 v1 ) $.\nax-top $a |- ( v128 v129 ) $.\n";
    src
}

#[test]
fn a_statement_whose_tree_needs_escaped_bytes_folds_as_the_export_parse_does() {
    let db = db(&escaped_source());
    let (stored, parsed) = (
        support::pack_all(&db).unwrap(),
        support::pack_all_kernel(&db).unwrap(),
    );
    let (stored, parsed) = (lib_from(stored), lib_from(parsed));
    let top = index(&stored, "ax-top");
    let (a, b) = (stored.statement(top), parsed.statement(top));
    assert_eq!(a.all(), b.all());
    assert_eq!(a.conclusion.tokens(), ["|-", "(", "v128", "v129", ")"]);
    assert_eq!(a.conclusion.len(), 3, "the wi node and its two variables");
}

#[test]
fn a_library_without_titles_puts_every_assertion_in_section_zero() {
    let lib = lib_from(support::logic_bytes());
    assert!(lib.titles().is_empty());
    assert_eq!((lib.section_of(0), lib.section_of(5)), (0, 0));
}

const MIXED: &str = "
$c ( ) -> wff |- $.
$v ph ps $.
wph $f wff ph $.
wps $f wff ps $.
wi $a wff ( ph -> ps ) $.
${
  junk-h $e |- ( ph ph ) $.
  junk-hyp $a |- ( ph -> ps ) $.
$}
${
  good-h $e |- ( ph -> ps ) $.
  junk-conc $a |- ( ph ph ) $.
$}
";

#[test]
fn each_statement_is_a_tree_where_the_library_stores_one_and_flat_where_it_does_not() {
    let db = db(MIXED);
    let all: Vec<mm::StmtId> = (0..db.stmts.len() as mm::StmtId).collect();
    let bytes = library::pack(&db, &all).expect("it packs");
    let n = bytes.len();
    let lib = Lib::new(
        Installed {
            name: "mixed".into(),
            bytes,
        },
        vec![Status::Unchecked; 3],
    )
    .unwrap();
    assert!(n > 0);
    let by = |label: &str| lib.statement(index(&lib, label));
    let flat = |e: &mcui::expr::Expr| e.len() == 1 && !e.foldable(0);
    let tree = |e: &mcui::expr::Expr| e.len() == 3 && e.foldable(0);
    let hyp = by("junk-hyp");
    assert!(
        flat(&hyp.hyps[0]),
        "the hypothesis the grammar cannot read is one node"
    );
    assert!(tree(&hyp.conclusion), "its conclusion keeps its tree");
    let conc = by("junk-conc");
    assert!(tree(&conc.hyps[0]), "the hypothesis keeps its tree");
    assert!(
        flat(&conc.conclusion),
        "the conclusion the grammar cannot read is one node"
    );
    assert_eq!(hyp.all().len(), 2);
}
