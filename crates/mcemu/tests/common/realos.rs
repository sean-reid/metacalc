//! The reference TI-OS 2.43 image for tests that need the real OS. The image stays on the machine
//! that has it, under `$HOME/ti84-rom`. Every test that boots it is `#[ignore]`d with
//! `needs the reference ROM image`, and `RealOs::boot` fails with a message when the image is
//! missing.

#![allow(dead_code)]

use mcemu::{Machine, Stop};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The flash page the app is installed on, as the real OS enters it.
pub const BASE: u8 = 0x69;
/// A key's scancode, named for the ones the tests press.
pub const KEY_APPS: u8 = 0x27;
pub const KEY_2: u8 = 0x1A;
pub const KEY_CLEAR: u8 = 0x0F;
/// The OS flags and the byte holding `onInterrupt`, in RAM page 81h.
pub const OS_FLAGS: usize = 0x09F0;
pub const ON_FLAGS: usize = OS_FLAGS + 9;
pub const ON_INTERRUPT: u8 = 0x10;

fn rom() -> Option<Vec<u8>> {
    let path = Path::new(&std::env::var_os("HOME")?).join("ti84-rom/84PSE_v243.rom");
    std::fs::read(path).ok()
}

pub struct RealOs {
    pub m: Machine,
}

impl RealOs {
    /// Boots the reference OS with `app` installed on `BASE`, turns it on, and returns at the
    /// home screen.
    pub fn boot(app: &[u8]) -> RealOs {
        let Some(rom) = rom() else {
            panic!(
                "the reference ROM is not at $HOME/ti84-rom/84PSE_v243.rom, which this test needs"
            );
        };
        let mut m = Machine::new_os(&rom).expect("the reference ROM boots");
        m.install_app(app, BASE).expect("the app installs");
        let mut os = RealOs { m };
        os.run_ms(2000);
        os.tap(Machine::KEY_ON, 150, 250);
        os.run_ms(3000);
        os
    }

    pub fn run_ms(&mut self, ms: u32) {
        self.run_us(ms * 1000);
    }

    pub fn run_us(&mut self, us: u32) {
        let run = self.m.run_time(us);
        assert_eq!(run.stop, Stop::Limit, "the emulator faulted");
    }

    pub fn tap(&mut self, key: u8, down_ms: u32, up_ms: u32) {
        self.m.press_key(key);
        self.run_ms(down_ms);
        self.m.release_key(key);
        self.run_ms(up_ms);
    }

    /// Starts the app from the APPS menu, where it is the second entry after Finance.
    pub fn launch(&mut self) {
        self.start_from_menu();
        let page = self.m.in_port(0x06);
        assert_eq!(
            page, BASE,
            "expected the app to run on flash page {BASE:02X}"
        );
    }

    /// `launch` for an app that waits in the OS's `_GetKey`, where port 6 reads the OS's page.
    pub fn launch_in_key_wait(&mut self) {
        self.start_from_menu();
    }

    fn start_from_menu(&mut self) {
        self.tap(KEY_APPS, 150, 600);
        let menu = self.screen();
        let lit =
            |rows: std::ops::Range<usize>| menu[rows.start * 96..rows.end * 96].iter().any(|&p| p);
        assert!(
            lit(0..8) && lit(8..16) && lit(16..24) && !lit(24..64),
            "expected the APPS menu with two entries, Finance and the app"
        );
        self.tap(KEY_2, 150, 300);
    }

    pub fn ram(&mut self, offset: usize, len: usize) -> Vec<u8> {
        self.m.ram_page(1)[offset..offset + len].to_vec()
    }

    pub fn on_flags(&mut self) -> u8 {
        self.m.ram_page(1)[ON_FLAGS]
    }

    /// The lit pixels of the screen, row by row.
    pub fn screen(&mut self) -> Vec<bool> {
        let lcd = self.m.lcd();
        (0..64).flat_map(|y| lcd.row(y).to_vec()).collect()
    }
}

/// Builds `make -C <dir> <args>` for an app with the assembler the environment names, into a fresh
/// directory under the target tree, and returns the directory.
pub fn make_app(dir: &str, name: &str, args: &[String]) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(dir);
    let build = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&build).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let make = Command::new("make")
        .args(["-s", "-C"])
        .arg(&source)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", build.display()))
        .arg(format!("FONT={}/ui_font", env!("OUT_DIR")))
        .args(args)
        .output()
        .expect("make runs");
    assert!(
        make.status.success(),
        "make: {}",
        String::from_utf8_lossy(&make.stdout)
    );
    build
}

/// Assembles a test routine that is an app, `tests/routines/<file>`, and returns the app file.
pub fn assemble_routine(file: &str) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{file}.8xk"));
    let wrapper = root.join("../../scripts/spasm.sh");
    let run = Command::new("sh")
        .arg(wrapper)
        .arg("-I")
        .arg(root.join("../../app/src"))
        .arg(root.join("tests/routines").join(file))
        .arg(&out)
        .output()
        .expect("the assembler runs");
    assert!(
        run.status.success(),
        "spasm: {}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::read(out).unwrap()
}

/// The address of `name` in the label file spasm wrote next to the app.
pub fn label_in(lab: &Path, name: &str) -> u16 {
    std::fs::read_to_string(lab)
        .unwrap()
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}
