//! Each case is a small database and whether metamath-exe reads it without error. The parser
//! must agree with the table, and the ignored test holds the table to metamath-exe itself.

use mcpack::mm::{parse_at, parse_at_capped};
use std::path::{Path, PathBuf};
use std::process::Command;

const BASE: &str = "$c wff |- $. $v p $. wp $f wff p $.\n";

struct Case {
    name: &'static str,
    reads: bool,
    /// Whether metamath-exe reads it, which differs from `reads` only where we differ on purpose.
    exe: bool,
    /// The files to write, the first being the one read.
    files: &'static [(&'static str, &'static str)],
}

const fn case(
    name: &'static str,
    reads: bool,
    files: &'static [(&'static str, &'static str)],
) -> Case {
    Case {
        name,
        reads,
        exe: reads,
        files,
    }
}

const fn differs(
    name: &'static str,
    reads: bool,
    exe: bool,
    files: &'static [(&'static str, &'static str)],
) -> Case {
    Case {
        name,
        reads,
        exe,
        files,
    }
}

const CASES: &[Case] = &[
    case("comment", true, &[("top.mm", "$( a $)\n$c a $.")]),
    case(
        "comment with lone dollars",
        true,
        &[("top.mm", "$( a$ $a a$a $[ $. $)\n$c a $.")],
    ),
    case(
        "open marker inside a token",
        false,
        &[("top.mm", "$( a$(b $)\n$c a $.")],
    ),
    case(
        "open marker ends a token",
        false,
        &[("top.mm", "$( a$( $)\n$c a $.")],
    ),
    case(
        "open marker starts a token",
        false,
        &[("top.mm", "$( $(b $)\n$c a $.")],
    ),
    case(
        "close marker inside a token",
        false,
        &[("top.mm", "$( a$)b $)\n$c a $.")],
    ),
    case(
        "close marker ends a token",
        false,
        &[("top.mm", "$( x$) $)\n$c a $.")],
    ),
    case(
        "close marker starts a token",
        false,
        &[("top.mm", "$( $)x $)\n$c a $.")],
    ),
    case(
        "dollar before a close marker",
        false,
        &[("top.mm", "$( $$) $)\n$c a $.")],
    ),
    case(
        "dollar before an open marker",
        false,
        &[("top.mm", "$( $$( $)\n$c a $.")],
    ),
    case(
        "nested comment",
        false,
        &[("top.mm", "$( a $( b $) $)\n$c a $.")],
    ),
    case("close marker alone", false, &[("top.mm", "$c a $. $)")]),
    case(
        "inclusion",
        true,
        &[("top.mm", "$[ inc.mm $]"), ("inc.mm", BASE)],
    ),
    case(
        "file included twice",
        true,
        &[("top.mm", "$[ inc.mm $] $[ inc.mm $]"), ("inc.mm", BASE)],
    ),
    case(
        "file included twice and more after",
        true,
        &[
            ("top.mm", "$[ inc.mm $] $[ inc.mm $] $c x $."),
            ("inc.mm", BASE),
        ],
    ),
    // metamath-exe reads `./inc.mm` as a second file and fails on the repeated statements; we
    // key files by canonical path, so both spellings are one file and the second is skipped.
    differs(
        "file included twice under another spelling",
        true,
        false,
        &[("top.mm", "$[ inc.mm $] $[ ./inc.mm $]"), ("inc.mm", BASE)],
    ),
    case(
        "inclusion inside a block",
        true,
        &[
            ("top.mm", "$c wff |- $. $v p $. ${ $[ inc.mm $] $}"),
            ("inc.mm", "wp $f wff p $."),
        ],
    ),
    case(
        "inclusion inside a statement",
        true,
        &[("top.mm", "$c a $[ inc.mm $] $."), ("inc.mm", "b c")],
    ),
    case(
        "block opened in the included file",
        true,
        &[("top.mm", "$[ inc.mm $] $}"), ("inc.mm", "${")],
    ),
    case(
        "inclusion inside a comment",
        true,
        &[("top.mm", "$( $[ missing.mm $] $)\n$c a $.")],
    ),
    case("missing file", false, &[("top.mm", "$[ missing.mm $]")]),
    case("no file name", false, &[("top.mm", "$[ $]")]),
    case(
        "two file names",
        false,
        &[("top.mm", "$[ a.mm b.mm $]"), ("a.mm", BASE)],
    ),
    case(
        "dollar in the file name",
        true,
        &[("top.mm", "$[ a$b.mm $]"), ("a$b.mm", BASE)],
    ),
    case(
        "unclosed inclusion",
        false,
        &[("top.mm", "$[ inc.mm"), ("inc.mm", BASE)],
    ),
    case(
        "inclusion closed by another keyword",
        false,
        &[("top.mm", "$[ inc.mm $.\n"), ("inc.mm", BASE)],
    ),
    case("close without open", false, &[("top.mm", "$] $c a $.")]),
    case(
        "second close",
        false,
        &[("top.mm", "$[ inc.mm $] $]"), ("inc.mm", BASE)],
    ),
    case(
        "comment opened in the included file",
        false,
        &[("top.mm", "$[ inc.mm $] x $)"), ("inc.mm", "$( open")],
    ),
    case(
        "file includes itself",
        false,
        &[("top.mm", "$[ top.mm $]\n$c a $.")],
    ),
    case(
        "files include each other",
        false,
        &[("top.mm", "$[ a.mm $]"), ("a.mm", "$[ top.mm $]")],
    ),
    case(
        "name read from where the database is read, not from the including file",
        true,
        &[
            ("top.mm", "$[ sub/a.mm $]"),
            ("sub/a.mm", "$[ b.mm $]"),
            ("b.mm", BASE),
        ],
    ),
    case(
        "name found only beside the including file",
        false,
        &[
            ("top.mm", "$[ sub/a.mm $]"),
            ("sub/a.mm", "$[ b.mm $]"),
            ("sub/b.mm", BASE),
        ],
    ),
    case(
        "path with a parent directory",
        true,
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

#[test]
fn the_parser_reads_what_the_table_says_metamath_exe_reads() {
    for (i, case) in CASES.iter().enumerate() {
        let dir = scratch(&i.to_string());
        write(&dir, case);
        let top = dir.join(case.files[0].0);
        let parsed = parse_at(&top, std::fs::read(&top).unwrap());
        assert_eq!(
            parsed.is_ok(),
            case.reads,
            "{}: {:?}",
            case.name,
            parsed.as_ref().err()
        );
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
            .args(["read top.mm", "exit"])
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
fn an_included_symlink_loop_and_fifo_are_refused() {
    let (dir, top) = include_dir("loop", "$[ a.mm $]", &[]);
    std::os::unix::fs::symlink(dir.join("b.mm"), dir.join("a.mm")).unwrap();
    std::os::unix::fs::symlink(dir.join("a.mm"), dir.join("b.mm")).unwrap();
    assert!(refusal(&top, 1 << 20, 1 << 20).contains("cannot read a.mm"));
    std::fs::write(&top, "$[ pipe $]").unwrap();
    let made = Command::new("mkfifo")
        .arg(dir.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success());
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
    std::fs::write(&top, b"$[ a\x1b\nb $]").unwrap();
    let message = refusal(&top, 1 << 20, 1 << 20);
    assert!(
        !message.contains('\x1b') && !message.contains('\n'),
        "{message}"
    );
    std::fs::write(&top, format!("$c {long} $. $c {long} $.")).unwrap();
    let src = std::fs::read(&top).unwrap();
    let message = parse_at(&top, src).err().unwrap().to_string();
    assert!(
        message.contains(&format!("{}...", "n".repeat(64))) && message.len() < 200,
        "{message}"
    );
    std::fs::write(&top, b"$c a $. a\x1b $.").unwrap();
    let src = std::fs::read(&top).unwrap();
    let message = parse_at(&top, src).err().unwrap().to_string();
    assert!(
        message.contains("outside printable ASCII") && !message.contains('\x1b'),
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
