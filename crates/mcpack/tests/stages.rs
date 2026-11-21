use mcpack::library::{self, Stage, StagesError, pack};
use mcpack::manifest::Manifest;
use mcpack::mm::{self, StmtId};
use std::path::{Path, PathBuf};
use std::process::Command;

const HEAD: &str = "
$c |- wff ( ) -> $.
$v p q $.
wp $f wff p $.
wq $f wff q $.
";

fn source(axioms: &[&str]) -> String {
    let mut src = HEAD.to_string();
    for label in axioms {
        src.push_str(&format!("{label} $a |- ( p -> q ) $.\n"));
    }
    src
}

fn packed(axioms: &[&str], assertions: usize) -> Vec<u8> {
    let db = mm::parse(source(axioms).into_bytes()).expect("database parses");
    let kept: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| {
            let stmt = db.stmt(id);
            !stmt.kind.is_assertion() || axioms[..assertions].contains(&stmt.label.as_str())
        })
        .collect();
    pack(&db, &kept).expect("the library packs")
}

fn manifest(name: &str, order: u32) -> Manifest {
    let text = format!(
        "name = \"{name}\"\ntitle = \"Title of {name}\"\ndescription = \"d\"\norder = {order}\nroots = [\"a\"]\n"
    );
    Manifest::parse(&text).expect("manifest parses")
}

const AXIOMS: [&str; 5] = ["ax-a", "ax-b", "ax-c", "ax-d", "ax-e"];

#[test]
fn libraries_cut_from_one_another_give_their_counts_in_order() {
    let stages = library::stages(vec![
        (manifest("big", 3), packed(&AXIOMS, 5)),
        (manifest("small", 1), packed(&AXIOMS, 2)),
        (manifest("mid", 2), packed(&AXIOMS, 3)),
    ])
    .expect("the cuts line up");
    let seen: Vec<(&str, usize, &str)> = stages
        .iter()
        .map(|s| (s.name.as_str(), s.count, s.title.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("small", 2, "Title of small"),
            ("mid", 3, "Title of mid"),
            ("big", 5, "Title of big")
        ]
    );
    assert_eq!(
        library::render_stages(&stages),
        "small 2 Title of small\nmid 3 Title of mid\nbig 5 Title of big\n"
    );
}

#[test]
fn the_lines_render_writes_parse_back_into_the_same_stages() {
    let stages = library::stages(vec![
        (manifest("small", 1), packed(&AXIOMS, 2)),
        (manifest("big", 2), packed(&AXIOMS, 5)),
    ])
    .unwrap();
    let text = library::render_stages(&stages);
    assert_eq!(library::parse_stages(&text), Ok(stages));
    assert_eq!(library::parse_stages(""), Ok(vec![]));
    let odd = |title: &str| Stage {
        name: "s".into(),
        title: title.into(),
        count: 3,
    };
    for title in ["", "a  b   c", " lead", "trail "] {
        let stages = vec![odd(title)];
        let text = library::render_stages(&stages);
        assert_eq!(library::parse_stages(&text), Ok(stages), "{title:?}");
    }
    for bad in [
        "small",
        "small x title",
        "small 2",
        " 2 title",
        "small -1 t",
    ] {
        assert!(library::parse_stages(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_library_that_is_not_the_start_of_the_next_names_the_first_difference() {
    let other = ["ax-a", "ax-x", "ax-c", "ax-d", "ax-e"];
    let err = library::stages(vec![
        (manifest("small", 1), packed(&other, 3)),
        (manifest("big", 2), packed(&AXIOMS, 5)),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        StagesError::NotPrefix(
            "small".into(),
            "big".into(),
            1,
            "ax-x".into(),
            "ax-b".into()
        )
    );
    assert_eq!(
        err.to_string(),
        "small is not the start of big: assertion 1 is ax-x in one and ax-b in the other"
    );
}

#[test]
fn a_library_is_compared_with_the_largest_not_only_with_its_neighbour() {
    let other = ["ax-a", "ax-x", "ax-c", "ax-d", "ax-e"];
    let err = library::stages(vec![
        (manifest("small", 1), packed(&other, 2)),
        (manifest("mid", 2), packed(&other, 3)),
        (manifest("large", 3), packed(&AXIOMS, 5)),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        StagesError::NotPrefix(
            "small".into(),
            "large".into(),
            1,
            "ax-x".into(),
            "ax-b".into()
        )
    );
}

#[test]
fn a_library_with_no_assertions_is_refused() {
    let err = library::stages(vec![
        (manifest("none", 1), packed(&AXIOMS, 0)),
        (manifest("big", 2), packed(&AXIOMS, 5)),
    ])
    .unwrap_err();
    assert_eq!(err, StagesError::Empty("none".into()));
    assert_eq!(err.to_string(), "none has no assertions");
}

#[test]
fn a_name_with_whitespace_or_none_is_refused() {
    for name in ["two words", "tab\there", ""] {
        let err = library::stages(vec![(manifest(name, 1), packed(&AXIOMS, 2))]).unwrap_err();
        assert_eq!(err, StagesError::BadName(name.into()), "{name:?}");
    }
}

#[test]
fn a_library_with_no_more_assertions_than_the_one_before_is_refused() {
    let err = library::stages(vec![
        (manifest("a", 1), packed(&AXIOMS, 3)),
        (manifest("b", 2), packed(&AXIOMS, 3)),
    ])
    .unwrap_err();
    assert_eq!(err, StagesError::NotLarger("b".into(), "a".into()));
}

#[test]
fn two_libraries_with_the_same_order_are_refused() {
    let err = library::stages(vec![
        (manifest("a", 1), packed(&AXIOMS, 2)),
        (manifest("b", 1), packed(&AXIOMS, 3)),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        StagesError::Order("a and b have the same order, 1".into())
    );
}

#[test]
fn a_library_that_does_not_decode_is_named() {
    let err = library::stages(vec![(manifest("junk", 1), vec![0; 4])]).unwrap_err();
    assert!(matches!(err, StagesError::Unreadable(ref name, _) if name == "junk"));
}

fn workspace(name: &str, small: &[&str], big: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pin = "repository = \"https://example.org/r\"\ncommit = \"abc\"\npath = \"x.mm\"\nsha256 = \"0\"\n";
    std::fs::write(dir.join("source.toml"), pin).unwrap();
    for (name, order, axioms, count) in [("small", 1, small, 2), ("big", 2, big, 5)] {
        let text = format!(
            "name = \"{name}\"\ntitle = \"Title of {name}\"\ndescription = \"d\"\norder = {order}\nroots = [\"a\"]\n"
        );
        std::fs::write(dir.join(format!("{name}.toml")), text).unwrap();
        std::fs::write(dir.join(format!("{name}.mclib")), packed(axioms, count)).unwrap();
    }
    dir
}

fn run(dir: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("stages")
        .arg(dir)
        .arg(dir.join("big.toml"))
        .arg(dir.join("small.toml"))
        .output()
        .expect("mcpack runs")
}

#[test]
fn the_stages_command_prints_one_line_per_stage_smallest_first() {
    let dir = workspace("stages-ok", &AXIOMS, &AXIOMS);
    let out = run(&dir);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "small 2 Title of small\nbig 5 Title of big\n"
    );
}

#[test]
fn the_stages_command_fails_with_the_difference_when_the_cut_is_wrong() {
    let other = ["ax-a", "ax-x", "ax-c", "ax-d", "ax-e"];
    let dir = workspace("stages-bad", &other, &AXIOMS);
    let out = run(&dir);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "small is not the start of big: assertion 1 is ax-x in one and ax-b in the other\n"
    );
}

#[test]
#[ignore = "needs the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn the_shipped_libraries_are_three_stages_of_ring() {
    let dir = PathBuf::from(
        std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries"),
    );
    let manifests = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("stages")
        .arg(&dir)
        .args(["ring", "prop", "pred"].map(|n| manifests.join(format!("{n}.toml"))))
        .output()
        .expect("mcpack runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "prop 779 Propositional calculus\npred 1171 Predicate calculus with equality\nring 3326 Ring theory\n"
    );
}
