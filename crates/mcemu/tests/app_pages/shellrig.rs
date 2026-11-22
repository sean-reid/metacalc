//! The app on the stub OS with counters in RAM: how many frames it blitted and how many times each
//! kind's enter ran, and the keys the shell takes.

use crate::app::*;
use crate::realos::BASE;
use crate::ui_support::same_frame;
use mcemu::{Machine, Stop};

pub const F1: u8 = 0x35;
pub const F2: u8 = 0x34;
pub const F3: u8 = 0x33;
pub const F4: u8 = 0x32;
pub const F5: u8 = 0x31;
pub const ENTER: u8 = 0x09;
pub const CLEAR: u8 = 0x0F;
pub const DOWN: u8 = 0x01;
pub const LEFT: u8 = 0x02;
pub const RIGHT: u8 = 0x03;
pub const UP: u8 = 0x04;
pub const SECOND: u8 = 0x36;
pub const MODE: u8 = 0x37;
/// The key that carries the letter A, and 2nd+1, which the app ignores.
pub const LETTER: u8 = 0x2F;
pub const ONE: u8 = 0x22;

pub const KIND_START: u8 = 1;
pub const KIND_NOTICE: u8 = 2;
pub const KIND_ABOUT: u8 = 3;
pub const NOTICE_ERROR: u8 = 2;

const TAP: u32 = 120_000;
const BLITS: u16 = 0xA700;
const ENTERS: u16 = 0xA710;
const FLAG: u16 = 0xA720;
const SCR_KEYS: u16 = 0xA730;
const KEY_HOOK: u16 = 0xA6C0;
const BLIT_HOOK: u16 = 0xA640;
const ENTER_HOOK: u16 = 0xA660;
const SAVE_SCREEN: u16 = 0x86EC;

/// `push hl`, `ld hl,(counter)`, `inc hl`, `ld (counter),hl`, `pop hl`, `jp then`.
fn counting_hook(counter: u16, then: u16) -> Vec<u8> {
    let [c0, c1] = counter.to_le_bytes();
    let [t0, t1] = then.to_le_bytes();
    vec![0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0xC3, t0, t1]
}

/// An enter that fails with `status` for every notice but the error notice, whose own enter
/// builds its line: `inc hl`, `ld a,(hl)`, `dec hl`, `cp 2`, `jp z,real`, `ld a,status`, `ret`.
fn failing_notices(status: u8, real: u16) -> Vec<u8> {
    let [r0, r1] = real.to_le_bytes();
    vec![
        0x23,
        0x7E,
        0x2B,
        0xFE,
        NOTICE_ERROR,
        0xCA,
        r0,
        r1,
        0x3E,
        status,
        0xC9,
    ]
}

/// An enter that fails with `status` while the byte at `FLAG` is not 0, and is `real` otherwise:
/// `ld a,(FLAG)`, `or a`, `jp z,real`, `ld a,status`, `ret`.
fn failing_while_flagged(status: u8, real: u16) -> Vec<u8> {
    let [f0, f1] = FLAG.to_le_bytes();
    let [r0, r1] = real.to_le_bytes();
    vec![0x3A, f0, f1, 0xB7, 0xCA, r0, r1, 0x3E, status, 0xC9]
}

/// An enter that fails once with `status` when the byte at `FLAG` is not 0 and clears it, and is
/// `real` otherwise: `ld a,(FLAG)`, `or a`, `jp z,real`, `xor a`, `ld (FLAG),a`, `ld a,status`, `ret`.
fn failing_once(status: u8, real: u16) -> Vec<u8> {
    let [f0, f1] = FLAG.to_le_bytes();
    let [r0, r1] = real.to_le_bytes();
    vec![
        0x3A, f0, f1, 0xB7, 0xCA, r0, r1, 0xAF, 0x32, f0, f1, 0x3E, status, 0xC9,
    ]
}

/// An enter that fails with `status`: `ld a,status`, `ret`.
fn failing_all(status: u8) -> Vec<u8> {
    vec![0x3E, status, 0xC9]
}

#[derive(Clone, Copy)]
pub enum Enter {
    Real,
    /// Every notice, the error notice included, fails with this status.
    FailAll(u8),
    /// The next notice enter after `Shell::flag` is set fails with this status, once.
    FailNoticeOnce(u8),
    /// Every notice but the error notice fails with this status.
    FailNotices(u8),
    /// The About screen's enter fails with this status while `Shell::flag` is set.
    FailAboutWhenFlagged(u8),
}

pub struct Shell {
    pub app: App,
}

fn patch_enter(m: &mut Machine, built: &Built, kind: u8, to: u16) {
    let at = usize::from(built.label("screen_kinds") - 0x4000) + usize::from(kind - 1) * 8;
    m.load_flash(BASE, at, &to.to_le_bytes());
}

impl Shell {
    /// The app booted to its first key wait with a counter on the blit and on every kind's enter.
    pub fn boot(built: &'static Built, enter: Enter) -> Shell {
        let app = App::boot_with(built, |m| {
            m.load_ram(BLITS, &[0; 0x40]);
            // scr_key starts `ld c,SCR_AT_KEY`, `call scr_on_top`: five bytes the hook runs itself.
            let key = built.label("scr_key");
            let [c0, c1] = SCR_KEYS.to_le_bytes();
            let [o0, o1] = built.label("scr_on_top").to_le_bytes();
            let [r0, r1] = (key + 5).to_le_bytes();
            let hook = [
                0xE5, 0x2A, c0, c1, 0x23, 0x22, c0, c1, 0xE1, 0x0E, 0x04, 0xCD, o0, o1, 0xC3, r0,
                r1,
            ];
            m.load_ram(KEY_HOOK, &hook);
            let [h0, h1] = KEY_HOOK.to_le_bytes();
            m.load_flash(BASE, usize::from(key - 0x4000), &[0xC3, h0, h1]);
            let far = built.label("far_page1");
            m.load_ram(BLIT_HOOK, &counting_hook(BLITS, far));
            hook_stub(m, built, "far_ui_blit", BLIT_HOOK);
            let reals = ["start_enter", "notice_enter", "about_enter"].map(|n| built.label(n));
            for (i, real) in reals.into_iter().enumerate() {
                let at = ENTER_HOOK + 0x20 * i as u16;
                let kind = i as u8 + 1;
                let counter = ENTERS + 2 * i as u16;
                let hook = match (enter, kind) {
                    (Enter::FailNotices(status), KIND_NOTICE) => {
                        let mut code = counting_hook(counter, 0);
                        code.truncate(9);
                        code.extend(failing_notices(status, real));
                        code
                    }
                    (Enter::FailNoticeOnce(status), KIND_NOTICE) => {
                        let mut code = counting_hook(counter, 0);
                        code.truncate(9);
                        code.extend(failing_once(status, real));
                        code
                    }
                    (Enter::FailAll(status), KIND_NOTICE) => {
                        let mut code = counting_hook(counter, 0);
                        code.truncate(9);
                        code.extend(failing_all(status));
                        code
                    }
                    (Enter::FailAboutWhenFlagged(status), KIND_ABOUT) => {
                        let mut code = counting_hook(counter, 0);
                        code.truncate(9);
                        code.extend(failing_while_flagged(status, real));
                        code
                    }
                    _ => counting_hook(counter, real),
                };
                m.load_ram(at, &hook);
                patch_enter(m, built, kind, at);
            }
        });
        Shell { app }
    }

    fn word(&mut self, at: u16) -> u16 {
        let b = self.app.m.read_ram(at, 2);
        u16::from_le_bytes([b[0], b[1]])
    }

    /// Frames blitted since the boot, the first one included.
    pub fn blits(&mut self) -> u16 {
        self.word(BLITS)
    }

    /// Times the enter of `kind` has run since the boot.
    pub fn enters(&mut self, kind: u8) -> u16 {
        self.word(ENTERS + 2 * u16::from(kind - 1))
    }

    /// Times the app has called scr_key since the boot.
    pub fn scr_keys(&mut self) -> u16 {
        self.word(SCR_KEYS)
    }

    pub fn flag(&mut self, on: bool) {
        self.app.m.load_ram(FLAG, &[u8::from(on)]);
    }

    pub fn depth(&mut self) -> usize {
        let at = self.app.label("scr_count");
        usize::from(self.app.m.read_ram(at, 1)[0])
    }

    /// The open frames, bottom first.
    pub fn frames(&mut self) -> Vec<Vec<u8>> {
        let depth = self.depth();
        let stack = self.app.label("SCR_STACK");
        self.app
            .m
            .read_ram(stack, 16 * depth)
            .chunks(16)
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// Fills every byte of SCR_STACK and SCR_VARS that holds nothing at rest with a pattern: the
    /// framework's count and table, and the frames of the open screens, stay as they are.
    pub fn dirty_dead(&mut self) {
        use crate::realos::varied;
        let depth = self.depth();
        let (vars, stack) = (self.app.label("SCR_VARS"), self.app.label("SCR_STACK"));
        let (vars_size, stack_size) = (
            usize::from(self.app.label("SCR_VARS_SIZE")),
            usize::from(self.app.label("SCR_STACK_SIZE")),
        );
        self.app.m.load_ram(vars + 1, &varied(2, 0xC3));
        self.app.m.load_ram(vars + 5, &varied(vars_size - 5, 0xC4));
        let live = 16 * depth;
        self.app
            .m
            .load_ram(stack + live as u16, &varied(stack_size - live, 0x5A));
    }

    pub fn top_kind(&mut self) -> u8 {
        self.frames().last().unwrap()[0]
    }

    /// Taps a key the app turns into a key code, which reaches scr_key exactly once.
    pub fn tap(&mut self, scan: u8) {
        let before = self.scr_keys();
        self.press(scan);
        assert_eq!(
            self.scr_keys(),
            before + 1,
            "scancode {scan:02X} reached scr_key once"
        );
    }

    /// Taps 2nd, which gives the app no key code.
    pub fn tap_modifier(&mut self, scan: u8) {
        let before = self.scr_keys();
        self.press(scan);
        assert_eq!(self.scr_keys(), before, "scancode {scan:02X} is no key");
    }

    /// Taps a key while a refresh fails, which the shell answers with the error notice: the key
    /// never reaches scr_key.
    pub fn tap_lost(&mut self, scan: u8) {
        let before = self.scr_keys();
        self.press(scan);
        assert_eq!(
            self.scr_keys(),
            before,
            "scancode {scan:02X} was lost to the failure"
        );
    }

    fn press(&mut self, scan: u8) {
        self.app.m.press_key(scan);
        assert_eq!(self.app.m.run_app(TAP).stop, Stop::Limit);
        self.app.m.release_key(scan);
        assert_eq!(self.app.m.run_app(4 * TAP).stop, Stop::Limit);
    }

    /// The regions' bytes after the app left: every one of both is filled as the home screen has it.
    pub fn assert_regions_put_away(&mut self) {
        let (stack, vars) = (self.app.label("SCR_STACK"), self.app.label("SCR_VARS"));
        let (stack_size, vars_size) = (
            usize::from(self.app.label("SCR_STACK_SIZE")),
            usize::from(self.app.label("SCR_VARS_SIZE")),
        );
        assert_eq!(
            self.app.m.read_ram(stack, stack_size),
            vec![0x20; stack_size]
        );
        assert_eq!(self.app.m.read_ram(vars, vars_size), vec![0; vars_size]);
    }

    /// Presses `scan` and runs until the app leaves.
    pub fn leave(&mut self, scan: u8) {
        self.app.m.press_key(scan);
        assert_eq!(
            self.app.m.run_app(8 * TAP).stop,
            Stop::Exited,
            "the app leaves"
        );
    }

    pub fn shows(&mut self, what: &str, want: &[u8]) {
        same_frame(what, &self.app.frame(), want);
    }

    /// The LCD's pixels packed 8 to a byte, which is what the OS copies over saveSScreen at an
    /// auto power down.
    pub fn lcd_bytes(&mut self) -> Vec<u8> {
        let lcd = self.app.m.lcd();
        (0..64)
            .flat_map(|y| lcd.row(y).to_vec())
            .collect::<Vec<bool>>()
            .chunks(8)
            .map(|px| px.iter().fold(0u8, |b, &lit| (b << 1) | u8::from(lit)))
            .collect()
    }

    /// What an auto power down does to the app's RAM: the screen's pixels land over saveSScreen.
    pub fn power_down(&mut self) {
        let pixels = self.lcd_bytes();
        self.app.m.load_ram(SAVE_SCREEN, &pixels);
    }

    pub fn sentinel(&mut self) -> (u16, u16) {
        (
            self.app.label("APP_SENTINEL"),
            self.app.label("APP_SENTINEL_FRAME"),
        )
    }
}

/// The frames the host model draws, packed as the app's frame is.
pub mod model {
    use crate::fixture;
    use crate::realos::start_model;
    use crate::ui_support::pack;
    use mcui::{Installed, Key, State, Statuses, Ui};

    pub fn start(library: Option<&[u8]>) -> Vec<u8> {
        pack(&start_model(library, 0))
    }

    pub fn notice(title: &str, line: &str) -> Vec<u8> {
        pack(&mcui::notice_frame(title, &[line]))
    }

    /// The model's own notice for a feature that is not built yet.
    pub fn unavailable(title: &str) -> Vec<u8> {
        pack(&mcui::unavailable_frame(title))
    }

    /// The error notice has no model: the title and the line are the app's, drawn by the model's
    /// Notice.
    pub fn error(status: u8) -> Vec<u8> {
        notice("Error", &format!("Status {status:02X}"))
    }

    /// A model with `libraries` libraries open on its My proofs row, which opens About on F3.
    pub fn ui(libraries: usize) -> Ui {
        let installed: Vec<Installed> = (0..libraries)
            .map(|n| Installed {
                name: format!("lib{n}"),
                bytes: fixture::library(),
            })
            .collect();
        let tables = installed
            .iter()
            .map(|lib| {
                mcpack::library::read(&lib.bytes)
                    .expect("the library reads")
                    .iter()
                    .map(|_| mcui::Status::Checked)
                    .collect()
            })
            .collect();
        let mut ui = Ui::new(
            installed,
            Statuses {
                libraries: tables,
                my_proofs: 0,
            },
        )
        .expect("the model loads");
        for _ in 0..libraries {
            ui.press(Key::Down);
        }
        ui
    }

    /// The model's About screen with `libraries` libraries installed.
    pub fn about(libraries: usize) -> Vec<u8> {
        let mut ui = ui(libraries);
        ui.press(Key::F3);
        assert_eq!(
            ui.state(),
            State::About,
            "the model opened the program's About"
        );
        pack(&ui.frame())
    }
}
