//! The check app on the reference OS: the real _PutC, the real interrupt handler, the real flags.

use crate::realos::*;
use crate::rig::{axioms, single_symbol_axioms};
use mcemu::Machine;
use std::path::{Path, PathBuf};

type Reached = fn(&mut Check) -> bool;

struct Check {
    os: RealOs,
    lab: PathBuf,
}

impl Check {
    fn start(name: &str, library: &[u8]) -> Check {
        let mut c = Check::boot(name, library);
        c.os.launch();
        c
    }

    fn boot(name: &str, library: &[u8]) -> Check {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("library.mclib");
        std::fs::write(&lib, library).unwrap();
        let build = make_app(
            "probe",
            &format!("{name}-build"),
            &[format!("LIBRARY={}", lib.display()), "kernchk".into()],
        );
        let app = std::fs::read(build.join("kernchk.8xk")).unwrap();
        let os = RealOs::boot(&app);
        Check {
            os,
            lab: build.join("kernchk.lab"),
        }
    }

    fn byte(&mut self, name: &str) -> u8 {
        let at = usize::from(label_in(&self.lab, name)) - 0x8000;
        self.os.ram(at, 1)[0]
    }

    fn word(&mut self, name: &str) -> u16 {
        let at = usize::from(label_in(&self.lab, name)) - 0x8000;
        let b = self.os.ram(at, 2);
        u16::from_le_bytes([b[0], b[1]])
    }
}

/// Runs a millisecond at a time until `done` holds.
fn until(c: &mut Check, what: &str, done: impl Fn(&mut Check) -> bool) {
    for _ in 0..20_000 {
        if done(c) {
            return;
        }
        c.os.run_ms(1);
    }
    panic!("never reached {what}");
}

fn lit_rows(c: &mut Check, rows: std::ops::Range<usize>) -> usize {
    let screen = c.os.screen();
    screen[rows.start * 96..rows.end * 96]
        .iter()
        .filter(|&&p| p)
        .count()
}

/// The report has five lines of eight scanlines; the progress screen has three.
fn report_shown(c: &mut Check) -> bool {
    lit_rows(c, 32..40) > 0 && lit_rows(c, 40..64) == 0
}

fn in_crc(c: &mut Check) -> bool {
    c.byte("K_CHUNK") >= 5
}

fn in_verify(c: &mut Check) -> bool {
    c.word("K_DONE") >= 200
}

/// Steps 20 us at a time until the driver has come out of its wait for ON and set K_STOPPED.
/// Returns the microseconds that took and what port 4 read at that instant.
fn until_stopped(c: &mut Check) -> (u32, u8) {
    for n in 0..200_000 {
        if c.byte("K_STOPPED") != 0 {
            return (n * 20, c.os.m.in_port(4));
        }
        c.os.run_us(20);
    }
    panic!("the driver never stopped");
}

/// Leaves the app and starts it again in the same run, and says whether the second check ran to
/// the end without stopping.
fn runs_again_to_the_end(c: &mut Check, count: u16) -> bool {
    c.os.tap(KEY_CLEAR, 150, 600);
    reruns_to_the_end(c, count)
}

/// Starts the app again from the home screen.
fn reruns_to_the_end(c: &mut Check, count: u16) -> bool {
    c.os.launch();
    until(c, "the end of the second run", |c| {
        c.word("K_DONE") == count
    });
    c.os.run_ms(100);
    c.byte("K_STOPPED") == 0 && c.word("K_ACCEPTED") == count
}

#[test]
#[ignore = "needs the reference ROM image"]
fn an_on_tap_of_any_length_stops_the_check_in_each_phase() {
    let lib = axioms(6000);
    let phases: [(&str, Reached, u8); 2] =
        [("the checksum", in_crc, 1), ("verification", in_verify, 2)];
    for (what, reached, stopped) in phases {
        for down_us in [20, 300, 3000, 30_000] {
            let mut c = Check::start("kernchk-os-tap", &lib);
            until(&mut c, what, reached);
            c.os.m.press_key(Machine::KEY_ON);
            c.os.run_us(down_us);
            c.os.m.release_key(Machine::KEY_ON);
            let msg = format!("{what}, tap of {down_us} us");
            let (_, port4) = until_stopped(&mut c);
            assert_eq!(port4 & 1, 0, "{msg}: the ON interrupt latch is clear");
            c.os.run_ms(1500);
            let at = c.word("K_DONE");
            assert_eq!(c.byte("K_STOPPED"), stopped, "{msg}");
            assert_eq!(
                c.os.on_flags() & ON_INTERRUPT,
                0,
                "{msg}: the flag is clear"
            );
            assert!(report_shown(&mut c), "{msg}: the report is on the screen");
            c.os.run_ms(500);
            assert_eq!(c.word("K_DONE"), at, "{msg}: the check stayed stopped");
            assert!(runs_again_to_the_end(&mut c, 6000), "{msg}: the next run");
        }
    }
}

#[test]
#[ignore = "needs the reference ROM image"]
fn a_held_on_stops_the_check_and_the_report_waits_for_the_key_to_come_up() {
    let lib = axioms(6000);
    for lag_us in [0, 300, 700, 1300, 2900] {
        let mut c = Check::start("kernchk-os-hold", &lib);
        until(&mut c, "verification", in_verify);
        c.os.run_us(lag_us);
        c.os.m.press_key(Machine::KEY_ON);
        c.os.run_ms(300);
        let msg = format!("{lag_us} us into a step");
        let at = c.word("K_DONE");
        c.os.run_ms(100);
        assert_eq!(c.word("K_DONE"), at, "{msg}: the check is waiting");
        assert_ne!(
            c.os.on_flags() & ON_INTERRUPT,
            0,
            "{msg}: the OS flagged the hold"
        );
        assert!(
            !report_shown(&mut c),
            "{msg}: nothing is reported while ON is down"
        );
        let held = c.os.screen();
        c.os.m.release_key(Machine::KEY_ON);
        let (freed, port4) = until_stopped(&mut c);
        assert_eq!(port4 & 1, 0, "{msg}: the ON interrupt latch is clear");
        let mut printed = freed;
        while c.os.screen() == held {
            c.os.run_us(100);
            printed += 100;
            assert!(
                printed < 70_000,
                "{msg}: nothing printed {printed} us after the release, the driver stopped at {freed} us"
            );
        }
        c.os.run_ms(600);
        assert_eq!(
            c.os.on_flags() & ON_INTERRUPT,
            0,
            "{msg}: the flag is clear"
        );
        assert!(
            report_shown(&mut c),
            "{msg}: the report follows the release"
        );
        assert_eq!(c.byte("K_STOPPED"), 2, "{msg}");
        c.os.tap(KEY_CLEAR, 150, 600);
        assert_eq!(
            lit_rows(&mut c, 8..64),
            0,
            "{msg}: the app left for the home screen"
        );
        assert!(reruns_to_the_end(&mut c, 6000), "{msg}: the next run");
    }
}

/// ON goes down after kernel_init's last call into the OS and is held through its return. The
/// library has one symbol, so init never reads the key, and the OS sets its break flag where the
/// driver's `ei` at the end of `kd_header` turns interrupts on. The driver must stop before it prints, or the OS breaks out of
/// the app at its next text call.
#[test]
#[ignore = "needs the reference ROM image"]
fn an_on_hold_the_init_missed_stops_the_check_before_any_text_is_printed() {
    let lib = single_symbol_axioms(6000);
    let mut c = Check::boot("kernchk-os-init-hold", &lib);
    let (from, to) = (
        label_in(&c.lab, "INIT_SYMBOLS_DONE"),
        label_in(&c.lab, "INIT_NAMES_DONE"),
    );
    c.os.tap(KEY_APPS, 150, 600);
    c.os.m.press_key(KEY_2);
    for n in 0..100_000 {
        assert!(n < 99_999, "never reached the end of kernel_init");
        let pc = c.os.m.pc();
        if c.os.m.in_port(6) == BASE && (from..to).contains(&pc) {
            break;
        }
        c.os.run_us(5);
    }
    c.os.m.press_key(Machine::KEY_ON);
    c.os.m.release_key(KEY_2);
    c.os.run_ms(300);
    assert!(
        !report_shown(&mut c),
        "nothing is reported while ON is down"
    );
    c.os.m.release_key(Machine::KEY_ON);
    c.os.run_ms(1500);
    assert_eq!(c.byte("K_STOPPED"), 3, "stopped as an init stop");
    assert_eq!(c.os.m.in_port(4) & 1, 0, "the ON interrupt latch is clear");
    assert_eq!(c.os.on_flags() & ON_INTERRUPT, 0, "the flag is clear");
    assert!(report_shown(&mut c), "the report is on the screen");
    assert!(runs_again_to_the_end(&mut c, 6000), "the next run");
}
