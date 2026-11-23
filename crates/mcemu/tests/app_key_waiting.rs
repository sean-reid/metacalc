//! app_key_waiting (app/src/keys.inc) against the stub OS and the reference OS: whether it sees a
//! key the next app_key returns without another press, and that it takes nothing from the OS.

#[path = "common/realos.rs"]
mod realos;

use mcemu::{Machine, Reached, STUB_ROM, Stop};
use realos::*;

const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/keys_wait_app.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/keys_wait_app.lab"));

const STUB_BASE: u8 = 0x30;
const RAM_BASE: usize = 0x8000;
const CMD: usize = 0xA800 - RAM_BASE;
const BUSY: usize = 0xA802 - RAM_BASE;
const INTS: usize = 0xA804 - RAM_BASE;
const FLAGS: usize = 0xA805 - RAM_BASE;
const CALLS: usize = 0xA806 - RAM_BASE;
const DONE: usize = 0xA808 - RAM_BASE;
const KEYA: usize = 0xA809 - RAM_BASE;
const KEYB: usize = 0xA80A - RAM_BASE;
const WRES: usize = 0xA810 - RAM_BASE;
const PRE: usize = 0xA820 - RAM_BASE;
const POST: usize = 0xA824 - RAM_BASE;
const KBD_SCAN: usize = 0x843F - RAM_BASE;

const SKIP_KEY: u8 = 1;
const LOOP: u8 = 2;
const COST: u8 = 4;

/// Scancodes, and the codes app_key gives for the ones the tests press.
const DOWN: u8 = 0x01;
const LEFT: u8 = 0x02;
const SECOND: u8 = 0x36;
const ALPHA: u8 = 0x30;
const MODE: u8 = 0x37;
const SEVEN: u8 = 0x24;
const CODE_DOWN: u8 = 2;
const CODE_LEFT: u8 = 3;
const CODE_LETTER: u8 = 12;
const CODE_QUIT: u8 = 13;

/// A busy pass is about 3,370 T-states: this many are about 250 ms at 6 MHz.
const PASSES: u16 = 445;
/// One timer interrupt, in microseconds.
const PERIOD_US: u32 = 1953;
const STEP_US: u32 = 500;

enum Rig {
    Stub(Machine),
    Real(RealOs),
}

/// What a round left.
#[derive(Debug, PartialEq)]
struct Out {
    wres: Vec<u8>,
    pre: Vec<u8>,
    post: Vec<u8>,
    done: u8,
    key: (u8, u8),
}

impl Rig {
    fn stub() -> Rig {
        let mut m = Machine::new(STUB_ROM).unwrap();
        m.install_app(APP, STUB_BASE).unwrap();
        m.launch(STUB_BASE).unwrap();
        let mut rig = Rig::Stub(m);
        rig.step(20_000);
        rig
    }

    fn real() -> Rig {
        let mut os = RealOs::boot(APP);
        os.tap(KEY_APPS, 150, 600);
        os.tap(KEY_2, 150, 300);
        os.run_ms(500);
        Rig::Real(os)
    }

    fn m(&mut self) -> &mut Machine {
        match self {
            Rig::Stub(m) => m,
            Rig::Real(os) => &mut os.m,
        }
    }

    fn step(&mut self, us: u32) {
        match self {
            Rig::Stub(m) => assert_eq!(m.run_app(us * 6).stop, Stop::Limit),
            Rig::Real(os) => os.run_us(us),
        }
    }

    fn ram(&mut self, at: usize, len: usize) -> Vec<u8> {
        self.m().ram_page(1)[at..at + len].to_vec()
    }

    fn poke(&mut self, at: usize, bytes: &[u8]) {
        self.m().ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// Runs `us` of emulated time, pressing and releasing as `events` say, each (microsecond
    /// from now, scancode, down).
    fn play(&mut self, events: &[(u32, u8, bool)], us: u32) {
        let mut t = 0;
        while t < us {
            for &(_, key, down) in events.iter().filter(|e| e.0 == t) {
                match down {
                    true => self.m().press_key(key),
                    false => self.m().release_key(key),
                }
            }
            self.step(STEP_US);
            t += STEP_US;
        }
    }

    fn start_round(&mut self, passes: u16, ints: bool, flags: u8, calls: u8) {
        self.poke(BUSY, &passes.to_le_bytes());
        self.poke(INTS, &[u8::from(ints)]);
        self.poke(FLAGS, &[flags]);
        self.poke(CALLS, &[calls]);
        self.poke(CMD, &[1]);
    }

    /// The microseconds a round of `passes` takes, measured with no key and no call.
    fn busy_us(&mut self, passes: u16, ints: bool) -> u32 {
        self.start_round(passes, ints, SKIP_KEY, 0);
        let mut t = 0;
        while self.ram(CMD, 1)[0] != 0 {
            assert!(t < 2_000_000, "the round never ended");
            self.step(STEP_US);
            t += STEP_US;
        }
        t
    }

    fn out(&mut self, calls: usize) -> Out {
        Out {
            wres: self.ram(WRES, calls),
            pre: self.ram(PRE, 3),
            post: self.ram(POST, 3),
            done: self.ram(DONE, 1)[0],
            key: (self.ram(KEYA, 1)[0], self.ram(KEYB, 1)[0]),
        }
    }

    fn done(&mut self) -> u8 {
        self.ram(DONE, 1)[0]
    }

    fn label(name: &str) -> u16 {
        LABELS
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(" = ")?;
                (k.eq_ignore_ascii_case(name)).then(|| u16::from_str_radix(&v[1..], 16).unwrap())
            })
            .unwrap_or_else(|| panic!("no label {name}"))
    }

    /// The T-states of one call of app_key_waiting made with interrupts off, from the call to the
    /// instruction after it, once `setup` has set the world up.
    fn cost(&mut self, setup: impl FnOnce(&mut Rig)) -> u32 {
        let page = if matches!(self, Rig::Stub(_)) {
            STUB_BASE
        } else {
            BASE
        };
        setup(self);
        self.start_round(0, true, SKIP_KEY | COST, 1);
        let (pre, post) = (Rig::label("kw_pre"), Rig::label("kw_post"));
        self.step(30_000);
        self.poke(CMD, &[2]);
        let reached = self.m().run_to(page, pre, 100_000);
        assert_eq!(reached, Reached::Target, "the app never reached the call");
        let t0 = self.m().clock();
        assert_eq!(self.m().run_to(page, post, 100_000), Reached::Target);
        let t = self.m().clock().wrapping_sub(t0);
        self.poke(CMD, &[0]);
        self.step(30_000);
        assert_eq!(self.ram(CMD, 1)[0], 0, "the round did not end");
        t
    }
}

/// The round every case plays: busy with interrupts as `ints` says, `calls` calls of the routine,
/// then app_key. `events` are (microseconds from the end of the busy loop, negative before it,
/// scancode, down), and the round runs `after_us` past that end.
fn round(rig: &mut Rig, ints: bool, calls: u8, events: &[(i32, u8, bool)], after_us: u32) -> Out {
    let busy = rig.busy_us(PASSES, ints);
    let at = |t: i32| (i64::from(busy) + i64::from(t)).max(0) as u32 / STEP_US * STEP_US;
    let events: Vec<(u32, u8, bool)> = events.iter().map(|&(t, k, d)| (at(t), k, d)).collect();
    rig.start_round(PASSES, ints, 0, calls);
    rig.play(&events, busy + after_us);
    rig.out(usize::from(calls))
}

const MS: i32 = 1000;

/// The call took nothing from the OS: the scan code and the key byte are as they were, and so is
/// the ON break flag. `gain` is for a call made with interrupts off, whose `ei` lets the OS's own
/// handler run: it may then take a key that was down, or set the flag.
fn assert_unchanged(out: &Out, gain: bool) {
    let same = |i: usize| out.pre[i] == out.post[i] || gain && out.pre[0] == 0;
    assert!(same(0) && same(1), "the call changed the OS's key bytes");
    let (before, after) = (out.pre[2] & ON_INTERRUPT, out.post[2] & ON_INTERRUPT);
    assert!(
        before == after || gain && before == 0,
        "the call changed the break flag"
    );
}

/// A key down at the end of the busy loop is seen, taken by nothing, and returned by app_key.
fn held_key(rig: Rig, calls: u8) -> Out {
    let mut rig = rig;
    round(&mut rig, true, calls, &[(-230 * MS, DOWN, true)], 60_000)
}

fn assert_held(mk: fn() -> Rig) {
    let out = held_key(mk(), 2);
    println!("held across the call: {out:?}");
    assert_eq!(out.wres, [1, 1], "a key held at the end is waiting");
    assert_eq!(out.pre[0], DOWN, "the OS holds the scan code");
    assert_unchanged(&out, false);
    assert_eq!(
        (out.done, out.key),
        (2, (CODE_DOWN, 0)),
        "app_key returns it"
    );
    let control = held_key(mk(), 0);
    assert_eq!((control.done, control.key), (out.done, out.key));
}

#[test]
fn a_key_held_at_the_end_of_the_busy_loop_is_waiting_and_comes_back_from_app_key() {
    assert_held(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_key_held_at_the_end_of_the_busy_loop_is_waiting_and_comes_back() {
    assert_held(Rig::real);
}

#[test]
fn the_stub_keeps_a_scan_code_as_the_reference_os_does() {
    scan_code_timeline(Rig::stub());
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_reference_os_keeps_a_scan_code_from_the_press_to_about_42_ms_after_the_release() {
    scan_code_timeline(Rig::real());
}

/// The scan code of a key nothing asks for: set within a period of the press, kept while the key
/// is down and for 30 ms after, gone 60 ms after the release.
fn scan_code_timeline(mut rig: Rig) {
    let at = |rig: &mut Rig, us: u32| {
        rig.step(us);
        rig.ram(KBD_SCAN, 1)[0]
    };
    assert_eq!(at(&mut rig, 100_000), 0, "no key, no code");
    rig.m().press_key(DOWN);
    assert_eq!(
        at(&mut rig, 2 * PERIOD_US + 10_000),
        DOWN,
        "set at the press"
    );
    assert_eq!(at(&mut rig, 200_000), DOWN, "kept while the key is down");
    rig.m().release_key(DOWN);
    assert_eq!(at(&mut rig, 30_000), DOWN, "kept 30 ms after the release");
    assert_eq!(at(&mut rig, 30_000), 0, "gone 60 ms after the release");
}

/// A round with one key pressed at `press` ms from the end of the busy loop and released at
/// `release`, if it is; then the round's outcome, and when app_key is still waiting a tap of Left
/// and what it returned.
fn tap_outcome(
    mk: fn() -> Rig,
    ints: bool,
    press: i32,
    release: Option<i32>,
    calls: u8,
) -> (Out, u8, (u8, u8)) {
    let mut rig = mk();
    let mut events = vec![(press * MS, DOWN, true)];
    events.extend(release.map(|r| (r * MS, DOWN, false)));
    let out = round(&mut rig, ints, calls, &events, 100_000);
    if out.done == 1 {
        rig.play(&[(0, LEFT, true), (30_000, LEFT, false)], 200_000);
    }
    let key = (rig.ram(KEYA, 1)[0], rig.ram(KEYB, 1)[0]);
    (out, rig.done(), key)
}

/// The call answers `waiting` for the key pressed and released as given, takes nothing, and the
/// next app_key returns Down at once when it answered NZ and waits for the next key when it
/// answered Z, as it does with no call.
fn assert_tap(mk: fn() -> Rig, ints: bool, press: i32, release: Option<i32>, waiting: bool) {
    let (out, done, key) = tap_outcome(mk, ints, press, release, 1);
    let (control, control_done, control_key) = tap_outcome(mk, ints, press, release, 0);
    println!("ints {ints} press {press} release {release:?}: {out:?}");
    assert_eq!(out.wres, [u8::from(waiting)]);
    assert_unchanged(&out, !ints);
    assert_eq!(
        (done, key),
        (control_done, control_key),
        "app_key differs with the call"
    );
    let want = if waiting {
        (2, (CODE_DOWN, 0))
    } else {
        (2, (CODE_LEFT, 0))
    };
    assert_eq!((done, key), want, "{control:?}");
}

macro_rules! tap_tests {
    ($($name:ident, $real:ident: $ints:expr, $press:expr, $release:expr, $waiting:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_tap(Rig::stub, $ints, $press, $release, $waiting);
        }

        #[test]
        #[ignore = "needs the reference ROM image"]
        fn $real() {
            assert_tap(Rig::real, $ints, $press, $release, $waiting);
        }
    )*};
}

tap_tests! {
    a_30_ms_tap_early_in_the_busy_loop_is_gone_by_its_end,
    on_the_reference_os_a_30_ms_tap_early_in_the_busy_loop_is_gone_by_its_end:
        true, -230, Some(-200), false;
    a_5_ms_tap_early_in_the_busy_loop_is_gone_by_its_end,
    on_the_reference_os_a_5_ms_tap_early_in_the_busy_loop_is_gone_by_its_end:
        true, -230, Some(-225), false;
    a_5_ms_tap_released_75_ms_before_the_end_is_gone,
    on_the_reference_os_a_5_ms_tap_released_75_ms_before_the_end_is_gone:
        true, -80, Some(-75), false;
    a_5_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end,
    on_the_reference_os_a_5_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end:
        true, -30, Some(-25), true;
    a_30_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end,
    on_the_reference_os_a_30_ms_tap_late_in_the_busy_loop_is_waiting_at_its_end:
        true, -60, Some(-30), true;
    taps_with_interrupts_off_are_never_seen_early,
    on_the_reference_os_taps_with_interrupts_off_are_never_seen_early:
        false, -230, Some(-200), false;
    a_5_ms_tap_late_with_interrupts_off_is_never_seen,
    on_the_reference_os_a_5_ms_tap_late_with_interrupts_off_is_never_seen:
        false, -30, Some(-25), false;
    a_key_pressed_with_interrupts_off_and_still_down_is_waiting,
    on_the_reference_os_a_key_pressed_with_interrupts_off_and_still_down_is_waiting:
        false, -20, None, true;
}

/// 2nd or ALPHA tapped late in the busy loop, the call, and then `key` tapped once app_key waits:
/// what app_key returns.
fn modifier_key(mk: fn() -> Rig, modifier: u8, key: u8, calls: u8) -> (Out, (u8, u8)) {
    let mut rig = mk();
    let out = round(
        &mut rig,
        true,
        calls,
        &[(-30 * MS, modifier, true), (-25 * MS, modifier, false)],
        100_000,
    );
    assert_eq!(out.done, 1, "app_key waits for the key after the modifier");
    rig.play(&[(0, key, true), (30_000, key, false)], 200_000);
    assert_eq!(rig.done(), 2);
    (out, (rig.ram(KEYA, 1)[0], rig.ram(KEYB, 1)[0]))
}

fn assert_modifier(mk: fn() -> Rig, modifier: u8, key: u8, want: (u8, u8)) {
    let (out, got) = modifier_key(mk, modifier, key, 1);
    let (_, control) = modifier_key(mk, modifier, key, 0);
    assert_eq!(out.wres, [1], "a pending modifier is waiting");
    assert_unchanged(&out, false);
    assert_eq!(got, control, "the call changed what the key gives");
    assert_eq!(got, want);
}

#[test]
fn a_pending_2nd_is_waiting_and_the_key_after_it_gives_the_same_with_the_call() {
    assert_modifier(Rig::stub, SECOND, MODE, (CODE_QUIT, 0));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pending_2nd_is_waiting_and_the_key_after_it_gives_the_same() {
    assert_modifier(Rig::real, SECOND, MODE, (CODE_QUIT, 0));
}

#[test]
fn a_pending_alpha_is_waiting_and_the_key_after_it_gives_the_same_with_the_call() {
    assert_modifier(Rig::stub, ALPHA, SEVEN, (CODE_LETTER, b'7'));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_pending_alpha_is_waiting_and_the_key_after_it_gives_the_same() {
    assert_modifier(Rig::real, ALPHA, SEVEN, (CODE_LETTER, b'7'));
}

/// 2nd tapped, then a key within the busy loop: the key replaces the modifier's scan code, so the
/// call answers NZ and app_key returns the key with no modifier, with or without the call.
fn assert_key_over_modifier(mk: fn() -> Rig) {
    let run = |calls| {
        let mut rig = mk();
        let events = [
            (-60 * MS, SECOND, true),
            (-55 * MS, SECOND, false),
            (-30 * MS, MODE, true),
            (-25 * MS, MODE, false),
        ];
        let out = round(&mut rig, true, calls, &events, 100_000);
        assert_unchanged(&out, false);
        (out.wres.clone(), out.done, out.key)
    };
    let (wres, done, key) = run(1);
    let (_, control_done, control_key) = run(0);
    assert_eq!(wres, [1]);
    assert_eq!((done, key), (control_done, control_key));
    println!("2nd then MODE in one busy loop: done {done} key {key:?}");
}

#[test]
fn a_key_pressed_after_a_2nd_in_the_busy_loop_gives_the_same_with_the_call() {
    assert_key_over_modifier(Rig::stub);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_key_pressed_after_a_2nd_in_the_busy_loop_gives_the_same() {
    assert_key_over_modifier(Rig::real);
}

/// Alpha lock set before the round: the key tapped late in it, the call and app_key's result.
fn lock_key(calls: u8) -> (Out, (u8, u8)) {
    let mut rig = Rig::real();
    rig.start_round(1, true, 0, 0);
    rig.play(
        &[
            (20_000, SECOND, true),
            (50_000, SECOND, false),
            (100_000, ALPHA, true),
            (130_000, ALPHA, false),
            (300_000, SEVEN, true),
            (330_000, SEVEN, false),
        ],
        600_000,
    );
    assert_eq!(rig.done(), 1);
    assert_eq!(rig.ram(KEYA, 2), [CODE_LETTER, b'7'], "locked 7");
    let out = round(
        &mut rig,
        true,
        calls,
        &[(-40 * MS, SEVEN, true), (-28 * MS, SEVEN, false)],
        100_000,
    );
    let key = (rig.ram(KEYA, 1)[0], rig.ram(KEYB, 1)[0]);
    (out, key)
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_alpha_lock_gives_the_same_key_with_the_call() {
    let (out, key) = lock_key(1);
    let (_, control) = lock_key(0);
    assert_eq!(out.wres, [1]);
    assert_unchanged(&out, false);
    assert_eq!(key, control);
    assert_eq!(key, (CODE_LETTER, b'7'));
}

/// The times, in microseconds from the press, at which app_key returns while Down is held for
/// 2.5 s with `calls` calls before each app_key.
fn repeat_times(mk: fn() -> Rig, calls: u8) -> Vec<u32> {
    let mut rig = mk();
    rig.start_round(0, true, LOOP, calls);
    rig.step(50_000);
    let base = rig.done();
    rig.m().press_key(DOWN);
    let mut times = vec![];
    let mut seen = base;
    for t in (STEP_US..=2_500_000).step_by(STEP_US as usize) {
        rig.step(STEP_US);
        let d = rig.done();
        for _ in seen..d {
            times.push(t);
        }
        seen = d;
    }
    rig.m().release_key(DOWN);
    times
}

fn assert_repeat(mk: fn() -> Rig, at_least: usize) {
    let without = repeat_times(mk, 0);
    let with = repeat_times(mk, 1);
    println!("returns without the call: {without:?}");
    assert!(without.len() >= at_least, "{without:?}");
    assert_eq!(with.len(), without.len(), "{with:?} against {without:?}");
    for (a, b) in with.iter().zip(&without) {
        assert!(a.abs_diff(*b) <= PERIOD_US, "{with:?} against {without:?}");
    }
}

#[test]
fn a_held_key_repeats_at_the_same_times_with_the_call_before_every_app_key() {
    assert_repeat(Rig::stub, 1);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_a_held_arrow_repeats_at_the_same_times_with_the_call_before_every_app_key() {
    assert_repeat(Rig::real, 15);
}

/// ON pressed at `press` ms from the end of the busy loop and released at `release` (never when
/// None), with `calls` calls: the outcome, then Left tapped once ON is up.
fn on_outcome(
    mk: fn() -> Rig,
    ints: bool,
    press: i32,
    release: Option<i32>,
    calls: u8,
) -> (Out, u8, (u8, u8)) {
    let mut rig = mk();
    let mut events = vec![(press * MS, Machine::KEY_ON, true)];
    events.extend(release.map(|r| (r * MS, Machine::KEY_ON, false)));
    let out = round(&mut rig, ints, calls, &events, 100_000);
    rig.m().release_key(Machine::KEY_ON);
    rig.step(300_000);
    rig.play(&[(0, LEFT, true), (30_000, LEFT, false)], 300_000);
    let key = (rig.ram(KEYA, 1)[0], rig.ram(KEYB, 1)[0]);
    (out, rig.done(), key)
}

fn assert_on(mk: fn() -> Rig, ints: bool, press: i32, release: Option<i32>, waiting: Option<bool>) {
    let (out, done, key) = on_outcome(mk, ints, press, release, 1);
    let (_, control_done, control_key) = on_outcome(mk, ints, press, release, 0);
    println!("ON ints {ints} press {press} release {release:?}: {out:?}");
    match waiting {
        Some(w) => assert_eq!(out.wres, [u8::from(w)]),
        None => assert_eq!(out.wres[0] == 1, out.pre[2] & ON_INTERRUPT != 0),
    }
    assert_unchanged(&out, !ints);
    assert_eq!(
        (done, key),
        (control_done, control_key),
        "app_key differs with the call"
    );
    assert_eq!((done, key), (2, (CODE_LEFT, 0)), "ON is never returned");
}

#[test]
fn on_held_through_the_end_of_the_busy_loop_is_waiting() {
    assert_on(Rig::stub, true, -100, None, Some(true));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_held_through_the_end_of_the_busy_loop_is_waiting() {
    assert_on(Rig::real, true, -100, None, Some(true));
}

#[test]
fn on_pressed_with_interrupts_off_and_still_down_is_waiting() {
    assert_on(Rig::stub, false, -20, None, Some(true));
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_on_pressed_with_interrupts_off_and_still_down_is_waiting() {
    assert_on(Rig::real, false, -20, None, Some(true));
}

#[test]
fn an_on_tap_in_the_busy_loop_is_waiting_exactly_when_the_os_flagged_it() {
    assert_on(Rig::stub, true, -200, Some(-170), None);
}

#[test]
#[ignore = "needs the reference ROM image"]
fn on_the_reference_os_an_on_tap_in_the_busy_loop_is_waiting_exactly_when_the_os_flagged_it() {
    assert_on(Rig::real, true, -200, Some(-170), None);
}

/// The most T-states a call may take, with the call: the whole routine (193 measured) and the
/// early answers from the scan code and the break flag (51 measured).
const COST_FULL: u32 = 210;
const COST_EARLY: u32 = 60;

fn assert_cost(mut rig: Rig, name: &str) {
    let none = rig.cost(|_| {});
    let code = rig.cost(|r| r.poke(KBD_SCAN, &[DOWN]));
    let flag = rig.cost(|r| {
        r.poke(0x89F0 + 9 - RAM_BASE, &[ON_INTERRUPT]);
    });
    let held = rig.cost(|r| {
        r.poke(0x89F0 + 9 - RAM_BASE, &[0]);
        r.m().press_key(DOWN);
        r.step(100_000);
        r.poke(KBD_SCAN, &[0]);
    });
    println!("{name} T-states: nothing {none}, scan code {code}, ON flag {flag}, key down {held}");
    assert!(none <= COST_FULL && held <= COST_FULL, "{none} {held}");
    assert!(code <= COST_EARLY && flag <= COST_EARLY, "{code} {flag}");
}

#[test]
fn the_call_costs_under_210_t_states_on_every_path_on_the_stub() {
    assert_cost(Rig::stub(), "stub");
}

#[test]
#[ignore = "needs the reference ROM image"]
fn the_call_costs_under_210_t_states_on_every_path_on_the_reference_os() {
    assert_cost(Rig::real(), "ROM");
}
