//! Which regions of RAM keep their bytes while a Flash App waits in the reference OS's `_GetKey`:
//! each candidate home for the app's screen state is filled with a pattern and compared after
//! every kind of event.

#[path = "common/realos.rs"]
mod realos;

use mcemu::Machine;
use realos::*;
use std::fmt::Write as _;

const RAM_BASE: usize = 0x8000;
const RESULT: usize = 0xA800 - RAM_BASE;
const POWER_DOWN: u8 = 0x10;
const STEP: u8 = 37;
const SCAN_ALPHA: u8 = 0x30;
const SCAN_2ND: u8 = 0x36;
const SCAN_UP: u8 = 0x04;
const SCAN_DOWN: u8 = 0x01;
const SCAN_ENTER: u8 = 0x09;
const SCAN_LAST: u8 = 0x38;
const NO_KEY: [u8; 7] = [5, 6, 7, 8, 0x10, 0x18, 0x29];
const CONTRAST: usize = 0x8447 - RAM_BASE;
const APP_FLAGS: usize = 0x09F0 + 13;
const BATTERY: [&str; 13] = [
    "_GetCSC",
    "_PutC",
    "_ClrLCDFull",
    "_GrBufCpy",
    "_NewLine",
    "_CreateAppVar",
    "_ChkFindSym",
    "_EnoughMem",
    "_Arc_Unarc to archive",
    "_ChkFindSym on the archived variable",
    "_LoadDEIndPaged",
    "_Arc_Unarc to unarchive",
    "_DelVarArc",
];

/// A region the app fills: its name, address, length and the first byte of its pattern. The
/// table is the one at the end of routines/screen_state_app.z80; SCR_STATE and the rest of
/// appData come from the labels the assembler wrote for the app.
type Region = (String, usize, usize, u8);

fn regions(lab: &std::path::Path) -> Vec<Region> {
    let state = usize::from(label_in(lab, "SCR_STATE"));
    let size = usize::from(label_in(lab, "SCR_STATE_SIZE"));
    assert!(
        state >= 0x8000 && state + size <= 0x8100,
        "SCR_STATE {state:#x}+{size:#x} is not inside appData"
    );
    let fixed = [
        ("textShadow", 0x8508, 128, 0x11),
        ("cmdShadow", 0x966E, 128, 0x22),
        ("statVars", 0x8A3A, 531, 0x33),
        ("tempSwapArea", 0x82A5, 323, 0x55),
        ("prevData", 0x9198, 283, 0x66),
        ("smallEditRAM", 0x90D3, 108, 0x77),
        ("saveSScreen", 0x86EC, 768, 0x88),
    ];
    let mut all: Vec<Region> = fixed
        .iter()
        .map(|&(name, addr, len, seed)| (name.to_string(), addr, len, seed))
        .collect();
    all.push(("SCR_STATE".to_string(), state, size, 0x44));
    all.push((
        "appData after SCR_STATE".to_string(),
        state + size,
        0x100 - size,
        0x45,
    ));
    all
}

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_add(STEP.wrapping_mul(i as u8)))
        .collect()
}

/// Assembles the app and returns it and the label file spasm wrote.
fn assemble(name: &str, defines: &[&str]) -> (Vec<u8>, std::path::PathBuf) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.8xk"));
    let run = std::process::Command::new("sh")
        .arg(root.join("../../scripts/spasm.sh"))
        .args(defines.iter().map(|d| format!("-D{d}")))
        .arg("-L")
        .arg("-I")
        .arg(root.join("../../app/src"))
        .arg(root.join("tests/routines/screen_state_app.z80"))
        .arg(&out)
        .output()
        .expect("the assembler runs");
    assert!(
        run.status.success(),
        "spasm: {}",
        String::from_utf8_lossy(&run.stdout)
    );
    (std::fs::read(&out).unwrap(), out.with_extension("lab"))
}

/// What an event changed: the region, and the runs of changed bytes as (offset, length).
type Damage = Vec<(String, Vec<(usize, usize)>)>;

struct Rig {
    os: RealOs,
    flags_at_start: u8,
    flags_now: u8,
    regions: Vec<Region>,
    /// The report: one line per event.
    log: Vec<(String, Damage)>,
}

impl Rig {
    fn start(defines: &[&str]) -> Rig {
        let (app, lab) = assemble(&format!("screen_state{}", defines.concat()), defines);
        let regions = regions(&lab);
        let mut os = RealOs::boot(&app);
        os.tap(KEY_APPS, 150, 600);
        os.tap(KEY_2, 150, 300);
        os.run_ms(500);
        let mut rig = Rig {
            os,
            flags_at_start: 0,
            flags_now: 0,
            regions,
            log: Vec::new(),
        };
        assert!(rig.damage().is_empty(), "the app filled the regions");
        rig
    }

    fn ram(&mut self, at: usize, len: usize) -> Vec<u8> {
        self.os.ram(at, len)
    }

    fn poke(&mut self, at: usize, bytes: &[u8]) {
        self.os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn count(&mut self) -> u8 {
        self.ram(RESULT, 1)[0]
    }

    /// The runs of bytes that differ from the patterns, per region.
    fn damage(&mut self) -> Damage {
        let mut out = Vec::new();
        for (name, addr, len, seed) in self.regions.clone() {
            let got = self.ram(addr - RAM_BASE, len);
            let want = pattern(len, seed);
            let mut runs: Vec<(usize, usize)> = Vec::new();
            for i in (0..len).filter(|&i| got[i] != want[i]) {
                match runs.last_mut() {
                    Some((start, n)) if *start + *n == i => *n += 1,
                    _ => runs.push((i, 1)),
                }
            }
            if !runs.is_empty() {
                out.push((name, runs));
            }
        }
        out
    }

    /// Records what the event changed, then writes the patterns back for the next event.
    fn record(&mut self, event: &str) -> Damage {
        let damage = self.damage();
        for (_, addr, len, seed) in self.regions.clone() {
            self.poke(addr - RAM_BASE, &pattern(len, seed));
        }
        self.log.push((event.to_string(), damage.clone()));
        damage
    }

    fn tap(&mut self, key: u8) {
        self.os.tap(key, 150, 300);
    }

    fn keys(&mut self, modifier: &[u8]) {
        for scan in 1..=SCAN_LAST {
            let modifier_key = [SCAN_2ND, SCAN_ALPHA].contains(&scan);
            if NO_KEY.contains(&scan) || modifier_key || scan == SCAN_ENTER {
                continue;
            }
            for &m in modifier {
                self.tap(m);
            }
            self.tap(scan);
        }
    }

    fn report(&self) -> String {
        let mut text = String::new();
        for (event, damage) in &self.log {
            if damage.is_empty() {
                writeln!(text, "{event}: nothing changed").unwrap();
            }
            for (region, runs) in damage {
                let total: usize = runs.iter().map(|r| r.1).sum();
                let shown: Vec<String> = runs
                    .iter()
                    .take(6)
                    .map(|(start, n)| format!("+{start:#x}..+{:#x}", start + n))
                    .collect();
                writeln!(
                    text,
                    "{event}: {region} {total} bytes in {} runs {}",
                    runs.len(),
                    shown.join(" ")
                )
                .unwrap();
            }
        }
        text
    }
}

fn measure(defines: &[&str]) -> Rig {
    let mut rig = Rig::start(defines);
    rig.flags_at_start = rig.ram(RESULT + 4, 1)[0];
    rig.flags_now = rig.os.m.ram_page(1)[APP_FLAGS];

    rig.tap(0x1B);
    rig.record("a plain key (5)");
    rig.keys(&[]);
    rig.record("every key plain");
    rig.keys(&[SCAN_2ND]);
    rig.record("every key after 2nd");
    rig.keys(&[SCAN_ALPHA]);
    rig.record("every key after ALPHA");
    rig.tap(SCAN_2ND);
    rig.tap(SCAN_ALPHA);
    rig.keys(&[]);
    rig.tap(SCAN_ALPHA);
    rig.record("every key under alpha lock");

    let contrast = rig.ram(CONTRAST, 1)[0];
    for _ in 0..4 {
        rig.tap(SCAN_2ND);
        rig.tap(SCAN_UP);
    }
    let up = rig.ram(CONTRAST, 1)[0];
    for _ in 0..4 {
        rig.tap(SCAN_2ND);
        rig.tap(SCAN_DOWN);
    }
    let down = rig.ram(CONTRAST, 1)[0];
    println!("contrast {contrast} after 2nd+up x4 {up}, after 2nd+down x4 {down}");
    rig.record("2nd+up four times and 2nd+down four times");

    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(500);
    rig.record("an ON tap");
    rig.os.m.press_key(Machine::KEY_ON);
    rig.os.run_ms(1500);
    rig.os.m.release_key(Machine::KEY_ON);
    rig.os.run_ms(500);
    rig.record("an ON hold of 1.5 s");
    rig.os.run_ms(10_000);
    rig.record("10 s of the OS interrupt in _GetKey");

    let keys = rig.count();
    rig.tap(SCAN_ENTER);
    for (n, call) in BATTERY.iter().enumerate() {
        let step = n as u8 + 1;
        let mut waited = 0;
        while rig.ram(RESULT + 2, 1)[0] != step {
            assert!(waited < 100, "the app did not reach {call}");
            rig.os.run_ms(100);
            waited += 1;
        }
        if step == 1 {
            rig.os.run_ms(5_000);
            rig.record("5 s of the OS interrupt with the app halted outside _GetKey");
        }
        rig.record(call);
        rig.poke(RESULT + 3, &[step]);
    }
    rig.os.run_ms(1000);
    assert_eq!(rig.count(), keys + 1, "the battery returned to the wait");

    let mut seconds = 0;
    while rig.os.m.in_port(3) & POWER_DOWN == 0 {
        assert!(seconds < 600, "no auto power down in 600 s");
        rig.os.run_ms(5_000);
        seconds += 5;
    }
    rig.record(&format!("auto power down inside _GetKey, at {seconds} s"));
    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(3000);
    rig.record("ON after the power down, 3 s later");
    rig.tap(0x1B);
    rig.record("a key after waking");
    rig
}

const APD: &str = "auto power down inside _GetKey";

impl Rig {
    /// What the one event whose name starts with `prefix` changed.
    fn event(&self, prefix: &str) -> &Damage {
        let mut hits = self.log.iter().filter(|(name, _)| name.starts_with(prefix));
        let hit = hits.next().unwrap_or_else(|| panic!("no event {prefix}"));
        assert!(hits.next().is_none(), "two events start with {prefix}");
        &hit.1
    }

    /// Every event that changed `region`, with the runs.
    fn changes_to(&self, region: &str) -> Vec<(&str, &Vec<(usize, usize)>)> {
        self.log
            .iter()
            .flat_map(|(event, damage)| {
                damage
                    .iter()
                    .filter(|(name, _)| name == region)
                    .map(move |(_, runs)| (event.as_str(), runs))
            })
            .collect()
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn screen_state_and_the_rest_of_app_data_keep_their_bytes_through_every_event() {
    for defines in [&[][..], &["NO_SAVE_TEXT"][..]] {
        let rig = measure(defines);
        println!("{defines:?}\n{}", rig.report());
        for region in [
            "SCR_STATE",
            "appData after SCR_STATE",
            "statVars",
            "cmdShadow",
        ]
        .into_iter()
        .chain(["prevData", "smallEditRAM"])
        {
            assert_eq!(rig.changes_to(region), [], "{region} with {defines:?}");
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn auto_power_down_zeroes_save_screen_and_nothing_else_in_the_candidates() {
    let rig = measure(&[]);
    let hit = rig.event(APD);
    assert_eq!(hit.len(), 1, "{hit:?}");
    assert_eq!(hit[0].0, "saveSScreen");
    let zeroed: usize = hit[0].1.iter().map(|r| r.1).sum();
    assert!(
        zeroed >= 760,
        "{zeroed} of 768 bytes differ from the pattern"
    );
    let others: Vec<_> = rig
        .changes_to("saveSScreen")
        .into_iter()
        .filter(|(event, _)| !event.starts_with(APD))
        .collect();
    assert_eq!(others, [], "only the power down touches it");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn put_c_writes_text_shadow_while_app_text_save_is_set_and_leaves_it_when_clear() {
    let rig = measure(&[]);
    assert_eq!(
        rig.flags_at_start, 0x0A,
        "the OS enters the app with appTextSave set"
    );
    assert_eq!(
        rig.changes_to("textShadow"),
        [("_PutC", &vec![(0, 1)])],
        "the character lands at the cursor's cell, row 0 column 0"
    );
    let clear = measure(&["NO_SAVE_TEXT"]);
    assert_eq!(clear.flags_now & 0x02, 0, "the app cleared appTextSave");
    assert_eq!(clear.changes_to("textShadow"), []);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn archiving_a_variable_writes_temp_swap_area_and_nothing_else_does() {
    let rig = measure(&[]);
    assert_eq!(
        rig.changes_to("tempSwapArea"),
        [(
            "_Arc_Unarc to archive",
            &vec![(0, 2), (0xE8, 1), (0xEA, 2), (0xFA, 4)]
        )]
    );
}
