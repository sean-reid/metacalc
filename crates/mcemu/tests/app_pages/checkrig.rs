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
/// The scans with no key down after which the OS takes the key it took last again, the most T-states
/// those take (46 ms at 15 MHz is 700,000), and the steps the wait for them runs in.
const RELEASE_SCANS: usize = 5;
/// The writes to port 1 of a scan that finds no key down: 00h and FFh.
const EMPTY_SCAN_WRITES: usize = 2;
const RELEASE_LIMIT: u32 = 1_200_000;
const RELEASE_STEP: u32 = 3_000;
/// The steps between two frames of the screen, CKS_EVERY in screen_checking.inc.
pub const EVERY: usize = 32;

pub struct Run {
    pub shell: Shell,
    pub built: &'static Built,
    /// Whether the machine runs the reference OS, whose variables are read from its symbol table.
    pub real: bool,
    /// The key last let go with `release` and the keypad scans since, until another key is pressed.
    released: Option<(u8, usize)>,
    pub released_at: u32,
    /// Whether a write of 00h to port 1 has come since the last other port's write, the port 1 writes
    /// of that scan so far, and the writes of the scan an 08h acknowledge closed that no 09h has
    /// followed yet.
    scan_open: bool,
    scan_writes: usize,
    scan_pending: Option<usize>,
    /// The interrupts acknowledged with 09h since the release, for the message of a wait that fails.
    acks: usize,
    /// Whether the test presses the key it just let go raw, to test the gap.
    raw_press_allowed: bool,
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
            released: None,
            released_at: 0,
            scan_open: false,
            scan_writes: 0,
            scan_pending: None,
            acks: 0,
            raw_press_allowed: false,
        }
    }

    /// A run over a shell on the reference OS whose machine has the rig's hooks and the kernel's
    /// counters installed.
    pub fn from_shell(shell: Shell, built: &'static Built) -> Run {
        Run {
            shell,
            built,
            real: true,
            released: None,
            released_at: 0,
            scan_open: false,
            scan_writes: 0,
            scan_pending: None,
            acks: 0,
            raw_press_allowed: false,
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

    /// The machine. While the key last let go with `release` is within its gap, a raw press of it
    /// is ignored by the OS, so it panics here, at the first use after the press; a test about
    /// that gap calls `allow_raw_press` once for each gap. The guard sees only presses made through
    /// this `Run`'s `m()`: a press through `shell.app.m`, `Shell::press`, `Shell::leave`,
    /// `Shell::tap_lost`, or through a `&mut Machine` kept across the window is not covered.
    pub fn m(&mut self) -> &mut Machine {
        self.count_scans();
        &mut self.shell.app.m
    }

    /// Lets the test press the key it just let go without waiting, which the OS ignores inside the
    /// gap. It covers the gap of the last `release` and ends at the next `release` or `press`.
    pub fn allow_raw_press(&mut self) {
        self.raw_press_allowed = true;
    }

    /// The keypad scans counted since the last `release`, 0 when none has been made.
    pub fn scans_since_release(&mut self) -> usize {
        self.count_scans();
        self.released.map_or(0, |(_, seen)| seen)
    }

    /// Adds the scans with no key down since the last count to the release's. The OS takes the key it
    /// took last again only after its fifth such scan, and scans only in an interrupt, so the count is
    /// of scans, not of time or of acknowledges. A scan is a write of 00h to port 1 and then FFh, 2
    /// writes with no key down and 18 with one, in an interrupt that goes on to acknowledge with 08h
    /// and 09h to port 3, with nothing but port 1 between the scan and the 08h. On the stub OS and at
    /// the app's idle point on the reference OS every timer interrupt scans, one in 9.28 ms. In the
    /// reference OS's key wait, pc 03A1 on page 06, where the shell sits between keys, three
    /// interrupts come in 9.28 ms and one scans: the other two have port 4 status 0Ch or 0Eh and
    /// acknowledge 08h, 09h without a write to port 1. The scan code byte cannot stand for the event:
    /// the shell's key read has zeroed it before the release, and the last-key byte the stub clears at
    /// the fifth scan the reference OS never clears. A stretch with interrupts off, as in the theorem's
    /// enter, has no scan, and a key pressed in it is held down through the first scans after it and
    /// ignored for good. An ON edge acknowledges with 08h alone and is not a scan.
    fn count_scans(&mut self) {
        let Some((scan, seen)) = &mut self.released else {
            return;
        };
        let m = &mut self.shell.app.m;
        let (writes, full) = m.take_port_writes_or_full();
        for w in &writes {
            match (w.port, w.value) {
                (1, 0) => {
                    self.scan_open = true;
                    self.scan_writes = 1;
                }
                (1, _) if self.scan_open => self.scan_writes += 1,
                (1, _) => {}
                (3, 8) => {
                    self.scan_pending = self.scan_open.then_some(self.scan_writes);
                    self.scan_open = false;
                }
                (3, 9) => {
                    self.acks += 1;
                    *seen += usize::from(self.scan_pending.take() == Some(EMPTY_SCAN_WRITES));
                }
                _ => self.scan_open = false,
            }
        }
        assert!(
            !full || *seen >= RELEASE_SCANS,
            "the port record filled with {seen} scans and {} interrupts counted after the release of \
             {scan:02X}",
            self.acks
        );
        assert!(
            self.raw_press_allowed || *seen >= RELEASE_SCANS || !m.key_is_down(*scan),
            "key {scan:02X} was pressed with the machine's press_key {seen} scans after its \
             release, which the OS ignores before the fifth: use Run::press"
        );
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

    /// The frame of the next blit and the table's values at that moment, which are the state the
    /// frame was built from since nothing runs between the draw and the blit.
    pub fn next_frame(&mut self) -> (Vec<u8>, Option<Vec<u8>>) {
        let frame = self.next_blit();
        let values = self.values();
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
        self.released = Some((scan, 0));
        self.released_at = self.shell.app.m.clock();
        (self.scan_open, self.scan_pending, self.acks) = (false, None, 0);
        self.raw_press_allowed = false;
    }

    /// Presses `scan`, after running on until five scans have followed the last `release` if
    /// that let go of the same key, which is when the OS has forgotten it. Another key is taken at once.
    pub fn press(&mut self, scan: u8) {
        let mut waited = 0;
        loop {
            self.count_scans();
            let seen = match self.released {
                Some((key, seen)) if key == scan && seen < RELEASE_SCANS => seen,
                _ => break,
            };
            let (acks, bytes) = (self.acks, self.os_bytes());
            assert!(
                waited < RELEASE_LIMIT,
                "{seen} scans and {acks} interrupts in {waited} T-states after the release of \
                 {scan:02X}, expected {RELEASE_SCANS} scans; interrupts may have been masked for the \
                 rest; scan code, byte 8443h and last key: {bytes:02X?}"
            );
            self.run(RELEASE_STEP);
            waited += RELEASE_STEP;
        }
        self.released = None;
        self.raw_press_allowed = false;
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

    /// The raw bytes of the status table, if it is in RAM.
    pub fn raw_table(&mut self) -> Option<Vec<u8>> {
        if self.real {
            return crate::real::vat::table(self.m());
        }
        self.m().read_var(table::VAR).map(|v| v.data)
    }

    /// Every variable the OS holds, as one string to compare a list taken before with one taken after.
    pub fn vars(&mut self) -> String {
        if self.real {
            return format!("{:?}", crate::real::vat::list(self.m()));
        }
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
