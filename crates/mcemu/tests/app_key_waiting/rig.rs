use crate::realos::*;
use mcemu::{Machine, Reached, STUB_ROM, Speed, Stop};

const APP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/keys_wait_app.8xk"));
const LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/keys_wait_app.lab"));

pub const STUB_BASE: u8 = 0x30;
pub const RAM_BASE: usize = 0x8000;
const CMD: usize = 0xA800 - RAM_BASE;
const BUSY: usize = 0xA802 - RAM_BASE;
const INTS: usize = 0xA804 - RAM_BASE;
const FLAGS: usize = 0xA805 - RAM_BASE;
const CALLS: usize = 0xA806 - RAM_BASE;
const DONE: usize = 0xA808 - RAM_BASE;
const KEYA: usize = 0xA809 - RAM_BASE;
const KEYB: usize = 0xA80A - RAM_BASE;
const IFF: usize = 0xA80B - RAM_BASE;
const BEND: usize = 0xA80C - RAM_BASE;
const WRES: usize = 0xA810 - RAM_BASE;
const PRE: usize = 0xA820 - RAM_BASE;
const POST: usize = 0xA824 - RAM_BASE;
const LOG: usize = 0xA900 - RAM_BASE;
pub const KBD_SCAN: usize = 0x843F - RAM_BASE;
pub const ON_FLAGS_AT: usize = 0x89F0 + 9 - RAM_BASE;

pub const SKIP_KEY: u8 = 1;
pub const LOOP: u8 = 2;
pub const COST: u8 = 4;
pub const POLL: u8 = 8;

/// Scancodes, and the codes app_key gives for the ones the tests press.
pub const DOWN: u8 = 0x01;
pub const LEFT: u8 = 0x02;
pub const SECOND: u8 = 0x36;
pub const ALPHA: u8 = 0x30;
pub const MODE: u8 = 0x37;
pub const SEVEN: u8 = 0x24;
pub const CODE_DOWN: u8 = 2;
pub const CODE_LEFT: u8 = 3;
pub const CODE_LETTER: u8 = 12;
pub const CODE_QUIT: u8 = 13;

/// A busy pass is about 3,370 T-states: this many are about 250 ms at 6 MHz.
pub const PASSES: u16 = 445;
/// One timer interrupt, in microseconds.
pub const PERIOD_US: u32 = 1953;
/// One scan of the keypad, in microseconds, as the reference OS does it.
pub const SCAN_US: u32 = 9200;
pub const STEP_US: u32 = 500;
pub const MS: i32 = 1000;

pub enum Rig {
    Stub(Machine),
    Real(RealOs),
}

/// What a round left. `returned` is the time app_key returned, in microseconds from the start of
/// the round, None while it still waits; `saw_code` says kbdScanCode held a code at some step
/// before the busy loop ended, which the app reports at BEND.
#[derive(Debug)]
pub struct Out {
    pub wres: Vec<u8>,
    pub pre: Vec<u8>,
    pub post: Vec<u8>,
    pub done: u8,
    pub key: (u8, u8),
    pub iff: u8,
    pub returned: Option<u32>,
    pub saw_code: bool,
}

/// What `play` saw: the time at which the app's log first grew, and kbdScanCode after each step.
pub struct Trace {
    pub returned: Option<u32>,
    pub codes: Vec<(u32, u8, bool)>,
}

impl Rig {
    pub fn stub() -> Rig {
        let mut m = Machine::new(STUB_ROM).unwrap();
        m.install_app(APP, STUB_BASE).unwrap();
        m.launch(STUB_BASE).unwrap();
        let mut rig = Rig::Stub(m);
        rig.step(20_000);
        rig
    }

    pub fn real() -> Rig {
        let mut os = RealOs::boot(APP);
        os.tap(KEY_APPS, 150, 600);
        os.tap(KEY_2, 150, 300);
        os.run_ms(500);
        Rig::Real(os)
    }

    pub fn m(&mut self) -> &mut Machine {
        match self {
            Rig::Stub(m) => m,
            Rig::Real(os) => &mut os.m,
        }
    }

    pub fn step(&mut self, us: u32) {
        match self {
            Rig::Stub(m) => assert_eq!(m.run_app(us * 6).stop, Stop::Limit),
            Rig::Real(os) => os.run_us(us),
        }
    }

    pub fn ram(&mut self, at: usize, len: usize) -> Vec<u8> {
        self.m().ram_page(1)[at..at + len].to_vec()
    }

    pub fn poke(&mut self, at: usize, bytes: &[u8]) {
        self.m().ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// Runs `us` of emulated time, pressing and releasing as `events` say, each (microsecond
    /// from now, scancode, down).
    pub fn play(&mut self, events: &[(u32, u8, bool)], us: u32) -> Trace {
        let first = self.done();
        let mut trace = Trace {
            returned: None,
            codes: vec![],
        };
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
            if trace.returned.is_none() && self.done() != first {
                trace.returned = Some(t);
            }
            let ended = self.ram(BEND, 1)[0] != 0;
            trace.codes.push((t, self.ram(KBD_SCAN, 1)[0], ended));
        }
        trace
    }

    pub fn start_round(&mut self, passes: u16, ints: bool, flags: u8, calls: u8) {
        self.poke(BUSY, &passes.to_le_bytes());
        self.poke(INTS, &[u8::from(ints)]);
        self.poke(FLAGS, &[flags]);
        self.poke(CALLS, &[calls]);
        self.poke(CMD, &[1]);
    }

    /// The microseconds a round of `passes` takes, measured with no key and no call.
    pub fn busy_us(&mut self, passes: u16, ints: bool) -> u32 {
        self.start_round(passes, ints, SKIP_KEY, 0);
        let mut t = 0;
        while self.ram(CMD, 1)[0] != 0 {
            assert!(t < 2_000_000, "the round never ended");
            self.step(STEP_US);
            t += STEP_US;
        }
        t
    }

    pub fn out(&mut self, calls: usize) -> Out {
        Out {
            wres: self.ram(WRES, calls),
            pre: self.ram(PRE, 3),
            post: self.ram(POST, 3),
            done: self.ram(DONE, 1)[0],
            key: self.key(),
            iff: self.ram(IFF, 1)[0],
            returned: None,
            saw_code: false,
        }
    }

    pub fn done(&mut self) -> u8 {
        self.ram(DONE, 1)[0]
    }

    pub fn key(&mut self) -> (u8, u8) {
        (self.ram(KEYA, 1)[0], self.ram(KEYB, 1)[0])
    }

    /// The keys app_key returned, in order.
    pub fn log(&mut self) -> Vec<u8> {
        let n = usize::from(self.done());
        self.ram(LOG, n)
    }

    pub fn label(name: &str) -> u16 {
        LABELS
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(" = ")?;
                (k.eq_ignore_ascii_case(name)).then(|| u16::from_str_radix(&v[1..], 16).unwrap())
            })
            .unwrap_or_else(|| panic!("no label {name}"))
    }

    /// The T-states of one call of app_key_waiting made with interrupts off, from the call to the
    /// instruction after it, at `speed`, once `setup` has set the world up.
    pub fn cost(&mut self, speed: Speed, setup: impl FnOnce(&mut Rig)) -> u32 {
        let page = if matches!(self, Rig::Stub(_)) {
            STUB_BASE
        } else {
            BASE
        };
        self.m().set_speed(speed);
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
        self.m().set_speed(Speed::Mhz6);
        t
    }
}

/// The round every case plays: busy with interrupts as `ints` says, `calls` calls of the routine,
/// then app_key. `events` are (microseconds from the end of the busy loop, negative before it,
/// scancode, down), and the round runs `after_us` past that end.
pub fn round(
    rig: &mut Rig,
    ints: bool,
    calls: u8,
    events: &[(i32, u8, bool)],
    after_us: u32,
) -> Out {
    let busy = rig.busy_us(PASSES, ints);
    let at = |t: i32| (i64::from(busy) + i64::from(t)).max(0) as u32 / STEP_US * STEP_US;
    let events: Vec<(u32, u8, bool)> = events.iter().map(|&(t, k, d)| (at(t), k, d)).collect();
    rig.start_round(PASSES, ints, 0, calls);
    let trace = rig.play(&events, busy + after_us);
    let mut out = rig.out(usize::from(calls));
    out.returned = trace.returned;
    out.saw_code = trace.codes.iter().any(|&(_, c, ended)| !ended && c != 0);
    out
}

/// The call took nothing from the OS: the scan code and the key byte are as they were, and so is
/// the ON break flag. `gain` is for a call made with interrupts off, whose `ei` lets the OS's own
/// handler run: it may then take a key that was down, or set the flag.
pub fn assert_unchanged(out: &Out, gain: bool) {
    let same = |i: usize| out.pre[i] == out.post[i] || gain && out.pre[0] == 0;
    assert!(same(0) && same(1), "the call changed the OS's key bytes");
    let (before, after) = (out.pre[2] & ON_INTERRUPT, out.post[2] & ON_INTERRUPT);
    assert!(
        before == after || gain && before == 0,
        "the call changed the break flag"
    );
}

/// app_key returned at the same time with the call and without it, within one interrupt period
/// (the call can move the OS's next scan, which sets the code, by that much), or kept waiting in
/// both.
pub fn assert_same_time(with: Option<u32>, without: Option<u32>) {
    match (with, without) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(
            a.abs_diff(b) <= PERIOD_US,
            "app_key returned at {a} us with the call and {b} us without"
        ),
        _ => panic!("app_key returned at {with:?} us with the call and {without:?} us without"),
    }
}
