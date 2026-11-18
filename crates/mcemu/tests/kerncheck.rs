#[path = "kerncheck/os.rs"]
mod os;
#[path = "common/realos.rs"]
mod realos;
#[path = "kerncheck/rig.rs"]
mod rig;

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
fn on_held_before_the_run_stops_it_in_the_checksum() {
    let bytes = axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.press_key(Machine::KEY_ON);
    assert_eq!(rig.finish(ENTRY, 3), 3);
    let (_, result) = screens(&rig.trace());
    assert_eq!(
        result,
        [
            "kernchk v1",
            "crc stopped",
            "checked 00000",
            "accepted 00000",
            "stopped 00000"
        ]
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
    let mut pc = rig.run_until_trace_has("\rchk 00000/", 5_000);
    let mut seen = 0;
    while seen < nth {
        let run = rig.m.run(pc, 40);
        assert_eq!(run.stop, Stop::Limit, "the run ended before a proof step");
        pc = rig.m.pc();
        seen += usize::from(proofs.contains(&pc));
    }
    rig.m.press_key(Machine::KEY_ON);
    let mut driver_looked = false;
    loop {
        let run = rig.m.run(pc, 40);
        pc = rig.m.pc();
        driver_looked |= pc == on_down;
        if run.stop == Stop::Halted {
            assert!(rig.finish(pc, 1) >= 1);
            return (driver_looked, rig);
        }
    }
}

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

const FLAGS: usize = 0x3800 + 9;

#[test]
fn after_an_on_stop_the_report_waits_for_the_key_to_come_up() {
    let bytes = axioms(300);
    let mut rig = Rig::new(&bytes, Speed::Mhz15);
    rig.m.press_key(Machine::KEY_ON);
    let halted_in_wait = label("ON_WAIT_HALTED");
    let mut pc = ENTRY;
    for held in 1..=6 {
        let run = rig.m.run(pc, LIMIT);
        assert_ne!(run.stop, Stop::Fault);
        pc = rig.m.pc();
        assert!(
            pc == halted_in_wait || pc + 1 == halted_in_wait,
            "poll {held}"
        );
        let trace = rig.trace();
        assert_eq!(
            count_of(&trace, "\u{c}"),
            1,
            "nothing is printed while ON is down"
        );
        pc = halted_in_wait;
    }
    rig.m.release_key(Machine::KEY_ON);
    assert_eq!(rig.finish(pc, 0), 0);
    let (_, result) = screens(&rig.trace());
    assert_eq!(
        result,
        [
            "kernchk v1",
            "crc stopped",
            "checked 00000",
            "accepted 00000",
            "stopped 00000"
        ]
    );
}

#[test]
fn the_pending_os_break_is_cleared_after_an_on_stop_and_nothing_else_is_touched() {
    let bytes = axioms(300);
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
        let halted_in_wait = label("ON_WAIT_HALTED");
        let mut waits = 0;
        loop {
            let run = rig.m.run(pc, LIMIT);
            assert_ne!(run.stop, Stop::Fault);
            let at = rig.m.pc();
            if at != halted_in_wait && at + 1 != halted_in_wait {
                break;
            }
            pc = halted_in_wait;
            waits += 1;
            if waits == holds {
                rig.m.release_key(Machine::KEY_ON);
            }
        }
        assert_eq!(
            waits,
            holds.max(1),
            "one halt takes a latched interrupt first"
        );
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
    let halted_in_wait = label("ON_WAIT_HALTED");
    let mut printed = None;
    for held in 1..=5 {
        let run = rig.m.run(pc, LIMIT);
        assert_ne!(run.stop, Stop::Fault);
        let at = rig.m.pc();
        assert!(
            at == halted_in_wait || at + 1 == halted_in_wait,
            "poll {held}"
        );
        let len = rig.trace().len();
        assert_eq!(*printed.get_or_insert(len), len, "output while ON is down");
        pc = halted_in_wait;
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
