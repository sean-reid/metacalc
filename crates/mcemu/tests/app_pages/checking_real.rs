//! The checking screen on the reference OS with the shipped libraries, ignored in the default run:
//! Run on prop, pred and each of ring's stages from the APPS menu, the bar and the counts moving, the
//! stops, 2nd+QUIT and 2nd+OFF in the middle and Run again, a power down after the end, and what
//! the screen's frames cost against the same job with no redraws.

use crate::app::*;
use crate::checking_frames::chunks_of;
use crate::checkrig::*;
use crate::realos::{BASE, KEY_2, KEY_APPS, RealOs};
use crate::shellrig::*;

/// How long the reference OS gets to settle after a key.
pub const SETTLE: u32 = 300_000;

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
    while run.blits() < 1 {
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
pub struct Timing {
    pub total: u64,
    pub frames: usize,
    pub gaps: Vec<u64>,
    /// The values the status table held at the last frame.
    pub values: Vec<u8>,
}

fn median(gaps: &[u64]) -> u64 {
    let mut sorted = gaps.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Holds F2 until the stage's `end` assertions have a value, polling every 0.5 ms for a new blit,
/// and returns the times of the frames. The table is read when a frame comes.
pub fn run_stage(run: &mut Run, end: usize) -> Timing {
    run_stage_watching(run, end, None)
}

/// `run_stage`, asserting at every poll before the end that the counter at `watch` is still 0.
pub fn run_stage_watching(run: &mut Run, end: usize, watch: Option<u16>) -> Timing {
    run.m().press_key(F2);
    let mut t = 0u64;
    let mut last = 0u64;
    let mut seen = run.blits();
    let mut gaps = Vec::new();
    let mut frames = 0;
    let mut base = None;
    let mut compared = 0;
    let values = loop {
        run.step_us(500);
        t += 500;
        let blits = run.blits();
        if blits != seen {
            seen = blits;
            frames += 1;
            if frames == 1 {
                run.m().release_key(F2);
            }
            gaps.push(t - last);
            last = t;
            if let Some(values) = run.values()
                && values[..end].iter().all(|&x| x != 0)
            {
                break values;
            }
        }
        // The OS holds the key's scan code for about 40 ms after its release and the shell takes it
        // through app_key, so the count is taken once that is over.
        if let Some(at) = watch {
            if base.is_none() && t >= 400_000 {
                base = Some(read_count(run, at));
            }
            if let Some(base) = base {
                assert_eq!(
                    read_count(run, at),
                    base,
                    "the key wait was entered at {t} us"
                );
                compared += 1;
            }
        }
        assert!(t < 4_000_000_000, "the check never ended");
    };
    run.m().release_key(F2);
    if watch.is_some() {
        assert!(base.is_some(), "the count was taken");
        assert!(compared > 1_000, "the count was compared {compared} times");
    }
    Timing {
        total: t,
        frames,
        gaps,
        values,
    }
}

/// The three offsets that turn the screen's redraws off: the jump that the step that is due a
/// frame takes to redraw becomes unconditional past it, and a chunk and the end of the CRC run answer SCR_STAY.
fn no_redraws(built: &Built) -> Vec<(u16, Vec<u8>)> {
    let page = &mcemu::decode_app(&built.file).unwrap()[0];
    let byte = |at: u16| page[usize::from(at - 0x4000)];
    let more = built.label("ki_more");
    let pattern = [
        0xE1,
        0x21,
        0x00,
        0x00,
        0x34,
        0x7E,
        0xFE,
        CHOSEN_EVERY,
        0x3E,
        0x00,
        0x20,
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

/// The host's verdict for each assertion of the shipped library.
fn verdicts(built: &Built) -> &'static [bool] {
    static ALL: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| host_verdicts(built.library.as_ref().unwrap()))
}

/// The frames a check of the first `end` assertions draws from a start with no table: the push, the
/// call that begins the CRC run and each chunk call but the last, whose frame is the first of the
/// steps; then one frame every `every` steps from the first, which is at 0, and the finished frame.
fn frames_of(built: &Built, end: usize, every: usize) -> usize {
    chunks_of(built) + 1 + end.div_ceil(every) + 1
}

/// The measurement for each stage and a figure for the steps between two frames: the screen's whole
/// check against the same job with no redraws, the overhead of the frames, their number, and the
/// longest and the median time between two.
fn measure(built: &'static Built, row: usize, every: u8) -> (f64, Timing) {
    let page = &mcemu::decode_app(&built.file).unwrap()[0];
    let more = built.label("ki_more");
    assert_eq!(
        page[usize::from(more + 7 - 0x4000)],
        CHOSEN_EVERY,
        "the figure is the compare's byte"
    );
    let end = built.stages[row].end;
    let patch = no_redraws(built);
    let patches: Vec<(u16, &[u8])> = patch.iter().map(|(a, b)| (*a, b.as_slice())).collect();
    let figure = [every];
    let with: Vec<(u16, &[u8])> = vec![(more + 7, &figure)];
    let want = expected_values(built, verdicts(built), end);
    let name = format!("{} every {every}", built.stages[row].name);
    let mut screen = launched(built, row, &with);
    let timing = run_stage(&mut screen, end);
    assert_eq!(
        usize::from(screen.verifies()),
        end,
        "{name}: each verified once"
    );
    assert_eq!(
        timing.values, want,
        "{name}: the screen's marks are the host's"
    );
    assert_eq!(
        timing.frames,
        frames_of(built, end, usize::from(every)),
        "{name}: the frames of the screen"
    );
    let mut plain = launched(built, row, &patches);
    let bare = run_stage(&mut plain, end);
    assert_eq!(
        usize::from(plain.verifies()),
        end,
        "{name}: the baseline verified each once"
    );
    assert_eq!(
        bare.values, want,
        "{name}: the baseline's marks are the host's"
    );
    assert_eq!(
        bare.frames, 2,
        "{name}: with no redraws the push and the end"
    );
    let overhead = (timing.total as f64 - bare.total as f64) * 100.0 / bare.total as f64;
    println!(
        "{} every {every}: {end} assertions, the screen {:.2} s, no redraws {:.2} s, overhead {overhead:.1} %, \
         {} frames, longest gap {:.0} ms, median gap {:.0} ms",
        built.stages[row].name,
        timing.total as f64 / 1e6,
        bare.total as f64 / 1e6,
        timing.frames,
        *timing.gaps.iter().max().unwrap() as f64 / 1e3,
        median(&timing.gaps) as f64 / 1e3,
    );
    (overhead, timing)
}

/// Run on prop, pred and ring from the screen against the same job driven with no redraws, for
/// several figures of steps between two frames.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_frames_of_a_check_stay_within_their_share_of_the_job() {
    let built = shipped_staged();
    for every in [16u8, 24, 32, 36, 40, 48, 64] {
        for row in 0..3 {
            measure(built, row, every);
        }
    }
    for row in 0..3 {
        let (overhead, timing) = measure(built, row, CHOSEN_EVERY);
        // Prop's 29 frames of the CRC run alone cost 5 percent of its check, so it is held to the
        // 10.5 percent measured and the other two to 10.
        let most = if row == 0 { 10.7 } else { 10.0 };
        assert!(
            overhead <= most,
            "stage {row}: the frames cost {overhead:.1} percent"
        );
        if row == 2 {
            assert!(median(&timing.gaps) <= 500_000, "ring: the median gap");
        }
    }
}

/// The steps between two frames the screen uses.
const CHOSEN_EVERY: u8 = 32;

/// Every stage of the shipped list, run from the APPS menu on a library with no table: each
/// assertion's mark is the host's verdict, nothing past the stage's end has a mark, the frames are
/// as many as the formula says and the last is the model's finished screen for that row.
#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_each_stage_ends_on_the_models_finished_screen() {
    let built = shipped_staged();
    assert_eq!(built.stages.len(), 3);
    for row in 0..built.stages.len() {
        let end = built.stages[row].end;
        let name = built.stages[row].name.clone();
        let mut run = launched(built, row, &[]);
        let timing = run_stage(&mut run, end);
        let frame = run.m().read_ram(UI_FRAME, FRAME_LEN);
        let values = run.values().unwrap();
        assert_eq!(
            values,
            expected_values(built, verdicts(built), end),
            "{name}"
        );
        assert_eq!(usize::from(run.verifies()), end, "{name}");
        assert_eq!(
            timing.frames,
            frames_of(built, end, usize::from(CHOSEN_EVERY)),
            "{name}: the frames"
        );
        let finished = checking_model(built, &values, row);
        assert!(matches!(
            finished.state(),
            mcui::State::Checking { finished: true, stage, .. } if stage == row
        ));
        shows(&format!("{name} finished"), &frame, &finished);
        assert_eq!(
            run.frames()[1][..3],
            [KIND_CHECKING, row as u8, 1],
            "{name}"
        );
    }
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
            // Reading: the push, crc_begin and two chunks. Stepping: past the CRC run, then two
            // frames of steps.
            let mut frames = 0;
            let wanted = match at {
                At::Reading => 4,
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

pub fn read_count(run: &mut Run, at: u16) -> u16 {
    let b = run.peek(at, 2);
    u16::from_le_bytes([b[0], b[1]])
}
