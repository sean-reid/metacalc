//! What the exit tests share: the reference OS driven to the home screen, the app started from it, and each way out.

#![allow(dead_code)]

use super::fixture;
use super::realos::*;
use mcemu::{Machine, Regs};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const STUB_BASE: u8 = 0x30;
pub const RAM_BASE: u16 = 0x8000;
/// cmdShadow and smallEditRAM, and the byte on each side of them.
pub const STACK: (u16, usize) = (0x966E, 128);
pub const VARS: (u16, usize) = (0x90D3, 108);
pub const NEIGHBOURS: [u16; 4] = [0x966D, 0x96F0, 0x90D2, 0x913F];
/// cmdShadCur, the two bytes after cmdShadow that the home screen restores its cursor from.
pub const CMD_CUR: u16 = 0x96EE;
/// The home entry line's buffer start and cursor pointers, and the first byte of the buffer.
pub const EDIT_TOP: u16 = 0x96F4;
pub const EDIT_CURSOR: u16 = 0x96F6;
pub const CMD_CURSOR: u16 = 0x96F2;
pub const EDIT_BUF: u16 = 0x9DB7;
pub const TEXT_SHADOW: u16 = 0x8508;
pub const CUR_ROW: u16 = 0x844B;
pub const CUR_COL: u16 = 0x844C;
pub const SCAN_ENTER: u8 = 0x09;
pub const THREE_ENTRIES: [u8; 6] = [SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER];
pub const LOWER_ROW_AND_5: [u8; 7] = [
    SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER, SCAN_5, SCAN_ENTER, SCAN_5,
];
pub const SCAN_2ND: u8 = 0x36;
pub const SCAN_MODE: u8 = 0x37;
pub const SCAN_5: u8 = 0x1B;
pub const KEY_1: u8 = 0x22;
pub const ERROR_CALL: u16 = 0xA800;
/// How far below the OS's SP at the wait the stack mask reaches: the deepest byte that differs is 4Bh below.
pub const STACK_BELOW: u16 = 0x60;
/// The OS's SP at its idle after either exit: the slots below it are dead.
pub const APP_STACK_TOP: u16 = 0xFFE5;
pub const SCAN_DOWN: u8 = 0x01;
pub const SCAN_F3: u8 = 0x33;
pub const SCAN_F1: u8 = 0x35;
pub const SCAN_F4: u8 = 0x32;
pub const POWER_DOWN: u8 = 0x10;
pub const KEY_GRAPH: u8 = 0x31;
pub const GRAPH_DRAW: u8 = 0x01;
pub const UI_FRAME: u16 = 0x9340;
pub const LIMIT: u32 = 20_000_000;

/// A build of the app, the labels it was assembled with and the screen it draws first.
pub struct Build {
    pub file: Vec<u8>,
    pub labels: PathBuf,
    pub start: Vec<bool>,
}

pub fn build(name: &str, library: Option<&[u8]>, status: u8) -> Build {
    build_staged(name, library, status, &[])
}

/// `build` with the stages `stages` as the build's stage list, kept beside the library under a name
/// of its own and handed to the build as STAGES.
pub fn build_staged(
    name: &str,
    library: Option<&[u8]>,
    status: u8,
    stages: &[mcui::Stage],
) -> Build {
    let mut args = vec![];
    if let Some(bytes) = library {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.mclib"));
        std::fs::write(&path, bytes).unwrap();
        args.push(format!("LIBRARY={}", path.display()));
        if !stages.is_empty() {
            let list = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.list"));
            let text: String = stages
                .iter()
                .map(|s| format!("{} {} {}\n", s.name, s.end, s.title))
                .collect();
            std::fs::write(&list, text).unwrap();
            args.push(format!("STAGES={}", list.display()));
        }
    }
    let dir = make_app("app", name, &args);
    Build {
        file: std::fs::read(dir.join("metacalc.8xk")).unwrap(),
        labels: dir.join("metacalc.lab"),
        start: if status == 0 {
            let shown = library.map(|bytes| Shown {
                stages: stages.to_vec(),
                ..Shown::new(name, bytes)
            });
            libraries_pixels(shown.as_ref())
        } else {
            error_pixels(status)
        },
    }
}

pub fn shell() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| build("exit-app", None, 0))
}

pub fn with_library() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| build("exit-app-lib", Some(&fixture::library()), 0))
}

/// The three stages the staged build is cut into.
pub fn staged_stages(library: &[u8]) -> Vec<mcui::Stage> {
    let count = usize::from(u16::from_le_bytes([library[14], library[15]]));
    (1..=3)
        .map(|n| mcui::Stage {
            name: format!("s{n}"),
            title: format!("Stage {n}"),
            end: n * count / 3,
        })
        .collect()
}

/// The fixture library as three stages.
pub fn staged() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| {
        let library = fixture::library();
        build_staged(
            "exit-app-staged",
            Some(&library),
            0,
            &staged_stages(&library),
        )
    })
}

pub fn refused() -> &'static Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    BUILD.get_or_init(|| {
        let bad = fixture::with(&fixture::library(), |b| b[0] ^= 0xFF);
        build("exit-app-bad", Some(&bad), 3)
    })
}

/// The bytes of the two regions that hold nothing while the shell waits at `depth`: all of both but
/// the framework's count and table, SCR_VARS offsets 0, 3 and 4, and the frames of the open screens.
pub fn dirt(depth: usize) -> Vec<(u16, Vec<u8>)> {
    let live = 16 * depth;
    vec![
        (VARS.0 + 1, varied(2, 0xC3)),
        (VARS.0 + 5, varied(VARS.1 - 5, 0xC4)),
        (STACK.0 + live as u16, varied(STACK.1 - live, 0x5A)),
    ]
}

/// What the framework holds at rest: its count and table, and the frames of the open screens.
pub fn live(os: &mut RealOs, depth: usize) -> Vec<Vec<u8>> {
    vec![
        read(os, VARS.0, 1),
        read(os, VARS.0 + 3, 2),
        read(os, STACK.0, 16 * depth),
    ]
}

pub fn read(os: &mut RealOs, addr: u16, len: usize) -> Vec<u8> {
    os.ram(usize::from(addr - RAM_BASE), len)
}

pub fn poke(os: &mut RealOs, addr: u16, bytes: &[u8]) {
    let at = usize::from(addr - RAM_BASE);
    os.m.ram_page(1)[at..at + bytes.len()].copy_from_slice(bytes);
}

pub fn dirty(os: &mut RealOs, depth: usize) {
    for (addr, bytes) in dirt(depth) {
        poke(os, addr, &bytes);
    }
}

pub fn neighbours(os: &mut RealOs) -> Vec<u8> {
    NEIGHBOURS.iter().map(|&a| read(os, a, 1)[0]).collect()
}

/// What the OS holds that an app could leave changed: cxMain to cxCurApp, iy+28h, iy+33h to iy+36h, and the hook pointers.
pub fn context(os: &mut RealOs) -> Vec<Vec<u8>> {
    vec![
        read(os, 0x858D, 14),
        read(os, 0x89F0 + 0x28, 1),
        read(os, 0x89F0 + 0x33, 4),
        read(os, 0x9B7C, 40),
    ]
}

/// The app waiting for a key on the reference OS, entered from a cleared home screen after a
/// graph was drawn, and what `context` and the text shadow held before the launch.
pub fn launched(app: &[u8], start: Option<&[bool]>) -> (RealOs, Vec<Vec<u8>>, Vec<u8>) {
    launched_after(app, start, &[])
}

/// `launched` with `keys` tapped on the cleared home screen first.
pub fn launched_after(
    app: &[u8],
    start: Option<&[bool]>,
    keys: &[u8],
) -> (RealOs, Vec<Vec<u8>>, Vec<u8>) {
    let mut os = RealOs::boot(app);
    os.tap(KEY_GRAPH, 150, 2500);
    os.tap(KEY_CLEAR, 150, 800);
    for &key in keys {
        os.tap(key, 150, 600);
    }
    assert_eq!(
        os.ram(GRAPH_FLAGS, 1)[0] & GRAPH_DRAW,
        0,
        "a valid graph to start from"
    );
    let before = context(&mut os);
    let text = read(&mut os, TEXT_SHADOW, 128);
    match start {
        Some(screen) => os.launch_in_key_wait(screen),
        None => {
            os.start_from_menu();
            os.run_ms(300);
        }
    }
    (os, before, text)
}

pub fn clear(os: &mut RealOs) {
    os.tap(KEY_CLEAR, 150, 1500);
}

/// Milliseconds to wait after 2nd: with an entry line open, or another context's buffer in use, the
/// OS redraws first and drops the next key if it comes within about a second, and an entry that
/// fills the screen takes longer. With two screens of the shell open it redraws the one below.
pub fn second_gap(os: &mut RealOs) -> u32 {
    let word = |os: &mut RealOs, at| {
        let b = read(os, at, 2);
        u32::from(u16::from_le_bytes([b[0], b[1]]))
    };
    let (top, cursor) = (word(os, EDIT_TOP), word(os, EDIT_CURSOR));
    let deep = if read(os, VARS.0, 1)[0] >= 2 { 2500 } else { 0 };
    if top == 0x9DB7 && cursor == top {
        300 + deep
    } else {
        1500 + deep + 10 * (cursor.saturating_sub(top))
    }
}

pub fn quit(os: &mut RealOs) {
    let gap = second_gap(os);
    os.tap(SCAN_2ND, 150, gap);
    os.tap(SCAN_MODE, 150, 1500);
}

pub fn off(os: &mut RealOs) {
    let gap = second_gap(os);
    os.tap(SCAN_2ND, 150, gap);
    os.m.press_key(Machine::KEY_ON);
    os.run_ms(300);
    os.m.release_key(Machine::KEY_ON);
    os.run_ms(2000);
    assert_ne!(os.m.in_port(3) & POWER_DOWN, 0, "2nd+OFF powered down");
    os.tap(Machine::KEY_ON, 150, 3000);
}

/// Runs `bcall(_ErrMemory)` where the OS waits in `_GetKey`, with the registers and the stack it
/// has there, from three bytes in free RAM, and gives the error screen time to draw.
pub fn raise_error(os: &mut RealOs) {
    let regs = os.m.regs();
    os.m.load_ram(ERROR_CALL, &[0xEF, 0xB9, 0x44]);
    os.m.set_regs(&Regs {
        pc: ERROR_CALL,
        ..regs
    });
    os.run_ms(1500);
}

pub fn lit_rows(os: &mut RealOs, rows: std::ops::Range<usize>) -> bool {
    os.screen()[rows.start * 96 * 8..rows.end * 96 * 8]
        .iter()
        .any(|&p| p)
}

/// The error screen draws its title and 1:Quit and nothing below row 2.
pub fn assert_error_screen(os: &mut RealOs, name: &str) {
    assert!(
        lit_rows(os, 0..2),
        "{name}: the error screen's title and 1:Quit"
    );
    assert!(
        !lit_rows(os, 3..8),
        "{name}: nothing below the error screen's choices"
    );
}

/// The error screen's 1:Quit.
pub fn error_quit(os: &mut RealOs) {
    raise_error(os);
    assert_error_screen(os, "1:Quit");
    os.tap(KEY_1, 150, 1500);
}

/// The error screen's 2:Goto, which only an app with a context of its own gets.
pub fn error_goto(os: &mut RealOs) {
    raise_error(os);
    assert_error_screen(os, "2:Goto");
    assert!(lit_rows(os, 2..3), "2:Goto is drawn");
    os.tap(KEY_2, 150, 1500);
}
pub type Exit = fn(&mut RealOs);

pub fn clear_twice(os: &mut RealOs) {
    clear(os);
    clear(os);
}

pub fn error_quit_then_clear(os: &mut RealOs) {
    error_quit(os);
    clear(os);
}

/// Each way out, and what a plain app's user does to end up where it does. The app leaves the home
/// screen a CLEAR key on CLEAR, 2nd+QUIT and 2:Goto, so the plain app's user presses CLEAR once more
/// after those. 1:Quit is the only choice a plain app's error screen has.
pub const EXITS: [(&str, Exit, Exit); 5] = [
    ("CLEAR", clear, clear_twice),
    ("2nd+QUIT", quit, clear_twice),
    ("2nd+OFF", off, off),
    ("error 1:Quit", error_quit, error_quit),
    ("error 2:Goto", error_goto, error_quit_then_clear),
];

/// Whether each way in `EXITS` ends at the app's own `app_exit`, which hands the OS a CLEAR key.
pub const HANDS_CLEAR: [bool; 5] = [true, true, false, false, true];

pub fn cursor(os: &mut RealOs) -> (u8, u8) {
    (read(os, CUR_ROW, 1)[0], read(os, CUR_COL, 1)[0])
}

/// Rows of text, 0 to 7, that have a lit pixel at some moment in a second and a half.
pub fn lit_text_rows(os: &mut RealOs) -> Vec<usize> {
    let mut rows = std::collections::BTreeSet::new();
    for _ in 0..30 {
        let lcd = os.screen();
        rows.extend((0..lcd.len()).filter(|&i| lcd[i]).map(|i| i / 96 / 8));
        os.run_ms(50);
    }
    rows.into_iter().collect()
}

/// What the home screen shows after ENTER: the cursor, the text shadow and the rows with ink.
pub fn after_enter(os: &mut RealOs) -> ((u8, u8), Vec<u8>, Vec<usize>) {
    os.tap(SCAN_ENTER, 150, 800);
    (cursor(os), read(os, TEXT_SHADOW, 128), lit_text_rows(os))
}

pub fn shadow_text(shadow: &[u8]) -> String {
    let text: String = shadow
        .iter()
        .map(|&b| {
            if (0x20..0x7F).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    text.trim().to_string()
}

pub fn banks(os: &mut RealOs) -> Vec<u8> {
    os.m.read_ram(0x8000, 0x8000)
}
