//! The Dist screen on the stub OS, opened with F2 from the theorem screen and driven against the host
//! model: the pairs the model shows, how mcglyph's place wraps them, and the app's own rows.

use crate::app::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use mcglyph::{Font, Item, Pad, place};
use mcui::canvas::{CONTENT_ROWS, TEXT_WIDTH};
use mcui::data::Lib;
use mcui::{Key, State};

pub const KIND_DIST: u8 = 9;

/// The model's pairs of assertion `i`, each as the width the model sets it at.
pub fn widths(lib: &Lib, i: usize) -> Vec<usize> {
    let font = Font::builtin();
    let ink = |t: &str| font.token_bitmap(t).unwrap_or_else(|| font.text(t)).width();
    let comma = font.text(",").width();
    lib.dist(i)
        .iter()
        .map(|(a, b)| ink(a) + comma + ink(b) + 2)
        .collect()
}

/// The pairs on each row of assertion `i`, as the model's place wraps them at the width of the screen.
pub fn rows_of(lib: &Lib, i: usize) -> Vec<Vec<usize>> {
    wrap(&widths(lib, i))
}

/// `widths` placed by mcglyph as the model places them: each row's pair widths.
pub fn wrap(widths: &[usize]) -> Vec<Vec<usize>> {
    let items: Vec<Item> = widths
        .iter()
        .map(|&width| Item {
            width,
            pad: Pad {
                before: true,
                ..Pad::default()
            },
        })
        .collect();
    place(&items, TEXT_WIDTH)
        .iter()
        .map(|line| line.tokens.iter().map(|t| t.width).collect())
        .collect()
}

/// The first row the model shows no further than: its `last`.
pub fn last_of(rows: usize) -> usize {
    rows.saturating_sub(CONTENT_ROWS)
}

/// The Dist screen's frame bytes: kind, stage, assertion, first row.
pub fn frame_of(walker: &mut Walker) -> [u8; 6] {
    let frames = walker.list.run.frames();
    frames.last().unwrap()[..6].try_into().unwrap()
}

/// F2 on the open theorem screen or its too large screen: the app draws the model's Dist screen.
pub fn open_dist(walker: &mut Walker) {
    let kind = walker.list.run.top_kind();
    assert_eq!(kind, KIND_THEOREM, "F2 is pressed on a theorem screen");
    let stage = walker.stage as u8;
    let n = walker.var16("TV_ASSERT");
    let text = walker.list.run.shell.app.label("APP_TEXT");
    let size = usize::from(walker.list.run.shell.app.label("APP_TEXT_SIZE"));
    walker.list.run.m().load_ram(text, &vec![0xA5; size]);
    assert!(walker.key(F2, Key::F2), "F2 draws the Dist screen");
    assert_eq!(walker.list.run.top_kind(), KIND_DIST, "the kind");
    let frame = frame_of(walker);
    assert_eq!(frame[0], KIND_DIST);
    assert_eq!(frame[1], stage, "the stage");
    assert_eq!(usize::from(u16::from_le_bytes([frame[2], frame[3]])), n);
    assert_eq!(frame[4..6], [0, 0], "the first row");
    assert!(matches!(walker.list.ui.state(), State::Dist { top: 0, .. }));
    walker
        .list
        .run
        .shell
        .assert_lcd_holds_the_frame("dist opened");
}

/// CLEAR on the Dist screen: the theorem screen comes back and is the model's refit.
pub fn leave(walker: &mut Walker) {
    let depth = walker.list.run.depth();
    assert!(walker.key(CLEAR, Key::Clear), "the screen comes back");
    assert_eq!(walker.list.run.top_kind(), KIND_THEOREM);
    assert_eq!(walker.list.run.depth(), depth - 1);
    walker.mirror = None;
    walker
        .list
        .run
        .shell
        .assert_lcd_holds_the_frame("theorem after dist");
}

/// The model's top row of the open Dist screen.
pub fn model_top(walker: &Walker) -> usize {
    match walker.list.ui.state() {
        State::Dist { top, .. } => top,
        other => panic!("the model is on {other:?}"),
    }
}

/// The rows the app counted when it entered the screen.
pub fn app_rows(walker: &mut Walker) -> usize {
    walker.var16("DV_ROWS")
}

/// The number of the assertion `label` in `lib`.
pub fn index_of(lib: &Lib, label: &str) -> usize {
    (0..lib.len())
        .find(|&i| lib.label(i) == label)
        .unwrap_or_else(|| panic!("the library has no assertion {label}"))
}

/// The theorem screen of `label`, with F2 pressed: the Dist screen.
pub fn dist_of(built: &'static Built, label: &str) -> Walker {
    let n = index_of(&lib_of(built), label);
    let mut walker = Walker::at(built, None, 0, n);
    walker.mirrored = false;
    open_dist(&mut walker);
    walker
}

/// The app's scratch the Dist screen keeps its variables in.
pub fn scratch(walker: &mut Walker) -> Vec<u8> {
    let app = &walker.list.run.shell.app;
    let (at, size) = (
        app.label("APP_TEXT"),
        usize::from(app.label("APP_TEXT_SIZE")),
    );
    walker.list.run.peek(at, size)
}

/// A key on the Dist screen: the model's frame is the app's, and the answer is the model's.
/// Returns whether the screen was redrawn.
pub fn press(walker: &mut Walker, scan: u8, key: Key) -> bool {
    let before = walker.list.run.frames();
    let kept = scratch(walker);
    let picture = walker.list.run.frame();
    let drew = walker.key(scan, key);
    if !drew {
        assert_eq!(walker.list.run.frames(), before, "{scan:02X}: the frames");
        assert_eq!(scratch(walker), kept, "{scan:02X}: the scratch");
        assert_eq!(walker.list.run.frame(), picture, "{scan:02X}: the picture");
    }
    walker
        .list
        .run
        .shell
        .assert_lcd_holds_the_frame(&format!("dist after {scan:02X}"));
    drew
}
