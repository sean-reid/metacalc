use mcpack::library::pack;
use mcpack::manifest::{Manifest, Pin};
use mcpack::mm::{self, StmtId};
use mcpack::page;
use std::path::{Path, PathBuf};
use std::process::Command;

const MIXED: &str = "
$c |- wff ( ) -> ` | < & \\ foo $.
$v ph ps $.
wph $f wff ph $.
wps $f wff ps $.
wi $a wff ( ph -> ps ) $.
wodd $a wff ` ph | ps < & \\ foo $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
${
  ax-mp.1 $e |- ph $.
  ax-mp.2 $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
df-foo $a |- ( ph -> ph ) $.
fact $a |- ( ps -> ps ) $.
${
  $d ph ps $.
  ax-d $a |- ` ph | ps < & \\ $.
$}
th $p |- ( ph -> ( ps -> ph ) ) $= wph wps ax-1 $.
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

fn render(src: &str) -> String {
    let manifest = Manifest::parse(MANIFEST).unwrap();
    page::render(&manifest, &pin(), &packed(src)).expect("page renders")
}

fn entry(label: &str, lines: &[&str]) -> String {
    let mut out = format!(
        "<div class=\"mm\" id=\"{label}\"><a href=\"https://us.metamath.org/mpeuni/{label}.html\">{label}</a>"
    );
    for line in lines {
        out.push_str(&format!("\n{line}"));
    }
    out.push_str("\n</div>\n\n");
    out
}

#[test]
fn a_page_groups_assertions_by_kind_and_shows_unicode_over_ascii() {
    let text = render(MIXED);
    let body = text.split_once("\n## Syntax\n").expect("a syntax group").1;
    let expected = [
        "\n2 syntax rules. They say how formulas, classes and variables are built from smaller parts, and claim nothing is provable.\n\n".to_string(),
        entry("wi", &["<p class=\"c\">wff ( \u{1D711} \u{2192} \u{1D713} )<samp>wff ( ph -&gt; ps )</samp></p>"]),
        entry("wodd", &["<p class=\"c\">wff \u{2018} \u{1D711} \u{2223} \u{1D713} &lt; &amp; \u{2216} foo<samp>wff ` ph | ps &lt; &amp; \\ foo</samp></p>"]),
        "\n## Axioms\n\n3 axioms. The library takes each of these as true without a proof.\n\n".to_string(),
        entry("ax-1", &["<p class=\"c\">\u{22A2} ( \u{1D711} \u{2192} ( \u{1D713} \u{2192} \u{1D711} ) )<samp>|- ( ph -&gt; ( ps -&gt; ph ) )</samp></p>"]),
        entry("ax-mp", &[
            "<p class=\"h\">\u{22A2} \u{1D711}<samp>|- ph</samp></p>",
            "<p class=\"h\">\u{22A2} ( \u{1D711} \u{2192} \u{1D713} )<samp>|- ( ph -&gt; ps )</samp></p>",
            "<p class=\"c\">\u{22A2} \u{1D713}<samp>|- ps</samp></p>",
        ]),
        entry("ax-d", &[
            "<p class=\"c\">\u{22A2} \u{2018} \u{1D711} \u{2223} \u{1D713} &lt; &amp; \u{2216}<samp>|- ` ph | ps &lt; &amp; \\</samp></p>",
            "<p class=\"d\">Distinct variables: \u{1D711},\u{1D713}<samp>ph ps</samp></p>",
        ]),
        "\n## Definitions\n\n1 definition. Each one introduces a symbol by stating what it means, and is accepted without a proof.\n\n".to_string(),
        entry("df-foo", &["<p class=\"c\">\u{22A2} ( \u{1D711} \u{2192} \u{1D711} )<samp>|- ( ph -&gt; ph )</samp></p>"]),
        "\n## Other unproved assertions\n\n1 other unproved assertion. These state a provable fact without a proof and are not named as axioms or definitions.\n\n".to_string(),
        entry("fact", &["<p class=\"c\">\u{22A2} ( \u{1D713} \u{2192} \u{1D713} )<samp>|- ( ps -&gt; ps )</samp></p>"]),
        "\n## Theorems\n\n1 theorem. Each one has a proof stored in the library, which the calculator replays to check it.\n\n".to_string(),
        entry("th", &["<p class=\"c\">\u{22A2} ( \u{1D711} \u{2192} ( \u{1D713} \u{2192} \u{1D711} ) )<samp>|- ( ph -&gt; ( ps -&gt; ph ) )</samp></p>"]),
    ]
    .concat();
    assert_eq!(
        body.trim_start_matches('\n'),
        expected.trim_start_matches('\n')
    );
}

#[test]
fn the_header_counts_the_groups_and_links_the_commit() {
    let text = render(MIXED);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "# T");
    assert_eq!(lines[2], "d");
    assert_eq!(
        lines[4],
        "- Contents: 2 syntax rules, 3 axioms, 1 definition, 1 other unproved assertion, 1 theorem"
    );
    assert!(lines[5].starts_with("- Size: "));
    assert!(lines[5].ends_with(" flash pages of 16,384 bytes"));
    assert_eq!(
        lines[6],
        "- Source: `x.mm` in [example.org/r](https://example.org/r) at commit [abc](https://example.org/r/commit/abc)"
    );
}

#[test]
fn the_notation_key_escapes_symbols_that_break_markdown_and_tables() {
    let text = render(MIXED);
    let rows: Vec<&str> = text.lines().filter(|l| l.starts_with("<tr><td>")).collect();
    assert_eq!(
        rows,
        [
            "<tr><td><samp>|-</samp></td><td>\u{22A2}</td><td>is provable (the turnstile)</td></tr>",
            "<tr><td><samp>wff</samp></td><td>wff</td><td>typecode of a formula</td></tr>",
            "<tr><td><samp>(</samp></td><td>(</td><td>grouping</td></tr>",
            "<tr><td><samp>)</samp></td><td>)</td><td>grouping</td></tr>",
            "<tr><td><samp>-&gt;</samp></td><td>\u{2192}</td><td>implies</td></tr>",
            "<tr><td><samp>`</samp></td><td>\u{2018}</td><td></td></tr>",
            "<tr><td><samp>|</samp></td><td>\u{2223}</td><td>such that, inside a class builder</td></tr>",
            "<tr><td><samp>&lt;</samp></td><td>&lt;</td><td>is less than</td></tr>",
            "<tr><td><samp>&amp;</samp></td><td>&amp;</td><td></td></tr>",
            "<tr><td><samp>\\</samp></td><td>\u{2216}</td><td>set difference</td></tr>",
            "<tr><td><samp>foo</samp></td><td>foo</td><td></td></tr>",
        ]
    );
    assert!(text.contains("<p>wff variables stand for any formula: \u{1D711}, \u{1D713}, written <samp>ph</samp>, <samp>ps</samp>.</p>"));
}

#[test]
fn a_token_without_a_unicode_spelling_stays_ascii_and_is_reported() {
    let text = render(MIXED);
    assert!(text.contains("<samp>foo</samp></td><td>foo</td>"));
    assert_eq!(page::unmapped(&packed(MIXED)).unwrap(), ["&", "foo"]);
}

fn temp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn doc_can_require_a_unicode_spelling_of_every_symbol() {
    let dir = temp("unicode");
    std::fs::write(dir.join("t.toml"), MANIFEST).unwrap();
    let pin = "repository = \"https://example.org/r\"\ncommit = \"abc\"\npath = \"x.mm\"\nsha256 = \"0\"\n";
    std::fs::write(dir.join("source.toml"), pin).unwrap();
    let library = dir.join("t.mclib");
    std::fs::write(&library, packed(MIXED)).unwrap();
    let page = dir.join("t.md");
    let _ = std::fs::remove_file(&page);
    let run = |strict: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mcpack"));
        cmd.arg("doc")
            .arg(dir.join("t.toml"))
            .arg("--library")
            .arg(&library)
            .arg("-o")
            .arg(&page);
        if strict {
            cmd.arg("--require-unicode");
        }
        cmd.output().expect("mcpack runs")
    };
    let out = run(true);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        format!("{}: no Unicode spelling for & foo\n", library.display())
    );
    assert!(!page.exists());
    assert_eq!(run(false).status.code(), Some(0));
    assert!(page.exists());
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
