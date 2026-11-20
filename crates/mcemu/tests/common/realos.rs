//! The reference TI-OS 2.43 image for tests that need the real OS. The image stays on the machine
//! that has it, under `$HOME/ti84-rom`. Every test that boots it is `#[ignore]`d with
//! `needs the reference ROM image`, and `RealOs::boot` fails with a message when the image is
//! missing.

#![allow(dead_code)]

use mcemu::{Machine, Stop};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

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
/// The byte the control app writes just before it hands the machine back: the first byte of
/// SCR_VARS in ram_map.inc, which crates/mcemu/tests/screen_state.rs measures to keep every byte
/// through leaving the app. The one definition: `assemble_routine` gives it to the assembler.
pub const EXIT_MARK: u16 = 0x90D3;
/// The OS flag byte holding `graphDraw`, bit 0, which the app sets as its last mark before it leaves.
pub const GRAPH_FLAGS: usize = OS_FLAGS + 3;

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

    /// `launch` for an app that waits in the OS's `_GetKey`, where port 6 reads the OS's page: the
    /// app is running when the screen is `start_screen`, the pixels it draws before it waits.
    pub fn launch_in_key_wait(&mut self, start_screen: &[bool]) {
        self.start_from_menu();
        self.run_ms(300);
        assert!(
            self.screen() == start_screen,
            "the app did not draw its start screen"
        );
    }

    pub fn start_from_menu(&mut self) {
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

    /// The screen's pixels packed 8 to a byte, row by row, the leftmost in bit 7: the 768 bytes the
    /// OS copies into saveSScreen at an auto power down.
    pub fn packed_screen(&mut self) -> Vec<u8> {
        self.screen()
            .chunks(8)
            .map(|px| px.iter().fold(0u8, |b, &lit| (b << 1) | u8::from(lit)))
            .collect()
    }

    /// The lit pixels of the screen, row by row.
    pub fn screen(&mut self) -> Vec<bool> {
        let lcd = self.m.lcd();
        (0..64).flat_map(|y| lcd.row(y).to_vec()).collect()
    }
}

/// What the model's canvas draws for the app's start screen: with no library the title bar and the
/// hint on row 3; with one the title bar, the label of its first assertion on row 1, that
/// assertion's conclusion on rows 2 to 6 and the hint on row 7. A status other than 0 is what the
/// screen shows as two hex digits on row 7 in place of everything under the title bar.
pub fn start_model(library: Option<&[u8]>, status: u8) -> mcglyph::Bitmap {
    use mcui::canvas::{Canvas, GUTTER, STATEMENT_WIDTH};
    use mcui::expr::Expr;
    use mcui::fold::{Folds, layout};
    let mut canvas = Canvas::default();
    canvas.title("metacalc", "");
    match (library, status) {
        (_, 1..) => canvas.text(1, 7, &format!("{status:02X}")),
        (Some(bytes), 0)
            if mcpack::library::decode(bytes)
                .expect("the library decodes")
                .assertions
                .is_empty() =>
        {
            panic!("a library with no assertion draws only a status: pass it")
        }
        (None, 0) => canvas.text(1, 3, "CLEAR to quit"),
        (Some(bytes), 0) => {
            let library = mcpack::library::decode(bytes).expect("the library decodes");
            let first = &library.assertions[0];
            canvas.text(1, 1, &first.label);
            let tokens = &first.conclusion;
            let expr = Expr::flat(vec![tokens[0].clone()], &tokens[1..]);
            let laid = layout(&expr, &Folds::none(&expr), STATEMENT_WIDTH);
            canvas.layout(&laid, GUTTER, 2, 2..7);
            canvas.text(1, 7, "CLEAR to quit")
        }
    };
    canvas.into_bitmap()
}

/// `start_model` as lit pixels, row by row, as `RealOs::screen` gives them.
pub fn start_pixels(library: Option<&[u8]>, status: u8) -> Vec<bool> {
    let bitmap = start_model(library, status);
    (0..64)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| bitmap.get(x, y))
        .collect()
}

/// Builds `make -C <dir> <args>` for an app with the assembler the environment names, into a
/// directory under the target tree, and returns the directory. A build runs once per process for
/// each directory name, so parallel tests never write one build directory at the same time.
pub fn make_app(dir: &str, name: &str, args: &[String]) -> PathBuf {
    static BUILT: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);
    let mut built = BUILT.lock().unwrap_or_else(|e| e.into_inner());
    built
        .get_or_insert_with(HashMap::new)
        .entry(name.to_string())
        .or_insert_with(|| build_app(dir, name, args))
        .clone()
}

fn build_app(dir: &str, name: &str, args: &[String]) -> PathBuf {
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

/// Assembles a test routine that is an app, `tests/routines/<file>`, and returns the app file. The
/// assembler runs once per process for each file.
pub fn assemble_routine(file: &str) -> Vec<u8> {
    static BUILT: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);
    let mut built = BUILT.lock().unwrap_or_else(|e| e.into_inner());
    built
        .get_or_insert_with(HashMap::new)
        .entry(file.to_string())
        .or_insert_with(|| assemble(file))
        .clone()
}

fn assemble(file: &str) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{file}.8xk"));
    let wrapper = root.join("../../scripts/spasm.sh");
    let run = Command::new("sh")
        .arg(wrapper)
        .arg(format!("-DEXIT_MARK=${EXIT_MARK:04X}"))
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
