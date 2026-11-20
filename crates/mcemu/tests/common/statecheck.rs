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
pub const STAT_VARS: (u16, usize) = (0x8A3A, 531);
pub const PREV_DATA: (u16, usize) = (0x9198, 283);

pub fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_add(STEP.wrapping_mul(i as u8)))
        .collect()
}

/// Contents with no two neighbouring bytes alike, so a restore from the wrong offset differs.
pub fn varied(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed ^ (i as u8).wrapping_mul(7).wrapping_add((i / 5) as u8))
        .collect()
}

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

/// A calculator running the probe: the stub OS or the reference OS.
#[derive(Clone, Copy, PartialEq)]
pub enum Scripts {
    None,
    Ram,
    Archived,
}

pub struct Rig {
    pub os: RealOs,
    lab: PathBuf,
    pub real: bool,
    /// Bank B as the OS had it before the app was started.
    home: Vec<u8>,
    /// An auto power down has run: the OS has put the screen in saveSScreen.
    pub apd: bool,
    /// The image makes script AppVars.
    pub scripts: Scripts,
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
            apd: false,
            scripts: Scripts::None,
        };
        rig.os.run_ms(200);
        rig
    }

    /// A rig over a calculator that is already running the app.
    pub fn from_os(mut os: RealOs, build: &Build) -> Rig {
        let home = os.m.ram_page(1).to_vec();
        Rig {
            os,
            lab: build.lab.clone(),
            real: true,
            home,
            apd: false,
            scripts: Scripts::None,
        }
    }

    /// The probe started from the APPS menu of the reference OS, where it is entry 2.
    pub fn real(build: &Build) -> Rig {
        Rig::real_with(build, |_| {})
    }

    /// The same, with `before` run on the booted OS first, and the image one that makes scripts.
    pub fn real_with(build: &Build, before: impl FnOnce(&mut Machine)) -> Rig {
        let mut os = RealOs::boot(&build.app);
        before(&mut os.m);
        let home = os.m.ram_page(1).to_vec();
        os.tap(realos::KEY_APPS, 150, 600);
        os.tap(realos::KEY_2, 150, 300);
        os.run_ms(1500);
        Rig {
            os,
            lab: build.lab.clone(),
            real: true,
            home,
            apd: false,
            scripts: Scripts::None,
        }
    }

    pub fn home_copy(&self) -> Vec<u8> {
        self.home.clone()
    }

    /// Every byte of bank B that differs from `home` and is not in a region the probe writes, its
    /// scratch, saveSScreen after a power down, or a run the OS or the probe's OS calls change.
    pub fn strays(&mut self) -> Vec<u16> {
        let now = self.os.m.ram_page(1).to_vec();
        let scratch = (self.label("S_BASE"), usize::from(self.label("S_SIZE")));
        let mut spans: Vec<(u16, usize)> = self.regions().iter().map(|r| (r.1, r.2)).collect();
        spans.push(scratch);
        spans.extend(OS_RUNS.iter().chain(PROBE_RUNS));
        match self.scripts {
            Scripts::None => {}
            Scripts::Ram => spans.extend(SCRIPT_RAM_RUNS),
            Scripts::Archived => spans.extend(SCRIPT_ARC_RUNS),
        }
        if self.apd {
            spans.push((SAVE_SCREEN, 768));
        }
        (0..now.len())
            .filter(|&i| now[i] != self.home[i])
            .map(|i| (RAM_BASE + i) as u16)
            .filter(|&a| {
                !spans
                    .iter()
                    .any(|&(s, l)| a >= s && usize::from(a) < usize::from(s) + l)
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
        let strays = self.strays();
        if std::env::var_os("STATECHECK_LEARN").is_some() {
            println!("LEARN {when}: {}", runs(&strays));
            return;
        }
        assert!(
            strays.is_empty(),
            "{when}: bytes outside the allowed runs changed: {}",
            runs(&strays)
        );
        self.assert_owner_data(when);
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
        self.os.tap(key, 150, 300);
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
        let mut pgm = b"P5\n96 64\n255\n".to_vec();
        pgm.extend(lcd.iter().map(|&p| if p { 0 } else { 255 }));
        let file = format!("statecheck-{}.pgm", name.replace([' ', ','], "-"));
        let _ = std::fs::write(Path::new(env!("CARGO_TARGET_TMPDIR")).join(file), pgm);
    }
}

pub fn start_screen(ver: &str, flags: u8, on: &str, off: &str, stale: bool) -> Vec<String> {
    [
        "statechk v1".to_string(),
        format!("ver {ver}"),
        format!("f33 {flags:02X}"),
        format!("ONSCRPT {on}"),
        format!("OFFSCRPT {off}"),
        if stale { "fill ok stale" } else { "fill ok" }.to_string(),
        "QUIT is a key".to_string(),
        "CLEAR exits".to_string(),
    ]
    .to_vec()
}

/// The lines for vars, stack and appd, a changed region as `@offset xcount`, then stat and ssc.
pub struct Report {
    pub vars: String,
    pub stack: String,
    pub appd: String,
    pub stat: &'static str,
    pub ssc: &'static str,
}

impl Report {
    pub fn ok() -> Report {
        Report {
            vars: "ok".into(),
            stack: "ok".into(),
            appd: "ok".into(),
            stat: "ok",
            ssc: "same",
        }
    }
}

pub fn key_screen(count: u16, code: u8, flags: u8, r: &Report) -> Vec<String> {
    [
        format!("keys {count:05} k={code:02X}"),
        format!("vars {}", r.vars),
        format!("stack {}", r.stack),
        format!("appd {}", r.appd),
        format!("stat {}", r.stat),
        format!("ssc {}", r.ssc),
        format!("f33 {flags:02X}"),
        "CLEAR exits".to_string(),
    ]
    .to_vec()
}

/// A changed region as the key screen prints it: the first offset in hex, then the count.
pub fn changed(first: usize, count: usize) -> String {
    format!("@{first:02X} x{count:03}")
}

/// The addresses as `ADDR+length` runs.
pub fn runs(addrs: &[u16]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < addrs.len() {
        let mut j = i;
        while j + 1 < addrs.len() && addrs[j + 1] == addrs[j] + 1 {
            j += 1;
        }
        out.push(format!("{:04X}+{}", addrs[i], j - i + 1));
        i = j + 1;
    }
    out.join(" ")
}

/// saveSScreen: the whole of it, 768 bytes.
pub const SAVE_SCREEN: u16 = 0x86EC;
include!("statecheck_runs.rs");

// What the reference OS tests share.

pub const SCAN_2ND: u8 = 0x36;
pub const SCAN_ALPHA: u8 = 0x30;
pub const SCAN_5: u8 = 0x1B;
pub const POWER_DOWN: u8 = 0x10;
pub const VER: &str = "02 2B";
/// The `_GetKey` code of 5.
pub const CODE_5: u8 = 0x93;
/// Bit 3 of iy+33h, which a tap of ON leaves set.
pub const ON_SEEN: u8 = 0x08;

/// The probe on the reference OS with patterns in statVars and prevData, which it must not touch.
pub fn probe() -> Rig {
    let (stat, prev) = (pattern(STAT_VARS.1, 0x33), pattern(PREV_DATA.1, 0x66));
    Rig::real_with(&assemble(PROBE, &[]), |m| {
        let page = m.ram_page(1);
        page[usize::from(STAT_VARS.0) - RAM_BASE..][..stat.len()].copy_from_slice(&stat);
        page[usize::from(PREV_DATA.0) - RAM_BASE..][..prev.len()].copy_from_slice(&prev);
    })
}

pub fn with_scripts(defines: &[&str]) -> Rig {
    let mut rig = Rig::real(&assemble(WITH_SCRIPTS, defines));
    rig.scripts = if defines.contains(&"ARCHIVE_SCRIPTS") {
        Scripts::Archived
    } else {
        Scripts::Ram
    };
    rig
}

pub fn flags(rig: &mut Rig) -> u8 {
    rig.peek(SCRIPT_FLAGS as u16, 1)[0]
}

pub fn set_flags(rig: &mut Rig, bits: u8) {
    let now = flags(rig);
    rig.poke(SCRIPT_FLAGS as u16, &[now | bits]);
}

/// The key screen after `count` keys, the last with `code`, and `report`.
pub fn assert_keys(rig: &mut Rig, name: &str, count: u16, code: u8, report: &Report) {
    // Bit 3 of iy+33h is set while the OS waits in _GetKey, and clear when the probe reads it.
    let f = flags(rig) & !ON_SEEN;
    assert_keys_with(rig, name, count, code, f, report);
}

/// The same with the flag byte the screen shows given.
pub fn assert_keys_with(rig: &mut Rig, name: &str, count: u16, code: u8, f: u8, report: &Report) {
    rig.assert_screen(name, &key_screen(count, code, f, report));
    assert_eq!(rig.keys(), count, "{name}: the probe's count");
}

/// Runs the OS until auto power down fires, wakes it with ON and lets it settle. The OS must
/// have put the whole screen in saveSScreen.
pub fn power_cycle(rig: &mut Rig) {
    let lcd = rig.os.packed_screen();
    let mut seconds = 0;
    while rig.os.m.in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        rig.os.run_ms(5_000);
        seconds += 5;
    }
    assert_eq!(
        rig.peek(SAVE_SCREEN, 768),
        lcd,
        "the power down left the screen's pixels in saveSScreen"
    );
    rig.apd = true;
    rig.tap(mcemu::Machine::KEY_ON);
    rig.os.run_ms(3000);
    assert_eq!(
        rig.os.m.in_port(3) & POWER_DOWN,
        0,
        "ON woke the calculator"
    );
}
