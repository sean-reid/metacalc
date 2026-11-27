//! An auto power down on the sections screen: the OS copies the screen's pixels over saveSScreen, which
//! holds the screen's derived state, so the next key makes the shell enter the screen again. The frame
//! after it is the model's, and the cursor and the first title are in the frame and stay where they were.

use crate::list_rig::*;
use crate::sections_depth::{ROM, STUB, Site, Table};
use crate::sections_rig::*;
use crate::shellrig::*;
use crate::ui_support::same_frame;
use mcui::{Key, Kind};

/// The screen opened, its cursor taken to a title and then up three rows inside the view, so that the
/// first title shown is not the one F5 gives (the cursor less five).
fn placed(site: &Site) -> (Lister, usize, usize) {
    let mut list = (site.list)(Kind::All, Table::None);
    list.open_sections();
    list.goto_title(25);
    for _ in 0..3 {
        assert!(list.sections_key(UP, Key::Up));
    }
    list.run.run(600_000);
    let at = list.sections();
    assert_eq!(
        (at.cursor, at.top),
        (22, 20),
        "inside the view, off F5's top"
    );
    assert_ne!(
        at.top,
        at.cursor.saturating_sub(5),
        "not the first row F5 gives"
    );
    (list, at.cursor, at.top)
}

/// The variables the screen derives, from SX_N to SX_END.
fn variables(list: &mut Lister) -> Vec<u8> {
    let app = &list.run.shell.app;
    let (first, end) = (app.label("SX_N"), app.label("SX_END"));
    list.run.peek(first, usize::from(end - first))
}

fn after_a_power_down(site: &Site, scan: u8, key: Key) {
    let (mut list, cursor, top) = placed(site);
    let (frame, vars) = (
        list.run.frames().last().cloned().unwrap(),
        variables(&mut list),
    );
    let len = list.sections().len;
    assert!(len > 6, "{} titles", len);
    list.run.shell.power_down();
    assert_ne!(
        variables(&mut list),
        vars,
        "the power down wrote over the derived state"
    );
    let blits = list.run.blits();
    list.tap(scan);
    list.ui.press(key);
    let drawn = list.run.next_blit();
    list.run.run(2_000);
    assert_eq!(list.run.blits(), blits + 1, "one frame for the refresh");
    assert_eq!(list.run.top_kind(), KIND_SECTIONS);
    let at = list.sections();
    same_frame(
        &format!("after the power down and key {scan:02X}"),
        &drawn,
        &model::frame(&list.ui),
    );
    let now = list.run.frames().last().cloned().unwrap();
    if matches!(key, Key::Letter(_)) {
        assert_eq!(
            now, frame,
            "a key the screen ignores leaves the frame as it was"
        );
        assert_eq!((at.cursor, at.top), (cursor, top));
        assert_eq!(
            variables(&mut list)[..2],
            vars[..2],
            "the enter builds the count again"
        );
    } else {
        assert_eq!(
            (usize::from(now[2]), usize::from(now[4])),
            (at.cursor, at.top),
            "the move is from where the cursor was, not from F5's"
        );
        assert_eq!(at.cursor, cursor + 1, "one title on");
        assert_eq!(at.top, top, "the first title stays");
    }
    assert_eq!(list.sections().len, len);
    let n = list.run.shell.app.label("SX_N");
    let b = list.run.peek(n, 2);
    assert_eq!(
        usize::from(u16::from_le_bytes([b[0], b[1]])),
        len,
        "the count is derived again"
    );
}

#[test]
fn a_power_down_on_the_sections_screen_is_refreshed_with_the_cursor_and_first_title_where_they_were()
 {
    after_a_power_down(&STUB, LETTER, Key::Letter('A'));
}

#[test]
fn a_key_that_moves_after_a_power_down_moves_from_the_cursor_the_frame_holds() {
    after_a_power_down(&STUB, DOWN, Key::Down);
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_power_down_on_the_sections_screen_is_refreshed_with_the_position_kept() {
    after_a_power_down(&ROM, LETTER, Key::Letter('A'));
    after_a_power_down(&ROM, DOWN, Key::Down);
}
