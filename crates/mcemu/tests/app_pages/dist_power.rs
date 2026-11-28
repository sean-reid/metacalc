//! An auto power down on the Dist screen: the enter and the draw run again on the next key and the first
//! row the frame holds is kept.

use crate::dist_lib::{self, ALL};
use crate::dist_rig::*;
use crate::shellrig::*;
use crate::theorem_rig::*;
use crate::ui_support::same_frame;
use mcui::Key;

#[test]
fn an_auto_power_down_on_the_screen_brings_back_the_frame_with_the_first_row_kept() {
    let built = dist_lib::built();
    let lib = lib_of(built);
    let rows = rows_of(&lib, index_of(&lib, ALL)).len();
    for scrolled in [0usize, 3, 9] {
        let mut walker = dist_of(built, ALL);
        for _ in 0..scrolled {
            assert!(press(&mut walker, DOWN, Key::Down));
        }
        assert_eq!(model_top(&walker), scrolled);
        let before = frame_of(&mut walker);
        assert_eq!(before[4..6], [scrolled as u8, 0], "the frame's first row");
        walker.list.run.run(600_000);
        walker.list.run.shell.power_down();
        let blits = walker.list.run.blits();
        let at = walker.list.run.shell.app.label("DV_ROWS");
        walker.list.run.m().load_ram(at, &[0xFF, 0xFF]);
        walker.tap(LETTER);
        let frame = walker.list.run.next_blit();
        walker.list.run.run(2_000);
        assert_eq!(app_rows(&mut walker), rows, "{scrolled}: the enter ran");
        assert_eq!(
            walker.list.run.blits(),
            blits + 1,
            "{scrolled}: one frame for the refresh"
        );
        walker.list.ui.press(Key::Letter('A'));
        walker.list.log.push(Key::Letter('A'));
        same_frame(&format!("after {scrolled}"), &frame, &walker.list.shown());
        assert_eq!(
            frame_of(&mut walker),
            before,
            "{scrolled}: the frame is kept"
        );
        assert_eq!(model_top(&walker), scrolled);
        let last = last_of(rows);
        if scrolled == 9 {
            for _ in 9..last {
                assert!(press(&mut walker, DOWN, Key::Down));
            }
            assert!(
                !press(&mut walker, DOWN, Key::Down),
                "Down stops at the last row"
            );
        }
    }
}
