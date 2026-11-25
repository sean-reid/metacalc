//! The builds the Libraries screen is run on, and what the model says about their keys.

use crate::app::*;
use crate::shellrig::*;
use mcui::{Key, State};
use std::sync::OnceLock;

/// The ends of the fixture library cut at `parts` stages of equal size, the last being its count.
fn ends(library: &[u8], parts: usize) -> Vec<usize> {
    let count = crate::table::header_of(library).1 as usize;
    (1..=parts).map(|n| n * count / parts).collect()
}

/// The fixture library cut at `parts` ends of equal size, the last being its count.
fn cut(name: &str, parts: usize) -> &'static Built {
    let library = crate::fixture::library();
    staged(name, &library, &ends(&library, parts))
}

/// `three_stages` built from the tree whose `app` directory is `source`.
pub fn three_stages_from(source: &std::path::Path, name: &str) -> &'static Built {
    let library = crate::fixture::library();
    staged_from(source, name, &library, &ends(&library, 3))
}

/// The fixture library as three stages.
pub fn three_stages() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| cut("pages-three", 3))
}

/// The fixture library as six stages, the most a build takes: seven rows, more than the six the
/// screen shows.
pub fn six_stages() -> &'static Built {
    static BUILT: OnceLock<&'static Built> = OnceLock::new();
    BUILT.get_or_init(|| cut("pages-six", 6))
}

/// The builds the stub runs: no library, the fixture library as one stage, as three and as six.
pub fn builds() -> [&'static Built; 4] {
    [plain(), with_library(), three_stages(), six_stages()]
}

/// The rows the Libraries screen of `built` has: a row for each stage and My proofs.
pub fn rows(built: &Built) -> usize {
    built.stages.len().max(usize::from(built.library.is_some())) + 1
}

/// The kind of screen each of the model's states is in the app's frames. The theorem list does not
/// exist yet: the app shows a notice in its place.
pub fn kind_of(state: &State) -> u8 {
    match state {
        State::Libraries { .. } => KIND_LIBRARIES,
        State::Notice { .. } | State::List { .. } => KIND_NOTICE,
        State::Checking { .. } => KIND_CHECKING,
        State::About => KIND_ABOUT,
        State::StageAbout { .. } => KIND_STAGE,
        other => panic!("the model is on {other:?}"),
    }
}

/// The model's key for the scancode of a key the app takes.
pub fn key_of(scan: u8) -> Key {
    match scan {
        F1 => Key::F1,
        F2 => Key::F2,
        F3 => Key::F3,
        F4 => Key::F4,
        F5 => Key::F5,
        ENTER => Key::Enter,
        CLEAR => Key::Clear,
        DOWN => Key::Down,
        UP => Key::Up,
        LEFT => Key::Left,
        RIGHT => Key::Right,
        _ => panic!("no key for scancode {scan:02X}"),
    }
}

/// A shell booted on `built` with a table of the library in RAM when `table` is given.
pub fn booted(built: &'static Built, table: Option<&[u8]>) -> Shell {
    let mut shell = Shell::boot_with(built, Enter::Real, |m| {
        if let Some(data) = table {
            crate::table::install(m, data);
        }
    });
    assert_eq!((shell.depth(), shell.blits()), (1, 1));
    shell
}

/// Presses `scan` on the shell and the model together; see `press_keys`.
pub fn press(shell: &mut Shell, ui: &mut mcui::Ui, built: &Built, scan: u8, at: &str) -> bool {
    press_keys(shell, ui, built, &[scan], Some(key_of(scan)), at)
}

/// The frame the app shows for what the model shows: the same, except that for the theorem list,
/// which the app does not have, it is the notice that stands in for it.
fn shown(ui: &mcui::Ui, built: &Built) -> Vec<u8> {
    match ui.state() {
        State::List { stage, .. } => model::unavailable(built.stage_name(stage)),
        _ => model::frame(ui),
    }
}

/// Presses the scancodes in turn on the shell, the last being the key itself and the others
/// modifiers that give no key, and `key` on the model, which has none when the scancodes make no
/// key, and checks what it did: the frame, the depth, the kind of the top screen and whether a
/// frame was blitted, which it is exactly when the screen changed. On the Libraries screen the
/// frame holds the cursor and the first row shown the model has. Returns whether the model is on
/// a screen the app stands in for, which is the theorem list.
pub fn press_keys(
    shell: &mut Shell,
    ui: &mut mcui::Ui,
    built: &Built,
    scans: &[u8],
    key: Option<Key>,
    at: &str,
) -> bool {
    let (before, depth, blits) = (shown(ui, built), ui.depth(), shell.blits());
    for &scan in &scans[..scans.len() - 1] {
        shell.tap_modifier(scan);
    }
    shell.tap(*scans.last().unwrap());
    if let Some(key) = key {
        ui.press(key);
    }
    let want = shown(ui, built);
    let at = format!("{at}, after {scans:02X?}");
    shell.shows(&at, &want);
    assert_eq!(shell.depth(), ui.depth(), "{at}: the depth");
    assert_eq!(shell.top_kind(), kind_of(&ui.state()), "{at}: the screen");
    let changed = want != before || ui.depth() != depth;
    assert_eq!(shell.blits(), blits + u16::from(changed), "{at}: the blits");
    assert_eq!(shell.lcd_bytes(), want, "{at}: the LCD");
    if let State::Libraries { cursor, top } = ui.state() {
        assert_eq!(
            shell.frames()[0][..3],
            [KIND_LIBRARIES, cursor as u8, top as u8],
            "{at}: the position in the frame"
        );
    }
    matches!(ui.state(), State::List { .. })
}
