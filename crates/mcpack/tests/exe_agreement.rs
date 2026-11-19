//! Each case is a small database and whether metamath-exe reads it and verifies its proofs without
//! error. The parser and the verifier must agree with the table, and the ignored test holds the
//! table to metamath-exe itself.

use mcpack::mm::{Database, Kind, StmtId, Verifier, parse_at, parse_at_capped};
use std::path::{Path, PathBuf};
use std::process::Command;

const BASE: &str = "$c wff |- $. $v p $. wp $f wff p $.\n";

struct Case {
    name: &'static str,
    reads: bool,
    /// Whether metamath-exe reads it, which differs from `reads` only where we differ on purpose.
    exe: bool,
    /// A fragment of the message of metamath-exe, where it refuses.
    exe_says: &'static str,
    /// A fragment of our message, where we refuse.
    we_say: &'static str,
    /// The files to write, the first being the one read.
    files: &'static [(&'static str, &'static str)],
}

const fn case(name: &'static str, files: &'static [(&'static str, &'static str)]) -> Case {
    Case {
        name,
        reads: true,
        exe: true,
        exe_says: "",
        we_say: "",
        files,
    }
}

const fn refused(
    name: &'static str,
    exe_says: &'static str,
    we_say: &'static str,
    files: &'static [(&'static str, &'static str)],
) -> Case {
    Case {
        name,
        reads: false,
        exe: false,
        exe_says,
        we_say,
        files,
    }
}

const fn only_we_read(
    name: &'static str,
    exe_says: &'static str,
    files: &'static [(&'static str, &'static str)],
) -> Case {
    Case {
        name,
        reads: true,
        exe: false,
        exe_says,
        we_say: "",
        files,
    }
}

const CASES: &[Case] = &[
    case("comment", &[("top.mm", "$( a $)\n$c a $.")]),
    case(
        "comment with lone dollars",
        &[("top.mm", "$( a$ $a a$a $[ $. $)\n$c a $.")],
    ),
    refused(
        "open marker inside a token",
        "Nested comments are not allowed",
        "$( inside a comment",
        &[("top.mm", "$( a$(b $)\n$c a $.")],
    ),
    refused(
        "open marker ends a token",
        "Nested comments are not allowed",
        "$( inside a comment",
        &[("top.mm", "$( a$( $)\n$c a $.")],
    ),
    refused(
        "open marker starts a token",
        "Nested comments are not allowed",
        "$( inside a comment",
        &[("top.mm", "$( $(b $)\n$c a $.")],
    ),
    refused(
        "close marker inside a token",
        "A keyword must be preceded by white space",
        "$) must stand alone",
        &[("top.mm", "$( a$)b $)\n$c a $.")],
    ),
    refused(
        "close marker ends a token",
        "A keyword must be preceded by white space",
        "$) must stand alone",
        &[("top.mm", "$( x$) $)\n$c a $.")],
    ),
    refused(
        "close marker starts a token",
        "A keyword must be followed by white space",
        "$) must stand alone",
        &[("top.mm", "$( $)x $)\n$c a $.")],
    ),
    refused(
        "dollar before a close marker",
        "A keyword must be preceded by white space",
        "$) must stand alone",
        &[("top.mm", "$( $$) $)\n$c a $.")],
    ),
    refused(
        "dollar before an open marker",
        "Nested comments are not allowed",
        "$( inside a comment",
        &[("top.mm", "$( $$( $)\n$c a $.")],
    ),
    refused(
        "nested comment",
        "Nested comments are not allowed",
        "$( inside a comment",
        &[("top.mm", "$( a $( b $) $)\n$c a $.")],
    ),
    refused(
        "close marker alone",
        "A comment terminator was found outside of a comment",
        "unexpected $)",
        &[("top.mm", "$c a $. $)")],
    ),
    case("inclusion", &[("top.mm", "$[ inc.mm $]"), ("inc.mm", BASE)]),
    case(
        "file included twice",
        &[("top.mm", "$[ inc.mm $] $[ inc.mm $]"), ("inc.mm", BASE)],
    ),
    case(
        "file included twice and more after",
        &[
            ("top.mm", "$[ inc.mm $] $[ inc.mm $] $c x $."),
            ("inc.mm", BASE),
        ],
    ),
    // metamath-exe reads `./inc.mm` as a second file and fails on the repeated statements; we
    // key files by canonical path, so both spellings are one file and the second is skipped.
    only_we_read(
        "file included twice under another spelling",
        "This symbol has already been declared in this scope",
        &[("top.mm", "$[ inc.mm $] $[ ./inc.mm $]"), ("inc.mm", BASE)],
    ),
    case(
        "inclusion inside a block",
        &[
            ("top.mm", "$c wff |- $. $v p $. ${ $[ inc.mm $] $}"),
            ("inc.mm", "wp $f wff p $."),
        ],
    ),
    case(
        "inclusion inside a statement",
        &[("top.mm", "$c a $[ inc.mm $] $."), ("inc.mm", "b c")],
    ),
    case(
        "block opened in the included file",
        &[("top.mm", "$[ inc.mm $] $}"), ("inc.mm", "${")],
    ),
    case(
        "inclusion inside a comment",
        &[("top.mm", "$( $[ missing.mm $] $)\n$c a $.")],
    ),
    refused(
        "missing file",
        "was not found",
        "cannot read missing.mm",
        &[("top.mm", "$[ missing.mm $]")],
    ),
    refused(
        "no file name",
        "is unterminated or has ill-formed",
        "$[ is not closed by $]",
        &[("top.mm", "$[ $]")],
    ),
    refused(
        "two file names",
        "is unterminated or has ill-formed",
        "$[ takes one file name and $]",
        &[("top.mm", "$[ a.mm b.mm $]"), ("a.mm", BASE)],
    ),
    case(
        "dollar in the file name",
        &[("top.mm", "$[ a$b.mm $]"), ("a$b.mm", BASE)],
    ),
    refused(
        "unclosed inclusion",
        "is unterminated or has ill-formed",
        "$[ is not closed by $]",
        &[("top.mm", "$[ inc.mm"), ("inc.mm", BASE)],
    ),
    refused(
        "inclusion closed by another keyword",
        "is unterminated or has ill-formed",
        "$[ takes one file name and $]",
        &[("top.mm", "$[ inc.mm $.\n"), ("inc.mm", BASE)],
    ),
    refused(
        "close without open",
        "is required before",
        "unexpected $]",
        &[("top.mm", "$] $c a $.")],
    ),
    refused(
        "second close",
        "is required before",
        "unexpected $]",
        &[("top.mm", "$[ inc.mm $] $]"), ("inc.mm", BASE)],
    ),
    refused(
        "comment opened in the included file",
        "End of comment not found",
        "unterminated comment",
        &[("top.mm", "$[ inc.mm $] x $)"), ("inc.mm", "$( open")],
    ),
    refused(
        "file includes itself",
        "DETECTED BUG",
        "top.mm includes itself",
        &[("top.mm", "$[ top.mm $]\n$c a $.")],
    ),
    refused(
        "files include each other",
        "DETECTED BUG",
        "top.mm includes itself",
        &[("top.mm", "$[ a.mm $]"), ("a.mm", "$[ top.mm $]")],
    ),
    case(
        "name read from where the database is read, not from the including file",
        &[
            ("top.mm", "$[ sub/a.mm $]"),
            ("sub/a.mm", "$[ b.mm $]"),
            ("b.mm", BASE),
        ],
    ),
    refused(
        "name found only beside the including file",
        "was not found",
        "cannot read b.mm",
        &[
            ("top.mm", "$[ sub/a.mm $]"),
            ("sub/a.mm", "$[ b.mm $]"),
            ("sub/b.mm", BASE),
        ],
    ),
    refused(
        "a pair declared around an axiom binds its uses",
        "be disjoint",
        "DisjointViolation",
        &[(
            "top.mm",
            "$c wff |- R $. $v x y $. vx $f wff x $. vy $f wff y $.
             $d x y $.
             ${ ax $a |- R x y $. $}
             th $p |- R x x $= vx vx ax $.",
        )],
    ),
    case(
        "a pair declared around a theorem lets it use the axiom",
        &[(
            "top.mm",
            "$c wff |- R $. $v x y $. vx $f wff x $. vy $f wff y $.
             ${ $d x y $. ax $a |- R x y $. $}
             ${ $d x y $. ${ th $p |- R x y $= vx vy ax $. $} $}",
        )],
    ),
    case(
        "path with a parent directory",
        &[
            ("top.mm", "$[ sub/../inc.mm $]"),
            ("sub/a.mm", ""),
            ("inc.mm", BASE),
        ],
    ),
];

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mcpack-agree-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, case: &Case) {
    for (name, text) in case.files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn first_bad_proof(db: &Database) -> Result<(), String> {
    let mut verifier = Verifier::new(db);
    for id in 0..db.stmts.len() as StmtId {
        let stmt = db.stmt(id);
        if stmt.kind == Kind::Theorem {
            verifier
                .verify(id)
                .map_err(|error| format!("{}: {error:?}", stmt.label))?;
        }
    }
    Ok(())
}

#[test]
fn the_parser_reads_what_the_table_says_metamath_exe_reads() {
    for (i, case) in CASES.iter().enumerate() {
        let dir = scratch(&i.to_string());
        write(&dir, case);
        let top = dir.join(case.files[0].0);
        let parsed = parse_at(&top, std::fs::read(&top).unwrap())
            .map_err(|error| error.to_string())
            .and_then(|db| first_bad_proof(&db));
        assert_eq!(
            parsed.is_ok(),
            case.reads,
            "{}: {:?}",
            case.name,
            parsed.as_ref().err()
        );
        if let Err(message) = parsed {
            assert!(
                message.contains(case.we_say),
                "{}: {message} does not say {}",
                case.name,
                case.we_say
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
#[ignore = "needs metamath-exe, built by scripts/fetch-metamath.sh"]
fn the_table_is_what_metamath_exe_reads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let exe =
        std::env::var_os("METAMATH").map_or_else(|| root.join(".metamath/metamath"), PathBuf::from);
    assert!(
        exe.is_file(),
        "{} is not there, run scripts/fetch-metamath.sh",
        exe.display()
    );
    for (i, case) in CASES.iter().enumerate() {
        let dir = scratch(&format!("exe{i}"));
        write(&dir, case);
        let out = Command::new(&exe)
            .current_dir(&dir)
            .args(["read top.mm /verify", "exit"])
            .output()
            .expect("metamath-exe runs");
        let text = String::from_utf8_lossy(&out.stdout);
        let reads = text.contains("No errors were found")
            && !text.contains("?Error")
            && !text.contains("BUG");
        println!(
            "{}: metamath-exe {}",
            case.name,
            if reads { "reads it" } else { "refuses it" }
        );
        assert_eq!(reads, case.exe, "{}:\n{text}", case.name);
        assert!(
            text.contains(case.exe_says),
            "{}: metamath-exe does not say {}:\n{text}",
            case.name,
            case.exe_says
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn an_error_names_the_included_file_and_line_it_is_in() {
    let dir = scratch("lines");
    let files = [
        ("top.mm", "$( one\ntwo $)\n$[ inc.mm $]\n\n$c oops\n"),
        ("inc.mm", "$c a $.\n\n$c a $.\n"),
    ];
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let top = dir.join("top.mm");
    let error = parse_at(&top, std::fs::read(&top).unwrap()).err().unwrap();
    assert_eq!(error.file.as_deref(), Some("inc.mm"));
    assert_eq!(error.line, 3);
    std::fs::write(dir.join("inc.mm"), "$c a $.\n").unwrap();
    let error = parse_at(&top, std::fs::read(&top).unwrap()).err().unwrap();
    assert_eq!((error.file, error.line), (None, 6));
    std::fs::remove_dir_all(&dir).unwrap();
}

fn include_dir(name: &str, top: &str, files: &[(&str, &[u8])]) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    for (file, bytes) in files {
        std::fs::write(dir.join(file), bytes).unwrap();
    }
    let path = dir.join("top.mm");
    std::fs::write(&path, top).unwrap();
    (dir, path)
}

fn refusal(path: &Path, max_file: u64, max_total: u64) -> String {
    let src = std::fs::read(path).unwrap();
    let error = parse_at_capped(path, src, max_file, max_total).err();
    error.expect("the database is refused").to_string()
}

#[test]
fn an_included_directory_is_refused_by_name() {
    let (dir, top) = include_dir("directory", "$[ sub $]", &[]);
    std::fs::create_dir(dir.join("sub")).unwrap();
    assert!(refusal(&top, 1 << 20, 1 << 20).contains("sub is not a regular file"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn an_included_symlink_loop_is_refused() {
    let (dir, top) = include_dir("loop", "$[ a.mm $]", &[]);
    std::os::unix::fs::symlink(dir.join("b.mm"), dir.join("a.mm")).unwrap();
    std::os::unix::fs::symlink(dir.join("a.mm"), dir.join("b.mm")).unwrap();
    assert!(refusal(&top, 1 << 20, 1 << 20).contains("cannot read a.mm"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn an_included_fifo_is_refused_without_blocking() {
    let (dir, top) = include_dir("fifo", "$[ pipe $]", &[]);
    let made = Command::new("mkfifo")
        .arg(dir.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success());
    let (sender, receiver) = std::sync::mpsc::channel();
    let path = top.clone();
    std::thread::spawn(move || sender.send(refusal(&path, 1 << 20, 1 << 20)));
    let message = receiver
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("the read of a FIFO did not finish");
    assert!(message.contains("pipe is not a regular file"), "{message}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_included_file_over_the_cap_is_refused_and_one_at_the_cap_is_read() {
    let (dir, top) = include_dir("cap", "$[ inc.mm $]", &[("inc.mm", BASE.as_bytes())]);
    let len = BASE.len() as u64;
    assert!(refusal(&top, len - 1, 1 << 20).contains(&format!("inc.mm is over {} bytes", len - 1)));
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at_capped(&top, src, len, 1 << 20).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_expanded_text_is_capped() {
    let (dir, top) = include_dir("total", "$[ inc.mm $]", &[("inc.mm", BASE.as_bytes())]);
    let total = "$[ inc.mm $]".len() as u64 + BASE.len() as u64;
    assert!(refusal(&top, 1 << 20, total - 1).contains("the files read pass"));
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at_capped(&top, src, 1 << 20, total).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A file that includes `next` first and then pads itself to `len` bytes with blanks.
fn padded(next: Option<&str>, len: usize) -> String {
    let head = next.map_or(String::new(), |n| format!("$[ {n} $]"));
    format!("{head}{}", " ".repeat(len - head.len()))
}

#[test]
fn every_file_read_counts_toward_the_total_while_it_is_still_open() {
    let files = [
        ("top.mm", padded(Some("a.mm"), 100)),
        ("a.mm", padded(Some("b.mm"), 100)),
        ("b.mm", padded(Some("c.mm"), 100)),
        ("c.mm", padded(None, 100)),
    ];
    let bytes: Vec<(&str, &[u8])> = files.iter().map(|(n, t)| (*n, t.as_bytes())).collect();
    let (dir, top) = include_dir("chain", &files[0].1, &bytes[1..]);
    let message = refusal(&top, 1 << 20, 399);
    assert!(
        message.starts_with("b.mm: ") && message.contains("pass 399 bytes at c.mm"),
        "{message}"
    );
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at_capped(&top, src, 1 << 20, 400).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_top_level_file_counts_toward_the_total() {
    let (dir, top) = include_dir("top-total", &padded(None, 300), &[]);
    let message = refusal(&top, 1 << 20, 299);
    assert!(
        message.contains("the database passes 299 bytes"),
        "{message}"
    );
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at_capped(&top, src, 1 << 20, 300).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

fn chain(name: &str, length: usize) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    for i in 0..=length {
        let next = (i < length).then(|| format!("f{}.mm", i + 1));
        let text = next.map_or(BASE.to_string(), |n| format!("$[ {n} $]"));
        let file = if i == 0 {
            "top.mm".into()
        } else {
            format!("f{i}.mm")
        };
        std::fs::write(dir.join(file), text).unwrap();
    }
    let top = dir.join("top.mm");
    (dir, top)
}

#[test]
fn inclusions_nest_one_hundred_deep_and_no_deeper() {
    let (dir, top) = chain("deepest", 100);
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at(&top, src).is_ok(), "100 nested inclusions");
    std::fs::remove_dir_all(&dir).unwrap();
    let (dir, top) = chain("too-deep", 101);
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(
        message.starts_with("f100.mm: ") && message.contains("inclusions nest more than 100 deep"),
        "{message}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn text_from_the_database_is_cut_and_escaped_in_messages() {
    let long = "n".repeat(300);
    let (dir, top) = include_dir("long-name", &format!("$[ {long} $]"), &[]);
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(
        message.contains(&format!("cannot read {}...:", "n".repeat(64))),
        "{message}"
    );
    assert!(message.len() < 200, "{message}");
    std::fs::write(&top, b"$[ a\x1b[2Jb\xff $]").unwrap();
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(message.contains("a\\u{1b}[2Jb\\xff"), "{message}");
    assert!(!message.contains('\x1b'), "{message}");
    std::fs::write(&top, format!("$c {long} $. $c {long} $.")).unwrap();
    let src = std::fs::read(&top).unwrap();
    let message = parse_at(&top, src).err().unwrap().to_string();
    assert!(
        message.contains(&format!("{}...", "n".repeat(64))) && message.len() < 200,
        "{message}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_binary_file_is_refused_without_quoting_it() {
    let binary: &[u8] = b"\x7fELF\x02\x01 secret\n";
    let (dir, top) = include_dir("binary", "$[ inc.mm $]", &[("inc.mm", binary)]);
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(
        message.contains("inc.mm is not a Metamath database"),
        "{message}"
    );
    assert!(
        !message.contains("ELF") && !message.contains("secret"),
        "{message}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn two_spellings_of_one_file_are_one_file() {
    let (dir, top) = include_dir(
        "spellings",
        "$[ inc.mm $] $[ ./inc.mm $] $[ sub/../inc.mm $]",
        &[("inc.mm", BASE.as_bytes())],
    );
    std::fs::create_dir(dir.join("sub")).unwrap();
    let src = std::fs::read(&top).unwrap();
    assert!(parse_at(&top, src).is_ok());
    std::fs::write(&top, "$[ inc.mm $]").unwrap();
    std::fs::write(dir.join("inc.mm"), "$[ ./inc.mm $]").unwrap();
    assert!(refusal(&top, 1 << 20, 1 << 20).contains("includes itself"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn pack_and_closure_refuse_inclusion_and_verify_reads_it() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/include");
    let includer = corpus.join("demo0-includer.mm");
    for command in ["pack", "closure"] {
        let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
            .arg(command)
            .arg(&includer)
            .arg("th1")
            .args(if command == "pack" {
                vec!["-o", "/dev/null"]
            } else {
                vec![]
            })
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{command} read an include");
        assert!(
            err.contains("file inclusion is not supported"),
            "{command}: {err}"
        );
    }
    let verified = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("verify")
        .arg(&includer)
        .output()
        .unwrap();
    assert!(verified.status.success());
}

#[cfg(unix)]
#[test]
fn an_included_file_with_an_escape_in_its_name_is_named_escaped() {
    let (dir, top) = include_dir(
        "escape-name",
        "$[ a\x1bb.mm $]",
        &[("a\x1bb.mm", b"$c a $. $c a $.")],
    );
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(message.starts_with("a\\u{1b}b.mm: line 1:"), "{message}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_long_label_is_cut_in_what_the_commands_print() {
    let long = "l".repeat(300);
    let (dir, top) = include_dir(
        "long-label",
        &format!("$c wff |- $. $v p $. wp $f wff p $. {long} $p |- p $= ? $."),
        &[],
    );
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("verify")
        .arg(&top)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.starts_with(&format!("{}...: ", "l".repeat(64))),
        "{err}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
