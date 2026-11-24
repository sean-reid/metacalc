//! Building the app and standing on one of its pages.

use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs, Shown, libraries_model, libraries_pixels};
use crate::ui_support::{pack, same_frame};
use mcemu::{Machine, Regs, STUB_ROM, Stop};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Where the driver and the callee's buffers sit, in RAM below the app's own regions.
pub const DRIVER: u16 = 0xA000;
pub const SCRATCH: u16 = 0xA800;
pub const STACK: u16 = 0xAF00;
const FLAGS: u16 = 0x89F0;
pub const UI_FRAME: u16 = 0x9340;
pub const UI_VARS: u16 = 0x86EC;
pub const FRAME_LEN: usize = 768;
/// The ranges a driver run writes: the driver, a callee's buffers, the parameter block, the
/// scratch, a stack and the strings of the frame test.
pub const DRIVER_RAM: [(u16, u16); 7] = [
    (0xA000, 0xA010),
    (0xA100, 0xA200),
    (0xA400, 0xA410),
    (0xA600, 0xA620),
    (0xA800, 0xA900),
    (0xAE00, 0xAF00),
    (0xB000, 0xB100),
];
const LIMIT: u32 = 100_000_000;
const BOOT: u32 = 3_000_000;

/// The pages' numbers as pages.inc lays them out, written here and not read from the app.
pub const DRAW_PAGE: u8 = BASE - 1;
pub const DATA_PAGE: u8 = BASE - 2;
/// The library's chunk 0 follows the three code pages.
pub const LIBRARY_PAGE: u8 = BASE - 3;

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
        assert!(archive.status.success(), "git archive {MAIN} failed");
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

/// The reference OS with the app started from the APPS menu, waiting in the OS's `_GetKey`, where
/// port 6 reads the OS's page and not the app's.
pub fn started_on_real_os(built: &'static Built) -> Machine {
    let mut os = RealOs::boot(&built.file);
    os.tap(KEY_APPS, 150, 600);
    os.tap(KEY_2, 150, 300);
    os.run_ms(500);
    os.m
}

/// Sends the entry page's far entry `stub` to the RAM routine at `hook` before it goes on: the
/// stub's last three bytes, its `jp far_pageN`, become `jp hook`. The hook ends with a jump to the
/// far routine itself.
pub fn hook_stub(m: &mut Machine, built: &Built, stub: &str, hook: u16) {
    let at = built.label(stub);
    let [lo, hi] = hook.to_le_bytes();
    m.load_flash(BASE, usize::from(at - 0x4000) + 4, &[0xC3, lo, hi]);
}

/// What a routine is called with.
#[derive(Clone, Copy, Default)]
pub struct In {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    /// The flags register.
    pub f: u8,
}

/// What it came back with, the T-states the driver's call and halt took included.
pub struct Done {
    pub regs: Regs,
    pub tstates: u32,
    pub page: u8,
}

impl Done {
    pub fn status(&self) -> u8 {
        (self.regs.af >> 8) as u8
    }
}

pub struct App {
    pub m: Machine,
    pub built: &'static Built,
    /// Where `call` puts its driver, its stack and the ranges it writes: `DRIVER_RAM` on the stub
    /// OS, a small place inside A800h to AA0Fh, which other real-OS tests write too, on the reference
    /// OS.
    pub driver: u16,
    pub stack: u16,
    pub scratch: Vec<(u16, u16)>,
}

impl App {
    /// The app installed, launched and run until it waits for a key, on the stub OS.
    pub fn boot(built: &'static Built) -> App {
        App::boot_with(built, |_| {})
    }

    /// `boot` with `setup` run on the launched machine before the app's first instruction: it may
    /// fill RAM or patch the installed pages.
    pub fn boot_with(built: &'static Built, setup: impl FnOnce(&mut Machine)) -> App {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&built.file, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        setup(&mut m);
        assert_eq!(m.run_app(BOOT).stop, Stop::Limit, "the app waits for a key");
        App::with(m, built)
    }

    pub fn with(m: Machine, built: &'static Built) -> App {
        App {
            m,
            built,
            driver: DRIVER,
            stack: STACK,
            scratch: DRIVER_RAM.to_vec(),
        }
    }

    /// `boot`, and the app has drawn the Libraries screen the model draws for its library: the frame
    /// it left is the model's, so it reached its key wait by its own path.
    pub fn start(built: &'static Built) -> App {
        let mut app = App::boot(built);
        let want = pack(&libraries_model(built.shown().as_ref()));
        same_frame("the Libraries screen", &app.frame(), &want);
        app
    }

    /// The app installed on the reference OS and started from the APPS menu, left at its key wait.
    pub fn on_real_os(built: &'static Built) -> App {
        let mut app = App::on_real_os_in(started_on_real_os(built), built);
        let screen = libraries_pixels(built.shown().as_ref());
        let lcd = app.m.lcd();
        let lit: Vec<bool> = (0..64)
            .flat_map(|y| (0..96).map(move |x| (x, y)))
            .map(|(x, y)| lcd.pixel(x, y))
            .collect();
        assert!(lit == screen, "the app has not reached its key wait");
        app
    }

    /// `m`, the reference OS with `built` waiting for a key, as an App with the driver and stack
    /// `call` and `timed` use there: a driver, its callee's buffer and a stack of 64 bytes, all in
    /// A800h to AA0Fh.
    pub fn on_real_os_in(m: Machine, built: &'static Built) -> App {
        let mut app = App::with(m, built);
        app.driver = 0xA800;
        app.stack = 0xAA00;
        app.scratch = vec![(0xA800, 0xA810), (0xA9C0, 0xAA00)];
        app
    }

    pub fn label(&self, name: &str) -> u16 {
        self.built.label(name)
    }

    pub fn frame(&mut self) -> Vec<u8> {
        self.m.read_ram(UI_FRAME, FRAME_LEN)
    }

    /// Calls `target` from the page numbered `page`, as port 6 reads there, by a driver in RAM that
    /// calls it and halts, with interrupts off, and puts port 6 back on the entry page.
    pub fn call(&mut self, page: u8, target: u16, input: In) -> Done {
        self.assert_scratch_is_free();
        let [lo, hi] = target.to_le_bytes();
        self.m
            .load_ram(self.driver, &[0xCD, lo, hi, 0x76, 0x18, 0xFD]);
        self.m.out_port(0x06, page);
        self.m.set_regs(&Regs {
            af: u16::from(input.a) << 8 | u16::from(input.f),
            bc: input.bc,
            de: input.de,
            hl: input.hl,
            ix: 0x1111,
            iy: FLAGS,
            sp: self.stack,
            pc: self.driver,
            af2: 0x2222,
            bc2: 0x3333,
            de2: 0x4444,
            hl2: 0x5555,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        let run = self.m.resume(LIMIT);
        assert!(
            matches!(run.stop, Stop::Limit | Stop::Halted),
            "the call to {target:04X} ended with {:?}",
            run.stop
        );
        let regs = self.m.regs();
        assert!(
            (self.driver + 3..=self.driver + 5).contains(&regs.pc),
            "the call to {target:04X} stopped at {:04X}",
            regs.pc
        );
        let done = Done {
            regs,
            tstates: run.tstates,
            page: self.m.in_port(0x06),
        };
        self.m.out_port(0x06, BASE);
        done
    }

    /// `call` for the reference OS, where the driver's HALT with interrupts off never ends a run:
    /// the driver jumps to the entry page's `screen_ret` instead, the run stops there, and the
    /// T-states are those from the driver's first instruction to it, less what the same driver takes
    /// to call a plain `ret`. Interrupts are off, so no handler's time is in it.
    pub fn timed(&mut self, page: u8, target: u16, input: In) -> Done {
        let overhead = self
            .run_timed(BASE, self.label("screen_ret"), In::default())
            .1;
        let (mut done, tstates) = self.run_timed(page, target, input);
        done.tstates = tstates - overhead;
        done
    }

    fn run_timed(&mut self, page: u8, target: u16, input: In) -> (Done, u32) {
        self.assert_scratch_is_free();
        let stop = self.label("screen_ret");
        let [lo, hi] = target.to_le_bytes();
        let [slo, shi] = stop.to_le_bytes();
        self.m
            .load_ram(self.driver, &[0xCD, lo, hi, 0xC3, slo, shi]);
        self.m.out_port(0x06, page);
        self.m.set_regs(&Regs {
            af: u16::from(input.a) << 8 | u16::from(input.f),
            bc: input.bc,
            de: input.de,
            hl: input.hl,
            ix: 0x1111,
            iy: FLAGS,
            sp: self.stack,
            pc: self.driver,
            af2: 0x2222,
            bc2: 0x3333,
            de2: 0x4444,
            hl2: 0x5555,
            i: 0xFF,
            iff1: 0,
            iff2: 0,
            im: 1,
        });
        let start = self.m.clock();
        let reached = self.m.run_to(BASE, stop, LIMIT);
        assert_eq!(reached, mcemu::Reached::Target, "the call to {target:04X}");
        let tstates = self.m.clock().wrapping_sub(start);
        let regs = self.m.regs();
        let done = Done {
            regs,
            tstates,
            page: self.m.in_port(0x06),
        };
        self.m.out_port(0x06, BASE);
        (done, tstates)
    }

    /// The RAM a driver run writes, `DRIVER_RAM`, lies clear of every region of the app's RAM map.
    fn assert_scratch_is_free(&self) {
        let regions = [
            ("UI_VARS", "UI_VARS_SIZE"),
            ("UI_FRAME", "UI_FRAME_SIZE"),
            ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
            ("FAR_VARS", "FAR_VARS_SIZE"),
            ("LIB_STATE", "LIB_STATE_SIZE"),
            ("APP_BUF", "APP_BUF_SIZE"),
            ("APP_SENTINEL", "APP_SENTINEL_SIZE"),
            ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
        ];
        for (base, size) in regions {
            let (low, len) = (self.label(base), self.label(size));
            for &(from, to) in &self.scratch {
                assert!(
                    to <= low || low + len <= from,
                    "the driver's RAM {from:04X} to {to:04X} overlaps {base} at {low:04X}"
                );
            }
        }
    }

    /// The entry page's far entry named `name`, called from the entry page.
    pub fn far(&mut self, name: &str, input: In) -> Done {
        let at = self.label(name);
        self.call(BASE, at, input)
    }
}
