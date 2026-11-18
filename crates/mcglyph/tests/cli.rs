use mcglyph::{Bitmap, Font, png};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mcglyph"))
        .args(args)
        .output()
        .expect("mcglyph runs")
}

fn width(png: &Path) -> u32 {
    let bytes = std::fs::read(png).unwrap();
    u32::from_be_bytes(bytes[16..20].try_into().unwrap())
}

#[test]
fn tokens_starting_with_a_dash_are_tokens() {
    let dir = temp("cli-dash");
    let png = dir.join("a.png");
    let out = run(&["-o", png.to_str().unwrap(), "|-", "-.", "ph", "->", "-u"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, format!("{}\n", png.display()).as_bytes());
    assert_eq!(width(&png), (96 + 4) * 4);
}

#[test]
fn an_unknown_dash_token_is_reported_as_a_token() {
    let dir = temp("cli-unknown");
    let png = dir.join("a.png");
    let out = run(&["-o", png.to_str().unwrap(), "-zzz"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(out.stderr, b"no glyphs for token -zzz\n");
    assert!(!png.exists());
}

#[test]
fn scale_sets_the_pixel_size() {
    let dir = temp("cli-scale");
    let png = dir.join("a.png");
    let out = run(&["--scale", "2", "-o", png.to_str().unwrap(), "ph"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(width(&png), (96 + 4) * 2);
}

#[test]
fn bad_arguments_print_the_usage_and_exit_2() {
    let dir = temp("cli-bad");
    let png = dir.join("a.png");
    let path = png.to_str().unwrap();
    for args in [
        vec!["ph"],
        vec!["-o"],
        vec!["-o", path],
        vec!["--scale", "0", "-o", path, "ph"],
        vec!["--scale", "x", "-o", path, "ph"],
        vec!["--scale"],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .starts_with("usage: mcglyph"),
            "{args:?}"
        );
    }
    assert!(!png.exists());
}

#[test]
fn a_statement_past_one_screen_is_written_as_numbered_files() {
    let dir = temp("cli-frames");
    let png = dir.join("long.png");
    let mut args = vec!["-o", png.to_str().unwrap()];
    args.extend(["(", "ph", "->", "ps", ")"].into_iter().cycle().take(400));
    let out = run(&args);
    assert_eq!(out.status.code(), Some(0));
    let first = dir.join("long-1.png");
    let second = dir.join("long-2.png");
    assert!(first.exists() && second.exists());
    assert!(!png.exists());
    let listed = String::from_utf8(out.stdout).unwrap();
    assert!(listed.starts_with(&format!("{}\n{}\n", first.display(), second.display())));
}

#[test]
fn the_keys_font_draws_each_label_as_an_inverted_box() {
    let dir = temp("cli-keys");
    let png = dir.join("k.png");
    let out = run(&[
        "--font",
        "keys",
        "--scale",
        "1",
        "-o",
        png.to_str().unwrap(),
        "Open",
        "Go",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let mut want = Bitmap::new(96, 64);
    for (row, label) in ["Open", "Go"].iter().enumerate() {
        let ink = Font::keys().text(label);
        for y in 0..8 {
            for x in 0..ink.width() + 4 {
                want.set(x, row * 8 + y, true);
            }
        }
        for y in 0..ink.height() {
            for x in 0..ink.width() {
                if ink.get(x, y) {
                    want.set(2 + x, row * 8 + 1 + y, false);
                }
            }
        }
    }
    assert!(want.get(0, 0) && !want.get(2, 2) && !want.get(40, 0) && !want.get(0, 16));
    assert_eq!(std::fs::read(&png).unwrap(), png::encode(&want, 1));
}

#[test]
fn the_keys_font_rejects_labels_it_cannot_draw() {
    let dir = temp("cli-keys-bad");
    let png = dir.join("k.png");
    let out = run(&["--font", "keys", "-o", png.to_str().unwrap(), "no way"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!png.exists());
}

#[test]
fn the_main_font_flag_gives_the_same_output_as_no_flag() {
    let dir = temp("cli-main");
    let (plain, flagged) = (dir.join("a.png"), dir.join("b.png"));
    let tokens = ["|-", "(", "ph", "->", "ps", ")"];
    let mut args = vec!["-o", plain.to_str().unwrap()];
    args.extend(tokens);
    assert_eq!(run(&args).status.code(), Some(0));
    let mut args = vec!["--font", "main", "-o", flagged.to_str().unwrap()];
    args.extend(tokens);
    assert_eq!(run(&args).status.code(), Some(0));
    assert_eq!(
        std::fs::read(&plain).unwrap(),
        std::fs::read(&flagged).unwrap()
    );
}

#[test]
fn a_font_flag_without_a_name_or_with_an_unknown_one_is_a_usage_error() {
    let dir = temp("cli-font-bad");
    let png = dir.join("a.png");
    let path = png.to_str().unwrap();
    for args in [
        vec!["-o", path, "ph", "--font"],
        vec!["--font", "tiny", "-o", path, "ph"],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .starts_with("usage: mcglyph"),
            "{args:?}"
        );
    }
    assert!(!png.exists());
}
