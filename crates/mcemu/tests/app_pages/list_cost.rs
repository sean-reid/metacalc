//! What the list costs: a cursor move with the frame it redraws, a settled frame, the idle call that
//! lays out a preview, and the enter. Printed on every passing run, with a ceiling a little above each
//! figure, on the stub OS here and on the reference OS in list_real.rs.

use crate::app::*;
use crate::builds::*;
use crate::realos::BASE;
use crate::shell_cost::*;
use mcemu::Stop;

const TAP: u32 = 120_000;
/// scr_key's code for DOWN, RIGHT and ENTER.
const DOWN_CODE: u8 = 2;
const RIGHT_CODE: u8 = 4;
const SCR_REDRAW: u8 = 1;

/// What a cursor move took: list_key, then scr_draw and ui_blit for the frame it redraws.
#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub key: u32,
    pub draw: u32,
    pub blit: u32,
}

impl Move {
    pub fn total(&self) -> u32 {
        self.key + self.draw + self.blit
    }
}

pub fn tap(app: &mut App, scan: u8) {
    app.m.press_key(scan);
    assert_eq!(app.m.run_app(TAP).stop, Stop::Limit);
    app.m.release_key(scan);
    assert_eq!(app.m.run_app(4 * TAP).stop, Stop::Limit);
}

/// An app on `built` with the table `data` whose list of stage 0 is open and settled.
pub fn open_list(built: &'static Built, data: &[u8]) -> App {
    let mut app = App::boot_with(built, |m| crate::table::install(m, data));
    tap(&mut app, 0x35);
    for _ in 0..8 {
        assert_eq!(app.m.run_app(1_000_000).stop, Stop::Limit);
    }
    let preview = app.label("LV_PREVIEW");
    assert_eq!(app.m.read_ram(preview, 1), [1], "the preview is settled");
    app
}

pub fn top_frame(app: &App) -> u16 {
    app.label("SCR_STACK") + 16
}

/// The key `code` taken by list_key on the open list, and the frame it redraws.
pub fn a_move(app: &mut App, code: u8) -> Move {
    let driver = driver_cost(app);
    let (key, frame) = (app.label("list_key"), top_frame(app));
    let done = app.call(
        BASE,
        key,
        In {
            a: code,
            hl: frame,
            ..In::default()
        },
    );
    assert_eq!(done.status(), SCR_REDRAW, "the key moved the cursor");
    let (draw, blit) = frame_of(app);
    Move {
        key: done.tstates - driver,
        draw,
        blit,
    }
}

fn frame_of(app: &mut App) -> (u32, u32) {
    frame(app)
}

/// The idle call that lays out the preview of the cursor's assertion, with the wait taken away.
pub fn the_fit(app: &mut App) -> u32 {
    let driver = driver_cost(app);
    let (count, preview) = (app.label("LV_COUNT"), app.label("LV_PREVIEW"));
    assert_eq!(app.m.read_ram(preview, 1), [0], "the preview is pending");
    app.m.load_ram(count, &[0]);
    let (idle, frame) = (app.label("list_idle"), top_frame(app));
    let done = app.call(
        BASE,
        idle,
        In {
            hl: frame,
            ..In::default()
        },
    );
    assert_eq!(done.status(), SCR_REDRAW, "a preview was drawn");
    done.tstates - driver
}

/// The most T-states a cursor move of the list may take with the frame it redraws: list_key, scr_draw
/// and ui_blit, a little above the 190,019 the move by a page takes on the stub. A list row has a label
/// and a mark to draw and a Libraries row a name and a progress text, and the list's frame has six
/// rows, so it costs more than the 161,900 of cost_stages.rs.
pub const MOVE_CEILING: u32 = 190_400;

#[test]
fn a_cursor_move_and_the_frame_it_redraws_cost_what_the_ceiling_says() {
    let built = three_stages();
    let data = alternating(built);
    let mut app = open_list(built, &data);
    let mut worst = 0;
    for (name, code) in [("row", DOWN_CODE), ("page", RIGHT_CODE)] {
        let m = a_move(&mut app, code);
        println!(
            "list, cursor move by {name}: list_key {} and the redraw {} (scr_draw {}, ui_blit {}): {} T-states",
            m.key,
            m.draw + m.blit,
            m.draw,
            m.blit,
            m.total()
        );
        assert!(m.total() < FRAME_BUDGET, "{name}: over the frame budget");
        worst = worst.max(m.total());
        let fit = the_fit(&mut app);
        let (draw, blit) = frame_of(&mut app);
        println!(
            "list, preview after the move by {name}: idle {fit}, then the settled frame {} (scr_draw {draw}, ui_blit {blit})",
            draw + blit
        );
        assert!(draw + blit < FRAME_BUDGET);
    }
    assert!(worst <= MOVE_CEILING, "{worst} over {MOVE_CEILING}");
}
