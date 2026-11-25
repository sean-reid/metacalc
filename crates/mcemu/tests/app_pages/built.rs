//! Building the app with `make -C app`, with a library or a stage list, and the build's labels.

use crate::realos::Shown;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

pub struct Built {
    pub file: Vec<u8>,
    pub labels: String,
    /// What the build printed for each code page: the bytes it uses.
    pub used: Vec<u32>,
    /// The library after the code pages, if any.
    pub library: Option<Vec<u8>>,
    /// The stages of the list the build was given beside the library; none for the one stage a
    /// build with no list has.
    pub stages: Vec<mcui::Stage>,
}

/// The name the build gives the one stage of a library with no stage list: the library file's.
pub const LIBRARY_NAME: &str = "library";

impl Built {
    /// The library as the model sees it, with no status table; none for a build with no library.
    pub fn shown(&self) -> Option<Shown<'_>> {
        self.library.as_deref().map(|bytes| Shown {
            name: LIBRARY_NAME,
            bytes,
            stages: self.stages.clone(),
            status: Vec::new(),
        })
    }

    /// The name of stage `i` of the library's one table, as its row shows it.
    pub fn stage_name(&self, i: usize) -> &str {
        self.stages.get(i).map_or(LIBRARY_NAME, |s| s.name.as_str())
    }

    pub fn label(&self, name: &str) -> u16 {
        self.try_label(name)
            .unwrap_or_else(|| panic!("no label {name}"))
    }

    pub fn try_label(&self, name: &str) -> Option<u16> {
        self.labels.lines().find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            key.eq_ignore_ascii_case(name)
                .then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
    }
}

/// `make -C app` into a directory of its own, with the library when there is one.
fn build(name: &str, library: Option<&[u8]>) -> Built {
    build_with(name, library, &[], &[])
}

/// `build` with more `make` arguments, such as `VERSION=1.2.3`.
fn build_with(name: &str, library: Option<&[u8]>, stages: &[mcui::Stage], more: &[&str]) -> Built {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app");
    build_from(&source, name, library, stages, more)
}

/// `build_with` for the app directory `source`, which has the rest of the tree beside it.
fn build_from(
    source: &Path,
    name: &str,
    library: Option<&[u8]>,
    stages: &[mcui::Stage],
    more: &[&str],
) -> Built {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let mut args = Vec::new();
    if let Some(bytes) = library {
        let path = dir.join("library.mclib");
        std::fs::write(&path, bytes).unwrap();
        args.push(format!("LIBRARY={}", path.display()));
    }
    if !stages.is_empty() {
        let list: String = stages
            .iter()
            .map(|s| format!("{} {} {}\n", s.name, s.end, s.title))
            .collect();
        std::fs::write(dir.join("stages.list"), list).unwrap();
    }
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let made = Command::new("make")
        .arg("-C")
        .arg(source)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}/build", dir.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")))
        .args(&args)
        .args(more)
        .arg("app")
        .output()
        .expect("make runs");
    let log = String::from_utf8_lossy(&made.stdout).into_owned();
    assert!(made.status.success(), "make: {log}");
    let used: Vec<u32> = log
        .lines()
        .filter_map(|l| l.split_once("code page "))
        .filter_map(|(_, rest)| rest.split_once(": ")?.1.split_once(" of ")?.0.parse().ok())
        .collect();
    let build = dir.join("build");
    Built {
        file: std::fs::read(build.join("metacalc.8xk")).unwrap(),
        labels: std::fs::read_to_string(build.join("metacalc.lab")).unwrap(),
        used,
        library: library.map(<[u8]>::to_vec),
        stages: stages.to_vec(),
    }
}

/// The app without a library.
pub fn plain() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| build("pages-plain", None))
}

/// The commit this branch was cut from, whose shell blocks in app_key after every frame and key.
pub const MAIN: &str = "cae6d0e";

/// The `app` directory of `MAIN`'s tree, extracted by git into a directory of its own with what its
/// Makefile reaches: the shell tests build the app there to compare the branch with it.
pub fn main_tree() -> &'static Path {
    static TREE: OnceLock<std::path::PathBuf> = OnceLock::new();
    TREE.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-{MAIN}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let archive = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["archive", MAIN])
            .args([
                "app",
                "scr",
                "ui",
                "fm",
                "lib",
                "kernel",
                "scripts",
                "Cargo.toml",
            ])
            .output()
            .expect("git runs");
        assert!(
            archive.status.success(),
            "git archive {MAIN} failed: {}",
            String::from_utf8_lossy(&archive.stderr)
        );
        let mut tar = Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(&dir)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("tar runs");
        std::io::Write::write_all(&mut tar.stdin.take().unwrap(), &archive.stdout).unwrap();
        assert!(tar.wait().unwrap().success(), "tar failed");
        dir.join("app")
    })
}

/// The app of the tree whose `app` directory is `source`, with no library.
pub fn plain_from(source: &Path, name: &str) -> &'static Built {
    Box::leak(Box::new(build_from(source, name, None, &[], &[])))
}

/// The app of the tree whose `app` directory is `source`, around `library` as the stages that end
/// at `ends`: the build the shell tests compare the branch against.
pub fn staged_from(source: &Path, name: &str, library: &[u8], ends: &[usize]) -> &'static Built {
    let stages = named_stages(ends);
    Box::leak(Box::new(build_from(
        source,
        name,
        Some(library),
        &stages,
        &[],
    )))
}

/// The app with the fm test library after its code pages.
pub fn with_library() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| build("pages-library", Some(&crate::fixture::library())))
}

/// The app around `library` with the stage list `stages` beside it, in a build directory of its
/// own named `name`.
pub fn staged_with(name: &str, library: &[u8], stages: &[mcui::Stage]) -> &'static Built {
    Box::leak(Box::new(build_with(name, Some(library), stages, &[])))
}

/// `staged_with` for the stages `s0`, `s1` and so on that end at `ends`, the last being the
/// library's count.
pub fn staged(name: &str, library: &[u8], ends: &[usize]) -> &'static Built {
    staged_with(name, library, &named_stages(ends))
}

/// The stages `s0`, `s1` and so on that end at `ends`.
fn named_stages(ends: &[usize]) -> Vec<mcui::Stage> {
    ends.iter()
        .enumerate()
        .map(|(i, &end)| mcui::Stage {
            name: format!("s{i}"),
            title: format!("Stage {i} up to {end}"),
            end,
        })
        .collect()
}

/// The app around `library` with no stage list, in a build directory of its own named `name`.
pub fn around(name: &str, library: &[u8]) -> &'static Built {
    Box::leak(Box::new(build(&format!("pages-{name}"), Some(library))))
}

/// The app built as if the workspace version were `version`, with no library.
pub fn versioned(version: &str) -> &'static Built {
    Box::leak(Box::new(build_with(
        &format!("pages-version-{version}"),
        None,
        &[],
        &[&format!("VERSION={version}")],
    )))
}

/// The app with a shipped library from `MC_LIBRARIES`, after its code pages.
pub fn shipped(name: &str) -> &'static Built {
    static BUILT: std::sync::Mutex<Vec<(String, &'static Built)>> =
        std::sync::Mutex::new(Vec::new());
    let mut all = BUILT.lock().unwrap();
    if let Some((_, built)) = all.iter().find(|(n, _)| n == name) {
        return built;
    }
    let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
    let bytes = std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
    let built: &'static Built = Box::leak(Box::new(build(&format!("pages-{name}"), Some(&bytes))));
    all.push((name.to_string(), built));
    built
}

/// The app around ring from `MC_LIBRARIES` as the three stages of the shipped list: prop, pred and
/// ring, each ending where its library does.
pub fn shipped_staged() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let dir = std::env::var("MC_LIBRARIES").expect("MC_LIBRARIES names the built libraries");
        let read =
            |name: &str| std::fs::read(format!("{dir}/{name}.mclib")).expect("library reads");
        let end = |bytes: &[u8]| usize::from(u16::from_le_bytes([bytes[14], bytes[15]]));
        let ring = read("ring");
        let stages: Vec<mcui::Stage> = [
            ("prop", "Propositional calculus"),
            ("pred", "Predicate calculus with equality"),
            ("ring", "Ring theory"),
        ]
        .map(|(name, title)| mcui::Stage {
            name: name.into(),
            title: title.into(),
            end: end(&read(name)),
        })
        .to_vec();
        staged_with("pages-ring-staged", &ring, &stages)
    })
}

/// The app with `bytes` as its library under a name of its own, for a library the reader or fm
/// refuses.
pub fn refused(name: &str, bytes: &[u8]) -> &'static Built {
    Box::leak(Box::new(build(
        &format!("pages-refused-{name}"),
        Some(bytes),
    )))
}
