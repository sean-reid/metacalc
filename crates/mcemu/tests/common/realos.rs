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
/// The bit of port 3 that is set while the calculator is powered down.
const POWERED_DOWN: u8 = 0x10;

pub fn rom() -> Option<Vec<u8>> {
    let path = Path::new(&std::env::var_os("HOME")?).join("ti84-rom/84PSE_v243.rom");
    std::fs::read(path).ok()
}

pub struct RealOs {
    pub m: Machine,
}

impl RealOs {
    /// Boots the reference OS with `app` installed on `BASE`, turns it on, and returns at the
    /// home screen. The app has no launch limit, as after the OS's own install.
    pub fn boot(app: &[u8]) -> RealOs {
        RealOs::boot_apps(&[(app, BASE)])
    }

    /// `boot` with several apps installed, each on its own base page.
    pub fn boot_apps(apps: &[(&[u8], u8)]) -> RealOs {
        let Some(rom) = rom() else {
            panic!(
                "the reference ROM is not at $HOME/ti84-rom/84PSE_v243.rom, which this test needs"
            );
        };
        let mut m = Machine::new_os(&rom).expect("the reference ROM boots");
        for &(app, base) in apps {
            m.install_app(app, base).expect("the app installs");
        }
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

    /// Runs the OS until its auto power down fires, then wakes it with ON.
    pub fn power_down_and_wake(&mut self) {
        let mut seconds = 0;
        while self.m.in_port(3) & POWERED_DOWN == 0 {
            assert!(seconds < 600, "no auto power down in 600 s");
            self.run_ms(5_000);
            seconds += 5;
        }
        self.tap(Machine::KEY_ON, 150, 3000);
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
            "the app did not draw its first screen"
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

/// Contents with no two neighbouring bytes alike, so a restore from the wrong offset differs.
pub fn varied(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed ^ (i as u8).wrapping_mul(7).wrapping_add((i / 5) as u8))
        .collect()
}

/// A machine whose screen `assert_blank_home` can watch.
pub trait Watch {
    fn lcd(&mut self) -> Vec<bool>;
    fn wait(&mut self, ms: u32);
    fn dump(&self, name: &str, lcd: &[bool]);
}

impl Watch for RealOs {
    fn lcd(&mut self) -> Vec<bool> {
        self.screen()
    }

    fn wait(&mut self, ms: u32) {
        self.run_ms(ms);
    }

    fn dump(&self, name: &str, lcd: &[bool]) {
        write_pgm(&format!("exit-{}.pgm", name.replace([' ', ','], "-")), lcd);
    }
}

/// Writes the LCD as a PGM under the target tree for looking at.
pub fn write_pgm(file: &str, lcd: &[bool]) {
    let mut pgm = b"P5\n96 64\n255\n".to_vec();
    pgm.extend(lcd.iter().map(|&p| if p { 0 } else { 255 }));
    let _ = std::fs::write(Path::new(env!("CARGO_TARGET_TMPDIR")).join(file), pgm);
}

/// The home screen is blank but for its cursor in the first cell, which shows at least once
/// in a second and a half, since it blinks.
pub fn assert_blank_home<W: Watch>(w: &mut W, name: &str) {
    let mut cursor = false;
    for _ in 0..30 {
        let lcd = w.lcd();
        let lit: Vec<usize> = (0..lcd.len()).filter(|&i| lcd[i]).collect();
        let blank = lit.iter().all(|&i| i / 96 < 8 && i % 96 < 6);
        if !blank {
            w.dump(&format!("{name}-bad"), &lcd);
        }
        assert!(blank, "{name}: blank but for the cursor");
        cursor |= !lit.is_empty();
        if cursor {
            w.dump(name, &lcd);
        }
        w.wait(50);
    }
    assert!(cursor, "{name}: the home screen shows its cursor");
}

/// The library an app is built around as the model sees it: the name the build gives its one stage
/// when there is no stage list (the library file's name without its extension), the bytes, the
/// stages of the build's list (none for one stage) and the status table (none for every assertion
/// unchecked, which is what the app shows when the store has no table for the library).
pub struct Shown<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
    pub stages: Vec<mcui::Stage>,
    pub status: Vec<mcui::Status>,
}

impl<'a> Shown<'a> {
    pub fn new(name: &'a str, bytes: &'a [u8]) -> Shown<'a> {
        Shown {
            name,
            bytes,
            stages: Vec::new(),
            status: Vec::new(),
        }
    }
}

/// The model's interface for an app built around `library`, open on the Libraries screen; `None`
/// is a build with no library.
pub fn libraries_ui(library: Option<&Shown>) -> mcui::Ui {
    let (libraries, stages, tables) = match library {
        None => (Vec::new(), Vec::new(), Vec::new()),
        Some(shown) => {
            let count = mcpack::library::decode(shown.bytes)
                .expect("the library decodes")
                .assertions
                .len();
            let status = if shown.status.is_empty() {
                vec![mcui::Status::Unchecked; count]
            } else {
                shown.status.clone()
            };
            let installed = mcui::Installed {
                name: shown.name.to_string(),
                bytes: shown.bytes.to_vec(),
            };
            (vec![installed], vec![shown.stages.clone()], vec![status])
        }
    };
    mcui::Ui::with_stages(
        libraries,
        stages,
        mcui::Statuses {
            libraries: tables,
            my_proofs: 0,
        },
    )
    .expect("the model loads")
}

/// What the model draws on the Libraries screen: `libraries_ui`'s frame.
pub fn libraries_model(library: Option<&Shown>) -> mcglyph::Bitmap {
    libraries_ui(library).frame()
}

/// `libraries_model` as lit pixels, row by row, as `RealOs::screen` gives them.
pub fn libraries_pixels(library: Option<&Shown>) -> Vec<bool> {
    let bitmap = libraries_model(library);
    (0..64)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| bitmap.get(x, y))
        .collect()
}

/// What the model's Notice draws for the error the app shows when a call refuses: the title
/// "Error" and "Status " with the status as two hex digits.
pub fn error_model(status: u8) -> mcglyph::Bitmap {
    mcui::notice_frame("Error", &[&format!("Status {status:02X}")])
}

/// `error_model` as lit pixels.
pub fn error_pixels(status: u8) -> Vec<bool> {
    let bitmap = error_model(status);
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

/// The label file spasm wrote for `assemble_routine(file)`, for `label_in`.
pub fn routine_labels(file: &str) -> PathBuf {
    assemble_routine(file);
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{file}.lab"))
}

fn assemble(file: &str) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{file}.8xk"));
    let wrapper = root.join("../../scripts/spasm.sh");
    let run = Command::new("sh")
        .arg(wrapper)
        .arg(format!("-DEXIT_MARK=${EXIT_MARK:04X}"))
        .arg("-L")
        .arg("-I")
        .arg(root.join("../../app/src"))
        .arg("-I")
        .arg(root.join("../../probe"))
        .arg("-I")
        .arg(root.join("../.."))
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
