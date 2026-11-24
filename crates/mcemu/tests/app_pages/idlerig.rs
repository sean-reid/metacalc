//! A kind whose idle routine is a script, on the stub OS and on the reference OS. The app under test
//! is the shipped bytes: only the idle slot of a kind's row in the kind table is patched, to
//! routines/idle_script.z80 in RAM, which burns the time and answers the action a slice says.

use crate::app::*;
pub use crate::realos::KEY_2 as LAUNCH_KEY;
use crate::realos::{BASE, KEY_2, KEY_APPS, ON_INTERRUPT, RealOs};
use crate::regions::declared;
use crate::shellrig::*;
use mcemu::{Machine, STUB_ROM, Stop};

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/idle_script.bin"));
const STUB_OS_LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/os.lab"));

/// Where the routine and its state sit; build.rs assembles the routine for IDLE and the same
/// counters the rig keeps (BLITS_AT and KEYS_AT in shellrig.rs).
pub const IDLE: u16 = 0xB700;
pub const STATE: u16 = 0xB800;
const LOG: u16 = STATE + 16;
const SLICES: u16 = STATE + 272;
const MAX_SLICES: usize = 16;
const END: u16 = SLICES + 32 * MAX_SLICES as u16;
/// T-states of one pass of the routine's burn loop.
pub const PASS: u32 = 3_356;
/// The OS's break flag byte and the scan code byte.
pub const KBD_SCAN: u16 = 0x843F;
/// The OS's byte for the key it last reported.
pub const KBD_KEY: u16 = 0x8445;
const FLAGS: u16 = 0x89F0;
pub const APD_TIMERS: u16 = 0x8448;

pub const STAY: u8 = 0;
pub const REDRAW: u8 = 1;
pub const PUSH: u8 = 2;
pub const POP: u8 = 3;

/// One call of the idle routine.
#[derive(Clone, Copy)]
pub struct Slice {
    passes: u16,
    ints_off: bool,
    action: u8,
    carry: bool,
    ticks: u16,
    frame: [u8; 16],
}

impl Slice {
    /// A slice that burns `ms` milliseconds at 6 MHz with interrupts on, answers SCR_STAY and says
    /// no more work is pending.
    pub fn ms(ms: u32) -> Slice {
        Slice {
            passes: (ms * 6_000 / PASS) as u16,
            ints_off: false,
            action: STAY,
            carry: false,
            ticks: 0,
            frame: [0; 16],
        }
    }

    /// Burns with interrupts off and turns them on before it returns.
    pub fn off(mut self) -> Slice {
        self.ints_off = true;
        self
    }

    /// Says more work is pending.
    pub fn more(mut self) -> Slice {
        self.carry = true;
        self
    }

    pub fn answers(mut self, action: u8) -> Slice {
        self.action = action;
        self
    }

    /// Calls app_tick `n` times after the burn.
    pub fn ticks(mut self, n: u16) -> Slice {
        self.ticks = n;
        self
    }

    /// Answers SCR_PUSH with this frame in scr_new.
    pub fn pushes(mut self, kind: u8, number: u8) -> Slice {
        self.action = PUSH;
        self.frame = [0; 16];
        self.frame[0] = kind;
        self.frame[1] = number;
        self
    }

    /// The pushed frame's parameter byte.
    pub fn param(mut self, p: u8) -> Slice {
        self.frame[2] = p;
        self
    }

    fn bytes(&self) -> Vec<u8> {
        let mut b = vec![0; 32];
        b[..2].copy_from_slice(&self.passes.to_le_bytes());
        b[2] = u8::from(self.ints_off);
        b[3] = self.action;
        b[4] = u8::from(self.carry);
        b[5..7].copy_from_slice(&self.ticks.to_le_bytes());
        b[16..].copy_from_slice(&self.frame);
        b
    }
}

/// The slices and the kinds whose idle is the routine; the other kinds keep the shipped idle.
pub struct Script {
    pub slices: Vec<Slice>,
    pub forever: bool,
    /// The script starts again at slice 0 after every scr_key call.
    pub restart: bool,
    pub kinds: Vec<u8>,
}

/// One slice the routine ran: its index, the counters of blits and of scr_key calls when it began,
/// and whether interrupts were on at its first instruction.
#[derive(Debug, PartialEq, Eq)]
pub struct Ran {
    pub index: u8,
    pub blits: u16,
    pub keys: u16,
    pub ints_on: bool,
}

/// The OS bytes an ON press can change.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OsState {
    pub flags: u8,
    pub scan: u8,
    pub key: u8,
    pub timers: (u8, u8),
    pub mask: u8,
}

pub struct State {
    pub calls: u16,
    pub logged: u8,
    pub over: u8,
    pub index: u8,
    pub in_slice: bool,
}

pub fn script(kinds: &[u8], slices: &[Slice]) -> Script {
    Script {
        slices: slices.to_vec(),
        forever: false,
        restart: false,
        kinds: kinds.to_vec(),
    }
}

/// A test body for the stub OS and again for the reference OS, where it is ignored.
macro_rules! twin {
    ($stub:ident, $real:ident, $body:ident) => {
        #[test]
        fn $stub() {
            $body(false);
        }
        #[test]
        #[ignore = "needs the reference ROM image"]
        fn $real() {
            $body(true);
        }
    };
}
pub(crate) use twin;

pub struct Rig {
    pub shell: Shell,
    pub real: bool,
    /// Microseconds run since the rig started.
    pub t: u32,
}

/// Panics when the routine's range overlaps a region of the app, a driver or a hook, or, on the
/// reference OS, the OS's own free-RAM bounds.
fn assert_free(built: &Built) {
    let mut busy: Vec<(String, u16, u16)> = declared(&built.labels)
        .into_iter()
        .filter(|&(from, _)| from >= 0x8000)
        .map(|(from, to)| ("an app region".to_string(), from, to))
        .collect();
    busy.extend(
        DRIVER_RAM
            .iter()
            .map(|&(a, b)| ("the driver".to_string(), a, b)),
    );
    busy.push(("the rig's hooks".to_string(), HOOKS.0, HOOKS.1));
    for (what, from, to) in busy {
        assert!(
            to <= IDLE || END <= from,
            "the idle routine's RAM {IDLE:04X} to {END:04X} overlaps {what} at {from:04X} to {to:04X}"
        );
    }
}

/// Nothing has used the routine's RAM on the reference OS yet: it is all zeros.
fn assert_os_leaves_it(m: &mut Machine) {
    let bytes = m.ram_page(1)[usize::from(IDLE - 0x8000)..usize::from(END - 0x8000)].to_vec();
    assert!(
        bytes.iter().all(|&b| b == 0),
        "the OS has written in {IDLE:04X} to {END:04X}"
    );
}

fn install(m: &mut Machine, built: &Built, script: &Script) {
    assert_free(built);
    assert!(script.slices.len() <= MAX_SLICES);
    m.load_ram(STATE, &vec![0; usize::from(END - STATE)]);
    m.load_ram(IDLE, CODE);
    let mut state = vec![0; 16];
    state[0] = script.slices.len() as u8;
    state[2] = u8::from(script.forever);
    state[8..10].copy_from_slice(&built.label("scr_new").to_le_bytes());
    state[10..12].copy_from_slice(&built.label("app_tick").to_le_bytes());
    state[13] = u8::from(script.restart);
    m.load_ram(STATE, &state);
    for (i, slice) in script.slices.iter().enumerate() {
        m.load_ram(SLICES + 32 * i as u16, &slice.bytes());
    }
    for &kind in &script.kinds {
        patch_idle(m, built, kind, IDLE);
    }
}

impl Rig {
    /// The app on the stub OS, launched and not yet run.
    pub fn stub(built: &'static Built, enter: Enter, script: &Script) -> Rig {
        let mut m = Machine::new(STUB_ROM).expect("stub ROM boots");
        m.install_app(&built.file, BASE).expect("the app installs");
        m.launch(BASE).expect("the app launches");
        let heap = m.heap_end();
        assert!(heap < HOOKS.0, "the stub heap ends at {heap:04X}");
        install_hooks(&mut m, built, enter);
        install(&mut m, built, script);
        Rig {
            shell: Shell {
                app: App::with(m, built),
                patience: 1,
            },
            real: false,
            t: 0,
        }
    }

    /// The app on the reference OS, started from the APPS menu with a tap short enough that the
    /// key is up before the first frame is done. Returns when the first frame is blitted.
    pub fn real(built: &'static Built, enter: Enter, script: &Script) -> Rig {
        Rig::real_launched(built, enter, script, false)
    }

    /// `real`, with the key that starts the app still down when it returns if `hold`.
    pub fn real_launched(built: &'static Built, enter: Enter, script: &Script, hold: bool) -> Rig {
        let mut os = RealOs::boot(&built.file);
        assert_os_leaves_it(&mut os.m);
        install_hooks(&mut os.m, built, enter);
        install(&mut os.m, built, script);
        os.tap(KEY_APPS, 150, 600);
        os.m.press_key(KEY_2);
        if !hold {
            os.run_ms(20);
            os.m.release_key(KEY_2);
        }
        let mut rig = Rig {
            shell: Shell {
                app: App::on_real_os_in(os.m, built),
                patience: 1,
            },
            real: true,
            t: 0,
        };
        rig.until("the first frame", 2_000_000, |r| r.shell.blits() >= 1);
        rig.t = 0;
        rig
    }

    /// The app started with `LAUNCH_KEY` down and taken by the OS, as the key that starts it from
    /// the APPS menu is: on the reference OS the menu takes the key's code and the app is
    /// entered with the key still down; on the stub the key is pressed after the launch and the
    /// OS's last-key byte is set, so the stub's scan does not set a code for it.
    pub fn launched_holding(
        real: bool,
        built: &'static Built,
        enter: Enter,
        script: &Script,
    ) -> Rig {
        if real {
            return Rig::real_launched(built, enter, script, true);
        }
        let mut rig = Rig::stub(built, enter, script);
        rig.m().press_key(LAUNCH_KEY);
        rig.m().load_ram(KBD_KEY, &[LAUNCH_KEY]);
        rig
    }

    pub fn new(real: bool, built: &'static Built, enter: Enter, script: &Script) -> Rig {
        if real {
            Rig::real(built, enter, script)
        } else {
            Rig::stub(built, enter, script)
        }
    }

    pub fn m(&mut self) -> &mut Machine {
        &mut self.shell.app.m
    }

    /// Runs `us` microseconds of emulated time at 6 MHz.
    pub fn step(&mut self, us: u32) {
        self.t += us;
        let real = self.real;
        let m = self.m();
        let stop = if real {
            m.run_time(us).stop
        } else {
            m.run_app(us * 6).stop
        };
        assert_eq!(stop, Stop::Limit, "the machine stopped");
    }

    /// Steps 0.5 ms at a time until `done`, and returns the time it took, or panics after `max`.
    pub fn until(&mut self, what: &str, max: u32, mut done: impl FnMut(&mut Rig) -> bool) -> u32 {
        let start = self.t;
        while !done(self) {
            assert!(self.t - start < max, "{what}: not reached in {max} us");
            self.step(500);
        }
        self.t - start
    }

    /// Runs `us` in steps of 0.5 ms, calling `each` after every step.
    pub fn watch(&mut self, us: u32, mut each: impl FnMut(&mut Rig)) {
        let end = self.t + us;
        while self.t < end {
            self.step(500);
            each(self);
        }
    }

    /// RAM in bank B, from RAM page 1 whatever the OS has mapped since the app left.
    pub fn ram(&mut self, at: u16, len: usize) -> Vec<u8> {
        assert!((0x8000..0xC000).contains(&at) && usize::from(at) + len <= 0xC000);
        let from = usize::from(at - 0x8000);
        self.m().ram_page(1)[from..from + len].to_vec()
    }

    /// SCR_STACK filled with spaces and SCR_VARS with zeros, the home screen's state.
    pub fn assert_put_away(&mut self) {
        let label = |rig: &Rig, n: &str| rig.shell.app.label(n);
        let (stack, vars) = (label(self, "SCR_STACK"), label(self, "SCR_VARS"));
        let sizes = (label(self, "SCR_STACK_SIZE"), label(self, "SCR_VARS_SIZE"));
        let (stack_size, vars_size) = (usize::from(sizes.0), usize::from(sizes.1));
        assert_eq!(self.ram(stack, stack_size), vec![0x20; stack_size]);
        assert_eq!(self.ram(vars, vars_size), vec![0; vars_size]);
    }

    pub fn state(&mut self) -> State {
        let s = self.ram(STATE, 8);
        State {
            calls: u16::from_le_bytes([s[4], s[5]]),
            logged: s[6],
            over: s[7],
            index: s[1],
            in_slice: s[3] != 0,
        }
    }

    /// The slices the routine has begun, in order.
    pub fn ran(&mut self) -> Vec<Ran> {
        let n = usize::from(self.state().logged);
        self.ram(LOG, 8 * n)
            .chunks(8)
            .map(|e| Ran {
                index: e[0],
                blits: u16::from_le_bytes([e[1], e[2]]),
                keys: u16::from_le_bytes([e[3], e[4]]),
                ints_on: e[5] != 0,
            })
            .collect()
    }

    /// Runs until the app has left through app_exit, which the stub reports and which on the
    /// reference OS is the end of app_putaway, before the OS has the machine back. Panics after
    /// `max` microseconds.
    pub fn run_until_left(&mut self, max: u32) {
        if self.real {
            self.run_to_label("app_putaway_flags", max * 6);
            return;
        }
        let start = self.t;
        loop {
            assert!(self.t - start < max, "the app did not leave in {max} us");
            self.t += 500;
            if self.m().run_app(3_000).stop == Stop::Exited {
                return;
            }
        }
    }

    pub fn scan_code(&mut self) -> u8 {
        self.ram(KBD_SCAN, 1)[0]
    }

    /// The OS's ON break flag.
    pub fn on_flag(&mut self) -> bool {
        self.ram(FLAGS + 9, 1)[0] & ON_INTERRUPT != 0
    }

    /// The shell's APP_HELD byte, from the build's label file.
    pub fn held(&mut self) -> u8 {
        let at = self.shell.app.label("APP_HELD");
        self.ram(at, 1)[0]
    }

    /// The OS's state an ON press leaves: the flag byte of onInterrupt, the scan code, the key
    /// byte, the two power down timer bytes and the interrupt mask port.
    pub fn os_state(&mut self) -> OsState {
        let flags = self.ram(FLAGS + 9, 1)[0];
        let timers = self.apd_timers();
        let scan = self.scan_code();
        let key = self.ram(KBD_KEY, 1)[0];
        let mask = self.m().in_port(3);
        OsState {
            flags,
            scan,
            key,
            timers,
            mask,
        }
    }

    pub fn apd_timers(&mut self) -> (u8, u8) {
        let b = self.ram(APD_TIMERS, 2);
        (b[0], b[1])
    }

    /// Presses `scan`, runs until the app is about to give it to scr_key, and releases it.
    pub fn tap_when_taken(&mut self, scan: u8) {
        self.m().press_key(scan);
        self.run_to_label("scr_key", 3_000_000);
        self.m().release_key(scan);
    }

    pub fn run_to_label(&mut self, label: &str, limit: u32) {
        let at = self.shell.app.label(label);
        let start = self.m().clock();
        loop {
            match self.m().run_to(BASE, at, limit) {
                mcemu::Reached::Target => return,
                mcemu::Reached::Vector38 => {}
                mcemu::Reached::Limit => panic!("the app never reached {label}"),
            }
            let spent = self.m().clock().wrapping_sub(start);
            assert!(
                spent < limit,
                "the app never reached {label} in {limit} T-states"
            );
        }
    }

    /// The program counter and port 6, which says where the app waits.
    pub fn at(&mut self) -> (u8, u16) {
        let page = self.m().in_port(6);
        (page, self.m().pc())
    }

    /// The stub OS's key wait: the code `_GetKey` runs.
    pub fn in_stub_key_wait(pc: u16) -> bool {
        let at = |name: &str| {
            STUB_OS_LABELS
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(" = $")?;
                    k.eq_ignore_ascii_case(name)
                        .then(|| u16::from_str_radix(v, 16).unwrap())
                })
                .unwrap_or_else(|| panic!("the stub has no label {name}"))
        };
        (0x0038..at("bcall")).contains(&pc) || (at("get_key")..at("key_plain")).contains(&pc)
    }
}
