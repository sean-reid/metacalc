//! What the statechk tests share: assembling the probe, a machine to run it on, the patterns it
//! fills, and the screens it draws. The tests write the figures out; they are not read from the
//! probe's source.

#![allow(dead_code)]

use super::realos::{self, RealOs};
use mcemu::{Machine, STUB_ROM};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const RAM_BASE: usize = 0x8000;
pub const STEP: u8 = 37;
/// The stub's base page for an app, the one the reference OS uses too.
pub const STUB_BASE: u8 = 0x69;
/// iy+33h: bit 0 gates ONSCRPT and bit 1 OFFSCRPT.
pub const SCRIPT_FLAGS: usize = 0x89F0 + 0x33;
pub const MARK: u16 = 0xA800;
/// The OS's text shadow, and the regions the probe writes besides SCR_VARS and SCR_STACK: name,
/// address, length and the first byte of the pattern. SCR_VARS and SCR_STACK come from the labels.
pub const TEXT_SHADOW: u16 = 0x8508;
pub const OTHERS: [(&str, u16, usize, u8); 3] = [
    ("appData", 0x8000, 256, 0x44),
    ("textShadow", TEXT_SHADOW, 128, 0x11),
    ("saveSScreen tail", 0x86EC + 768 - 128, 128, 0x88),
];
pub const SCR_VARS: u16 = 0x90D3;
pub const SCR_STACK: u16 = 0x966E;
pub const APP_DATA: u16 = 0x8000;
/// The regions the probe restores and the labels of their saved copies.
pub const SAVES: [(&str, &str); 4] = [
    ("SCR_VARS", "S_SAVE_VARS"),
    ("SCR_STACK", "S_SAVE_STACK"),
    ("appData", "S_SAVE_DATA"),
    ("textShadow", "S_SAVE_TEXT"),
];

pub fn region_len(name: &str) -> usize {
    match name {
        "SCR_VARS" => 108,
        "SCR_STACK" | "textShadow" => 128,
        _ => 256,
    }
}

pub const STAT_VARS: (u16, usize) = (0x8A3A, 531);
pub const PREV_DATA: (u16, usize) = (0x9198, 283);

pub fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_add(STEP.wrapping_mul(i as u8)))
        .collect()
}

pub use super::realos::{Watch, assert_blank_home, varied};

pub struct Build {
    pub app: Vec<u8>,
    pub lab: PathBuf,
}

/// Assembles `source` (from the crate's root) with `defines` once per set, into a file of its
/// own, and returns the app and its label file.
pub fn assemble(source: &str, defines: &[&str]) -> Build {
    type Built = (Vec<u8>, PathBuf);
    static BUILDS: Mutex<Option<HashMap<String, Built>>> = Mutex::new(None);
    let stem = Path::new(source).file_stem().unwrap().to_string_lossy();
    let name = format!("{stem}{}", defines.concat());
    let mut builds = BUILDS.lock().unwrap_or_else(|e| e.into_inner());
    let (app, lab) = builds
        .get_or_insert_with(HashMap::new)
        .entry(name.clone())
        .or_insert_with(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.8xk"));
            let run = std::process::Command::new("sh")
                .arg(root.join("../../scripts/spasm.sh"))
                .args(defines.iter().map(|d| format!("-D{d}")))
                .args(["-L", "-I"])
                .arg(root.join("../../app/src"))
                .arg("-I")
                .arg(root.join("../../probe"))
                .arg(root.join(source))
                .arg(&out)
                .output()
                .expect("the assembler runs");
            assert!(
                run.status.success(),
                "spasm: {}",
                String::from_utf8_lossy(&run.stdout)
            );
            (std::fs::read(&out).unwrap(), out.with_extension("lab"))
        })
        .clone();
    Build { app, lab }
}

pub const PROBE: &str = "../../probe/statecheck.z80";
pub const WITH_SCRIPTS: &str = "tests/routines/statecheck_scripts.z80";

/// One thing done to a calculator after the app started, so a control image can be given the
/// same.
#[derive(Clone)]
pub enum Act {
    Tap(u8, u32, u32),
    Press(u8),
    Release(u8),
    Run(u32),
    Poke(u16, Vec<u8>),
}

/// A calculator running the probe: the stub OS or the reference OS.
pub struct Rig {
    pub os: RealOs,
    lab: PathBuf,
    pub real: bool,
    /// Bank B as the OS had it before the app was started.
    home: Vec<u8>,
    /// Everything done since the app started, and what was in RAM before it.
    acts: Vec<Act>,
    before: Vec<(u16, Vec<u8>)>,
    defines: Vec<String>,
    /// The control image given the same acts, made when the first stray check needs it.
    control: Option<Box<Control>>,
    /// The screen the OS put in saveSScreen at an auto power down.
    pub apd_pixels: Option<Vec<u8>>,
    /// The four saved copies as the probe made them at its start.
    saves0: Vec<Vec<u8>>,
}

impl Rig {
    /// The probe on the stub OS, as launched. `before` writes RAM before the app is installed.
    pub fn stub(build: &Build, before: impl FnOnce(&mut Machine)) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        before(&mut m);
        m.install_app(&build.app, STUB_BASE)
            .expect("the app installs");
        m.launch(STUB_BASE).expect("the app launches");
        let mut rig = Rig {
            os: RealOs { m },
            lab: build.lab.clone(),
            real: false,
            home: Vec::new(),
            acts: Vec::new(),
            before: Vec::new(),
            defines: Vec::new(),
            control: None,
            apd_pixels: None,
            saves0: Vec::new(),
        };
        rig.os.run_ms(200);
        rig
    }

    /// The probe started from the APPS menu of the reference OS, where it is entry 2, with
    /// `before` written to RAM first. `defines` are the ones the image was assembled with.
    pub fn real_with(build: &Build, before: &[(u16, Vec<u8>)], defines: &[&str]) -> Rig {
        let mut os = RealOs::boot(&build.app);
        poke_all(&mut os.m, before);
        let home = os.m.ram_page(1).to_vec();
        launch_entry_2(&mut os);
        let mut rig = Rig {
            os,
            lab: build.lab.clone(),
            real: true,
            home,
            acts: Vec::new(),
            before: before.to_vec(),
            defines: defines.iter().map(|d| d.to_string()).collect(),
            control: None,
            apd_pixels: None,
            saves0: Vec::new(),
        };
        rig.saves0 = SAVES
            .iter()
            .map(|(region, save)| rig.peek(rig.label(save), region_len(region)))
            .collect();
        rig
    }

    /// Runs the OS for `ms`, which a control image is given too.
    pub fn run_ms(&mut self, ms: u32) {
        self.acts.push(Act::Run(ms));
        self.os.run_ms(ms);
    }

    pub fn press(&mut self, key: u8) {
        self.acts.push(Act::Press(key));
        self.os.m.press_key(key);
    }

    pub fn release(&mut self, key: u8) {
        self.acts.push(Act::Release(key));
        self.os.m.release_key(key);
    }

    pub fn tap_for(&mut self, key: u8, down_ms: u32, up_ms: u32) {
        self.acts.push(Act::Tap(key, down_ms, up_ms));
        self.os.tap(key, down_ms, up_ms);
    }

    /// Bank B bytes the probe left that differ from before the app started and are not in a
    /// region it writes, its scratch, a run its own OS calls change, the pixels an auto power
    /// down saved, or something the control image did to the same byte in the same scenario.
    pub fn strays(&mut self) -> Vec<u16> {
        self.stray_values().iter().map(|s| s.0).collect()
    }

    /// The same with the value the probe's run holds and the control's, if it changed the byte.
    fn stray_values(&mut self) -> Vec<(u16, u8, Option<u8>)> {
        let (now, control) = (self.os.m.ram_page(1).to_vec(), self.control_diff());
        let scratch = (self.label("S_BASE"), usize::from(self.label("S_SIZE")));
        let mut spans: Vec<(u16, usize)> = self.regions().iter().map(|r| (r.1, r.2)).collect();
        spans.push(scratch);
        spans.extend(PROBE_RUNS.iter().chain(TIMER_RUNS));
        (0..now.len())
            .filter(|&i| now[i] != self.home[i])
            .filter(|&i| control[i] != Some(now[i]))
            .map(|i| ((RAM_BASE + i) as u16, now[i], control[i]))
            .filter(|&(a, now, _)| {
                let in_span = spans
                    .iter()
                    .any(|&(s, l)| a >= s && usize::from(a) < usize::from(s) + l);
                let saved = self.apd_pixels.as_ref().is_some_and(|p| {
                    a.checked_sub(SAVE_SCREEN)
                        .and_then(|off| p.get(usize::from(off)))
                        .is_some_and(|&b| b == now)
                });
                !in_span && !saved
            })
            .collect()
    }

    /// statVars and prevData hold every byte they held before the app started.
    pub fn assert_owner_data(&mut self, when: &str) {
        for (name, (addr, len)) in [("statVars", STAT_VARS), ("prevData", PREV_DATA)] {
            let at = usize::from(addr) - RAM_BASE;
            let want = self.home[at..at + len].to_vec();
            assert_eq!(self.peek(addr, len), want, "{name} {when}");
        }
    }

    /// No byte outside the allowed runs differs from before the app started.
    pub fn assert_confined(&mut self, when: &str) {
        if std::env::var_os("STATECHECK_LEARN").is_some() {
            assert!(
                std::env::var_os("CI").is_none(),
                "STATECHECK_LEARN checks nothing"
            );
            let values: Vec<String> = self
                .stray_values()
                .iter()
                .map(|(a, now, ctl)| format!("{a:04X}={now:02X}/{ctl:02X?}"))
                .collect();
            println!("LEARN {when}: {}", values.join(" "));
            let control = self.control_diff();
            let changed: Vec<u16> = (0..control.len())
                .filter(|&i| control[i].is_some())
                .map(|i| (RAM_BASE + i) as u16)
                .collect();
            println!("CONTROL {when}: {}", runs(&changed));
            return;
        }
        let strays = self.strays();
        println!("{when}: {} bytes differ from the control", strays.len());
        assert!(
            strays.is_empty(),
            "{when}: bytes outside the allowed runs changed: {}",
            runs(&strays)
        );
        self.assert_owner_data(when);
    }

    /// The address of a restored region.
    pub fn addr(&self, region: &str) -> u16 {
        match region {
            "textShadow" => TEXT_SHADOW,
            "appData" => APP_DATA,
            n => self.label(n),
        }
    }

    /// `len` bytes at `addr` as they were before the app started.
    pub fn home_at(&self, addr: u16, len: usize) -> Vec<u8> {
        let at = usize::from(addr) - RAM_BASE;
        self.home[at..at + len].to_vec()
    }

    pub fn label(&self, name: &str) -> u16 {
        realos::label_in(&self.lab, name)
    }

    pub fn peek(&mut self, addr: u16, len: usize) -> Vec<u8> {
        let at = usize::from(addr) - RAM_BASE;
        self.os.m.ram_page(1)[at..at + len].to_vec()
    }

    /// Writes RAM as a test does, and into the baseline too, so the check for stray writes does
    /// not count the test's own.
    pub fn poke(&mut self, addr: u16, bytes: &[u8]) {
        let at = usize::from(addr) - RAM_BASE;
        self.os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
        if self.real {
            self.acts.push(Act::Poke(addr, bytes.to_vec()));
        }
        if !self.home.is_empty() {
            self.home[at..at + bytes.len()].copy_from_slice(bytes);
        }
    }

    /// A byte or a word of the probe's scratch, by label.
    pub fn byte(&mut self, name: &str) -> u8 {
        self.peek(self.label(name), 1)[0]
    }

    pub fn word(&mut self, name: &str) -> u16 {
        let b = self.peek(self.label(name), 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// The regions the probe writes: name, address, length, seed.
    pub fn regions(&self) -> Vec<(String, u16, usize, u8)> {
        let size = |n: &str| usize::from(self.label(&format!("{n}_SIZE")));
        let mut all = vec![
            (
                "SCR_VARS".to_string(),
                self.label("SCR_VARS"),
                size("SCR_VARS"),
                0x77,
            ),
            (
                "SCR_STACK".to_string(),
                self.label("SCR_STACK"),
                size("SCR_STACK"),
                0x22,
            ),
        ];
        all.extend(OTHERS.map(|(n, a, l, s)| (n.to_string(), a, l, s)));
        all
    }

    /// Every region holds its pattern, textShadow excepted on the reference OS, where the probe's own text lands in it.
    pub fn assert_patterns(&mut self, when: &str) {
        for (name, addr, len, seed) in self.regions() {
            if self.real && name == "textShadow" {
                continue;
            }
            assert_eq!(self.peek(addr, len), pattern(len, seed), "{name} {when}");
        }
    }

    pub fn tap(&mut self, key: u8) {
        self.tap_for(key, 150, 300);
    }

    pub fn keys(&mut self) -> u16 {
        self.word("S_KEYS")
    }

    /// The text the OS holds for the eight rows, one string per row, trailing spaces cut.
    pub fn text(&mut self) -> Vec<String> {
        self.peek(TEXT_SHADOW, 128)
            .chunks(16)
            .map(|row| {
                row.iter()
                    .map(|&c| {
                        if (0x20..0x7F).contains(&c) {
                            c as char
                        } else {
                            ' '
                        }
                    })
                    .collect::<String>()
            })
            .collect()
    }

    /// The screen is `rows`: each row's characters are in the OS's text and the LCD shows lit
    /// pixels in their columns and none past them. The OS does not clear the text past a row's
    /// end, so the LCD tells where it stops.
    pub fn assert_screen(&mut self, name: &str, rows: &[String]) {
        assert_eq!(rows.len(), 8, "{name}: eight rows");
        let (text, lcd) = (self.text(), self.os.screen());
        self.dump(name, &lcd);
        for (r, want) in rows.iter().enumerate() {
            assert!(want.len() <= 16, "{name}: row {r} fits 16 characters");
            assert!(
                text[r].starts_with(want.as_str()),
                "{name}: row {r} is {:?}, want {want:?}",
                text[r]
            );
            let lit = |cols: std::ops::Range<usize>| {
                (r * 8..r * 8 + 8).any(|y| {
                    lcd[y * 96 + cols.start..y * 96 + cols.end]
                        .iter()
                        .any(|&p| p)
                })
            };
            assert_eq!(
                lit(0..want.len() * 6),
                !want.is_empty(),
                "{name}: row {r} shows {want:?}"
            );
            assert!(
                !lit(want.len() * 6..96),
                "{name}: row {r} shows more than {want:?}"
            );
        }
    }

    /// Writes the LCD as a PGM under the target tree for looking at.
    pub fn dump(&self, name: &str, lcd: &[bool]) {
        let file = format!("statecheck-{}.pgm", name.replace([' ', ','], "-"));
        realos::write_pgm(&file, lcd);
    }
}

impl Watch for Rig {
    fn lcd(&mut self) -> Vec<bool> {
        self.os.screen()
    }

    fn wait(&mut self, ms: u32) {
        self.run_ms(ms);
    }

    fn dump(&self, name: &str, lcd: &[bool]) {
        Rig::dump(self, name, lcd);
    }
}

/// saveSScreen: the whole of it, 768 bytes.
pub const SAVE_SCREEN: u16 = 0x86EC;
include!("statecheck_runs.rs");
include!("statecheck_screens.rs");
include!("statecheck_control.rs");
include!("statecheck_real.rs");
