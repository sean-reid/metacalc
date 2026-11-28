//! What the list costs on the stub OS: a cursor move with the frame it redraws, a settled frame and
//! the idle call that lays out a preview. Printed on every passing run, with a ceiling a little above
//! each figure. The reference OS's figures, and the cost of opening the list, are in list_real.rs and
//! list_real_cost.rs.

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
    assert_eq!(app.peek(preview, 1), [1], "the preview is settled");
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
    assert_eq!(app.peek(preview, 1), [0], "the preview is pending");
    app.poke(count, &[0]);
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
/// and ui_blit, a little above the 192,428 the move by a page takes on the stub. A list row has a label
/// and a mark to draw and a Libraries row a name and a progress text, and the list's frame has six
/// rows, so it costs more than the 163,768 of cost_stages.rs.
pub const MOVE_CEILING: u32 = 192_511;
/// The least the same move takes, 189,647 by a row: a list that drew no row or no mark would take less.
pub const MOVE_FLOOR: u32 = 187_811;
/// The most and the least the idle call that lays out a preview takes on the fixture's first two
/// previews after a move, 50,843 and 47,576.
pub const FIT_RANGE: (u32, u32) = (47_000, 51_100);
/// The most a settled frame, scr_draw and ui_blit, takes: 202,904 after the move by a row.
pub const SETTLED_CEILING: u32 = 203_039;
/// The least it takes, 201,659 after the move by a page, within 2%.
pub const SETTLED_FLOOR: u32 = 197_239;

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
        assert!(
            m.total() >= MOVE_FLOOR,
            "{name}: {} under {MOVE_FLOOR}",
            m.total()
        );
        worst = worst.max(m.total());
        let fit = the_fit(&mut app);
        let (draw, blit) = frame_of(&mut app);
        println!(
            "list, preview after the move by {name}: idle {fit}, then the settled frame {} (scr_draw {draw}, ui_blit {blit})",
            draw + blit
        );
        assert!(
            (FIT_RANGE.0..=FIT_RANGE.1).contains(&fit),
            "{name}: the fit took {fit}"
        );
        assert!(
            (SETTLED_FLOOR..=SETTLED_CEILING).contains(&(draw + blit)),
            "{name}: {}",
            draw + blit
        );
    }
    assert!(worst <= MOVE_CEILING, "{worst} over {MOVE_CEILING}");
}

/// The idle call on every assertion of fm's nodes fixture, on the stub OS at 6 MHz with interrupts off.
/// The fixture holds the slowest conclusion the model's caps allow: 253 nested operators in 255
/// tokens, whose every level takes a pass over the tokens. It is the longest a preview holds the
/// keypad, and the figure the list's header states.
#[test]
fn the_worst_preview_inside_the_caps_costs_this_and_a_key_waits_that_long() {
    let library = crate::fixture::nodes_library();
    let built = around("list-nodes", &library);
    let count = usize::from(crate::table::header_of(&library).1);
    let index = crate::list_exits::index_of(built, "deep");
    let mut app = App::boot(built);
    tap(&mut app, 0x35);
    for _ in 0..4 {
        assert_eq!(app.m.run_app(1_000_000).stop, Stop::Limit);
    }
    let (frame, driver) = (top_frame(&app), driver_cost(&mut app));
    let (count_at, preview, idle) = (
        app.label("LV_COUNT"),
        app.label("LV_PREVIEW"),
        app.label("list_idle"),
    );
    let mut costs = Vec::new();
    for n in 0..count {
        let at = (n as u16).to_le_bytes();
        app.poke(frame + 2, &[at[0], at[1], at[0], at[1]]);
        app.poke(count_at, &[0]);
        app.poke(preview, &[0]);
        let done = app.call(
            BASE,
            idle,
            In {
                hl: frame,
                ..In::default()
            },
        );
        assert_eq!(
            done.status(),
            SCR_REDRAW,
            "assertion {n}: a preview is laid out"
        );
        costs.push(done.tstates - driver);
    }
    let worst = *costs.iter().max().unwrap();
    let at = costs.iter().position(|&c| c == worst).unwrap();
    println!(
        "stub, the idle call over the nodes fixture's {count} assertions: worst {worst} T-states, {:.2} s at 6 MHz, assertion {at}",
        f64::from(worst) / 6_000_000.0
    );
    assert_eq!(at, index, "the slowest is the 253 nested operators");
    let mut sorted = costs.clone();
    sorted.sort_unstable();
    println!(
        "the next three: {:?}",
        &sorted[sorted.len() - 4..sorted.len() - 1]
    );
    assert!(
        sorted[sorted.len() - 2] < worst / 3,
        "no other conclusion is within a third of it"
    );
    crate::list_rig::within("the worst preview", worst, WORST_PREVIEW);
}

/// What it took when measured.
pub const WORST_PREVIEW: u32 = 37_707_410;
