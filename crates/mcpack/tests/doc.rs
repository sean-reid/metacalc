use mcpack::library::pack;
use mcpack::manifest::{Manifest, Pin};
use mcpack::mm::{self, StmtId};
use mcpack::page;
use std::path::{Path, PathBuf};
use std::process::Command;

const TICK: &str = "
$c ` `` wff |- $.
$v x y $.
vx $f wff x $.
vy $f wff y $.
tick-mid $a |- x ` y $.
tick-start $a ` x $.
tick-end $a |- x ` $.
tick-run $a |- ` x `` y $.
tick-all $a ` ` $.
${
  $d x y $.
  tick-dv $a |- ` x y $.
$}
";

const BAD: &str = "
$c R wff |- $.
$v x y $.
vx $f wff x $.
vy $f wff y $.
${
  $d x y $.
  ax-d $a |- R x y $.
$}
bad $p |- R x y $= vx vy ax-d $.
";

const MANIFEST: &str = "name = \"t\"\ntitle = \"T\"\ndescription = \"d\"\nroots = [\"a\"]\n";

fn pin() -> Pin {
    Pin {
        repository: "https://example.org/r".into(),
        commit: "abc".into(),
        path: "x.mm".into(),
        sha256: "0".repeat(64),
    }
}

fn packed(src: &str) -> Vec<u8> {
    let db = mm::parse(src.as_bytes().to_vec()).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    pack(&db, &all).expect("the database packs")
}

#[test]
fn backtick_tokens_get_a_fence_that_holds_them() {
    let manifest = Manifest::parse(MANIFEST).unwrap();
    let text = page::render(&manifest, &pin(), &packed(TICK)).expect("page renders");
    let lines: Vec<&str> = text.lines().collect();
    let entry = |label: &str, rest: &str| {
        format!("- <span id=\"{label}\"></span>**{label}** (axiom){rest}")
    };
    assert_eq!(
        lines[lines.len() - 6..],
        [
            entry("tick-mid", ": ``|- x ` y``"),
            entry("tick-start", ": `` ` x ``"),
            entry("tick-end", ": `` |- x ` ``"),
            entry("tick-run", ": ```|- ` x `` y```"),
            entry("tick-all", ": `` ` ` ``"),
            entry("tick-dv", ": ``|- ` x y`` (distinct: `x y`)")
        ][..]
    );
}

fn temp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn doc_refuses_a_library_that_loads_but_fails_the_kernel() {
    let dir = temp("kernel-fail");
    std::fs::write(dir.join("t.toml"), MANIFEST).unwrap();
    let pin = "repository = \"https://example.org/r\"\ncommit = \"abc\"\npath = \"x.mm\"\nsha256 = \"0\"\n";
    std::fs::write(dir.join("source.toml"), pin).unwrap();
    let library = dir.join("bad.mclib");
    std::fs::write(&library, packed(BAD)).unwrap();
    let page = dir.join("bad.md");
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("doc")
        .arg(dir.join("t.toml"))
        .arg("--library")
        .arg(&library)
        .arg("-o")
        .arg(&page)
        .output()
        .expect("mcpack runs");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        format!(
            "{}: bad: DisjointViolation\n1 assertions failed the kernel check\n",
            library.display()
        )
    );
    assert!(!page.exists());
}
