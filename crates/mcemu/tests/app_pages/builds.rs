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

/// The kind of screen each of the model's states is in the app's frames.
pub fn kind_of(state: &State) -> u8 {
    match state {
        State::Libraries { .. } => KIND_LIBRARIES,
        State::Notice { .. } => KIND_NOTICE,
        State::List { .. } => KIND_LIST,
        State::Checking { .. } => KIND_CHECKING,
        State::About => KIND_ABOUT,
        State::StageAbout { .. } => KIND_STAGE,
        State::Theorem { .. } => crate::theorem_rig::KIND_THEOREM,
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

/// Runs the app until the preview of the open list is settled and the frame it asks for is blitted,
/// and does the same idle work on the model. Nothing happens when no list is open or the preview is
/// settled already.
pub fn settle_list(shell: &mut Shell, ui: &mut mcui::Ui) {
    if shell.top_kind() != KIND_LIST || !matches!(ui.state(), State::List { .. }) {
        return;
    }
    let (blits, pending) = (shell.blits(), list_preview(shell) == 0);
    let mut spent = 0u32;
    while list_preview(shell) == 0 {
        assert_eq!(shell.app.m.run_app(20_000).stop, mcemu::Stop::Limit);
        spent += 20_000;
        assert!(spent < 60_000_000, "the preview never settled");
    }
    let drew = ui.idle();
    let want = blits + u16::from(pending && drew);
    let mut waited = 0u32;
    while shell.blits() < want && waited < 2_000_000 {
        assert_eq!(shell.app.m.run_app(20_000).stop, mcemu::Stop::Limit);
        waited += 20_000;
    }
    assert_eq!(shell.app.m.run_app(300_000).stop, mcemu::Stop::Limit);
    assert_eq!(shell.blits(), want, "the frame of the preview");
}

/// The list screen's preview state in RAM: 0 pending, 1 laid out, 2 nothing to show.
pub fn list_preview(shell: &mut Shell) -> u8 {
    shell.at_rest();
    let at = shell.app.label("LV_PREVIEW");
    shell.app.m.read_ram(at, 1)[0]
}

/// Presses `scan` on a stage row, which opens its list in the app and the model, and runs the
/// list to its settled frame.
pub fn open_list(shell: &mut Shell, ui: &mut mcui::Ui, built: &Built, scan: u8, at: &str) {
    press(shell, ui, built, scan, at);
    assert_eq!(shell.top_kind(), KIND_LIST, "{at}: the list");
    settle_list(shell, ui);
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
    _built: &Built,
    scans: &[u8],
    key: Option<Key>,
    at: &str,
) -> bool {
    settle_list(shell, ui);
    let (before, depth, blits) = (model::frame(ui), ui.depth(), shell.blits());
    for &scan in &scans[..scans.len() - 1] {
        shell.tap_modifier(scan);
    }
    shell.tap(*scans.last().unwrap());
    if let Some(key) = key {
        ui.press(key);
    }
    let pressed = model::frame(ui);
    let changed = pressed != before || ui.depth() != depth;
    let mut idled = false;
    if matches!(ui.state(), State::List { .. }) && list_preview(shell) != 0 {
        idled = ui.idle();
    }
    let want = model::frame(ui);
    let at = format!("{at}, after {scans:02X?}");
    shell.shows(&at, &want);
    assert_eq!(shell.depth(), ui.depth(), "{at}: the depth");
    assert_eq!(shell.top_kind(), kind_of(&ui.state()), "{at}: the screen");
    assert_eq!(
        shell.blits(),
        blits + u16::from(changed) + u16::from(idled),
        "{at}: the blits"
    );
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
