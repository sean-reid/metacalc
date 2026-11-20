//! Building the app and standing on one of its pages.

use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs, start_model};
use crate::ui_support::{pack, same_frame};
use mcemu::{Machine, Regs, STUB_ROM, Stop};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Where the driver and the callee's buffers sit, in RAM below the app's own regions.
pub const DRIVER: u16 = 0xA000;
pub const SCRATCH: u16 = 0xA800;
const STACK: u16 = 0xAF00;
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
}

impl Built {
    pub fn label(&self, name: &str) -> u16 {
        self.labels
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(" = ")?;
                key.eq_ignore_ascii_case(name)
                    .then(|| u16::from_str_radix(&value[1..], 16).unwrap())
            })
            .unwrap_or_else(|| panic!("no label {name}"))
    }
}

/// `make -C app` into a directory of its own, with the library when there is one.
fn build(name: &str, library: Option<&[u8]>) -> Built {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let mut args = Vec::new();
    if let Some(bytes) = library {
        let path = dir.join("library.mclib");
        std::fs::write(&path, bytes).unwrap();
        args.push(format!("LIBRARY={}", path.display()));
    }
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app");
    let made = Command::new("make")
        .arg("-C")
        .arg(&source)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}/build", dir.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")))
        .args(&args)
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
    }
}

/// The app without a library.
pub fn plain() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| build("pages-plain", None))
}

/// The app with the fm test library after its code pages.
pub fn with_library() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| build("pages-library", Some(&crate::fixture::library())))
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

/// What a routine is called with.
#[derive(Clone, Copy, Default)]
pub struct In {
    pub a: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
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
}

impl App {
    /// The app installed, launched and run until it waits for a key, on the stub OS.
    pub fn boot(built: &'static Built) -> App {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&built.file, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        assert_eq!(m.run_app(BOOT).stop, Stop::Limit, "the app waits for a key");
        App { m, built }
    }

    /// `boot`, and the app has drawn the start screen the model draws for its library: the frame it
    /// left is the model's, so it reached its key wait by its own path.
    pub fn start(built: &'static Built) -> App {
        let mut app = App::boot(built);
        let want = pack(&start_model(built.library.as_deref(), 0));
        same_frame("the start screen", &app.frame(), &want);
        app
    }

    /// The app installed on the reference OS and started from the APPS menu, left at its key wait.
    pub fn on_real_os(built: &'static Built) -> App {
        App {
            m: started_on_real_os(built),
            built,
        }
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
        self.m.load_ram(DRIVER, &[0xCD, lo, hi, 0x76, 0x18, 0xFD]);
        self.m.out_port(0x06, page);
        self.m.set_regs(&Regs {
            af: u16::from(input.a) << 8,
            bc: input.bc,
            de: input.de,
            hl: input.hl,
            ix: 0x1111,
            iy: FLAGS,
            sp: STACK,
            pc: DRIVER,
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
            (DRIVER + 3..=DRIVER + 5).contains(&regs.pc),
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

    /// The RAM a driver run writes, `DRIVER_RAM`, lies clear of every region of the app's RAM map.
    fn assert_scratch_is_free(&self) {
        let regions = [
            ("UI_VARS", "UI_VARS_SIZE"),
            ("UI_FRAME", "UI_FRAME_SIZE"),
            ("KD_SCRATCH", "KD_SCRATCH_SIZE"),
            ("FAR_VARS", "FAR_VARS_SIZE"),
            ("LIB_STATE", "LIB_STATE_SIZE"),
            ("APP_BUF", "APP_BUF_SIZE"),
            ("FM_SCRATCH", "FM_SCRATCH_SIZE"),
        ];
        for (base, size) in regions {
            let (low, len) = (self.label(base), self.label(size));
            for (from, to) in DRIVER_RAM {
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
