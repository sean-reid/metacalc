use mcpack::build;
use mcpack::library::{self, check, pack};
use mcpack::manifest::{Manifest, sha256_hex};
use mcpack::mm::{self, Database, Kind, ProofBody, StmtId};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DB: &str = "
$c ( ) -> wff |- $.
$v p q r $.
wp $f wff p $.
wq $f wff q $.
wr $f wff r $.
wi $a wff ( p -> q ) $.
ax-1 $a |- ( p -> ( q -> p ) ) $.
${
  min $e |- p $.
  maj $e |- ( p -> q ) $.
  ax-mp $a |- q $.
$}
${
  a1i.1 $e |- p $.
  a1i $p |- ( q -> p ) $= ( wi ax-1 ax-mp ) ABADCABEF $.
$}
${
  use.1 $e |- r $.
  use $p |- ( q -> ( p -> r ) ) $= ( wi a1i ) ACEBCADFF $.
$}
";

fn manifest(roots: &str, before: &str) -> Manifest {
    let text =
        format!("name = \"t\"\ntitle = \"T\"\ndescription = \"d\"\nroots = [{roots}]\n{before}");
    Manifest::parse(&text).expect("manifest parses")
}

fn labels(db: &Database, ids: &[StmtId]) -> Vec<String> {
    ids.iter().map(|&id| db.stmt(id).label.clone()).collect()
}

#[test]
fn manifest_reads_every_field() {
    let m = manifest("\"a\", \"b\"", "before = \"c\"\n");
    assert_eq!(m.name, "t");
    assert_eq!(m.title, "T");
    assert_eq!(m.description, "d");
    assert_eq!(m.roots, ["a", "b"]);
    assert_eq!(m.before.as_deref(), Some("c"));
    assert_eq!(manifest("\"a\"", "").before, None);
}

#[test]
fn manifest_without_a_field_is_refused() {
    let err = Manifest::parse("name = \"t\"\ntitle = \"T\"\nroots = [\"a\"]\n").unwrap_err();
    assert!(err.to_string().contains("missing field `description`"));
}

#[test]
fn manifest_with_an_unknown_field_is_refused() {
    let text = "name = \"t\"\ntitle = \"T\"\ndescription = \"d\"\nroots = [\"a\"]\nbefor = \"c\"\n";
    let err = Manifest::parse(text).unwrap_err();
    assert!(err.to_string().contains("unknown field `befor`"));
}

#[test]
fn the_checked_in_manifests_parse() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
    for name in ["prop", "pred", "ring"] {
        let m = Manifest::load(&dir.join(format!("{name}.toml"))).expect("manifest loads");
        assert_eq!(m.name, name);
        assert_eq!(m.roots, ["ringlz", "ringnegl", "ringcom"]);
    }
    assert!(mcpack::manifest::Pin::load(&dir.join("ring.toml")).is_ok());
}

#[test]
fn sha256_matches_the_published_vectors() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn the_cut_closure_is_dependency_closed_and_packs() {
    let db = mm::parse(DB.as_bytes().to_vec()).expect("database parses");
    let whole = build::closure(&db, &manifest("\"use\"", "")).expect("closure builds");
    assert_eq!(
        labels(&db, &whole),
        [
            "wp", "wq", "wr", "wi", "ax-1", "min", "maj", "ax-mp", "a1i.1", "a1i", "use.1", "use"
        ]
    );

    let cut = build::closure(&db, &manifest("\"use\"", "before = \"a1i\"\n")).expect("cut builds");
    assert_eq!(
        labels(&db, &cut),
        [
            "wp", "wq", "wr", "wi", "ax-1", "min", "maj", "ax-mp", "a1i.1"
        ]
    );
    for &id in &cut {
        let stmt = db.stmt(id);
        let mut deps: Vec<StmtId> = stmt.frame.hyps.clone();
        if let Some(Ok(proof)) = &stmt.proof {
            let (ProofBody::Normal(refs) | ProofBody::Compressed { labels: refs, .. }) =
                &proof.body;
            deps.extend(refs);
        }
        assert!(deps.iter().all(|dep| cut.contains(dep)), "{}", stmt.label);
    }
    let bytes = pack(&db, &cut).expect("the cut packs");
    assert_eq!(check(&bytes), Ok(3));
}

#[test]
fn a_boundary_that_does_not_exist_is_refused() {
    let db = mm::parse(DB.as_bytes().to_vec()).expect("database parses");
    let err = build::closure(&db, &manifest("\"use\"", "before = \"nope\"\n")).unwrap_err();
    assert_eq!(err.to_string(), "no statement is labelled nope");
}

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name)
}

/// Writes a manifest for demo0.mm and a pin with the given digest into a fresh directory.
fn workspace(name: &str, sha256: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("demo.toml"),
        "name = \"demo\"\ntitle = \"Demo\"\ndescription = \"One theorem.\"\nroots = [\"th1\"]\n",
    )
    .unwrap();
    let pin = format!(
        "repository = \"https://example.org/demo\"\ncommit = \"abc123\"\npath = \"demo0.mm\"\nsha256 = \"{sha256}\"\n"
    );
    std::fs::write(dir.join("source.toml"), pin).unwrap();
    dir
}

fn mcpack(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .args(args)
        .output()
        .expect("mcpack runs")
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("output is text")
}

#[test]
fn build_refuses_a_database_that_is_not_the_pinned_one() {
    let dir = workspace("pin-mismatch", &"0".repeat(64));
    let database = corpus("demo0.mm");
    let real = sha256_hex(&std::fs::read(&database).unwrap());
    let library = dir.join("demo.mclib");
    let out = mcpack(&[
        Path::new("build"),
        &dir.join("demo.toml"),
        Path::new("--database"),
        &database,
        Path::new("-o"),
        &library,
    ]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = text(out.stderr);
    assert_eq!(
        stderr,
        format!(
            "{}: the database has sha256 {real}, the pin wants {} (demo0.mm at abc123)\n",
            database.display(),
            "0".repeat(64)
        )
    );
    assert!(!library.exists());
}

fn build_demo(dir: &Path) -> PathBuf {
    let library = dir.join("demo.mclib");
    let out = mcpack(&[
        Path::new("build"),
        &dir.join("demo.toml"),
        Path::new("--database"),
        &corpus("demo0.mm"),
        Path::new("-o"),
        &library,
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(out.stdout).starts_with("8 assertions, "));
    library
}

#[test]
fn doc_writes_the_page_of_a_built_library() {
    let real = sha256_hex(&std::fs::read(corpus("demo0.mm")).unwrap());
    let dir = workspace("page", &real);
    let library = build_demo(&dir);
    let bytes = std::fs::read(&library).unwrap();
    let page = dir.join("demo.md");
    let out = mcpack(&[
        Path::new("doc"),
        &dir.join("demo.toml"),
        Path::new("--library"),
        &library,
        Path::new("-o"),
        &page,
    ]);
    assert_eq!(out.status.code(), Some(0));
    let page = std::fs::read_to_string(&page).unwrap();
    let lines: Vec<&str> = page.lines().collect();
    assert_eq!(lines[0], "# Demo");
    assert_eq!(lines[2], "One theorem.");
    assert_eq!(
        lines[4],
        "- Source: `demo0.mm` in [example.org/demo](https://example.org/demo/tree/abc123) at commit `abc123`"
    );
    assert_eq!(lines[5], "- Axioms: 7");
    assert_eq!(lines[6], "- Theorems: 1");
    assert_eq!(
        lines[7],
        format!("- Size: {} bytes in 1 pages of 16384 bytes", bytes.len())
    );
    assert_eq!(lines[9], "## Assertions");
    assert_eq!(
        lines[11..],
        [
            "- <span id=\"tze\"></span>**tze** (axiom): `term 0`",
            "- <span id=\"tpl\"></span>**tpl** (axiom): `term ( t + r )`",
            "- <span id=\"weq\"></span>**weq** (axiom): `wff t = r`",
            "- <span id=\"wim\"></span>**wim** (axiom): `wff ( P -> Q )`",
            "- <span id=\"a1\"></span>**a1** (axiom): `|- ( t = r -> ( t = s -> r = s ) )`",
            "- <span id=\"a2\"></span>**a2** (axiom): `|- ( t + 0 ) = t`",
            "- <span id=\"mp\"></span>**mp** (axiom) given `|- P`, `|- ( P -> Q )`: `|- Q`",
            "- <span id=\"th1\"></span>**th1** (theorem): `|- t = t`",
        ]
    );
}

#[test]
fn doc_refuses_a_library_that_is_damaged() {
    let real = sha256_hex(&std::fs::read(corpus("demo0.mm")).unwrap());
    let dir = workspace("damaged", &real);
    let library = build_demo(&dir);
    let bytes = std::fs::read(&library).unwrap();
    let mut flipped = bytes.clone();
    *flipped.last_mut().unwrap() ^= 1;
    for (name, damaged) in [
        ("cut.mclib", bytes[..bytes.len() - 5].to_vec()),
        ("flip.mclib", flipped),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, damaged).unwrap();
        let page = dir.join("damaged.md");
        let out = mcpack(&[
            Path::new("doc"),
            &dir.join("demo.toml"),
            Path::new("--library"),
            &path,
            Path::new("-o"),
            &page,
        ]);
        assert_eq!(out.status.code(), Some(1), "{name}");
        assert_eq!(
            text(out.stderr),
            format!(
                "{}: the packed library does not load: Malformed\n",
                path.display()
            )
        );
        assert!(!page.exists());
    }
}

#[test]
fn the_decoder_spells_every_assertion_as_the_database_does() {
    let src = std::fs::read(corpus("demo0.mm")).unwrap();
    let db = mm::parse(src).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &all).expect("the database packs");
    let decoded = library::read(&bytes).expect("the library decodes");
    let name = |tokens: &[u32]| -> Vec<String> {
        tokens
            .iter()
            .map(|&s| db.syms[s as usize].name.clone())
            .collect()
    };
    let expected: Vec<&StmtId> = all
        .iter()
        .filter(|&&id| matches!(db.stmt(id).kind, Kind::Axiom | Kind::Theorem))
        .collect();
    assert_eq!(decoded.len(), expected.len());
    for (got, &&id) in decoded.iter().zip(&expected) {
        let stmt = db.stmt(id);
        assert_eq!(got.label, stmt.label);
        assert_eq!(got.theorem, stmt.kind == Kind::Theorem);
        assert_eq!(got.conclusion, name(&stmt.expr));
        let hyps: Vec<library::Hypothesis> = stmt
            .frame
            .hyps
            .iter()
            .map(|&h| match db.stmt(h) {
                hyp if hyp.kind == Kind::Float => library::Hypothesis::Float {
                    typecode: name(&hyp.expr)[0].clone(),
                    variable: name(&hyp.expr)[1].clone(),
                },
                hyp => library::Hypothesis::Essential(name(&hyp.expr)),
            })
            .collect();
        assert_eq!(got.hypotheses, hyps, "{}", stmt.label);
        let dv: Vec<(String, String)> = stmt
            .frame
            .dv
            .iter()
            .map(|&(a, b)| (name(&[a])[0].clone(), name(&[b])[0].clone()))
            .collect();
        assert_eq!(got.disjoint, dv, "{}", stmt.label);
    }
}

#[test]
fn the_decoder_never_panics_on_a_damaged_library() {
    let db = mm::parse(DB.as_bytes().to_vec()).expect("database parses");
    let all: Vec<StmtId> = (0..db.stmts.len() as StmtId).collect();
    let bytes = pack(&db, &all).expect("the database packs");
    for len in 0..bytes.len() {
        assert!(library::read(&bytes[..len]).is_err(), "cut at {len}");
    }
    for at in 0..bytes.len() {
        let mut damaged = bytes.clone();
        damaged[at] ^= 0xFF;
        let _ = library::read(&damaged);
    }
}
