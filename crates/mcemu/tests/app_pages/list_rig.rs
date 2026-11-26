//! The theorem list on the stub OS, driven key by key against the host model: the frame a key
//! redraws, the frame after the idle work, and the app's own variables read from RAM.

use crate::app::*;
use crate::checkrig::Run;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, State, Status, Ui};

/// The most T-states the app may take to settle a preview or to reach a frame.
pub const LIMIT: u32 = 60_000_000;

pub const PENDING: u8 = 0;
pub const ROW: u8 = 1;
pub const NOTHING: u8 = 2;

/// The code scr_key is called with for a scancode.
pub fn code_of(scan: u8) -> u8 {
    match scan {
        UP => 1,
        DOWN => 2,
        LEFT => 3,
        RIGHT => 4,
        ENTER => 5,
        CLEAR => 6,
        F1 => 7,
        F2 => 8,
        F3 => 9,
        F4 => 10,
        F5 => 11,
        LETTER | ONE => 12,
        _ => panic!("no key code for scancode {scan:02X}"),
    }
}

pub struct Lister {
    pub run: Run,
    pub ui: Ui,
    pub built: &'static Built,
    /// The marks the model was given, and every key it has been pressed with since the Libraries
    /// screen, to build the model again at the same place.
    pub status: Vec<Status>,
    pub log: Vec<Key>,
    /// The preview state and the wait the app had when the frame of the last key was blitted.
    pub seen: (u8, u8),
    /// The model that has the marks list_enter derived the title's counts from, when the table has
    /// been lost since: the title bar is compared with its frame and the rest with `ui`'s.
    /// `stale_titles` counts the frames whose title bar differs from `ui`'s.
    pub title_from: Option<Ui>,
    pub stale_titles: usize,
}

/// What the model's list says about itself.
pub struct Pos {
    pub cursor: usize,
    pub top: usize,
    pub len: usize,
    pub pending: bool,
}

impl Lister {
    /// The app on `built` with the status table `table` and the model with `status`, both on the
    /// Libraries screen with the cursor on `stage`.
    pub fn at_stage(
        built: &'static Built,
        table: Option<(&[u8], &[Status])>,
        stage: usize,
    ) -> Lister {
        let run = Run::boot(built, table.map(|t| t.0));
        let ui = crate::shellrig::model::ui(built, table.map_or(&[][..], |t| t.1));
        let mut lister = Lister {
            run,
            ui,
            built,
            status: table.map_or(Vec::new(), |t| t.1.to_vec()),
            log: Vec::new(),
            seen: (0, 0),
            title_from: None,
            stale_titles: 0,
        };
        for _ in 0..stage {
            lister.key(DOWN, Key::Down);
        }
        lister
    }

    /// `at_stage`, then ENTER, and the list is shown with its preview pending, as the model has it.
    pub fn open(built: &'static Built, table: Option<(&[u8], &[Status])>, stage: usize) -> Lister {
        let mut lister = Lister::at_stage(built, table, stage);
        lister.key(ENTER, Key::Enter);
        assert_eq!(lister.run.top_kind(), KIND_LIST, "the list is open");
        lister
    }

    /// Taps `scan`, which the shell hands to scr_key, and compares the frame the key redraws, or
    /// the absence of one, and the depth, with the model after `key`. Returns whether a frame was
    /// drawn.
    pub fn key(&mut self, scan: u8, key: Key) -> bool {
        let blits = self.run.blits();
        let (before, depth) = (self.shown(), self.ui.depth());
        self.tap(scan);
        self.ui.press(key);
        if let Some(old) = self.title_from.as_mut() {
            old.press(key);
        }
        self.log.push(key);
        let want = self.shown();
        let redraws = want != before || self.ui.depth() != depth;
        if redraws {
            let frame = self.run.next_blit();
            self.seen = (self.var("LV_PREVIEW"), self.var("LV_COUNT"));
            self.same(&format!("after {scan:02X}"), &frame, &want);
            let mut spent = 0;
            while self.run.blits() == blits && spent < 20_000 {
                self.run.run(500);
                spent += 500;
            }
            self.run.run(2_000);
        } else {
            self.run.run(300_000);
        }
        assert_eq!(
            self.run.blits(),
            blits + u16::from(redraws),
            "after {scan:02X}: the blits"
        );
        assert_eq!(self.run.depth(), self.ui.depth(), "after {scan:02X}: depth");
        redraws
    }

    /// `same_frame`, with the title bar's rows taken from `title_from` when it is set.
    fn same(&mut self, what: &str, got: &[u8], want: &[u8]) {
        const TITLE: usize = 8 * 12;
        let Some(old) = &self.title_from else {
            return same_frame(what, got, want);
        };
        let mut want = want.to_vec();
        let old = crate::shellrig::model::frame(old);
        self.stale_titles += usize::from(want[..TITLE] != old[..TITLE]);
        want[..TITLE].copy_from_slice(&old[..TITLE]);
        same_frame(what, got, &want);
    }

    /// Holds `scan` until scr_key takes it and lets go.
    pub fn tap(&mut self, scan: u8) {
        let code = u16::from(code_of(scan));
        let before = self.run.scr_keys();
        self.run.m().press_key(scan);
        loop {
            self.run.run_to("scr_key", LIMIT);
            if self.run.m().regs().af >> 8 == code {
                break;
            }
            self.run.m().resume(1);
        }
        self.run.m().release_key(scan);
        self.run.m().resume(100);
        // The hook that counts the call needs about as many T-states as that, so an interrupt that lands
        // in them leaves the count behind for the length of its handler.
        let mut waited = 0;
        while self.run.scr_keys() == before && waited < 20_000 {
            self.run.m().resume(100);
            waited += 100;
        }
        assert_eq!(
            self.run.scr_keys(),
            before + 1,
            "{scan:02X} reached scr_key"
        );
    }

    /// The frame the model draws for what is open, packed.
    pub fn shown(&self) -> Vec<u8> {
        crate::shellrig::model::frame(&self.ui)
    }

    pub fn pos(&self) -> Pos {
        match self.ui.state() {
            State::List {
                cursor,
                top,
                len,
                pending,
                ..
            } => Pos {
                cursor,
                top,
                len,
                pending,
            },
            other => panic!("the model is on {other:?}"),
        }
    }

    /// The app's variable of the list screen, one byte.
    pub fn var(&mut self, name: &str) -> u8 {
        self.run.byte(name)
    }

    /// The frame of the open list: its stage, cursor and top.
    pub fn frame_bytes(&mut self) -> [u8; 6] {
        let frames = self.run.frames();
        frames.last().unwrap()[..6].try_into().unwrap()
    }

    /// Runs until the preview is settled, which is when idle has answered once with the
    /// count at 0, and returns whether a frame was blitted for it. The model does its idle work
    /// and the frame is compared with its.
    pub fn settle(&mut self) -> bool {
        if !self.pos().pending {
            assert_ne!(
                self.var("LV_PREVIEW"),
                PENDING,
                "the model and the app agree"
            );
            let (got, want) = (self.run.frame(), self.shown());
            self.same("the settled list", &got, &want);
            return false;
        }
        let blits = self.run.blits();
        let mut spent = 0;
        while self.var("LV_PREVIEW") == PENDING {
            self.run.run(20_000);
            spent += 20_000;
            assert!(spent < LIMIT, "the preview never settled");
        }
        self.run.run(300_000);
        let drew = self.ui.idle();
        assert_eq!(
            self.run.blits(),
            blits + u16::from(drew),
            "the blits of the idle work"
        );
        assert_eq!(
            self.var("LV_PREVIEW"),
            if drew { ROW } else { NOTHING },
            "what the app settled on"
        );
        let (got, want) = (self.run.frame(), self.shown());
        self.same("the settled list", &got, &want);
        drew
    }

    /// The app's position is the model's.
    pub fn same_position(&mut self, at: &str) {
        let pos = self.pos();
        let frame = self.frame_bytes();
        assert_eq!(frame[0], KIND_LIST, "{at}: the kind");
        let cursor = usize::from(u16::from_le_bytes([frame[2], frame[3]]));
        let top = usize::from(u16::from_le_bytes([frame[4], frame[5]]));
        assert_eq!((cursor, top), (pos.cursor, pos.top), "{at}: cursor and top");
        assert!(pos.len > 0 || (cursor, top) == (0, 0), "{at}");
    }
}

/// The counters of the entry page's far entries to fm, in RAM the rig's other hooks leave free.
const COUNTERS: u16 = 0xBC40;
const HOOKS: u16 = 0xBC50;
pub const FM_ENTRIES: [&str; 5] = [
    "far_fm_open",
    "far_fm_statement",
    "far_fm_fit",
    "far_fm_clip",
    "far_fm_draw",
];

impl Lister {
    /// Counts every call the app makes to each of `FM_ENTRIES` from now on.
    pub fn count_fm_calls(&mut self) {
        let built = self.built;
        let far = built.label("far_page1").to_le_bytes();
        let m = self.run.m();
        m.load_ram(COUNTERS, &[0; 10]);
        for (i, stub) in FM_ENTRIES.into_iter().enumerate() {
            let at = HOOKS + 0x10 * i as u16;
            let [c0, c1] = (COUNTERS + 2 * i as u16).to_le_bytes();
            m.load_ram(
                at,
                &[
                    0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0xC3, far[0], far[1],
                ],
            );
            hook_stub(m, built, stub, at);
        }
    }

    /// How many calls each of `FM_ENTRIES` has had since `count_fm_calls`.
    pub fn fm_calls(&mut self) -> [u16; 5] {
        let b = self.run.peek(COUNTERS, 10);
        std::array::from_fn(|i| u16::from_le_bytes([b[2 * i], b[2 * i + 1]]))
    }
}

/// Counters and hooks of `fail_at`, in RAM no other hook of the rig uses.
const FAIL_COUNTERS: u16 = 0xBD00;
const FAIL_HOOKS: u16 = 0xBD20;

impl Lister {
    /// Makes the `call`th call from now on to the far entry `stub` answer `status` in A, which is not
    /// 0, with its routine not called; the other calls go on to the routine. `slot` numbers the
    /// hook's counter, 0 to 3.
    pub fn fail_at(&mut self, slot: usize, stub: &str, call: u8, status: u8) {
        assert!(slot < 4 && status != 0);
        let built = self.built;
        let page = if stub.starts_with("far_fm_") {
            "far_page1"
        } else {
            assert!(stub.starts_with("far_lib_"), "{stub}");
            "far_page2"
        };
        let far = built.label(page).to_le_bytes();
        let counter = FAIL_COUNTERS + 2 * slot as u16;
        let [c0, c1] = counter.to_le_bytes();
        let at = FAIL_HOOKS + 0x20 * slot as u16;
        let m = self.run.m();
        m.load_ram(counter, &[0, 0]);
        m.load_ram(
            at,
            &[
                0xF5, 0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0x7D, 0xFE, call, 0x28, 0x05, 0xE1,
                0xF1, 0xC3, far[0], far[1], 0xE1, 0xF1, 0x3E, status, 0xB7, 0xC9,
            ],
        );
        hook_stub(m, built, stub, at);
    }

    /// How many calls the entry hooked by `fail_at` in `slot` has had since.
    pub fn fail_calls(&mut self, slot: usize) -> u16 {
        let b = self.run.peek(FAIL_COUNTERS + 2 * slot as u16, 2);
        u16::from_le_bytes([b[0], b[1]])
    }
}

impl Lister {
    /// The model on the Libraries screen again, then taken through every key this list has had, with
    /// no idle work: what the list shows when its preview is pending.
    pub fn replayed(&self) -> Ui {
        let mut ui = crate::shellrig::model::ui(self.built, &self.status);
        for &key in &self.log {
            ui.press(key);
        }
        ui
    }
}

/// The library of real proofs as three stages, built in a directory of this unit's own.
pub fn proofs_three() -> &'static Built {
    static BUILT: std::sync::OnceLock<&'static Built> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let library = crate::fixture::proofs();
        let n = crate::table::header_of(&library).1 as usize;
        staged("pages-list-proofs", &library, &[n / 3, 2 * n / 3, n])
    })
}

/// Asserts that a deterministic cost is within a percent above and two below what it measured when
/// its ceiling was set, so a change to the code that costs time or saves it is seen.
pub fn within(what: &str, got: u32, measured: u32) {
    let (low, high) = (measured - measured / 50, measured + measured / 100);
    assert!(
        (low..=high).contains(&got),
        "{what}: {got} T-states, measured {measured} (allowed {low} to {high})"
    );
}
