//! What the tests read of the rig: the routine's state and log, the OS bytes a key or an ON press
//! changes, and where the app is waiting.

use crate::idlerig::Rig;
use crate::idlerig_script::{LOG, STATE};
use crate::realos::{BASE, ON_INTERRUPT};

const STUB_OS_LABELS: &str = include_str!(concat!(env!("OUT_DIR"), "/os.lab"));

/// The OS's break flag byte and the scan code byte.
pub const KBD_SCAN: u16 = 0x843F;
/// The OS's byte for the key it last reported.
pub const KBD_KEY: u16 = 0x8445;
const FLAGS: u16 = 0x89F0;
pub const APD_TIMERS: u16 = 0x8448;

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

/// What app_key_waiting answered.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub a: u8,
    pub zero: bool,
    pub carry: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Where {
    OsKeyWait,
    WaitingForOnUp,
    Elsewhere(u8, u16),
}

pub struct State {
    pub calls: u16,
    pub logged: u8,
    pub over: u8,
    pub index: u8,
    pub in_slice: bool,
}

impl Rig {
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

    /// The shell's APP_ON byte, from the build's label file.
    pub fn app_on(&mut self) -> u8 {
        let at = self.shell.app.label("APP_ON");
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

    /// Runs to the instruction after the call of app_key_waiting at the idle point and returns the
    /// registers there: A, the flags, and so what the call answered.
    pub fn answer_at_the_idle_point(&mut self) -> Answer {
        let at = self.shell.app.label("app_idle") + 3;
        self.run_to_addr("the return of app_key_waiting", at, 6_000_000);
        let af = self.m().regs().af;
        Answer {
            a: (af >> 8) as u8,
            zero: af & 0x40 != 0,
            carry: af & 1 != 0,
        }
    }

    /// Where the app is: in the OS's key wait, in app_key's wait for ON to go up, or in other code
    /// of the app.
    pub fn whereabouts(&mut self) -> Where {
        let (page, pc) = self.at();
        let on = self.shell.app.label("ak_on");
        if page == BASE && (on..on + 9).contains(&pc) {
            Where::WaitingForOnUp
        } else if (self.real && page != BASE) || (!self.real && Rig::in_stub_key_wait(pc)) {
            Where::OsKeyWait
        } else {
            Where::Elsewhere(page, pc)
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
