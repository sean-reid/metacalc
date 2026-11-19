#[path = "kerncheck/os.rs"]
mod os;
#[path = "common/realos.rs"]
mod realos;
#[path = "kerncheck/rig.rs"]
mod rig;
#[path = "common/stubos.rs"]
mod stubos;

use mcemu::{Machine, Speed, Stop};
use mcpack::library::{CHUNK, crc32};
use rig::*;
use std::path::Path;
use std::process::Command;

#[test]
fn a_library_that_passes_is_reported_whole_with_its_checksum() {
    let bytes = axioms(2000);
    assert!(bytes.len() > 4 * CHUNK);
    let mut rig = Rig::new(&bytes, Speed::Mhz6);
    rig.run_to_end();
    let trace = rig.trace();
    let (progress_screen, result) = screens(&trace);
    assert_eq!(result, report(&bytes));
    assert_eq!(result[1], format!("crc {:08X}", crc32(&bytes[8..])));
    assert_eq!(progress_screen, progress(&bytes, 2000, 2000));
    assert_eq!(
        count_of(&trace, "\rchk "),
        1 + 2000 / 16 + 1,
        "a line at the start, one per 16 assertions, one at the end"
    );
    assert!(count_of(&trace, "\rchk 00016/02000") == 1);
    assert!(count_of(&trace, "\rchk 00032/02000") == 1);
    assert_eq!(count_of(&trace, "\rcrc "), 1 + bytes.len().div_ceil(CHUNK));
    assert_eq!(rig.word("K_DONE"), 2000);
    assert_eq!(rig.word("K_ACCEPTED"), 2000);
}

#[test]
fn a_library_with_failing_proofs_reports_the_first_refusal_and_its_status() {
    for name in [
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier-bad1.mm",
        "demo0-bad1.mm",
    ] {
        let bytes = corpus(name);
        let want = model(&bytes);
        assert!(want.first.is_some(), "{name} has a proof the model refuses");
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        rig.run_to_end();
        let (_, result) = screens(&rig.trace());
        assert_eq!(result, report(&bytes), "{name}");
        let (id, status) = want.first.unwrap();
        assert_eq!(rig.word("K_FIRST"), id as u16, "{name}");
        assert_eq!(rig.byte(label("K_FIRST_ST")), status, "{name}");
        assert_eq!(rig.word("K_DONE") as usize, want.count, "{name}");
        assert_eq!(
            rig.word("K_ACCEPTED") as usize,
            want.count - want.refused,
            "{name}"
        );
    }
}

#[test]
fn a_refusal_is_printed_with_the_status_the_kernel_returned() {
    let mut seen = std::collections::BTreeSet::new();
    for name in [
        "anatomy-bad1.mm",
        "anatomy-bad2.mm",
        "anatomy-bad3.mm",
        "big-unifier-bad1.mm",
        "big-unifier-bad2.mm",
        "big-unifier-bad3.mm",
        "demo0-bad1.mm",
    ] {
        let bytes = corpus(name);
        if let Some((_, status)) = model(&bytes).first {
            let mut rig = Rig::new(&bytes, Speed::Mhz15);
            rig.run_to_end();
            let (_, result) = screens(&rig.trace());
            assert!(result.contains(&format!("status {status:05}")), "{name}");
            seen.insert(status);
        }
    }
    assert!(!seen.is_empty());
}

#[test]
fn a_wrong_checksum_is_reported_and_nothing_is_verified() {
    let mut bytes = axioms(300);
    bytes[4] ^= 0x10;
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (progress_screen, result) = screens(&rig.trace());
    assert_eq!(
        result,
        ["kernchk v1", "crc bad", "checked 00000", "accepted 00000"]
    );
    assert_eq!(progress_screen[..2], ["kernchk v1", "crc 00000/00001"][..]);

    let mut bytes = axioms(2000);
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(
        result,
        ["kernchk v1", "crc bad", "checked 00000", "accepted 00000"]
    );
}

#[test]
fn a_library_the_kernel_refuses_to_load_is_reported_by_status() {
    let mut bytes = axioms(100);
    bytes[0] = b'X';
    reseal(&mut bytes);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, ["kernchk v1", "init st 00001"]);
}

#[test]
fn the_cpu_runs_at_15_mhz_and_returns_to_the_speed_it_had() {
    let bytes = axioms(40);
    for (start, was) in [(Speed::Mhz6, 0u8), (Speed::Mhz15, 1)] {
        let mut rig = Rig::new(&bytes, start);
        rig.run_to_end();
        assert_eq!(rig.byte(OUT_SPEED), 1, "during the run, from {start:?}");
        assert_eq!(rig.byte(OUT_AFTER), was, "after the run, from {start:?}");
    }
}

#[test]
fn on_held_before_the_run_stops_it_before_the_checksum_starts() {
    let bytes = single_symbol_axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.press_key(Machine::KEY_ON);
    assert_eq!(rig.finish(ENTRY, 3), 3);
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, INIT_STOPPED);
    assert_eq!(
        rig.byte(OUT_FLAGGED),
        0,
        "a text call ran with the flag set"
    );
}

#[test]
fn on_pressed_during_the_checksum_stops_it_at_that_chunk() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rcrc 00002/", 100_000);
    rig.m.press_key(Machine::KEY_ON);
    assert!(rig.finish(pc, 2) >= 1);
    let (_, result) = screens(&rig.trace());
    let at = rig.word("K_STOP_AT");
    assert!((2..=3).contains(&at), "stopped at chunk {at}");
    assert_eq!(
        result,
        [
            "kernchk v1".to_string(),
            "crc stopped".to_string(),
            "checked 00000".to_string(),
            "accepted 00000".to_string(),
            format!("stopped {at:05}"),
        ]
    );
}

#[test]
fn on_pressed_during_verification_stops_there_and_reports_where() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
    rig.m.press_key(Machine::KEY_ON);
    assert!(rig.finish(pc, 2) >= 1);
    let (_, result) = screens(&rig.trace());
    let done = rig.word("K_DONE");
    assert!((64..2000).contains(&usize::from(done)), "stopped at {done}");
    assert_eq!(
        result,
        [
            "kernchk v1".to_string(),
            format!("crc {:08X}", crc32(&bytes[8..])),
            format!("checked {done:05}"),
            format!("accepted {:05}", rig.word("K_ACCEPTED")),
            format!("stopped {done:05}"),
        ]
    );
    assert_eq!(rig.word("K_STOP_AT"), done);
}

/// Presses ON the `nth` time the run is seen inside the kernel's proof check, runs to the end, and
/// says whether the driver saw the key before the kernel did.
fn press_in_proof(bytes: &[u8], nth: usize) -> (bool, Rig) {
    let proofs = label("VERIFY_BODY")..label("KERNEL_CRC_BEGIN");
    let on_down = label("KD_ON_SEEN");
    let mut rig = Rig::new(bytes, Speed::Mhz15);
    rig.run_until_trace_has("\rchk 00000/", 5_000);
    let mut seen = 0;
    while seen < nth {
        let run = rig.m.resume(40);
        assert_eq!(run.stop, Stop::Limit, "the run ended before a proof step");
        seen += usize::from(proofs.contains(&rig.m.pc()));
    }
    rig.m.press_key(Machine::KEY_ON);
    let mut driver_looked = false;
    for n in 0..MAX_STEPS {
        let run = rig.m.resume(40);
        let pc = rig.m.pc();
        driver_looked |= pc == on_down;
        if rig.in_wait() {
            assert!(rig.finish(label("ON_RELEASE_AND_CLEAR"), 1) >= 1);
            return (driver_looked, rig);
        }
        assert_ne!(
            run.stop,
            Stop::Halted,
            "the routine ended without entering the wait, at pc {pc:04x} after {n} steps"
        );
    }
    panic!(
        "no wait after {MAX_STEPS} steps of 40 T-states, at pc {:04x}",
        rig.m.pc()
    );
}

/// The most 40 T-state steps `press_in_proof` takes after the press: one call of the kernel is at
/// most 0.83 million T-states, so this is about 100 times that.
const MAX_STEPS: usize = 2_000_000;

#[test]
fn on_pressed_inside_a_proof_makes_the_kernel_return_14_and_the_driver_stop() {
    let bytes = corpus("big-unifier.mm");
    let mut by_kernel = 0;
    for nth in [1, 5, 20, 60, 150, 400, 900] {
        let (driver_looked, mut rig) = press_in_proof(&bytes, nth);
        let done = rig.word("K_DONE");
        let (_, result) = screens(&rig.trace());
        assert_eq!(result.last().unwrap(), &format!("stopped {done:05}"));
        assert_eq!(rig.word("K_STOP_AT"), done);
        assert_eq!(rig.byte(label("K_HAVE_FIRST")), 0, "14 is not a refusal");
        by_kernel += usize::from(!driver_looked);
    }
    assert!(by_kernel > 0, "the kernel never returned 14 to the driver");
}

const UI_CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kerncheck_ui.bin"));
const UI_LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/kerncheck_ui.lab"));

fn ui_label(name: &str) -> u16 {
    UI_LABELS
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&value[1..], 16).unwrap())
        })
        .unwrap_or_else(|| panic!("no label {name}"))
}

/// Where the image copies the UI's variables after drawing, before it calls the driver.
const UI_SNAPSHOT: u16 = 0xB000;

#[test]
fn a_kernel_driver_run_leaves_every_byte_of_the_uis_variables_alone() {
    let bytes = axioms(300);
    let mut rig = Rig::with_code(&bytes, Speed::Mhz15, UI_CODE);
    let scratch = ui_label("K_PARAMS");
    let scratch_len = usize::from(ui_label("K_END") - ui_label("K_PARAMS"));
    let before = rig.ram(scratch, scratch_len);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, report(&bytes));
    assert_eq!(rig.word_at(ui_label("K_ACCEPTED")), 300);
    assert_ne!(
        rig.ram(scratch, scratch_len),
        before,
        "the driver wrote nothing to its scratch"
    );
    let vars = ui_label("UI_VARS");
    let len = usize::from(ui_label("UI_VARS_SIZE"));
    let drawn = rig.ram(UI_SNAPSHOT, len);
    assert_eq!(
        rig.word_at(ui_label("UI_FONTP")),
        ui_label("UI_FONT") - ui_label("SLOT")
    );
    assert!(drawn.iter().any(|&b| b != 0), "the UI set no variables");
    assert_eq!(rig.ram(vars, len), drawn);
}

#[test]
fn the_driver_leaves_no_trace_of_the_kernel_in_the_pages_it_does_not_own() {
    let bytes = corpus("demo0.mm");
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    for page in [0u8, 3] {
        rig.m.ram_page(page).fill(0x6D);
    }
    rig.run_to_end();
    let os = rig.m.ram_page(0).to_vec();
    let changed: Vec<usize> = os
        .iter()
        .enumerate()
        .filter(|&(i, &b)| b != 0x6D && i < 0x4000 - 64)
        .map(|(i, _)| i)
        .collect();
    assert!(changed.is_empty(), "page 80h changed at {changed:x?}");
}

#[test]
fn the_app_builds_from_a_library_and_passes_the_checker() {
    let probe = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../probe");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("kernchk");
    std::fs::create_dir_all(&dir).unwrap();
    let library = dir.join("library.mclib");
    std::fs::write(&library, corpus("demo0.mm")).unwrap();
    let spasm = std::env::var_os("SPASM").unwrap_or_else(|| "spasm".into());
    let make = Command::new("make")
        .args(["-s", "-C"])
        .arg(&probe)
        .arg(format!("SPASM={}", spasm.to_string_lossy()))
        .arg(format!("BUILD={}", dir.display()))
        .arg(format!("LIBRARY={}", library.display()))
        .arg("kernchk")
        .output()
        .expect("make runs");
    assert!(
        make.status.success(),
        "make: {}",
        String::from_utf8_lossy(&make.stdout)
    );
    let check = Command::new("sh")
        .arg(probe.join("../app/scripts/check-8xk.sh"))
        .arg("-l")
        .arg(&library)
        .arg(dir.join("kernchk.8xk"))
        .arg("kernchk")
        .output()
        .expect("the checker runs");
    assert!(
        check.status.success(),
        "check-8xk: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}

const FLAGS: usize = 0x09F0 + 9;

#[test]
fn after_an_on_stop_the_report_waits_for_the_key_to_come_up() {
    let bytes = single_symbol_axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.press_key(Machine::KEY_ON);
    let mut pc = ENTRY;
    for _ in 1..=6 {
        pc = rig.until_waiting(pc);
        let trace = rig.trace();
        assert_eq!(
            count_of(&trace, "\u{c}"),
            1,
            "nothing is printed while ON is down"
        );
    }
    rig.m.release_key(Machine::KEY_ON);
    assert_eq!(rig.finish(pc, 0), 0);
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, INIT_STOPPED);
}

#[test]
fn the_pending_os_break_is_cleared_after_an_on_stop_and_nothing_else_is_touched() {
    let bytes = single_symbol_axioms(300);
    for press in [true, false] {
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        rig.m.ram_page(1)[FLAGS] = if press { 0xFF } else { 0xEF };
        if press {
            rig.m.press_key(Machine::KEY_ON);
            assert_eq!(rig.finish(ENTRY, 2), 2);
        } else {
            rig.run_to_end();
        }
        assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "ON pressed: {press}");
        assert_eq!(rig.m.ram_page(1)[FLAGS - 1], 0, "the flags next to it");
        let (_, result) = screens(&rig.trace());
        assert_eq!(result.len(), if press { 5 } else { 4 });
    }
}

#[test]
fn the_wait_returns_with_interrupts_on_whether_or_not_on_was_down() {
    let bytes = axioms(10);
    for holds in [0usize, 3] {
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        rig.m.ram_page(1)[FLAGS] = 0xFF;
        let entry = label("ONBREAK_ENTRY");
        let mut pc = entry;
        if holds > 0 {
            rig.m.press_key(Machine::KEY_ON);
        }
        let mut waits = 0;
        let mut tstates = 0u64;
        // the quiet case is one halt and the end; a held key is one slice per wait
        let most = if holds == 0 { 100 } else { holds + 1000 };
        for slice in 0.. {
            assert!(
                slice < most,
                "no end after {slice} slices, {waits} waits, at pc {:04x}",
                rig.m.pc()
            );
            let run = rig.m.run(pc, SLICE);
            tstates += u64::from(run.tstates);
            match run.stop {
                Stop::Halted => break,
                Stop::Limit => {
                    waits += 1;
                    if waits == holds {
                        rig.m.release_key(Machine::KEY_ON);
                    }
                    pc = entry;
                }
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(waits, holds, "slices that ended in the wait");
        // the routine without its halt takes about 60 T-states; the halt waits for the timer
        assert!(tstates > 1000, "the halt did not wait: {tstates} T-states");
        assert_eq!(
            rig.byte(0xA004),
            1,
            "interrupts on, ON held for {holds} polls"
        );
        assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "the break flag is cleared");
    }
}

const OUT_FLAGGED: u16 = 0xA006;

fn assert_stopped_at_done(rig: &mut Rig) {
    let done = rig.word("K_DONE");
    let (_, result) = screens(&rig.trace());
    assert_eq!(result.last().unwrap(), &format!("stopped {done:05}"));
    assert_eq!(rig.word("K_STOP_AT"), done);
}

#[test]
fn an_on_tap_over_before_the_next_poll_still_stops_the_run() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rcrc 00002/", 100_000);
    rig.m.press_key(Machine::KEY_ON);
    rig.m.release_key(Machine::KEY_ON);
    rig.finish(pc, 0);
    assert_eq!(rig.byte(label("K_STOPPED")), 1, "stopped in the checksum");

    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
    rig.m.press_key(Machine::KEY_ON);
    rig.m.release_key(Machine::KEY_ON);
    rig.finish(pc, 0);
    assert_eq!(rig.byte(label("K_STOPPED")), 2, "stopped in verification");
    assert_stopped_at_done(&mut rig);
    assert!(rig.word("K_DONE") < 2000);
}

#[test]
fn a_break_flag_the_os_set_stops_the_run_with_no_key_down() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
    rig.m.ram_page(1)[FLAGS] = 0xFF;
    rig.finish(pc, 0);
    assert_eq!(rig.byte(label("K_STOPPED")), 2);
    assert_stopped_at_done(&mut rig);
    assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "cleared, and only that bit");
}

#[test]
fn nothing_is_printed_after_an_on_stop_until_the_flag_is_clear() {
    let bytes = axioms(2000);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let mut pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
    while pc != label("KD_ON_SEEN") {
        assert_eq!(rig.m.run(pc, 1).stop, Stop::Limit);
        pc = rig.m.pc();
    }
    rig.m.press_key(Machine::KEY_ON);
    rig.m.ram_page(1)[FLAGS] |= 0x10;
    let mut printed = None;
    for _ in 1..=5 {
        pc = rig.until_waiting(pc);
        let len = rig.trace().len();
        assert_eq!(*printed.get_or_insert(len), len, "output while ON is down");
    }
    rig.m.release_key(Machine::KEY_ON);
    assert_eq!(rig.finish(pc, 0), 0);
    assert!(rig.trace().len() > printed.unwrap(), "the report follows");
    assert_eq!(
        rig.byte(OUT_FLAGGED),
        0,
        "a text call ran with the flag set"
    );
    assert_eq!(rig.m.ram_page(1)[FLAGS] & 0x10, 0);
    assert_stopped_at_done(&mut rig);
}

const INIT_STOPPED: [&str; 5] = [
    "kernchk v1",
    "init stopped",
    "checked 00000",
    "accepted 00000",
    "stopped 00000",
];

/// Runs until the driver waits for ON, checks nothing but the first clear has been printed over
/// several waits, and returns where to go on from.
fn wait_quietly(rig: &mut Rig) -> u16 {
    let mut pc = ENTRY;
    for _ in 0..4 {
        pc = rig.until_waiting(pc);
        assert_eq!(rig.trace(), [0x0C], "nothing is printed while ON is down");
    }
    pc
}

/// The test image's kernel_init is replaced by `ld a,14; ret`, which is what the kernel returns
/// when ON is down in its symbol comparison, so the driver's handling is tested on its own,
/// whether or not ON is down.
#[test]
fn an_init_reported_interrupted_is_a_stop_and_not_a_failure() {
    let bytes = axioms(300);
    let mut code = CODE.to_vec();
    let at = usize::from(label("KERNEL_INIT") - ENTRY);
    code[at..at + 3].copy_from_slice(&[0x3E, 14, 0xC9]);
    for held in [true, false] {
        let mut rig = Rig::with_code(&bytes, Speed::Mhz15, &code);
        rig.m.ram_page(1)[FLAGS] = 0xFF;
        if held {
            rig.m.press_key(Machine::KEY_ON);
            let pc = wait_quietly(&mut rig);
            rig.m.release_key(Machine::KEY_ON);
            rig.finish(pc, 0);
        } else {
            rig.finish(ENTRY, 0);
        }
        let (_, result) = screens(&rig.trace());
        assert_eq!(result, INIT_STOPPED, "ON held: {held}");
        assert_eq!(rig.byte(label("K_INIT")), 0, "ON held: {held}");
        assert_eq!(rig.byte(label("K_STOPPED")), 3, "ON held: {held}");
        assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "ON held: {held}");
    }
}

/// The unpatched kernel returns 14 from kernel_init when ON is down while it compares symbol names,
/// in a library with more than one symbol.
#[test]
fn on_down_before_the_run_stops_the_real_init_and_the_next_run_checks_the_library() {
    let bytes = axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.ram_page(1)[FLAGS] = 0xFF;
    rig.m.press_key(Machine::KEY_ON);
    let pc = wait_quietly(&mut rig);
    rig.m.release_key(Machine::KEY_ON);
    rig.finish(pc, 0);
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, INIT_STOPPED);
    assert_eq!(rig.byte(label("K_INIT")), 0);
    assert_eq!(rig.byte(label("K_STOPPED")), 3);
    assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "the break flag is cleared");

    rig.finish(ENTRY, 0);
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, report(&bytes));
    assert_eq!(rig.word("K_DONE"), model(&bytes).count as u16);
    assert_eq!(rig.byte(label("K_STOPPED")), 0);
}

/// ON goes down after the leading clear and is held through kernel_init, which reads the key only
/// in the comparison of symbol names. The OS handler runs where the driver turns interrupts on after
/// init and sets the break flag, so the driver has to poll before it prints.
#[test]
fn an_on_hold_the_init_missed_stops_the_run_before_any_text_is_printed() {
    let bytes = single_symbol_axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    let mut pc = ENTRY;
    while pc != label("KERNEL_INIT") {
        assert_eq!(rig.m.run(pc, 1).stop, Stop::Limit);
        pc = rig.m.pc();
    }
    rig.m.press_key(Machine::KEY_ON);
    rig.finish(pc, 3);
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, INIT_STOPPED);
    assert_eq!(rig.byte(label("K_INIT")), 0);
    assert_eq!(rig.byte(label("K_STOPPED")), 3);
    assert_eq!(
        rig.byte(OUT_FLAGGED),
        0,
        "a text call ran with the flag set"
    );
    assert_eq!(
        rig.m.ram_page(1)[FLAGS] & 0x10,
        0,
        "the break flag is clear"
    );
}

/// The driver prints after a kernel call and before its next poll, which is safe only while
/// interrupts stay off from the call to the print. ON goes down at offsets through both phases and
/// stays down, so an interrupt taken at any print would set the flag there.
#[test]
fn a_hold_through_the_checksum_and_verification_never_reaches_a_text_call() {
    let bytes = axioms(2000);
    for (marker, step, stopped) in [("\rcrc 00002/", 100_000, 1), ("\rchk 00064/", 5_000, 2)] {
        for offset in (0..40_000).step_by(331) {
            let mut rig = Rig::new(&bytes, Speed::Mhz15);
            let mut pc = rig.run_until_trace_has(marker, step);
            if offset > 0 {
                assert_eq!(rig.m.run(pc, offset).stop, Stop::Limit);
                pc = rig.m.pc();
            }
            rig.m.press_key(Machine::KEY_ON);
            let msg = format!("{marker:?} +{offset}");
            rig.finish(pc, 2);
            assert_eq!(rig.byte(label("K_STOPPED")), stopped, "{msg}");
            assert_eq!(rig.byte(OUT_FLAGGED), 0, "{msg}: flagged text call");
        }
    }
}

/// The progress line is rewritten after every 16th assertion, straight after the kernel call that
/// finished it. ON goes down during that call.
#[test]
fn a_hold_during_the_call_before_a_progress_print_never_reaches_a_text_call() {
    let bytes = axioms(2000);
    for last in [79u16, 95, 143] {
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        let mut pc = rig.run_until_trace_has("\rchk 00064/", 5_000);
        while rig.word("K_DONE") != last {
            assert_eq!(rig.m.run(pc, 40).stop, Stop::Limit);
            pc = rig.m.pc();
        }
        assert_eq!(rig.m.run(pc, 600).stop, Stop::Limit);
        pc = rig.m.pc();
        let proofs = label("VERIFY_BODY")..label("KERNEL_CRC_BEGIN");
        assert!(
            proofs.contains(&pc),
            "assertion {last}: pc {pc:04x} is outside the verify body"
        );
        assert_eq!(rig.word("K_DONE"), last, "the call has not returned");
        rig.m.press_key(Machine::KEY_ON);
        rig.finish(pc, 2);
        assert_eq!(rig.byte(label("K_STOPPED")), 2, "assertion {last}");
        assert_eq!(rig.byte(OUT_FLAGGED), 0, "assertion {last}: flagged text");
    }
}

/// ON goes down after the driver's poll after init has read the key and before the first print of
/// the checksum, which is where interrupts would otherwise still be on.
#[test]
fn an_on_hold_between_the_poll_after_init_and_the_first_print_stops_the_checksum() {
    let bytes = single_symbol_axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    assert_eq!(rig.m.run(ENTRY, 1).stop, Stop::Limit);
    let mut pc = rig.m.pc();
    while pc != label("KD_ON_SEEN") {
        assert_eq!(rig.m.resume(1).stop, Stop::Limit);
        pc = rig.m.pc();
    }
    let window = label("KD_INITED")..label("KD_HEADER");
    while !window.contains(&pc) {
        assert_eq!(rig.m.resume(1).stop, Stop::Limit);
        pc = rig.m.pc();
    }
    assert!(!String::from_utf8_lossy(&rig.trace()).contains("crc"));
    rig.m.press_key(Machine::KEY_ON);
    rig.finish_from(None, 3);
    assert_eq!(
        rig.byte(OUT_FLAGGED),
        0,
        "a text call ran with the flag set"
    );
    assert_eq!(rig.byte(label("K_STOPPED")), 1, "stopped in the checksum");
}

#[test]
fn a_library_wrong_only_in_its_fourth_magic_byte_is_refused_by_init() {
    let mut bytes = axioms(100);
    bytes[3] ^= 0x01;
    reseal(&mut bytes);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.run_to_end();
    let (_, result) = screens(&rig.trace());
    assert_eq!(result, ["kernchk v1", "init st 00001"]);
}

#[test]
fn the_speed_comes_back_when_init_refuses_the_library() {
    let mut bytes = axioms(100);
    bytes[0] = b'X';
    reseal(&mut bytes);
    for (start, was) in [(Speed::Mhz6, 0u8), (Speed::Mhz15, 1)] {
        let mut rig = Rig::new(&bytes, start);
        rig.run_to_end();
        assert_eq!(rig.byte(label("K_INIT")), 1, "init refused, from {start:?}");
        assert_eq!(rig.byte(OUT_AFTER), was, "after the run, from {start:?}");
    }
}

/// A library of axioms whose length is in `lengths`.
fn axioms_with_length(lengths: std::ops::Range<usize>) -> Vec<u8> {
    let middle = (lengths.start + lengths.end) / 2;
    let mut count = middle / (axioms(1000).len() / 1000);
    for _ in 0..8 {
        let bytes = axioms(count);
        if lengths.contains(&bytes.len()) {
            return bytes;
        }
        count = count * middle / bytes.len();
    }
    panic!("no library of {lengths:?} bytes");
}

/// The length plus 3FFFh carries out of 16 bits, so the chunk count needs the carry into the
/// length's third byte.
#[test]
fn the_chunk_count_is_right_when_the_length_rounds_up_across_a_byte() {
    for lengths in [0xC001..0x1_0000, 0x1_C001..0x2_0000] {
        let bytes = axioms_with_length(lengths);
        let chunks = bytes.len().div_ceil(CHUNK);
        let mut rig = Rig::new(&bytes, Speed::Mhz15);
        rig.run_to_end();
        assert_eq!(usize::from(rig.byte(label("K_CHUNKS"))), chunks);
        let (progress_screen, result) = screens(&rig.trace());
        let count = model(&bytes).count;
        assert_eq!(progress_screen, progress(&bytes, count, count));
        assert_eq!(result, report(&bytes));
    }
}

/// With ON down the wait never ends and the OS break flag stays as the OS left it; after the
/// release the flag is clear and the routine returns.
#[test]
fn the_break_flag_is_cleared_only_once_on_is_up() {
    let bytes = axioms(10);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.ram_page(1)[FLAGS] = 0xFF;
    rig.m.press_key(Machine::KEY_ON);
    let entry = label("ONBREAK_ENTRY");
    for wait in 1..=4 {
        let run = rig.m.run(entry, SLICE);
        assert_eq!(run.stop, Stop::Limit, "wait {wait} ended with ON down");
        assert_ne!(
            rig.m.ram_page(1)[FLAGS] & 0x10,
            0,
            "the flag was cleared in wait {wait}, with ON still down"
        );
    }
    rig.m.release_key(Machine::KEY_ON);
    let mut pc = entry;
    for _ in 0..100 {
        let run = rig.m.run(pc, SLICE);
        if run.stop == Stop::Halted {
            assert_eq!(rig.m.ram_page(1)[FLAGS], 0xEF, "the flag after the release");
            assert_eq!(rig.byte(0xA004), 1, "interrupts on at return");
            return;
        }
        assert_eq!(run.stop, Stop::Limit);
        pc = entry;
    }
    panic!("the wait did not end within 100 slices of the release");
}

const KERNEL_PAGE: u8 = 3;
const KERNEL_PAGE_FF00: std::ops::Range<usize> = 0x3F00..0x3FC0;

fn seed_kernel_state(rig: &mut Rig) {
    let page = rig.m.ram_page(KERNEL_PAGE);
    let at = |name: &str| usize::from(label(name) - 0xC000);
    page[at("KV_READY")..at("KV_READY") + 2].copy_from_slice(&[0xA5, 0x5A]);
    page[at("KV_SAVED_SYM_LIMIT")..at("KV_SAVED_SYM_LIMIT") + 2].copy_from_slice(&[0x34, 0x12]);
    page[KERNEL_PAGE_FF00].fill(0x6D);
}

#[test]
fn a_ready_mark_left_in_kernel_ram_does_not_change_the_run_or_write_the_os_part_of_the_page() {
    let bytes = axioms(40);
    let mut clean = Rig::new(&bytes, Speed::Mhz15);
    clean.run_to_end();
    let want = screens(&clean.trace());
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    seed_kernel_state(&mut rig);
    rig.run_to_end();
    assert_eq!(screens(&rig.trace()), want);
    assert_eq!(rig.m.ram_page(KERNEL_PAGE)[KERNEL_PAGE_FF00], [0x6D; 0xC0]);
}

#[test]
fn the_app_clears_the_ready_mark_before_it_reaches_kernel_init() {
    let bytes = axioms(40);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    seed_kernel_state(&mut rig);
    let init = label("KERNEL_INIT");
    let mut run = rig.m.run(ENTRY, 1);
    for _ in 0..1_000_000 {
        if rig.m.pc() == init {
            break;
        }
        assert_eq!(run.stop, Stop::Limit, "the app ended before kernel_init");
        run = rig.m.resume(1);
    }
    assert_eq!(rig.m.pc(), init, "kernel_init was never reached");
    let at = usize::from(label("KV_READY") - 0xC000);
    assert_eq!(rig.m.ram_page(KERNEL_PAGE)[at..at + 2], [0, 0]);
}
