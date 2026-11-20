//! Which regions of RAM keep their bytes while a Flash App waits in the reference OS's `_GetKey`:
//! each candidate home for the app's screen state is filled with a pattern and compared after
//! every kind of event.

#[path = "common/keytables.rs"]
mod keytables;
#[path = "common/realos.rs"]
mod realos;

use keytables::{ALPHA, PLAIN, SECOND};
use mcemu::Machine;
use realos::*;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Mutex;

const RAM_BASE: usize = 0x8000;
const RESULT: usize = 0xA800 - RAM_BASE;
const STEP_AT: usize = RESULT + 2;
const GO: usize = RESULT + 3;
const FLAGS_AT_START: usize = RESULT + 4;
const B_ARCHIVED: usize = RESULT + 5;
const B_UNARCHIVED: usize = RESULT + 6;
const MARK_ON: usize = RESULT + 0x200;
const MARK_OFF: usize = RESULT + 0x201;
/// The OS flag byte at iy+33h: it runs ONSCRPT when bit 0 is set and OFFSCRPT when bit 1 is.
const SCRIPT_FLAGS: usize = 0x09F0 + 0x33;
const B_ON: usize = RESULT + 0x202;
const B_OFF: usize = RESULT + 0x203;
/// What the ON script writes to MARK_ON.
const SCRIPT_RAN_ON: u8 = 0x5A;
const SAVE_SCREEN: (usize, usize) = (0x86EC, 768);
const POWER_DOWN: u8 = 0x10;
const STEP: u8 = 37;
const SCAN_ALPHA: u8 = 0x30;
const SCAN_2ND: u8 = 0x36;
const SCAN_UP: u8 = 0x04;
const SCAN_DOWN: u8 = 0x01;
const SCAN_ENTER: u8 = 0x09;
const SCAN_MODE: u8 = 0x37;
const SCAN_LAST: u8 = 0x38;
/// editFlags bit 2 in iy+1: the home screen's edit buffer is open.
const EDIT_OPEN: u8 = 0x04;
const NO_KEY: [u8; 7] = [5, 6, 7, 8, 0x10, 0x18, 0x29];
const CONTRAST: usize = 0x8447 - RAM_BASE;
const APP_FLAGS: usize = 0x09F0 + 13;
const BATTERY: [&str; 14] = [
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
    "_PutS",
];

/// A region the app fills: its name, address, length and the first byte of its pattern. The
/// table is the one at the end of routines/screen_state_app.z80; SCR_STACK and SCR_VARS come
/// from the labels the assembler wrote for the app, so they are the regions of ram_map.inc.
type Region = (String, usize, usize, u8);

fn regions(lab: &std::path::Path) -> Vec<Region> {
    let of = |name: &str| {
        (
            usize::from(label_in(lab, name)),
            usize::from(label_in(lab, &format!("{name}_SIZE"))),
        )
    };
    let (stack, vars) = (of("SCR_STACK"), of("SCR_VARS"));
    [
        ("textShadow", 0x8508, 128, 0x11),
        ("SCR_STACK", stack.0, stack.1, 0x22),
        ("statVars", 0x8A3A, 531, 0x33),
        ("appData", 0x8000, 256, 0x44),
        ("tempSwapArea", 0x82A5, 323, 0x55),
        ("prevData", 0x9198, 283, 0x66),
        ("SCR_VARS", vars.0, vars.1, 0x77),
        ("saveSScreen", SAVE_SCREEN.0, SAVE_SCREEN.1, 0x88),
    ]
    .map(|(name, addr, len, seed)| (name.to_string(), addr, len, seed))
    .to_vec()
}

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_add(STEP.wrapping_mul(i as u8)))
        .collect()
}

type Build = (Vec<u8>, std::path::PathBuf);

/// Assembles the app with these defines once per set, whichever test asks first, into a file of
/// its own, and returns the app and the label file spasm wrote.
fn assemble(defines: &[&str]) -> Build {
    static BUILDS: Mutex<Option<HashMap<String, Build>>> = Mutex::new(None);
    let name = format!("screen_state{}", defines.concat());
    let mut builds = BUILDS.lock().unwrap_or_else(|e| e.into_inner());
    builds
        .get_or_insert_with(HashMap::new)
        .entry(name.clone())
        .or_insert_with(|| {
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
        })
        .clone()
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
    /// saveSScreen as the auto power down left it, and the LCD's pixels packed 8 to a byte, the
    /// leftmost in bit 7, as the app left the screen before the wait.
    save_screen_after_apd: Vec<u8>,
    lcd_at_apd: Vec<u8>,
    /// The ONSCRPT and OFFSCRPT markers after the power down and after waking.
    marks: Vec<(&'static str, (u8, u8))>,
    /// B from _ChkFindSym on the battery's variable after archiving and after unarchiving.
    found: (u8, u8),
    contrast: (u8, u8, u8),
}

impl Rig {
    fn start(defines: &[&str]) -> Rig {
        let (app, lab) = assemble(defines);
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
            save_screen_after_apd: Vec::new(),
            lcd_at_apd: Vec::new(),
            marks: Vec::new(),
            found: (0xFF, 0xFF),
            contrast: (0, 0, 0),
        };
        assert!(rig.damage().is_empty(), "the app filled the regions");
        rig.flags_at_start = rig.ram(FLAGS_AT_START, 1)[0];
        rig.flags_now = rig.os.m.ram_page(1)[APP_FLAGS];
        rig
    }

    fn ram(&mut self, at: usize, len: usize) -> Vec<u8> {
        self.os.ram(at, len)
    }

    fn poke(&mut self, at: usize, bytes: &[u8]) {
        self.os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn count(&mut self) -> u16 {
        let word = self.ram(RESULT, 2);
        u16::from_le_bytes([word[0], word[1]])
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
    fn record(&mut self, event: &str) {
        let damage = self.damage();
        for (_, addr, len, seed) in self.regions.clone() {
            self.poke(addr - RAM_BASE, &pattern(len, seed));
        }
        self.log.push((event.to_string(), damage));
    }

    fn tap(&mut self, key: u8) {
        self.os.tap(key, 150, 300);
    }

    /// Taps every key after `modifier`, and returns how many returns of `_GetKey` that is: one for
    /// each key the table gives a code.
    fn keys(&mut self, modifier: &[u8], table: &[u8; 56]) -> u16 {
        let mut returns = 0;
        for scan in 1..=SCAN_LAST {
            let modifier_key = [SCAN_2ND, SCAN_ALPHA].contains(&scan);
            if NO_KEY.contains(&scan) || modifier_key || scan == SCAN_ENTER {
                continue;
            }
            for &m in modifier {
                self.tap(m);
            }
            self.tap(scan);
            returns += u16::from(table[usize::from(scan) - 1] != 0);
        }
        returns
    }

    /// Taps `what`, the app's count must have grown by `returns`, and records the event.
    fn sweep(&mut self, event: &str, modifier: &[u8], table: &[u8; 56]) {
        let before = self.count();
        let returns = self.keys(modifier, table);
        assert!(returns > 20, "{event}: the table has {returns} keys");
        assert_eq!(
            self.count() - before,
            returns,
            "{event}: returns of _GetKey"
        );
        self.record(event);
    }

    fn marks(&mut self) -> (u8, u8) {
        (self.ram(MARK_ON, 1)[0], self.ram(MARK_OFF, 1)[0])
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
        for (when, (on, off)) in &self.marks {
            writeln!(
                text,
                "{when}: ONSCRPT mark {on:#04x}, OFFSCRPT mark {off:#04x}"
            )
            .unwrap();
        }
        text
    }

    /// Runs the OS until auto power down fires inside `_GetKey`, wakes it with ON and presses a
    /// key, recording what each changed.
    fn power_cycle(&mut self) {
        self.lcd_at_apd = self
            .os
            .screen()
            .chunks(8)
            .map(|px| px.iter().fold(0u8, |b, &lit| (b << 1) | u8::from(lit)))
            .collect();
        let mut seconds = 0;
        while self.os.m.in_port(3) & POWER_DOWN == 0 {
            assert!(seconds < 600, "no auto power down in 600 s");
            self.os.run_ms(5_000);
            seconds += 5;
        }
        self.save_screen_after_apd = self.ram(SAVE_SCREEN.0 - RAM_BASE, SAVE_SCREEN.1);
        let marks = self.marks();
        self.marks.push(("after the power down", marks));
        self.record(&format!("{APD}, at {seconds} s"));
        self.tap(Machine::KEY_ON);
        self.os.run_ms(3000);
        let marks = self.marks();
        self.marks.push(("after ON, 3 s later", marks));
        self.record("ON after the power down, 3 s later");
        let before = self.count();
        self.tap(0x1B);
        assert_eq!(self.count(), before + 1, "a key after waking returns once");
        self.record("a key after waking");
    }
}

/// The app waits in `_GetKey` and the OS wakes and sleeps.
fn measure_power(defines: &[&str], script_flags: u8) -> Rig {
    let mut rig = Rig::start(defines);
    rig.os.m.ram_page(1)[SCRIPT_FLAGS] |= script_flags;
    rig.power_cycle();
    rig
}

fn measure(defines: &[&str]) -> Rig {
    let mut rig = Rig::start(defines);
    let before = rig.count();
    rig.tap(0x1B);
    assert_eq!(rig.count(), before + 1, "a plain key returns once");
    rig.record("a plain key (5)");
    rig.sweep("every key plain", &[], &PLAIN);
    rig.sweep("every key after 2nd", &[SCAN_2ND], &SECOND);
    rig.sweep("every key after ALPHA", &[SCAN_ALPHA], &ALPHA);
    rig.tap(SCAN_2ND);
    rig.tap(SCAN_ALPHA);
    rig.sweep("every key under alpha lock", &[], &ALPHA);
    rig.tap(SCAN_ALPHA);

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
    rig.contrast = (contrast, up, down);
    assert!(
        up > contrast,
        "2nd+up raised the contrast {contrast} to {up}"
    );
    assert_eq!(down, contrast, "2nd+down brought it back");
    rig.record("2nd+up four times and 2nd+down four times");

    let before = rig.count();
    rig.tap(Machine::KEY_ON);
    rig.os.run_ms(500);
    rig.record("an ON tap");
    rig.os.m.press_key(Machine::KEY_ON);
    rig.os.run_ms(1500);
    rig.os.m.release_key(Machine::KEY_ON);
    rig.os.run_ms(500);
    assert_eq!(rig.count(), before, "ON returns nothing");
    rig.record("an ON hold of 1.5 s");
    rig.os.run_ms(10_000);
    rig.record("10 s of the OS interrupt in _GetKey");

    let keys = rig.count();
    rig.tap(SCAN_ENTER);
    assert_eq!(rig.count(), keys + 1, "ENTER returned");
    for (n, call) in BATTERY.iter().enumerate() {
        let step = n as u8 + 1;
        let mut waited = 0;
        while rig.ram(STEP_AT, 1)[0] != step {
            assert!(waited < 100, "the app did not reach {call}");
            rig.os.run_ms(100);
            waited += 1;
        }
        if step == 1 {
            rig.os.run_ms(5_000);
            rig.record("5 s of the OS interrupt with the app halted outside _GetKey");
        }
        rig.record(call);
        rig.poke(GO, &[step]);
    }
    rig.os.run_ms(1000);
    assert_eq!(rig.count(), keys + 1, "the battery returned to the wait");
    rig.found = (rig.ram(B_ARCHIVED, 1)[0], rig.ram(B_UNARCHIVED, 1)[0]);

    rig.power_cycle();
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
        assert!(
            self.regions.iter().any(|r| r.0 == region),
            "no region {region}"
        );
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
fn app_data_and_the_other_survivors_keep_their_bytes_through_every_event() {
    for defines in [&[][..], &["NO_SAVE_TEXT"][..]] {
        let rig = measure(defines);
        println!("{defines:?}\n{}", rig.report());
        for region in ["appData", "statVars", "SCR_STACK", "prevData", "SCR_VARS"] {
            assert_eq!(rig.changes_to(region), [], "{region} with {defines:?}");
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn auto_power_down_replaces_save_screen_with_the_screen_and_changes_nothing_else_in_the_candidates()
{
    let rig = measure(&[]);
    let hit = rig.event(APD);
    assert_eq!(hit.len(), 1, "{hit:?}");
    assert_eq!(hit[0].0, "saveSScreen");
    assert!(
        rig.lcd_at_apd.iter().any(|&b| b != 0),
        "the screen shows the app's text"
    );
    assert_eq!(
        rig.save_screen_after_apd, rig.lcd_at_apd,
        "the OS leaves the screen's pixels in saveSScreen"
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
fn put_c_and_put_s_write_text_shadow_while_app_text_save_is_set_and_leave_it_when_clear() {
    let rig = measure(&[]);
    assert_eq!(
        rig.flags_at_start, 0x0A,
        "the OS enters the app with appTextSave set"
    );
    assert_eq!(
        rig.changes_to("textShadow"),
        [("_PutC", &vec![(0, 1)]), ("_PutS", &vec![(16, 13)])],
        "_PutC lands at row 0 column 0, and _PutS after the _NewLine on row 1"
    );
    let clear = measure(&["NO_SAVE_TEXT"]);
    assert_eq!(clear.flags_now & 0x02, 0, "the app cleared appTextSave");
    assert_eq!(clear.changes_to("textShadow"), []);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn archiving_a_variable_writes_temp_swap_area_and_unarchiving_does_not() {
    let rig = measure(&[]);
    assert_ne!(rig.found.0, 0, "the variable was archived after step 9");
    assert_eq!(rig.found.1, 0, "and in RAM after step 12");
    assert_eq!(
        rig.changes_to("tempSwapArea"),
        [(
            "_Arc_Unarc to archive",
            &vec![(0, 2), (0xE8, 1), (0xEA, 2), (0xFA, 4)]
        )]
    );
}

const WAKE: &str = "ON after the power down, 3 s later";

/// The AppVar's `B` from `_ChkFindSym` once the app made it: 0 for in RAM.
fn found(rig: &mut Rig) -> (u8, u8) {
    (rig.ram(B_ON, 1)[0], rig.ram(B_OFF, 1)[0])
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_on_script_whose_flag_is_set_runs_on_waking_and_is_copied_over_the_start_of_app_data() {
    for defines in [&["MAKE_ON"][..], &["MAKE_ON", "MAKE_OFF"][..]] {
        let mut rig = measure_power(defines, 1);
        println!("{defines:?}\n{}", rig.report());
        assert_eq!(found(&mut rig).0, 0, "ONSCRPT is in RAM");
        assert_eq!(
            rig.marks,
            [
                ("after the power down", (0, 0)),
                ("after ON, 3 s later", (SCRIPT_RAN_ON, 0))
            ],
            "the script ran when the OS woke, and the OFF script did not at the power down"
        );
        assert_eq!(
            rig.changes_to("appData"),
            [(WAKE, &vec![(0, 64)])],
            "the OS copies the script's size word and code to 8000h"
        );
        for region in ["SCR_STACK", "SCR_VARS"] {
            assert_eq!(rig.changes_to(region), [], "{region} with the script run");
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_scripts_do_not_run_without_their_flags_or_when_archived_or_off_at_auto_power_down() {
    let cases: [(&[&str], u8); 6] = [
        (&[], 3),
        (&["MAKE_ON"], 0),
        (&["MAKE_ON", "MAKE_OFF"], 0),
        (&["MAKE_OFF"], 3),
        (&["MAKE_ON", "MAKE_OFF", "ARCHIVE_SCRIPTS"], 3),
        (&["MAKE_ON", "ARCHIVE_SCRIPTS"], 1),
    ];
    for (defines, flags) in cases {
        let mut rig = measure_power(defines, flags);
        let (on, off) = found(&mut rig);
        let archived = defines.contains(&"ARCHIVE_SCRIPTS");
        assert_eq!(
            on != 0,
            archived && defines.contains(&"MAKE_ON"),
            "ONSCRPT's B {on}"
        );
        assert_eq!(
            off != 0,
            archived && defines.contains(&"MAKE_OFF"),
            "OFFSCRPT's B {off}"
        );
        assert_eq!(
            rig.marks,
            [
                ("after the power down", (0, 0)),
                ("after ON, 3 s later", (0, 0))
            ],
            "{defines:?} with flags {flags}: no script ran"
        );
        for region in ["appData", "SCR_STACK", "SCR_VARS"] {
            assert_eq!(
                rig.changes_to(region),
                [],
                "{region}, {defines:?} with flags {flags}"
            );
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn leaving_the_app_redraws_the_home_screen_in_text_shadow_and_leaves_the_screen_state_regions() {
    let mut rig = Rig::start(&["EXIT_ON_QUIT"]);
    let edit_flags = rig.os.m.ram_page(1)[0x09F0 + 1];
    assert_eq!(edit_flags & EDIT_OPEN, 0, "no edit buffer is open on entry");
    rig.tap(0x1B);
    assert_eq!(rig.count(), 1, "a key reaches the running app");
    rig.tap(SCAN_2ND);
    rig.tap(SCAN_MODE);
    rig.os.run_ms(3000);
    assert_eq!(rig.count(), 2, "QUIT was returned to the app");
    rig.tap(0x1B);
    assert_eq!(
        rig.count(),
        2,
        "keys no longer reach the app: the OS left it"
    );
    rig.record("2nd+MODE, leaving the app");
    let (event, damage) = rig.log.last().unwrap();
    println!("{event}\n{}", rig.report());
    let names: Vec<&str> = damage.iter().map(|d| d.0.as_str()).collect();
    assert_eq!(
        names,
        ["textShadow", "tempSwapArea"],
        "the home screen redraw"
    );
    for region in ["SCR_STACK", "SCR_VARS"] {
        assert_eq!(rig.changes_to(region), [], "{region} on leaving");
    }
}
