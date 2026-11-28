//! The checking screen run between keys on the stub OS: the app's own idle loop with the real
//! kernel and the status table, stopped at each frame it is about to blit, with the host model's
//! frame as the oracle, and a counter on every call of the kernel's verify.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shellrig::*;
use crate::table;
use crate::ui_support::same_frame;
use mcemu::{Machine, Reached};
use mcui::{Installed, Key, Status};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

#[path = "checkrig_scans.rs"]
mod scans_count;
use scans_count::RELEASE_SCANS;
pub use scans_count::Scans;

/// The counters of the kernel's verify, crc_begin and crc_chunk entries and their hooks, in RAM the
/// rig's hooks leave free.
pub const VERIFIES: u16 = 0xBC00;
const CRC_BEGINS: u16 = 0xBC02;
const CRC_CHUNKS: u16 = 0xBC04;
const HOOKS_AT: u16 = 0xBC10;
/// The byte of the OS flags that holds onInterrupt.
const ON_FLAGS: u16 = 0x89F9;
/// The most T-states one wait for a frame or a key may take.
pub const WAIT: u32 = 600_000_000;
/// The most T-states the wait for five scans may take (five scans of 9.28 ms at 15 MHz are 696,000),
/// and the steps it runs in.
const RELEASE_LIMIT: u32 = 1_200_000;
const RELEASE_STEP: u32 = 3_000;
/// The steps between two frames of the screen, CKS_EVERY in screen_checking.inc.
pub const EVERY: usize = 32;

fn scans(shared: &Mutex<Scans>) -> MutexGuard<'_, Scans> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct Run {
    pub shell: Shell,
    pub built: &'static Built,
    /// Whether the machine runs the reference OS, whose variables are read from its symbol table.
    pub real: bool,
    /// The keypad scans since the key last let go with `release`, shared with the machine's press hook.
    scans: Arc<Mutex<Scans>>,
    pub released_at: u32,
}

/// Puts the counters and hooks of the kernel's entries in a machine the app is installed on.
pub fn hook_kernel(m: &mut Machine, built: &Built) {
    let far = built.label("far_page2").to_le_bytes();
    m.load_ram(VERIFIES, &[0; 6]);
    for (i, (stub, counter)) in [
        ("far_kernel_verify", VERIFIES),
        ("far_kernel_crc_begin", CRC_BEGINS),
        ("far_kernel_crc_chunk", CRC_CHUNKS),
    ]
    .into_iter()
    .enumerate()
    {
        let at = HOOKS_AT + 0x10 * i as u16;
        let [c0, c1] = counter.to_le_bytes();
        m.load_ram(
            at,
            &[
                0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0xC3, far[0], far[1],
            ],
        );
        hook_stub(m, built, stub, at);
    }
}

impl Run {
    /// The app on `built` booted on the stub OS with `data` as the status table if there is one,
    /// and a counter on the kernel's verify.
    pub fn boot(built: &'static Built, data: Option<&[u8]>) -> Run {
        let mut shell = booted(built, data);
        hook_kernel(&mut shell.app.m, built);
        Run {
            shell,
            built,
            real: false,
            scans: Arc::default(),
            released_at: 0,
        }
    }

    /// A run over a shell on the reference OS whose machine has the rig's hooks and the kernel's
    /// counters installed.
    pub fn from_shell(shell: Shell, built: &'static Built) -> Run {
        Run {
            shell,
            built,
            real: true,
            scans: Arc::default(),
            released_at: 0,
        }
    }

    /// Bytes of bank B, read from RAM page 1 whatever the OS has mapped there at the moment.
    pub fn peek(&mut self, at: u16, len: usize) -> Vec<u8> {
        assert!((0x8000..0xC000).contains(&at) && usize::from(at) + len <= 0xC000);
        let from = usize::from(at - 0x8000);
        self.m().ram_page(1)[from..from + len].to_vec()
    }

    pub fn blits(&mut self) -> u16 {
        let b = self.peek(BLITS, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    pub fn scr_keys(&mut self) -> u16 {
        let b = self.peek(SCR_KEYS, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// The frame on the canvas.
    pub fn frame(&mut self) -> Vec<u8> {
        self.peek(UI_FRAME, FRAME_LEN)
    }

    /// The open frames, bottom first.
    pub fn frames(&mut self) -> Vec<Vec<u8>> {
        let depth = self.depth();
        let stack = self.shell.app.label("SCR_STACK");
        self.peek(stack, 16 * depth)
            .chunks(16)
            .map(<[u8]>::to_vec)
            .collect()
    }

    pub fn top_kind(&mut self) -> u8 {
        self.frames().last().unwrap()[0]
    }

    /// The machine. While the key last let go with `release` is within its gap, a press of it by
    /// any path is ignored by the OS, so the machine's `press_key` panics; a test about that gap
    /// calls `allow_raw_press` once for each gap.
    pub fn m(&mut self) -> &mut Machine {
        self.count_scans();
        &mut self.shell.app.m
    }

    /// Lets the test press the key it just let go without waiting, which the OS ignores inside the
    /// gap. It covers the gap of the last `release` and ends at the next `release` or `press`.
    pub fn allow_raw_press(&mut self) {
        scans(&self.scans).raw_press_allowed = true;
    }

    /// The keypad scans counted since the last `release`, 0 when none has been made.
    pub fn scans_since_release(&mut self) -> usize {
        self.count_scans();
        scans(&self.scans).seen()
    }

    fn count_scans(&mut self) {
        scans(&self.scans).count(&mut self.shell.app.m);
    }

    /// How many times the kernel's verify has been called since the boot.
    pub fn verifies(&mut self) -> u16 {
        let b = self.peek(VERIFIES, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// How many times the kernel's crc_begin and crc_chunk have been called since the boot.
    pub fn crc_calls(&mut self) -> (u16, u16) {
        let b = self.peek(CRC_BEGINS, 4);
        (
            u16::from_le_bytes([b[0], b[1]]),
            u16::from_le_bytes([b[2], b[3]]),
        )
    }

    pub fn byte(&mut self, label: &str) -> u8 {
        let at = self.shell.app.label(label);
        self.peek(at, 1)[0]
    }

    /// Runs to the instruction at `label` on the app's page and leaves the machine there, within
    /// `limit` T-states.
    pub fn run_to(&mut self, label: &str, limit: u32) {
        let at = self.shell.app.label(label);
        let m = self.m();
        let start = m.clock();
        loop {
            match m.run_to(BASE, at, limit) {
                Reached::Target => return,
                Reached::Vector38 => {}
                Reached::Limit => {
                    let (pc, page) = (m.pc(), m.in_port(6));
                    panic!(
                        "the app never reached {label} in {limit} T-states: at {pc:04X} on page {page:02X}"
                    )
                }
            }
            if m.clock().wrapping_sub(start) >= limit {
                let (pc, page) = (m.pc(), m.in_port(6));
                panic!(
                    "the app never reached {label} in {limit} T-states: at {pc:04X} on page {page:02X}"
                );
            }
        }
    }

    /// Runs to the next blit and returns the frame about to be shown, which stands in UI_FRAME;
    /// the machine moves one instruction past the blit's entry.
    pub fn next_blit(&mut self) -> Vec<u8> {
        self.run_to("far_ui_blit", WAIT);
        let frame = self.peek(UI_FRAME, FRAME_LEN);
        self.m().resume(1);
        frame
    }

    /// The frame of the next blit and the table's values one instruction past the blit's entry,
    /// which are the state the frame was built from since nothing runs between the draw and the
    /// blit. The values are read where the machine stands, without running it on.
    pub fn next_frame(&mut self) -> (Vec<u8>, Option<Vec<u8>>) {
        let frame = self.next_blit();
        let values = self.values_here();
        (frame, values)
    }

    /// Holds `scan` down until the shell hands it to scr_key, then lets go.
    pub fn tap(&mut self, scan: u8) {
        let code = u16::from(key_code(scan));
        self.press(scan);
        loop {
            self.run_to("scr_key", WAIT);
            if self.m().regs().af >> 8 == code {
                break;
            }
            self.m().resume(1);
        }
        self.release(scan);
        self.m().resume(1);
    }

    /// Lets go of `scan`. The OS takes the same key again only after its fifth scan with no key
    /// down, so a `press` of that key first runs on until five scans have followed the release;
    /// the frames the key draws are caught after the tap returns, which running on here would pass.
    /// A test about the gap after a release calls `Machine::release_key` itself.
    pub fn release(&mut self, scan: u8) {
        self.m().release_key(scan);
        self.m().clear_port_writes();
        scans(&self.scans).release(scan);
        let shared = Arc::clone(&self.scans);
        self.shell
            .app
            .m
            .set_press_hook(Some(Box::new(move |m, key| {
                let mut counted = scans(&shared);
                counted.count(m);
                counted.check_press(key);
            })));
        self.released_at = self.shell.app.m.clock();
    }

    /// Presses `scan`, after running on until five scans have followed the last `release` if
    /// that let go of the same key, which is when the OS has forgotten it. Another key is taken at once.
    pub fn press(&mut self, scan: u8) {
        let mut waited = 0;
        loop {
            self.count_scans();
            let (seen, acks) = {
                let counted = scans(&self.scans);
                match counted.released {
                    Some((key, seen)) if key == scan && seen < RELEASE_SCANS => {
                        (seen, counted.acks)
                    }
                    _ => break,
                }
            };
            let bytes = self.os_bytes();
            assert!(
                waited < RELEASE_LIMIT,
                "{seen} scans and {acks} interrupts in {waited} T-states after the release of \
                 {scan:02X}, expected {RELEASE_SCANS} scans; interrupts may have been masked for the \
                 rest; scan code, byte 8443h and last key: {bytes:02X?}"
            );
            self.run(RELEASE_STEP);
            waited += RELEASE_STEP;
        }
        {
            let mut counted = scans(&self.scans);
            counted.released = None;
            counted.raw_press_allowed = false;
        }
        self.shell.app.m.set_press_hook(None);
        self.shell.app.m.press_key(scan);
    }

    /// The OS's scan code, its count of scans with no key down (5 after a scan with a key down, 1 or 0
    /// after four or five without) and its last key.
    fn os_bytes(&mut self) -> [u8; 3] {
        let ram = self.shell.app.m.ram_page(1);
        [ram[0x43F], ram[0x443], ram[0x445]]
    }

    /// Runs `t` T-states.
    pub fn run(&mut self, t: u32) {
        let real = self.real;
        let m = self.m();
        let run = if real {
            m.run_time(t.div_ceil(6))
        } else {
            m.run_app(t)
        };
        assert_eq!(run.stop, mcemu::Stop::Limit, "the machine stopped");
    }

    /// The raw bytes of the status table, if it is in RAM, read once the CPU is outside any call that
    /// maps another bank: on the stub the machine runs on until it is.
    pub fn raw_table(&mut self) -> Option<Vec<u8>> {
        if !self.real {
            self.shell.at_rest();
        }
        self.raw_table_here()
    }

    /// `raw_table` without running the machine, for a read at a stop chosen on purpose: the stub's
    /// lookup runs a stub entry that reads and writes through the mapped banks, so it panics when
    /// the CPU is inside a call that maps another.
    pub fn raw_table_here(&mut self) -> Option<Vec<u8>> {
        if self.real {
            return crate::real::vat::table(self.m());
        }
        assert!(
            !self.shell.in_call(),
            "the stop is inside a call that maps another bank"
        );
        self.m().read_var(table::VAR).map(|v| v.data)
    }

    /// Every variable the OS holds, as one string to compare a list taken before with one taken after.
    pub fn vars(&mut self) -> String {
        if self.real {
            return format!("{:?}", crate::real::vat::list(self.m()));
        }
        self.shell.at_rest();
        format!("{:?}", self.m().var_list())
    }

    /// Makes the kernel's chunk call number `call`, counted from 1, answer `status` at once with
    /// nothing read, and calls the kernel for every other.
    pub fn fail_chunk(&mut self, call: u8, status: u8) {
        const AT: u16 = 0xBC80;
        let far = self.built.label("far_page2").to_le_bytes();
        let [c0, c1] = CRC_CHUNKS.to_le_bytes();
        let code = [
            0xF5, 0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0x7D, 0xFE, call, 0xE1, 0x28, 0x04, 0xF1,
            0xC3, far[0], far[1], 0xF1, 0x3E, status, 0xC9,
        ];
        let built = self.built;
        self.m().load_ram(AT, &code);
        hook_stub(self.m(), built, "far_kernel_crc_chunk", AT);
    }

    /// The value, 0 to 3, the table holds for each assertion.
    pub fn values(&mut self) -> Option<Vec<u8>> {
        Some(values_of(&self.raw_table()?))
    }

    /// `values` without running the machine, as `raw_table_here` reads.
    pub fn values_here(&mut self) -> Option<Vec<u8>> {
        Some(values_of(&self.raw_table_here()?))
    }

    /// How many assertions have a value, read as `values_here` does.
    pub fn settled_here(&mut self) -> usize {
        self.values_here()
            .expect("a table")
            .iter()
            .filter(|&&v| v != 0)
            .count()
    }

    /// How many assertions have a value.
    pub fn settled(&mut self) -> usize {
        self.values()
            .expect("a table")
            .iter()
            .filter(|&&v| v != 0)
            .count()
    }

    /// Holds each of `scans` down for 120,000 T-states and runs 480,000 more after it, as the
    /// shell rig's taps do, with the check going on meanwhile.
    pub fn tap_scans(&mut self, scans: &[u8]) {
        for &scan in scans {
            self.m().press_key(scan);
            self.run(120_000);
            self.m().release_key(scan);
            self.run(480_000);
        }
    }

    /// Sets the OS's ON break flag, as its interrupt does for an ON press that has come and gone.
    pub fn set_on_flag(&mut self) {
        let flags = self.peek(ON_FLAGS, 1)[0];
        self.m().load_ram(ON_FLAGS, &[flags | 0x10]);
    }

    /// Whether the OS's ON break flag is set.
    pub fn on_flag(&mut self) -> bool {
        self.peek(ON_FLAGS, 1)[0] & 0x10 != 0
    }

    /// The number the table keeps as the next assertion to check.
    pub fn next_number(&mut self) -> u16 {
        let data = self.raw_table().expect("a table");
        u16::from_le_bytes([data[8], data[9]])
    }

    pub fn library(&self) -> &'static [u8] {
        self.built.library.as_deref().expect("a library")
    }

    pub fn depth(&mut self) -> usize {
        let at = self.shell.app.label("scr_count");
        usize::from(self.peek(at, 1)[0])
    }
}

/// The value, 0 to 3, a table's bytes hold for each assertion.
/// The code scr_key is called with for a scancode.
pub fn key_code(scan: u8) -> u8 {
    match scan {
        CLEAR => 6,
        F1 => 7,
        F2 => 8,
        F3 => 9,
        F4 => 10,
        F5 => 11,
        DOWN => 2,
        _ => panic!("no key code for scancode {scan:02X}"),
    }
}

pub fn values_of(data: &[u8]) -> Vec<u8> {
    let count = usize::from(u16::from_le_bytes([data[6], data[7]]));
    (0..count)
        .map(|i| (data[10 + i / 4] >> (2 * (i % 4))) & 3)
        .collect()
}

/// The model's status for each value a table holds.
pub fn status_of(values: &[u8]) -> Vec<Status> {
    values
        .iter()
        .map(|&v| match v {
            0 => Status::Unchecked,
            2 => Status::Failed,
            _ => Status::Checked,
        })
        .collect()
}

/// The model on the checking screen of stage `row` with these values in the status table.
pub fn checking_model(built: &Built, values: &[u8], row: usize) -> mcui::Ui {
    let mut ui = model::ui(built, &status_of(values));
    for _ in 0..row {
        ui.press(Key::Down);
    }
    ui.press(Key::F2);
    assert!(
        matches!(ui.state(), mcui::State::Checking { .. }),
        "the model opened the checking screen"
    );
    ui
}

/// The model's reading phase of stage `row`, `read` chunks in.
pub fn reading_model(built: &Built, row: usize, read: usize) -> mcui::Ui {
    let count = table::header_of(built.library.as_ref().unwrap()).1;
    let mut ui = checking_model(built, &vec![0; usize::from(count)], row);
    assert!(ui.begin_reading(), "the model began reading");
    for _ in 0..read {
        assert!(ui.read_chunk(), "the model read a chunk");
    }
    ui
}

/// Whether the host model's kernel accepts each assertion of `library`, one verify after another on
/// a single kernel.
pub fn host_verdicts(library: &[u8]) -> Vec<bool> {
    let installed = Installed {
        name: "library".into(),
        bytes: library.to_vec(),
    };
    let count = usize::from(table::header_of(library).1);
    let lib = mcui::data::Lib::new(installed, vec![Status::Unchecked; count])
        .expect("the host model loads the library");
    (0..count).map(|i| lib.verify(i)).collect()
}

/// The values the table holds once the host's verdicts are in for the first `n` assertions and
/// the rest are unchecked.
pub fn expected_values(built: &Built, verdicts: &[bool], n: usize) -> Vec<u8> {
    let status: Vec<Status> = verdicts
        .iter()
        .enumerate()
        .map(|(i, &ok)| match (i < n, ok) {
            (false, _) => Status::Unchecked,
            (true, true) => Status::Checked,
            (true, false) => Status::Failed,
        })
        .collect();
    table::values(built, &status)
}

/// Compares a frame with the model's.
pub fn shows(what: &str, got: &[u8], ui: &mcui::Ui) {
    same_frame(what, got, &model::frame(ui));
}
