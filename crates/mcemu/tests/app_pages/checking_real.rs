//! The checking screen on the reference OS with the shipped libraries, ignored in the default run:
//! Run on prop, pred and each of ring's stages from the APPS menu, the bar and the counts moving, the
//! stops, 2nd+QUIT and 2nd+OFF in the middle and Run again, a power down after the end, and what
//! the screen's frames cost against the same job with no redraws.

use crate::app::*;
use crate::builds::*;
use crate::checkrig::*;
use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs};
use crate::shellrig::*;
use mcemu::Machine;

/// How long the reference OS gets to settle after a key.
const SETTLE: u32 = 300_000;

/// A run of the app on the reference OS, started from the APPS menu with `patch` applied to its
/// pages first, which waits at its Libraries screen with the cursor on stage `row`.
pub fn launched(built: &'static Built, row: usize, patch: &[(u16, &[u8])]) -> Run {
    let mut os = RealOs::boot(&built.file);
    for &(at, bytes) in patch {
        os.m.load_flash(BASE, usize::from(at - 0x4000), bytes);
    }
    install_hooks(&mut os.m, built, Enter::Real);
    hook_kernel(&mut os.m, built);
    os.tap(KEY_APPS, 150, 600);
    os.m.press_key(KEY_2);
    os.run_ms(20);
    os.m.release_key(KEY_2);
    let mut run = Run::from_shell(
        Shell {
            app: App::on_real_os_in(os.m, built),
            patience: 1,
        },
        built,
    );
    let mut waited = 0;
    while run.shell.blits() < 1 {
        run.step_us(500);
        waited += 500;
        assert!(waited < 3_000_000, "the first frame");
    }
    run.step_us(SETTLE);
    for _ in 0..row {
        run.press_us(DOWN, 150_000);
    }
    run
}

impl Run {
    /// Runs `us` microseconds of emulated time.
    pub fn step_us(&mut self, us: u32) {
        let stop = self.m().run_time(us).stop;
        assert_eq!(stop, mcemu::Stop::Limit, "the machine stopped");
    }

    /// Holds a key for `hold` microseconds and lets the OS settle.
    pub fn press_us(&mut self, scan: u8, hold: u32) {
        self.m().press_key(scan);
        self.step_us(hold);
        self.m().release_key(scan);
        self.step_us(SETTLE);
    }
}

/// What a run on the screen took.
#[derive(Debug)]
struct Timing {
    total: u64,
    frames: usize,
    gaps: Vec<u64>,
}

fn median(gaps: &[u64]) -> u64 {
    let mut sorted = gaps.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Holds F2 until the stage's `end` assertions have a value, polling every 0.5 ms for a new blit,
/// and returns the times of the frames. The table is read when a frame comes.
fn run_stage(run: &mut Run, end: usize) -> Timing {
    run_stage_watching(run, end, None)
}

/// `run_stage`, asserting at every poll before the end that the counter at `watch` is still 0.
fn run_stage_watching(run: &mut Run, end: usize, watch: Option<u16>) -> Timing {
    run.m().press_key(F2);
    let mut t = 0u64;
    let mut last = 0u64;
    let mut seen = run.shell.blits();
    let mut gaps = Vec::new();
    let mut frames = 0;
    loop {
        run.step_us(500);
        t += 500;
        let blits = run.shell.blits();
        if blits != seen {
            seen = blits;
            frames += 1;
            gaps.push(t - last);
            last = t;
            if run
                .values()
                .is_some_and(|v| v[..end].iter().all(|&x| x != 0))
            {
                break;
            }
        }
        if let Some(at) = watch {
            assert_eq!(read_count(run, at), 0, "the key wait was entered at {t} us");
        }
        assert!(t < 4_000_000_000, "the check never ended");
    }
    run.m().release_key(F2);
    Timing {
        total: t,
        frames,
        gaps,
    }
}

/// The three offsets that turn the screen's redraws off: the jump that every sixteenth step takes
/// to redraw becomes unconditional past it, and a chunk and the end of the CRC run answer SCR_STAY.
fn no_redraws(built: &Built) -> Vec<(u16, Vec<u8>)> {
    let page = &mcemu::decode_app(&built.file).unwrap()[0];
    let byte = |at: u16| page[usize::from(at - 0x4000)];
    let more = built.label("ki_more");
    let pattern = [
        0xE1, 0x21, 0x00, 0x00, 0x34, 0x7E, 0xFE, 16, 0x3E, 0x00, 0x20,
    ];
    for (i, &want) in pattern.iter().enumerate() {
        if matches!(i, 2 | 3) {
            continue;
        }
        assert_eq!(byte(more + i as u16), want, "ki_more byte {i}");
    }
    let chunk = built.label("ki_chunk");
    assert_eq!(
        [byte(chunk), byte(chunk + 1), byte(chunk + 2)],
        [0xE1, 0x3E, SCR_REDRAW],
        "ki_chunk"
    );
    vec![(more + 10, vec![0x18]), (chunk + 2, vec![0])]
}

const SCR_REDRAW: u8 = 1;

/// Run on prop, pred and ring from the screen against the same job driven with no redraws: the
/// overhead of the frames, the number of frames and the longest and the median time between two.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_frames_of_a_check_cost_under_ten_percent_of_the_job() {
    let built = shipped_staged();
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let patch = no_redraws(built);
    let patches: Vec<(u16, &[u8])> = patch.iter().map(|(a, b)| (*a, b.as_slice())).collect();
    for (row, name) in ["prop", "pred", "ring"].into_iter().enumerate() {
        let end = built.stages[row].end;
        let mut screen = launched(built, row, &[]);
        let timing = run_stage(&mut screen, end);
        let values = screen.values().unwrap();
        assert_eq!(
            values[..end],
            expected_values(built, &verdicts, end)[..end],
            "{name}: the host's verdicts"
        );
        assert_eq!(
            usize::from(screen.verifies()),
            end,
            "{name}: each verified once"
        );
        let mut plain = launched(built, row, &patches);
        let bare = run_stage(&mut plain, end);
        assert_eq!(
            bare.frames, 2,
            "{name}: with no redraws the push and the end"
        );
        let overhead = (timing.total as f64 - bare.total as f64) * 100.0 / bare.total as f64;
        let longest = *timing.gaps.iter().max().unwrap();
        println!(
            "{name}: {end} assertions, the screen {:.1} s, no redraws {:.1} s, overhead {overhead:.1} %, \
             {} frames, longest gap {:.0} ms, median gap {:.0} ms",
            timing.total as f64 / 1e6,
            bare.total as f64 / 1e6,
            timing.frames,
            longest as f64 / 1e3,
            median(&timing.gaps) as f64 / 1e3,
        );
        assert!(
            overhead <= 10.0,
            "{name}: the frames cost {overhead:.1} percent"
        );
        if name == "ring" {
            assert!(median(&timing.gaps) <= 500_000, "ring: the median gap");
        }
    }
}

/// Compared with the model at the last frame of each stage, on the reference OS: the finished
/// screen.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_each_stage_ends_on_the_models_finished_screen() {
    let built = shipped_staged();
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let end = built.stages[0].end;
    let mut run = launched(built, 0, &[]);
    run_stage(&mut run, end);
    let frame = run.m().read_ram(UI_FRAME, FRAME_LEN);
    let values = expected_values(built, &verdicts, end);
    shows("prop finished", &frame, &checking_model(built, &values, 0));
}

/// F1, CLEAR and ON in the middle of the reading and of the steps on prop, each followed by a second
/// Run that finishes with no assertion verified twice.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_stop_in_the_middle_keeps_the_progress_and_a_second_run_finishes() {
    use crate::checking_flow::{At, stopped_and_run_again};
    let built = shipped_staged();
    let end = built.stages[0].end;
    for at in [At::Reading, At::Stepping] {
        for stop in [Some(F1), Some(CLEAR), None] {
            let mut run = launched(built, 0, &[]);
            run.m().press_key(F2);
            // Reading: the push and crc_begin and a chunk. Stepping: past the CRC run, then two
            // frames of steps.
            let mut frames = 0;
            let wanted = match at {
                At::Reading => 3,
                At::Stepping => usize::MAX,
            };
            loop {
                run.next_blit();
                frames += 1;
                if frames >= wanted {
                    break;
                }
                if run.values().is_some() {
                    run.next_blit();
                    run.next_blit();
                    break;
                }
            }
            run.m().release_key(F2);
            stopped_and_run_again(run, at, stop, end);
        }
    }
}

/// Keys as the real OS takes them: 2nd then MODE is 2nd+QUIT, and 2nd then ON is 2nd+OFF, after
/// which the OS has powered down and ON wakes it.
fn leave(run: &mut Run, off: bool) {
    run.press_us(SECOND, 150_000);
    run.step_us(1_500_000);
    if off {
        run.m().press_key(Machine::KEY_ON);
        run.step_us(300_000);
        run.m().release_key(Machine::KEY_ON);
        run.step_us(2_000_000);
        assert_ne!(run.m().in_port(3) & 0x10, 0, "2nd+OFF powered down");
        run.press_us(Machine::KEY_ON, 150_000);
        run.step_us(3_000_000);
    } else {
        run.press_us(MODE, 150_000);
        run.step_us(1_500_000);
    }
}

/// 2nd+QUIT and 2nd+OFF in the middle of a check leave the job where it was: nothing is written for
/// "next", and after a launch from the APPS menu a Run carries on to the end with every assertion
/// verified once.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_leaving_in_the_middle_and_launching_again_carries_the_check_on() {
    let built = shipped_staged();
    let end = built.stages[0].end;
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    for off in [false, true] {
        let name = if off { "2nd+OFF" } else { "2nd+QUIT" };
        let mut run = launched(built, 0, &[]);
        run.m().press_key(F2);
        loop {
            run.next_blit();
            if run.values().is_some() {
                run.next_blit();
                run.next_blit();
                break;
            }
        }
        run.m().release_key(F2);
        let held = run.values().unwrap().iter().filter(|&&v| v != 0).count();
        assert!(held > 0 && held < end, "{name}: part way, {held}");
        leave(&mut run, off);
        assert_eq!(run.next_number(), 0, "{name}: leaving wrote no next");
        let after = run.values().unwrap().iter().filter(|&&v| v != 0).count();
        assert!(after >= held, "{name}: the table keeps what it had");
        // The home screen has the machine: launch the app again from the APPS menu.
        run.step_us(1_000_000);
        run.press_us(KEY_APPS, 150_000);
        run.step_us(600_000);
        run.m().press_key(KEY_2);
        run.step_us(20_000);
        run.m().release_key(KEY_2);
        let before = run.shell.blits();
        let mut waited = 0;
        while run.shell.blits() == before {
            run.step_us(500);
            waited += 500;
            assert!(
                waited < 3_000_000,
                "{name}: the first frame after the launch"
            );
        }
        run.step_us(SETTLE);
        let values = run.values().unwrap();
        let settled = values.iter().filter(|&&v| v != 0).count();
        assert_eq!(settled, after, "{name}: the table as it was left");
        let verifies = run.verifies();
        let timing = run_stage(&mut run, end);
        println!(
            "{name}: carried on from {settled}, {} frames",
            timing.frames
        );
        let values = run.values().unwrap();
        assert_eq!(
            values[..end],
            expected_values(built, &verdicts, end)[..end],
            "{name}"
        );
        assert_eq!(
            usize::from(run.verifies()),
            end,
            "{name}: none verified twice"
        );
        assert!(run.verifies() > verifies);
    }
}

/// While the check runs the app never waits in the OS's key wait, which is the only place the OS
/// powers the calculator down; when it ends it does, and the refresh on wake runs the enter again,
/// which counts the table and shows "All checked" with no CRC run.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_check_holds_off_the_power_down_and_the_refresh_after_it_is_from_the_table()
{
    let built = shipped_staged();
    let end = built.stages[0].end;
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let mut run = launched(built, 0, &[]);
    let waits = hook_key_wait(run.m(), built);
    let timing = run_stage_watching(&mut run, end, Some(waits));
    println!(
        "a check of {end} assertions took {:.1} s with no power down",
        timing.total as f64 / 1e6
    );
    let crc = run.crc_calls();
    // The shell now waits for a key, where the OS's auto power down fires.
    let mut seconds = 0;
    while run.m().in_port(3) & 0x10 == 0 {
        run.step_us(5_000_000);
        seconds += 5;
        assert!(seconds < 900, "no power down in 900 s");
    }
    assert!(read_count(&mut run, waits) > 0, "the key wait was entered");
    println!("the OS powered down {seconds} s after the check ended");
    run.press_us(Machine::KEY_ON, 150_000);
    run.step_us(3_000_000);
    run.m().press_key(F4);
    run.next_blit();
    run.m().release_key(F4);
    let frame = run.m().read_ram(UI_FRAME, FRAME_LEN);
    let values = expected_values(built, &verdicts, end);
    shows("after the wake", &frame, &checking_model(built, &values, 0));
    assert_eq!(run.crc_calls(), crc, "no CRC run");
}

/// Counts every entry of app_key: its first instructions, `ei` and the bcall of _GetKey, run from
/// RAM, which counts and goes on at the same place.
fn hook_key_wait(m: &mut Machine, built: &Built) -> u16 {
    const COUNTER: u16 = 0xBC50;
    const HOOK: u16 = 0xBC60;
    let at = built.label("app_key");
    let page = &mcemu::decode_app(&built.file).unwrap()[0];
    let original = &page[usize::from(at - 0x4000)..usize::from(at - 0x4000) + 4];
    assert_eq!(
        original[..2],
        [0xFB, 0xEF],
        "app_key starts with ei and a bcall"
    );
    let [c0, c1] = COUNTER.to_le_bytes();
    let [r0, r1] = (at + 4).to_le_bytes();
    m.load_ram(COUNTER, &[0, 0]);
    m.load_ram(
        HOOK,
        &[
            0xE5,
            0x2A,
            c0,
            c1,
            0x23,
            0x22,
            c0,
            c1,
            0xE1,
            0xFB,
            0xEF,
            original[2],
            original[3],
            0xC3,
            r0,
            r1,
        ],
    );
    let [h0, h1] = HOOK.to_le_bytes();
    m.load_flash(BASE, usize::from(at - 0x4000), &[0xC3, h0, h1]);
    COUNTER
}

fn read_count(run: &mut Run, at: u16) -> u16 {
    let b = run.m().read_ram(at, 2);
    u16::from_le_bytes([b[0], b[1]])
}
