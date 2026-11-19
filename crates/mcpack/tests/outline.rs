use std::path::Path;
use std::process::{Command, Output};

const LOGIC: &str = "
$c ( ) -> wff |- $.
$v p q r $.
wp $f wff p $.
wq $f wff q $.
wr $f wff r $.
wi $a wff ( p -> q ) $.
ax-1 $a |- ( p -> ( q -> p ) ) $.
ax-2 $a |- ( ( p -> ( q -> r ) ) -> ( ( p -> q ) -> ( p -> r ) ) ) $.
${
  min $e |- p $.
  maj $e |- ( p -> q ) $.
  ax-mp $a |- q $.
$}
${
  a1i.1 $e |- p $.
  a1i $p |- ( q -> p ) $= wp wq wp wi a1i.1 wp wq ax-1 ax-mp $.
$}
${
  a2i.1 $e |- ( p -> ( q -> r ) ) $.
  a2i $p |- ( ( p -> q ) -> ( p -> r ) ) $=
    wp wq wr wi wi wp wq wi wp wr wi wi a2i.1 wp wq wr ax-2 ax-mp $.
$}
${
  mpd.1 $e |- ( p -> q ) $.
  mpd.2 $e |- ( p -> ( q -> r ) ) $.
  mpd $p |- ( p -> r ) $=
    wp wq wi wp wr wi mpd.1 wp wq wr mpd.2 a2i ax-mp $.
$}
${
  twice.1 $e |- p $.
  twice $p |- ( p -> p ) $= ( wi a1i ax-mp ) AACAACAABDZAACAACFDE $.
  twice-n $p |- ( p -> p ) $=
    wp wp wi wp wp wi wp wp twice.1 a1i wp wp wi wp wp wi wp wp twice.1 a1i a1i ax-mp $.
$}
plain $p |- ( p -> ( q -> p ) ) $= wp wq ax-1 $.
${
  foo.1 $e |- p $.
  foo $p |- p $= foo.1 $.
$}
bad $p |- ( p -> p ) $= wp wq ax-1 $.
syntax-reuse $p |- ( ( p -> p ) -> ( ( p -> p ) -> ( p -> p ) ) ) $= ( wi ax-1 ) AABZDC $.
syntax $p wff ( p -> p ) $= wp wp wi $.
$v ph ps $.
wph $f wff ph $.
wps $f wff ps $.
ax-s $a |- ( ph -> ( ps -> ph ) ) $.
th-s $p |- ( ph -> ( ps -> ph ) ) $= wph wps ax-s $.
${
  hyp.1 $e |- ph $.
  hyp $p |- ph $= hyp.1 $.
$}
";

fn outline(dir: &str, label: &str) -> Output {
    outline_with(dir, label, &[])
}

fn outline_with(dir: &str, label: &str, flags: &[&str]) -> Output {
    outline_of(LOGIC, dir, label, flags)
}

fn outline_of(source: &str, dir: &str, label: &str, flags: &[&str]) -> Output {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(dir);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let database = dir.join("logic.mm");
    std::fs::write(&database, source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("outline")
        .arg(&database)
        .arg(label)
        .args(flags)
        .output()
        .expect("mcpack runs")
}

fn stdout(label: &str) -> String {
    let out = outline(&format!("outline-ok-{label}"), label);
    assert_eq!(out.status.code(), Some(0), "{label}");
    assert_eq!(out.stderr, b"");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn nested_essential_hypotheses_indent_under_their_parent() {
    assert_eq!(
        stdout("mpd"),
        "\
|- ( p -> r )  [ax-mp]
  |- ( p -> q )  [mpd.1]
  |- ( ( p -> q ) -> ( p -> r ) )  [a2i]
    |- ( p -> ( q -> r ) )  [mpd.2]
"
    );
}

const TWICE: &str = "\
|- ( p -> p )  [ax-mp]
  |- ( p -> p )  [a1i]
    |- p  [twice.1]
  |- ( ( p -> p ) -> ( p -> p ) )  [a1i]
    |- ( p -> p )  [a1i]
      |- p  [twice.1]
";

#[test]
fn a_saved_step_appears_at_every_place_it_is_used() {
    assert_eq!(stdout("twice"), TWICE);
}

#[test]
fn compressed_and_normal_proofs_give_the_same_tree() {
    assert_eq!(stdout("twice-n"), TWICE);
}

#[test]
fn a_theorem_applying_one_axiom_has_a_single_node() {
    assert_eq!(stdout("plain"), "|- ( p -> ( q -> p ) )  [ax-1]\n");
}

#[test]
fn an_unknown_label_exits_2() {
    let out = outline("outline-unknown", "nope");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"no statement is labelled nope\n");
}

#[test]
fn an_axiom_exits_1() {
    let out = outline("outline-axiom", "ax-1");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"ax-1: it is not a theorem\n");
}

#[test]
fn a_syntax_theorem_exits_1() {
    let out = outline("outline-syntax", "syntax");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        out.stderr,
        b"syntax: it does not conclude a provable statement\n"
    );
}

#[test]
fn a_proof_ending_in_a_hypothesis_prints_the_hypothesis() {
    assert_eq!(stdout("foo"), "|- p  [foo.1]\n");
}

#[test]
fn a_proof_that_does_not_verify_exits_1_with_its_error() {
    let out = outline("outline-bad", "bad");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(
        out.stderr,
        b"bad: its proof does not verify: WrongConclusion\n"
    );
}

#[test]
fn a_reused_syntax_step_leaves_no_trace_in_the_tree() {
    assert_eq!(
        stdout("syntax-reuse"),
        "|- ( ( p -> p ) -> ( ( p -> p ) -> ( p -> p ) ) )  [ax-1]\n"
    );
}

fn links(dir: &str, text: &str) -> String {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("links");
    std::fs::write(&file, text).unwrap();
    file.to_str().unwrap().to_string()
}

#[test]
fn unicode_spells_each_token_and_keeps_the_shape() {
    let out = outline_with("outline-unicode", "th-s", &["--unicode"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\u{22A2} ( \u{1D711} \u{2192} ( \u{1D713} \u{2192} \u{1D711} ) )  [ax-s]\n"
    );
}

#[test]
fn the_default_output_stays_ascii() {
    assert_eq!(stdout("hyp"), "|- ph  [hyp.1]\n");
}

#[test]
fn unicode_fails_naming_a_token_without_a_spelling() {
    let out = outline_with("outline-unicode-missing", "plain", &["--unicode"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"plain: no Unicode spelling for q\n");
}

#[test]
fn html_links_an_assertion_and_leaves_a_hypothesis_plain() {
    let file = links("outline-links-a", "ax-s ../libraries/prose.md\n");
    let out = outline_with("outline-html-a", "th-s", &["--html", "--links", &file]);
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "<ul class=\"tree\" role=\"list\" data-statement=\"|- ( ph -&gt; ( ps -&gt; ph ) )\">\n  <li><span class=\"st\">\u{22A2} ( \u{1D711} \u{2192} ( \u{1D713} \u{2192} \u{1D711} ) )</span> <a class=\"lb\" href=\"../libraries/prose.md#ax-s\">ax-s</a></li>\n</ul>\n"
    );
    let file = links("outline-links-h", "ax-s x.md\n");
    let out = outline_with("outline-html-h", "hyp", &["--html", "--links", &file]);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "<ul class=\"tree\" role=\"list\" data-statement=\"|- ph\">\n  <li><span class=\"st\">\u{22A2} \u{1D711}</span> <span class=\"lb\">hyp.1</span></li>\n</ul>\n"
    );
}

#[test]
fn html_fails_for_an_assertion_on_no_page() {
    let file = links("outline-links-none", "other x.md\n");
    let out = outline_with("outline-html-none", "th-s", &["--html", "--links", &file]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"th-s: ax-s is on no library page\n");
}

fn spell(input: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("spell")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("mcpack runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn spell_prints_each_line_in_unicode_and_fails_on_an_unknown_token() {
    let out = spell("|- ( ph -> ps )\n\n");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\u{22A2} ( \u{1D711} \u{2192} \u{1D713} )\n\n"
    );
    let out = spell("ph zz yy zz\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"no Unicode spelling for zz yy\n");
}

const SYMBOLS: &str = "
$c ( ) -> wff |- < \" & $.
$v ph .0. $.
wph $f wff ph $.
wz $f wff .0. $.
wl $a wff ( ph < .0. ) $.
wq $a wff ( ph \" .0. ) $.
wa $a wff ( ph & .0. ) $.
ax-br $a |- ( ( ph < .0. ) -> ( ph \" .0. ) ) $.
th $p |- ( ( ph < .0. ) -> ( ph \" .0. ) ) $= wph wz ax-br $.
amp $a |- ( ph & .0. ) $.
amp2 $p |- ( ph & .0. ) $= wph wz amp $.
";

#[test]
fn html_escapes_tokens_and_marks_a_symbol_variable() {
    let file = links("outline-links-symbols", "ax-br x.md\n");
    let out = outline_of(
        SYMBOLS,
        "outline-html-symbols",
        "th",
        &["--html", "--links", &file],
    );
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "<ul class=\"tree\" role=\"list\" data-statement=\"|- ( ( ph &lt; .0. ) -&gt; ( ph &quot; .0. ) )\">\n  <li><span class=\"st\">\u{22A2} ( ( \u{1D711} &lt; <span class=\"s\">0</span> ) \u{2192} ( \u{1D711} \u{201C} <span class=\"s\">0</span> ) )</span> <a class=\"lb\" href=\"x.md#ax-br\">ax-br</a></li>\n</ul>\n"
    );
}

#[test]
fn a_token_with_an_ampersand_has_no_spelling_so_no_html_is_written() {
    let out = outline_of(SYMBOLS, "outline-html-amp", "amp2", &["--html"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"amp2: no Unicode spelling for &\n");
}

const TWICE_HTML: &str = r#"<ul class="tree" role="list" data-statement="|- ( p -&gt; p )">
  <li><span class="st">⊢ ( 𝑝 → 𝑝 )</span> <a class="lb" href="x.md#ax-mp">ax-mp</a>
    <ul role="list">
      <li><span class="st">⊢ ( 𝑝 → 𝑝 )</span> <a class="lb" href="y.md#a1i">a1i</a>
        <ul role="list">
          <li><span class="st">⊢ 𝑝</span> <span class="lb">twice.1</span></li>
        </ul>
      </li>
      <li><span class="st">⊢ ( ( 𝑝 → 𝑝 ) → ( 𝑝 → 𝑝 ) )</span> <a class="lb" href="y.md#a1i">a1i</a>
        <ul role="list">
          <li><span class="st">⊢ ( 𝑝 → 𝑝 )</span> <a class="lb" href="y.md#a1i">a1i</a>
            <ul role="list">
              <li><span class="st">⊢ 𝑝</span> <span class="lb">twice.1</span></li>
            </ul>
          </li>
        </ul>
      </li>
    </ul>
  </li>
</ul>
"#;

#[test]
fn html_nests_each_goal_with_subgoals_and_repeats_a_saved_step() {
    let file = links("outline-links-twice", "ax-mp x.md\na1i y.md\n");
    let out = outline_with("outline-html-twice", "twice", &["--html", "--links", &file]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8(out.stdout).unwrap(), TWICE_HTML);
}

#[test]
fn links_without_html_is_refused() {
    let file = links("outline-links-alone", "ax-1 x.md\n");
    let out = outline_with("outline-links-alone-run", "plain", &["--links", &file]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(out.stdout, b"");
    assert!(String::from_utf8_lossy(&out.stderr).contains("--html"));
}

#[test]
fn a_links_file_may_have_blank_lines_and_crlf_endings() {
    let file = links("outline-links-crlf", "\r\nax-s ../p.md\r\n\r\n");
    let out = outline_with("outline-html-crlf", "th-s", &["--html", "--links", &file]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("href=\"../p.md#ax-s\">ax-s</a>")
    );
}

#[test]
fn a_links_line_without_a_page_is_refused() {
    let file = links("outline-links-bad", "ax-s\n");
    let out = outline_with("outline-html-bad", "th-s", &["--html", "--links", &file]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no page in"));
}
