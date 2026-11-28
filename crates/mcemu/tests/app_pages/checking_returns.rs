//! The interrupt state and the speed at every return of the checking screen's enter, key and draw
//! routines, at 6 and at 15 MHz with interrupts on at the call, each probed where the shell's call
//! of the routine returns.

use crate::app::*;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use mcemu::Machine;

const SPEEDS: [u8; 2] = [0, 1];
const PHASE_STEP: u8 = 0;
const PHASE_READ: u8 = 1;
const PHASE_DONE: u8 = 2;
const PHASE_LEAVE: u8 = 3;
/// What CKS_PHASE holds before an enter, which every enter that succeeds overwrites.
const UNSET: u8 = 0x7F;
/// The notice's number for CK_READER with the reader's status 1: (CK_READER - CK_REFUSED) * 32 + 1.
const READER_FAILED: u16 = 0x61;
/// The job's statuses check_begin answers with.
const CK_NEED_CRC: u16 = 2;
const SCR_STAY: u8 = 0;
const SCR_POP: u8 = 3;
const TRIES: usize = 20_000;
const STEP_LIMIT: usize = 3_000_000;

/// A booted run on `built` at `speed`, with F2 tapped on the first stage's row: the enter is next.
fn opened(built: &'static Built, table: Option<&[u8]>, speed: u8) -> Run {
    opened_after(built, table, speed, |_| {})
}

/// `opened` with `setup` run on the booted machine before F2.
fn opened_after(
    built: &'static Built,
    table: Option<&[u8]>,
    speed: u8,
    setup: impl FnOnce(&mut Run),
) -> Run {
    let mut run = Run::boot(built, table);
    run.m().out_port(0x20, speed);
    setup(&mut run);
    run.tap(F2);
    run
}

/// Runs to the next call of `routine` for which `ready` is true and leaves the machine on its first
/// instruction.
fn call_of(run: &mut Run, routine: &str, mut ready: impl FnMut(&mut Run) -> bool) {
    let at = run.shell.app.label(routine);
    for _ in 0..TRIES {
        run.run_to_addr(routine, at, WAIT);
        if ready(run) {
            return;
        }
        run.m().resume(1);
    }
    panic!("no call of {routine} met the condition");
}

/// From the first instruction of a routine to the instruction after the shell's call of it. Asserts
/// interrupts are on and the speed port holds `speed` at the call, runs `during` once at the call,
/// and returns the registers at the return after checking the stack is balanced.
fn through(run: &mut Run, speed: u8, during: impl FnOnce(&mut Run)) -> mcemu::Regs {
    let regs = run.m().regs();
    assert_eq!(regs.iff1, 1, "interrupts are on at the call");
    assert_eq!(run.m().in_port(0x20) & 1, speed, "the speed at the call");
    let back = run.m().read_ram(regs.sp, 2);
    let back = u16::from_le_bytes([back[0], back[1]]);
    during(run);
    if (0x4000..0x8000).contains(&back) {
        run.run_to_addr("the return", back, WAIT);
    } else {
        // The shell's call came from a page mapped in bank B, which run_to cannot name.
        let mut steps = 0;
        while run.m().regs().pc != back || run.m().regs().sp != regs.sp + 2 {
            run.m().resume(1);
            steps += 1;
            assert!(
                steps < STEP_LIMIT,
                "the routine never returned to {back:04X}"
            );
        }
    }
    let after = run.m().regs();
    assert_eq!(after.sp, regs.sp + 2, "the stack is balanced");
    after
}

fn as_at_the_call(run: &mut Run, regs: &mcemu::Regs, speed: u8, what: &str) {
    assert_eq!(regs.iff1, 1, "{what}: interrupts are on at the return");
    assert_eq!(
        run.m().in_port(0x20) & 1,
        speed,
        "{what}: the speed at the return"
    );
}

/// What the enter returns for each way check_begin can answer, at both speeds: A and CKS_PHASE tell
/// which answer it was.
#[test]
fn checking_enter_returns_with_interrupts_on_and_the_speed_it_had_for_every_answer_of_the_job() {
    for speed in SPEEDS {
        let built = proofs();
        let table = empty_table(built);
        let phase_at = |run: &mut Run| run.byte("CKS_PHASE");
        // CK_READY: a table with assertions left.
        let mut run = opened(built, Some(&table), speed);
        call_of(&mut run, "checking_enter", |_| true);
        let phase = run.shell.app.label("CKS_PHASE");
        let regs = through(&mut run, speed, |r| r.m().load_ram(phase, &[UNSET]));
        assert_eq!(
            (regs.af >> 8, phase_at(&mut run)),
            (0, PHASE_STEP),
            "CK_READY steps"
        );
        as_at_the_call(&mut run, &regs, speed, "CK_READY");
        // CK_NEED_CRC: no table.
        let mut run = opened(padded(), None, speed);
        call_of(&mut run, "checking_enter", |_| true);
        let regs = through(&mut run, speed, |r| r.m().load_ram(phase, &[UNSET]));
        assert_eq!(
            (regs.af >> 8, phase_at(&mut run)),
            (0, PHASE_READ),
            "CK_NEED_CRC reads"
        );
        as_at_the_call(&mut run, &regs, speed, "CK_NEED_CRC");
        // CK_PAUSED: ON down as the kernel is started.
        let mut run = opened(built, Some(&table), speed);
        call_of(&mut run, "checking_enter", |_| true);
        let regs = through(&mut run, speed, |r| {
            r.m().load_ram(phase, &[UNSET]);
            r.m().press_key(Machine::KEY_ON);
        });
        assert_eq!(
            (regs.af >> 8, phase_at(&mut run)),
            (0, PHASE_LEAVE),
            "CK_PAUSED leaves"
        );
        as_at_the_call(&mut run, &regs, speed, "CK_PAUSED");
        // A failing status: the reader answers ST_MALFORMED to a library of length 0 once the kernel has begun.
        let mut run = opened_after(built, Some(&table), speed, |r| {
            let length = r.shell.app.label("lr_length");
            r.m().load_ram(length, &[0, 0, 0]);
        });
        call_of(&mut run, "checking_enter", |_| true);
        let length = run.shell.app.label("lr_length");
        assert_eq!(
            run.m().read_ram(length, 3),
            [0, 0, 0],
            "the reader's length"
        );
        let regs = through(&mut run, speed, |r| r.m().load_ram(phase, &[UNSET]));
        assert_eq!(
            (regs.af >> 8, phase_at(&mut run)),
            (READER_FAILED, UNSET),
            "a failing status answers the notice's number and starts nothing"
        );
        as_at_the_call(&mut run, &regs, speed, "a failing status");
    }
}

/// The registers at the return of check_begin, from its first instruction, which the machine is on.
fn begin_returns(run: &mut Run) -> mcemu::Regs {
    let sp = run.m().regs().sp;
    let back = run.m().read_ram(sp, 2);
    let back = u16::from_le_bytes([back[0], back[1]]);
    run.run_to_addr("the return of check_begin", back, WAIT);
    run.m().regs()
}

/// The enter of a library with no table, whose header read fails after check_begin has answered
/// CK_NEED_CRC: the reader's length is cut to 8 bytes at that return, so the 4 bytes at offset 8 are
/// past it and check_begin's own read, of offset 4, is not. The enter takes the kn_reader exit,
/// calls check_end and answers the notice's number for CK_READER, at both speeds.
#[test]
fn checking_enter_returns_with_interrupts_on_and_the_speed_it_had_when_the_header_read_fails() {
    for speed in SPEEDS {
        let mut run = opened(padded(), None, speed);
        call_of(&mut run, "checking_enter", |_| true);
        let phase = run.shell.app.label("CKS_PHASE");
        let length = run.shell.app.label("lr_length");
        let (reader, end) = (
            run.shell.app.label("kn_reader"),
            run.shell.app.label("check_end"),
        );
        let begin = run.shell.app.label("check_begin");
        let whole = run.m().read_ram(length, 3);
        assert!(
            whole[0] != 0 || whole[1] != 0,
            "a library longer than 8: {whole:02X?}"
        );
        let regs = through(&mut run, speed, |r| {
            r.m().load_ram(phase, &[UNSET]);
            r.run_to_addr("check_begin", begin, WAIT);
            let answered = begin_returns(r);
            assert_eq!(
                answered.af >> 8,
                CK_NEED_CRC,
                "check_begin read its CRC and wants it run"
            );
            assert_eq!(
                r.m().read_ram(length, 3),
                whole,
                "check_begin left the length alone"
            );
            r.m().load_ram(length, &[8, 0, 0]);
            r.run_to_addr("the reader's failure", reader, WAIT);
            assert_eq!(r.byte("CKS_PHASE"), UNSET, "nothing started");
            r.run_to_addr("check_end", end, WAIT);
        });
        let status = regs.af >> 8;
        println!("speed {speed}: the enter answers {status:02X}");
        assert_eq!(
            status, READER_FAILED,
            "CK_READER with the reader's status 1"
        );
        assert_eq!(run.byte("CKS_PHASE"), UNSET, "no phase was set");
        as_at_the_call(&mut run, &regs, speed, "the header read failing");
    }
}

/// Holds `scan` down, runs to the key routine's call with the key's code in A, and returns the
/// registers at the return.
fn key_through(run: &mut Run, speed: u8, scan: u8, check: impl FnOnce(&mut Run)) -> mcemu::Regs {
    run.m().press_key(scan);
    let code = key_code(scan);
    call_of(run, "checking_key", |r| {
        (r.m().regs().af >> 8) as u8 == code
    });
    check(run);
    let regs = through(run, speed, |r| r.m().release_key(scan));
    as_at_the_call(run, &regs, speed, "the key routine");
    regs
}

/// F1, CLEAR and a key the screen ignores, in the middle of reading the library and in the middle
/// of the steps, at both speeds.
#[test]
fn checking_key_returns_with_interrupts_on_and_the_speed_it_had_for_f1_clear_and_an_ignored_key() {
    for speed in SPEEDS {
        for (scan, answer) in [(F1, SCR_POP), (CLEAR, SCR_POP), (F4, SCR_STAY)] {
            let reading = |run: &mut Run| {
                assert_eq!(run.byte("CKS_PHASE"), PHASE_READ, "reading");
                let chunks = run.crc_calls().1;
                assert!(
                    chunks >= 2 && usize::from(chunks) < chunks_of(padded()),
                    "in the middle of the CRC run: {chunks} chunk calls"
                );
            };
            let mut run = opened(padded(), None, speed);
            while run.crc_calls().1 < 3 {
                run.next_blit();
            }
            let regs = key_through(&mut run, speed, scan, reading);
            assert_eq!(regs.af >> 8, u16::from(answer), "reading, key {scan:02X}");
            let built = proofs();
            let steps = |run: &mut Run| {
                assert_eq!(run.byte("CKS_PHASE"), PHASE_STEP, "stepping");
                let values = run.values_here().expect("a table");
                let done = values.iter().filter(|&&v| v != 0).count();
                assert!(
                    done > 0 && done < values.len(),
                    "in the middle of the steps: {done} of {}",
                    values.len()
                );
            };
            let mut run = opened(built, Some(&empty_table(built)), speed);
            run.next_blit();
            run.next_blit();
            let regs = key_through(&mut run, speed, scan, steps);
            assert_eq!(regs.af >> 8, u16::from(answer), "steps, key {scan:02X}");
        }
    }
}

/// The draw in the reading phase, in the steps with a label to show and finished, at both speeds.
#[test]
fn checking_draw_returns_with_interrupts_on_and_the_speed_it_had_in_every_phase() {
    for speed in SPEEDS {
        let built = proofs();
        let table = empty_table(built);
        let phase = |r: &mut Run| r.byte("CKS_PHASE");
        let unsettled = |r: &mut Run| r.values_here().is_some_and(|v| v.contains(&0));
        let mut run = opened(padded(), None, speed);
        call_of(&mut run, "checking_draw", |r| phase(r) == PHASE_READ);
        assert!(run.values_here().is_none(), "no table yet");
        let regs = through(&mut run, speed, |_| {});
        as_at_the_call(&mut run, &regs, speed, "reading");
        let mut run = opened(built, Some(&table), speed);
        call_of(&mut run, "checking_draw", |r| {
            phase(r) == PHASE_STEP && r.settled_here() > 0 && unsettled(r)
        });
        let regs = through(&mut run, speed, |_| {});
        as_at_the_call(&mut run, &regs, speed, "steps with a label");
        let mut run = opened(built, Some(&table), speed);
        call_of(&mut run, "checking_draw", |r| {
            phase(r) == PHASE_DONE && !unsettled(r)
        });
        let regs = through(&mut run, speed, |_| {});
        as_at_the_call(&mut run, &regs, speed, "finished");
    }
}
