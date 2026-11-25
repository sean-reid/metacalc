//! The scripted idle routine as the tests lay it out: where it and its state sit in RAM, the slices a
//! test writes, and the install that patches a kind's idle slot to it.

use crate::app::*;
use crate::regions::declared;
use crate::shellrig::*;
use mcemu::Machine;

pub const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/idle_script.bin"));
const DEFINES: &str = include_str!(concat!(env!("OUT_DIR"), "/idle_script.defs"));

/// Where the routine and its state sit; build.rs assembles the routine for ENTRY = IDLE, STATE and
/// the rig's counters BLITS and SCR_KEYS (shellrig.rs), and install checks they are the same.
pub const IDLE: u16 = 0xB700;
pub const STATE: u16 = 0xB800;
pub const LOG: u16 = STATE + 16;
pub const SLICES: u16 = STATE + 272;
const MAX_SLICES: usize = 16;
pub const END: u16 = SLICES + 32 * MAX_SLICES as u16;
/// T-states of one pass of the routine's burn loop.
pub const PASS: u32 = 3_356;

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

pub fn script(kinds: &[u8], slices: &[Slice]) -> Script {
    Script {
        slices: slices.to_vec(),
        forever: false,
        restart: false,
        kinds: kinds.to_vec(),
    }
}

/// Panics unless the defines build.rs gave the routine are the Rust constants of the rig.
fn assert_defines() {
    let want = [
        ("ENTRY", IDLE),
        ("STATE", STATE),
        ("BLITS", BLITS),
        ("KEYS", SCR_KEYS),
    ];
    let got: Vec<(&str, u16)> = DEFINES
        .lines()
        .map(|l| {
            let (name, value) = l.split_once("=$").expect("a define NAME=$ADDRESS");
            (name, u16::from_str_radix(value, 16).unwrap())
        })
        .collect();
    assert_eq!(got, want, "build.rs and idlerig.rs place the routine alike");
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
pub fn assert_os_leaves_it(m: &mut Machine) {
    let bytes = m.ram_page(1)[usize::from(IDLE - 0x8000)..usize::from(END - 0x8000)].to_vec();
    assert!(
        bytes.iter().all(|&b| b == 0),
        "the OS has written in {IDLE:04X} to {END:04X}"
    );
}

pub fn install(m: &mut Machine, built: &Built, script: &Script) {
    assert_defines();
    assert_free(built);
    assert!(script.slices.len() <= MAX_SLICES);
    m.load_ram(STATE, &vec![0; usize::from(END - STATE)]);
    m.load_ram(IDLE, CODE);
    let mut state = vec![0; 16];
    state[0] = script.slices.len() as u8;
    state[2] = u8::from(script.forever);
    state[8..10].copy_from_slice(&built.label("scr_new").to_le_bytes());
    state[10..12].copy_from_slice(&built.try_label("app_tick").unwrap_or(0).to_le_bytes());
    state[13] = u8::from(script.restart);
    m.load_ram(STATE, &state);
    for (i, slice) in script.slices.iter().enumerate() {
        m.load_ram(SLICES + 32 * i as u16, &slice.bytes());
    }
    for &kind in &script.kinds {
        patch_idle(m, built, kind, IDLE);
    }
}

/// The ranges of the routine's RAM the routine and the test write: its code, its state and log,
/// and the slices the test wrote.
pub fn written(slices: usize) -> [(u16, u16); 3] {
    [
        (IDLE, IDLE + CODE.len() as u16),
        (STATE, SLICES),
        (SLICES, SLICES + 32 * slices as u16),
    ]
}
