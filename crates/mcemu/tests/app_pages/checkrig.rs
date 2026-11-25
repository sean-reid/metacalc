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
/// The most T-states one wait for a frame or a key may take.
pub const WAIT: u32 = 600_000_000;
/// The steps between two frames of the screen, CKS_EVERY in screen_checking.inc.
pub const EVERY: usize = 32;

pub struct Run {
    pub shell: Shell,
    pub built: &'static Built,
    /// Whether the machine runs the reference OS, whose variables are read from its symbol table.
    pub real: bool,
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
        }
    }

    /// A run over a shell on the reference OS whose machine has the rig's hooks and the kernel's
    /// counters installed.
    pub fn from_shell(shell: Shell, built: &'static Built) -> Run {
        Run {
            shell,
            built,
            real: true,
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

    pub fn m(&mut self) -> &mut Machine {
        &mut self.shell.app.m
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

    pub fn word(&mut self, label: &str) -> u16 {
        let at = self.shell.app.label(label);
        let b = self.peek(at, 2);
        u16::from_le_bytes([b[0], b[1]])
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
        let code: u16 = match scan {
            CLEAR => 6,
            F1 => 7,
            F2 => 8,
            F3 => 9,
            F4 => 10,
            F5 => 11,
            DOWN => 2,
            _ => panic!("no key code for scancode {scan:02X}"),
        };
        self.m().press_key(scan);
        loop {
            self.run_to("scr_key", WAIT);
            if self.m().regs().af >> 8 == code {
                break;
            }
            self.m().resume(1);
        }
        self.m().release_key(scan);
        self.m().resume(1);
    }

    /// Runs `t` T-states.
    pub fn run(&mut self, t: u32) {
        let run = self.m().run_app(t);
        assert_eq!(run.stop, mcemu::Stop::Limit, "the machine stopped");
    }

    /// The raw bytes of the status table, if it is in RAM.
    pub fn raw_table(&mut self) -> Option<Vec<u8>> {
        if self.real {
            return crate::real::vat::table(self.m());
        }
        self.m().read_var(table::VAR).map(|v| v.data)
    }

    /// The value, 0 to 3, the table holds for each assertion.
    pub fn values(&mut self) -> Option<Vec<u8>> {
        let data = self.raw_table()?;
        let count = usize::from(u16::from_le_bytes([data[6], data[7]]));
        Some(
            (0..count)
                .map(|i| (data[10 + i / 4] >> (2 * (i % 4))) & 3)
                .collect(),
        )
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
